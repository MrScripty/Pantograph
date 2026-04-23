use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{NodeEngineError, Result};
use crate::events::EventSink;
use crate::extensions::ExecutorExtensions;
use crate::model_dependencies::ModelRefV2;

use super::{build_extra_settings, build_model_ref_v2, infer_task_type_primary, kv_cache};

// ---------------------------------------------------------------------------
// PyTorch handlers (behind pytorch-nodes feature)
// ---------------------------------------------------------------------------

/// Ensure the PyTorch worker module (and its sibling modules) are loaded into
/// the Python interpreter.  Safe to call multiple times -- only the first call
/// actually loads.
pub(crate) fn ensure_torch_worker_initialised(
    py: pyo3::Python<'_>,
) -> std::result::Result<(), String> {
    if py.import("pantograph_torch_worker").is_ok() {
        return Ok(());
    }

    use pyo3::types::PyAnyMethods;

    let sys = py
        .import("sys")
        .map_err(|e| format!("Failed to import sys: {}", e))?;
    let modules = sys
        .getattr("modules")
        .map_err(|e| format!("Failed to get sys.modules: {}", e))?;

    // Register sibling modules first so worker.py's imports resolve.
    for (name, source, file_name, module_name) in [
        (
            "block_diffusion",
            include_str!("../../../inference/torch/block_diffusion.py"),
            c"block_diffusion.py",
            c"block_diffusion",
        ),
        (
            "autoregressive",
            include_str!("../../../inference/torch/autoregressive.py"),
            c"autoregressive.py",
            c"autoregressive",
        ),
        (
            "worker_runtime",
            include_str!("../../../inference/torch/worker_runtime.py"),
            c"worker_runtime.py",
            c"worker_runtime",
        ),
        (
            "worker_transformers",
            include_str!("../../../inference/torch/worker_transformers.py"),
            c"worker_transformers.py",
            c"worker_transformers",
        ),
    ] {
        let module_code = std::ffi::CString::new(source)
            .map_err(|e| format!("Invalid {} source: {}", name, e))?;
        let module = pyo3::types::PyModule::from_code(py, &module_code, file_name, module_name)
            .map_err(|e| format!("Failed to load {}: {}", name, e))?;
        modules
            .set_item(name, &module)
            .map_err(|e| format!("Failed to register {}: {}", name, e))?;
    }

    // Now load the worker module.
    let code = std::ffi::CString::new(include_str!("../../../inference/torch/worker.py"))
        .map_err(|e| format!("Invalid worker source: {}", e))?;
    pyo3::types::PyModule::from_code(
        py,
        &code,
        c"pantograph_torch_worker",
        c"pantograph_torch_worker",
    )
    .map_err(|e| format!("Failed to load worker: {}", e))?;

    log::info!("PyTorch worker module initialised with embedded sibling modules");
    Ok(())
}

pub(crate) async fn execute_pytorch_inference(
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
    resolved_model_ref: Option<ModelRefV2>,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    // Detect if the prompt input is a masked prompt JSON object
    let masked_prompt_json = inputs
        .get("prompt")
        .filter(|p| p.get("type").and_then(|t| t.as_str()) == Some("masked_prompt"))
        .map(|p| serde_json::to_string(p).unwrap_or_default());

    let prompt = if let Some(p_str) = inputs.get("prompt").and_then(|p| p.as_str()) {
        p_str.to_string()
    } else if let Some(p_obj) = inputs.get("prompt") {
        // For masked prompt objects, concatenate all segment texts as the plain prompt
        if let Some(segments) = p_obj.get("segments").and_then(|s| s.as_array()) {
            segments
                .iter()
                .filter_map(|seg| seg.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        } else {
            return Err(NodeEngineError::ExecutionFailed(
                "Missing prompt input: not a string or masked prompt".to_string(),
            ));
        }
    } else {
        return Err(NodeEngineError::ExecutionFailed(
            "Missing prompt input".to_string(),
        ));
    };

    let model_path = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?
        .to_string();

    let system_prompt = inputs
        .get("system_prompt")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let temperature = inputs
        .get("temperature")
        .and_then(|t| t.as_f64())
        .unwrap_or(0.7);
    let max_tokens = inputs
        .get("max_tokens")
        .and_then(|m| m.as_i64())
        .unwrap_or(512);
    let device = inputs
        .get("device")
        .and_then(|d| d.as_str())
        .unwrap_or("auto")
        .to_string();
    let model_type = inputs
        .get("model_type")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());

    let model_name = std::path::Path::new(&model_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("pytorch-model")
        .to_string();

    // Phase 1: Check if model is already loaded, load if needed
    {
        let mp = model_path.clone();
        let dev = device.clone();
        let mt = model_type.clone();

        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| -> std::result::Result<(), String> {
                use pyo3::types::{PyAnyMethods, PyDictMethods};

                // Ensure worker + sibling modules are initialised
                ensure_torch_worker_initialised(py)?;
                let worker = py
                    .import("pantograph_torch_worker")
                    .map_err(|e| format!("Failed to import worker: {}", e))?;

                // Check if the correct model is already loaded
                let info = worker
                    .call_method0("get_loaded_info")
                    .map_err(|e| format!("get_loaded_info failed: {}", e))?;

                let needs_load = if info.is_none() {
                    true
                } else {
                    let loaded_path: String = info
                        .get_item("model_path")
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                        .unwrap_or_default();
                    loaded_path != mp
                };

                if needs_load {
                    log::info!("PyTorchInference: loading model from '{}'", mp);
                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("model_path", &mp).unwrap();
                    kwargs.set_item("device", &dev).unwrap();
                    if let Some(ref mt_val) = mt {
                        kwargs.set_item("model_type", mt_val).unwrap();
                    }
                    worker
                        .call_method("load_model", (), Some(&kwargs))
                        .map_err(|e| format!("Model load failed: {}", e))?;
                    log::info!("PyTorchInference: model loaded successfully");
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(NodeEngineError::ExecutionFailed)?;
    }

    let _restored_kv_cache = kv_cache::restore_pytorch_input_handle(
        inputs,
        extensions,
        task_id,
        execution_id,
        event_sink,
    )
    .await?;

    // Read model-specific inference settings to forward as Python kwargs
    let extra_settings = build_extra_settings(inputs);
    // Keep top_p explicit even when inference_settings schema is missing.
    let top_p = inputs
        .get("top_p")
        .and_then(|v| v.as_f64())
        .or_else(|| extra_settings.get("top_p").and_then(|v| v.as_f64()))
        .unwrap_or(0.95);

    // Phase 2: Generate — streaming or non-streaming
    let response_text = if let Some(sink) = event_sink {
        // Streaming: iterate Python generator via mpsc channel
        // Channel carries (mode, text) tuples: mode is "append" or "replace"
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<std::result::Result<(String, String), String>>(32);
        let p = prompt.clone();
        let sp = system_prompt.clone();
        let mpj = masked_prompt_json.clone();
        let extra = extra_settings.clone();

        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| {
                use pyo3::types::{PyAnyMethods, PyDictMethods, PyTypeMethods};

                if let Err(e) = ensure_torch_worker_initialised(py) {
                    let _ = tx.blocking_send(Err(e));
                    return;
                }
                let worker = match py.import("pantograph_torch_worker") {
                    Ok(w) => w,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(format!("Failed to get worker: {}", e)));
                        return;
                    }
                };

                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("prompt", &p).unwrap();
                if let Some(ref sys) = sp {
                    kwargs.set_item("system_prompt", sys).unwrap();
                }
                kwargs.set_item("max_tokens", max_tokens).unwrap();
                kwargs.set_item("temperature", temperature).unwrap();
                kwargs.set_item("top_p", top_p).unwrap();
                if let Some(ref mpj_val) = mpj {
                    kwargs.set_item("masked_prompt_json", mpj_val).unwrap();
                }

                // Forward model-specific inference settings as kwargs
                for (key, value) in &extra {
                    if let Some(n) = value.as_i64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(n) = value.as_f64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(s) = value.as_str() {
                        kwargs.set_item(key.as_str(), s).unwrap();
                    } else if let Some(b) = value.as_bool() {
                        kwargs.set_item(key.as_str(), b).unwrap();
                    }
                }

                let generator = match worker.call_method("generate_tokens", (), Some(&kwargs)) {
                    Ok(g) => g,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(format!("Failed to create generator: {}", e)));
                        return;
                    }
                };

                let iter = match generator.try_iter() {
                    Ok(it) => it,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(format!("Generator not iterable: {}", e)));
                        return;
                    }
                };

                for item in iter {
                    match item {
                        Ok(token_obj) => {
                            // Try dict first: {"mode": "append"|"replace", "text": "..."}
                            let result = if let Ok(dict) =
                                token_obj.downcast::<pyo3::types::PyDict>()
                            {
                                let mode = dict
                                    .get_item("mode")
                                    .ok()
                                    .flatten()
                                    .and_then(|v| v.extract::<String>().ok())
                                    .unwrap_or_else(|| "append".to_string());
                                let text = dict
                                    .get_item("text")
                                    .ok()
                                    .flatten()
                                    .and_then(|v| v.extract::<String>().ok())
                                    .unwrap_or_default();
                                Ok((mode, text))
                            } else if let Ok(text) = token_obj.extract::<String>() {
                                // Backwards compat: plain string → append
                                Ok(("append".to_string(), text))
                            } else {
                                Err(format!(
                                    "Token extraction failed: expected dict or string, got {:?}",
                                    token_obj.get_type().name()
                                ))
                            };
                            if tx.blocking_send(result).is_err() {
                                return;
                            }
                        }
                        Err(e) => {
                            let _ = tx.blocking_send(Err(format!("Generator error: {}", e)));
                            return;
                        }
                    }
                }
            });
        });

        let mut full_response = String::new();
        while let Some(token_result) = rx.recv().await {
            let (mode, text) = token_result.map_err(|e| {
                NodeEngineError::ExecutionFailed(format!("PyTorch generation error: {}", e))
            })?;
            if mode == "replace" {
                full_response = text.clone();
            } else {
                full_response.push_str(&text);
            }
            let _ = sink.send(crate::WorkflowEvent::task_stream(
                task_id,
                execution_id,
                "stream",
                serde_json::json!({"mode": mode, "text": text}),
            ));
        }

        full_response
    } else {
        // Non-streaming: single blocking call
        let p = prompt.clone();
        let sp = system_prompt.clone();
        let mpj = masked_prompt_json.clone();
        let extra = extra_settings;

        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| -> std::result::Result<String, String> {
                use pyo3::types::{PyAnyMethods, PyDictMethods};

                ensure_torch_worker_initialised(py)?;
                let worker = py
                    .import("pantograph_torch_worker")
                    .map_err(|e| format!("Failed to get worker: {}", e))?;

                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("prompt", &p).unwrap();
                if let Some(ref sys) = sp {
                    kwargs.set_item("system_prompt", sys).unwrap();
                }
                kwargs.set_item("max_tokens", max_tokens).unwrap();
                kwargs.set_item("temperature", temperature).unwrap();
                kwargs.set_item("top_p", top_p).unwrap();
                if let Some(ref mpj_val) = mpj {
                    kwargs.set_item("masked_prompt_json", mpj_val).unwrap();
                }

                // Forward model-specific inference settings as kwargs
                for (key, value) in &extra {
                    if let Some(n) = value.as_i64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(n) = value.as_f64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(s) = value.as_str() {
                        kwargs.set_item(key.as_str(), s).unwrap();
                    } else if let Some(b) = value.as_bool() {
                        kwargs.set_item(key.as_str(), b).unwrap();
                    }
                }

                let result = worker
                    .call_method("generate", (), Some(&kwargs))
                    .map_err(|e| format!("Generation failed: {}", e))?;

                result
                    .extract::<String>()
                    .map_err(|e| format!("Failed to extract result: {}", e))
            })
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(NodeEngineError::ExecutionFailed)?
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response_text));
    let task_type_primary = infer_task_type_primary("pytorch-inference", inputs);
    let model_ref = build_model_ref_v2(
        resolved_model_ref,
        "pytorch",
        &model_name,
        &model_path,
        &task_type_primary,
        inputs,
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::to_value(model_ref).unwrap_or_else(|_| {
            serde_json::json!({
                "contractVersion": 2,
                "engine": "pytorch",
                "modelId": model_name,
                "modelPath": model_path,
                "taskTypePrimary": task_type_primary,
            })
        }),
    );
    let kv_cache_output = match kv_cache::capture_pytorch_output_handle(
        task_id,
        execution_id,
        extensions,
        event_sink,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "PyTorchInference: failed to capture KV cache output for '{}': {}",
                task_id,
                error
            );
            serde_json::Value::Null
        }
    };
    outputs.insert("kv_cache_out".to_string(), kv_cache_output);
    Ok(outputs)
}

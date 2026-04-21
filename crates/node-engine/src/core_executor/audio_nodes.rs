use std::collections::HashMap;

use crate::error::{NodeEngineError, Result};
use crate::model_dependencies::ModelRefV2;

use super::build_model_ref_v2;

// ---------------------------------------------------------------------------
// Audio generation handler (behind audio-nodes feature)
// ---------------------------------------------------------------------------

/// Ensure the Stable Audio worker module (and its sibling) are loaded into
/// the Python interpreter.  Safe to call multiple times -- only the first call
/// actually loads.
fn ensure_audio_worker_initialised(py: pyo3::Python<'_>) -> std::result::Result<(), String> {
    if py.import("pantograph_audio_worker").is_ok() {
        return Ok(());
    }

    use pyo3::types::PyAnyMethods;

    let sys = py
        .import("sys")
        .map_err(|e| format!("Failed to import sys: {}", e))?;
    let modules = sys
        .getattr("modules")
        .map_err(|e| format!("Failed to get sys.modules: {}", e))?;

    // Register sibling module first so worker.py's imports resolve
    let sa_code = std::ffi::CString::new(include_str!("../../../inference/audio/stable_audio.py"))
        .map_err(|e| format!("Invalid stable_audio source: {}", e))?;
    let sa_module =
        pyo3::types::PyModule::from_code(py, &sa_code, c"stable_audio.py", c"stable_audio")
            .map_err(|e| format!("Failed to load stable_audio: {}", e))?;
    modules
        .set_item("stable_audio", &sa_module)
        .map_err(|e| format!("Failed to register stable_audio: {}", e))?;

    // Now load the worker module (which imports from stable_audio)
    let code = std::ffi::CString::new(include_str!("../../../inference/audio/worker.py"))
        .map_err(|e| format!("Invalid audio worker source: {}", e))?;
    pyo3::types::PyModule::from_code(
        py,
        &code,
        c"pantograph_audio_worker",
        c"pantograph_audio_worker",
    )
    .map_err(|e| format!("Failed to load audio worker: {}", e))?;

    log::info!("Audio worker module initialised (with stable_audio sibling)");
    Ok(())
}

pub(crate) async fn execute_audio_generation(
    inputs: &HashMap<String, serde_json::Value>,
    resolved_model_ref: Option<ModelRefV2>,
) -> Result<HashMap<String, serde_json::Value>> {
    let model_path = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?
        .to_string();

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?
        .to_string();

    let duration = inputs
        .get("duration")
        .and_then(|d| d.as_f64())
        .unwrap_or(30.0);
    let steps = inputs
        .get("num_inference_steps")
        .and_then(|s| s.as_i64())
        .unwrap_or(100);
    let guidance_scale = inputs
        .get("guidance_scale")
        .and_then(|g| g.as_f64())
        .unwrap_or(7.0);
    let seed = inputs.get("seed").and_then(|s| s.as_i64()).unwrap_or(-1);

    // Phase 1: Load model if needed
    {
        let mp = model_path.clone();
        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| -> std::result::Result<(), String> {
                use pyo3::types::{PyAnyMethods, PyDictMethods};

                ensure_audio_worker_initialised(py)?;
                let worker = py
                    .import("pantograph_audio_worker")
                    .map_err(|e| format!("Failed to import audio worker: {}", e))?;

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
                    log::info!("AudioGeneration: loading model from '{}'", mp);
                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("model_path", &mp).unwrap();
                    kwargs.set_item("device", "auto").unwrap();
                    worker
                        .call_method("load_model", (), Some(&kwargs))
                        .map_err(|e| format!("Audio model load failed: {}", e))?;
                    log::info!("AudioGeneration: model loaded successfully");
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(NodeEngineError::ExecutionFailed)?;
    }

    // Phase 2: Generate audio
    let mut result = {
        let p = prompt;
        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(
                |py| -> std::result::Result<HashMap<String, serde_json::Value>, String> {
                    use pyo3::types::PyAnyMethods;

                    let worker = py
                        .import("pantograph_audio_worker")
                        .map_err(|e| format!("Failed to get audio worker: {}", e))?;

                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("prompt", &p).unwrap();
                    kwargs.set_item("duration", duration).unwrap();
                    kwargs.set_item("steps", steps).unwrap();
                    kwargs.set_item("guidance_scale", guidance_scale).unwrap();
                    kwargs.set_item("seed", seed).unwrap();

                    let result = worker
                        .call_method("generate_audio_from_text", (), Some(&kwargs))
                        .map_err(|e| format!("Audio generation failed: {}", e))?;

                    // Extract dict fields
                    let audio_base64: String = result
                        .get_item("audio_base64")
                        .ok()
                        .and_then(|v| v.extract().ok())
                        .unwrap_or_default();
                    let duration_seconds: f64 = result
                        .get_item("duration_seconds")
                        .ok()
                        .and_then(|v| v.extract().ok())
                        .unwrap_or(0.0);
                    let sample_rate: i64 = result
                        .get_item("sample_rate")
                        .ok()
                        .and_then(|v| v.extract().ok())
                        .unwrap_or(44100);

                    let mut outputs = HashMap::new();
                    outputs.insert("audio".to_string(), serde_json::json!(audio_base64));
                    outputs.insert(
                        "duration_seconds".to_string(),
                        serde_json::json!(duration_seconds),
                    );
                    outputs.insert("sample_rate".to_string(), serde_json::json!(sample_rate));
                    Ok(outputs)
                },
            )
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(NodeEngineError::ExecutionFailed)?
    };

    let model_name = std::path::Path::new(&model_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio-model")
        .to_string();
    let model_ref = build_model_ref_v2(
        resolved_model_ref,
        "stable_audio",
        &model_name,
        &model_path,
        "text-to-audio",
        inputs,
    );
    result.insert(
        "model_ref".to_string(),
        serde_json::to_value(model_ref).unwrap_or_else(|_| {
            serde_json::json!({
                "contractVersion": 2,
                "engine": "stable_audio",
                "modelId": model_name,
                "modelPath": model_path,
                "taskTypePrimary": "text-to-audio",
            })
        }),
    );

    Ok(result)
}

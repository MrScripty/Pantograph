//! PyTorch backend implementation (in-process via PyO3)
//!
//! Embeds a Python interpreter to run PyTorch inference directly in the
//! Pantograph process. Supports HuggingFace models, dLLMs (e.g., TraDo),
//! and Sherry ternary quantized models.
//!
//! The Python worker module (`torch/worker.py`) is embedded at compile time
//! via `include_str!` and loaded into `sys.modules` on first use.

use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::Stream;
use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{
    BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use crate::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use crate::process::ProcessSpawner;
use crate::types::{RerankRequest, RerankResponse};
use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};

/// The Python worker source, embedded at compile time.
const WORKER_PY: &str = include_str!("../../torch/worker.py");

/// The block diffusion module source, embedded at compile time.
const BLOCK_DIFFUSION_PY: &str = include_str!("../../torch/block_diffusion.py");

/// The autoregressive module source, embedded at compile time.
const AUTOREGRESSIVE_PY: &str = include_str!("../../torch/autoregressive.py");

/// Whether the Python worker module has been initialised.
static WORKER_INITIALISED: AtomicBool = AtomicBool::new(false);

/// Ensure the worker module is loaded into the Python interpreter.
///
/// Safe to call multiple times — only the first call actually loads.
/// Registers the sibling modules (`block_diffusion`, `autoregressive`)
/// into `sys.modules` so the worker can import them normally.
fn ensure_worker_initialised(py: Python<'_>) -> PyResult<()> {
    if WORKER_INITIALISED.load(Ordering::Acquire) {
        return Ok(());
    }

    // Register sibling modules first so worker.py's imports resolve
    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;

    let bd_code = std::ffi::CString::new(BLOCK_DIFFUSION_PY).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid block_diffusion source: {}", e))
    })?;
    let bd_module = PyModule::from_code(py, &bd_code, c"block_diffusion.py", c"block_diffusion")?;
    modules.set_item("block_diffusion", &bd_module)?;

    let ar_code = std::ffi::CString::new(AUTOREGRESSIVE_PY).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid autoregressive source: {}", e))
    })?;
    let ar_module = PyModule::from_code(py, &ar_code, c"autoregressive.py", c"autoregressive")?;
    modules.set_item("autoregressive", &ar_module)?;

    // Now load the worker module (which imports from block_diffusion and autoregressive)
    let code = std::ffi::CString::new(WORKER_PY).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid worker source: {}", e))
    })?;
    PyModule::from_code(
        py,
        &code,
        c"pantograph_torch_worker",
        c"pantograph_torch_worker",
    )?;

    WORKER_INITIALISED.store(true, Ordering::Release);
    log::info!(
        "PyTorch worker module initialised (with block_diffusion + autoregressive siblings)"
    );
    Ok(())
}

/// Get a reference to the already-loaded worker module.
fn worker_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    ensure_worker_initialised(py)?;
    py.import("pantograph_torch_worker")
}

fn extract_live_kv_info(value: &Bound<'_, PyAny>) -> Result<PyTorchLiveKvInfo, BackendError> {
    let token_count = value
        .get_item("token_count")
        .map_err(|e| BackendError::Inference(format!("Missing KV token_count: {}", e)))?
        .extract::<usize>()
        .map_err(|e| BackendError::Inference(format!("Invalid KV token_count: {}", e)))?;
    let model_path = value
        .get_item("model_path")
        .map_err(|e| BackendError::Inference(format!("Missing KV model_path: {}", e)))?
        .extract::<String>()
        .map_err(|e| BackendError::Inference(format!("Invalid KV model_path: {}", e)))?;
    let model_type = value
        .get_item("model_type")
        .map_err(|e| BackendError::Inference(format!("Missing KV model_type: {}", e)))?
        .extract::<String>()
        .map_err(|e| BackendError::Inference(format!("Invalid KV model_type: {}", e)))?;
    let device = value
        .get_item("device")
        .map_err(|e| BackendError::Inference(format!("Missing KV device: {}", e)))?
        .extract::<String>()
        .map_err(|e| BackendError::Inference(format!("Invalid KV device: {}", e)))?;

    Ok(PyTorchLiveKvInfo {
        token_count,
        model_path,
        model_type,
        device,
    })
}

fn extract_loaded_model_info(value: &Bound<'_, PyAny>) -> Result<LoadedModelInfo, BackendError> {
    let model_path = value
        .get_item("model_path")
        .map_err(|e| BackendError::Inference(format!("Missing loaded model_path: {}", e)))?
        .extract::<String>()
        .map_err(|e| BackendError::Inference(format!("Invalid loaded model_path: {}", e)))?;
    let model_type = value
        .get_item("model_type")
        .map_err(|e| BackendError::Inference(format!("Missing loaded model_type: {}", e)))?
        .extract::<String>()
        .map_err(|e| BackendError::Inference(format!("Invalid loaded model_type: {}", e)))?;
    let device = value
        .get_item("device")
        .map_err(|e| BackendError::Inference(format!("Missing loaded device: {}", e)))?
        .extract::<String>()
        .map_err(|e| BackendError::Inference(format!("Invalid loaded device: {}", e)))?;

    Ok(LoadedModelInfo {
        model_path,
        model_type,
        device,
    })
}

/// PyTorch backend using in-process PyO3 embedded Python.
///
/// Loads models via HuggingFace transformers with `trust_remote_code=True`,
/// supporting standard models, dLLM architectures, and Sherry quantised models.
pub struct PyTorchBackend {
    /// Whether the backend has been initialised and is ready
    ready: bool,
    /// Currently loaded model metadata
    loaded_model: Option<LoadedModelInfo>,
}

#[derive(Debug, Clone)]
pub struct LoadedModelInfo {
    pub model_path: String,
    pub model_type: String,
    pub device: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyTorchLiveKvInfo {
    pub token_count: usize,
    pub model_path: String,
    pub model_type: String,
    pub device: String,
}

pub fn kv_cache_runtime_fingerprint_for_live_kv(
    info: &PyTorchLiveKvInfo,
) -> KvCacheRuntimeFingerprint {
    kv_cache_runtime_fingerprint_for_loaded_model(&LoadedModelInfo {
        model_path: info.model_path.clone(),
        model_type: info.model_type.clone(),
        device: info.device.clone(),
    })
}

pub fn kv_cache_model_fingerprint_for_live_kv(info: &PyTorchLiveKvInfo) -> ModelFingerprint {
    kv_cache_model_fingerprint_for_loaded_model(&LoadedModelInfo {
        model_path: info.model_path.clone(),
        model_type: info.model_type.clone(),
        device: info.device.clone(),
    })
}

pub fn kv_cache_runtime_fingerprint_for_loaded_model(
    loaded: &LoadedModelInfo,
) -> KvCacheRuntimeFingerprint {
    PyTorchBackend::kv_cache_runtime_fingerprint_for_loaded_model(loaded)
}

pub fn kv_cache_model_fingerprint_for_loaded_model(loaded: &LoadedModelInfo) -> ModelFingerprint {
    PyTorchBackend::kv_cache_model_fingerprint_for_loaded_model(loaded)
}

pub fn supports_live_kv_reuse(model_type: &str) -> bool {
    model_type == "dllm"
}

pub async fn active_loaded_model_info() -> Result<LoadedModelInfo, BackendError> {
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| -> Result<LoadedModelInfo, BackendError> {
            let worker = worker_module(py).map_err(|e| {
                BackendError::Inference(format!("Failed to get worker module: {}", e))
            })?;
            let result = worker.call_method0("get_loaded_info").map_err(|e| {
                BackendError::Inference(format!("PyTorch get_loaded_info failed: {}", e))
            })?;
            if result.is_none() {
                return Err(BackendError::Inference(
                    "PyTorch KV operations require an active loaded model".to_string(),
                ));
            }
            extract_loaded_model_info(&result)
        })
    })
    .await
    .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))?
}

pub async fn save_live_kv_snapshot(path: &Path) -> Result<PyTorchLiveKvInfo, BackendError> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| -> Result<PyTorchLiveKvInfo, BackendError> {
            let worker = worker_module(py).map_err(|e| {
                BackendError::Inference(format!("Failed to get worker module: {}", e))
            })?;
            let result = worker
                .call_method1("save_live_kv_cache", (path.to_string_lossy().to_string(),))
                .map_err(|e| BackendError::Inference(format!("PyTorch KV save failed: {}", e)))?;
            extract_live_kv_info(&result)
        })
    })
    .await
    .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))?
}

pub async fn restore_live_kv_snapshot(path: &Path) -> Result<PyTorchLiveKvInfo, BackendError> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| -> Result<PyTorchLiveKvInfo, BackendError> {
            let worker = worker_module(py).map_err(|e| {
                BackendError::Inference(format!("Failed to get worker module: {}", e))
            })?;
            let result = worker
                .call_method1(
                    "restore_live_kv_cache",
                    (path.to_string_lossy().to_string(),),
                )
                .map_err(|e| {
                    BackendError::Inference(format!("PyTorch KV restore failed: {}", e))
                })?;
            extract_live_kv_info(&result)
        })
    })
    .await
    .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))?
}

pub async fn clear_live_kv_snapshot() -> Result<(), BackendError> {
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| -> Result<(), BackendError> {
            let worker = worker_module(py).map_err(|e| {
                BackendError::Inference(format!("Failed to get worker module: {}", e))
            })?;
            worker
                .call_method0("clear_live_kv_cache")
                .map_err(|e| BackendError::Inference(format!("PyTorch KV clear failed: {}", e)))?;
            Ok(())
        })
    })
    .await
    .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))?
}

impl PyTorchBackend {
    pub fn new() -> Self {
        Self {
            ready: false,
            loaded_model: None,
        }
    }

    /// Get static capabilities (for registry info before instantiation)
    pub fn static_capabilities() -> BackendCapabilities {
        BackendCapabilities {
            vision: false,
            image_generation: false,
            embeddings: false,
            reranking: false,
            gpu: true,
            device_selection: true,
            streaming: true,
            tool_calling: false,
            external_connection: false,
        }
    }

    /// Check if Python 3 is available on the system
    pub fn check_availability() -> (bool, Option<String>) {
        match which::which("python3") {
            Ok(_) => (true, None),
            Err(_) => (
                false,
                Some("python3 not found in PATH. Install Python 3 with PyTorch.".to_string()),
            ),
        }
    }

    fn can_reuse_loaded_model(
        &self,
        model_path: &str,
        device: &str,
        model_type: Option<&str>,
    ) -> bool {
        self.loaded_model.as_ref().is_some_and(|loaded| {
            loaded.model_path == model_path
                && loaded.device == device
                && model_type.is_none_or(|requested| loaded.model_type == requested)
        })
    }

    fn active_loaded_model(&self) -> Result<&LoadedModelInfo, BackendError> {
        self.loaded_model.as_ref().ok_or_else(|| {
            BackendError::Inference(
                "KV cache operations require an active loaded PyTorch model".to_string(),
            )
        })
    }

    fn kv_cache_runtime_fingerprint_for_loaded_model(
        loaded: &LoadedModelInfo,
    ) -> KvCacheRuntimeFingerprint {
        KvCacheRuntimeFingerprint {
            runtime_id: canonical_runtime_id("pytorch"),
            backend_key: canonical_runtime_backend_key("pytorch"),
            tokenizer_fingerprint: format!("pytorch:{}:{}", loaded.model_path, loaded.model_type),
            prompt_format_fingerprint: Some(format!("pytorch_{}", loaded.model_type)),
            runtime_build_fingerprint: Some(loaded.device.clone()),
        }
    }

    fn kv_cache_model_fingerprint_for_loaded_model(loaded: &LoadedModelInfo) -> ModelFingerprint {
        ModelFingerprint {
            model_id: loaded.model_path.clone(),
            config_hash: format!("pytorch:{}", loaded.model_type),
        }
    }

    fn require_live_kv_slot(slot_id: u32) -> Result<(), BackendError> {
        if slot_id == 0 {
            return Ok(());
        }
        Err(BackendError::Config(
            "PyTorch backend exposes only a single live KV slot at slot_id 0".to_string(),
        ))
    }

    /// Load a model into the embedded Python runtime.
    pub async fn load_model(
        &mut self,
        model_path: &str,
        device: &str,
        model_type: Option<&str>,
    ) -> Result<LoadedModelInfo, BackendError> {
        let model_path = model_path.to_string();
        let device = device.to_string();
        let model_type = model_type.map(|s| s.to_string());

        let info = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<LoadedModelInfo, BackendError> {
                let worker = worker_module(py).map_err(|e| {
                    BackendError::StartupFailed(format!("Failed to load worker module: {}", e))
                })?;

                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("model_path", &model_path).unwrap();
                kwargs.set_item("device", &device).unwrap();
                if let Some(ref mt) = model_type {
                    kwargs.set_item("model_type", mt).unwrap();
                }

                let result = worker
                    .call_method("load_model", (), Some(&kwargs))
                    .map_err(|e| BackendError::Inference(format!("Model load failed: {}", e)))?;

                let info = LoadedModelInfo {
                    model_path: result
                        .get_item("model_path")
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                        .unwrap_or_default(),
                    model_type: result
                        .get_item("model_type")
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                        .unwrap_or_else(|| "text-generation".to_string()),
                    device: result
                        .get_item("device")
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                        .unwrap_or_else(|| "cpu".to_string()),
                };

                Ok(info)
            })
        })
        .await
        .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))??;

        self.loaded_model = Some(info.clone());
        self.ready = true;
        Ok(info)
    }

    /// Unload the current model and free GPU memory.
    pub async fn unload_model(&mut self) -> Result<(), BackendError> {
        tokio::task::spawn_blocking(|| {
            Python::with_gil(|py| -> Result<(), BackendError> {
                let worker = worker_module(py).map_err(|e| {
                    BackendError::Inference(format!("Failed to get worker module: {}", e))
                })?;
                worker
                    .call_method0("unload_model")
                    .map_err(|e| BackendError::Inference(format!("Unload failed: {}", e)))?;
                Ok(())
            })
        })
        .await
        .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))??;

        self.loaded_model = None;
        Ok(())
    }

    /// Generate a complete response (non-streaming).
    ///
    /// When `masked_prompt_json` is `Some`, the JSON is passed through to the
    /// Python worker so it can perform masked (anchor-preserving) generation.
    pub async fn generate(
        &self,
        prompt: String,
        system_prompt: Option<String>,
        max_tokens: i64,
        temperature: f64,
        top_p: f64,
        masked_prompt_json: Option<String>,
    ) -> Result<String, BackendError> {
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<String, BackendError> {
                let worker = worker_module(py).map_err(|e| {
                    BackendError::Inference(format!("Failed to get worker module: {}", e))
                })?;

                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("prompt", &prompt).unwrap();
                if let Some(ref sys) = system_prompt {
                    kwargs.set_item("system_prompt", sys).unwrap();
                }
                kwargs.set_item("max_tokens", max_tokens).unwrap();
                kwargs.set_item("temperature", temperature).unwrap();
                kwargs.set_item("top_p", top_p).unwrap();
                if let Some(ref mpj) = masked_prompt_json {
                    kwargs.set_item("masked_prompt_json", mpj).unwrap();
                }

                let result = worker
                    .call_method("generate", (), Some(&kwargs))
                    .map_err(|e| BackendError::Inference(format!("Generation failed: {}", e)))?;

                result.extract::<String>().map_err(|e| {
                    BackendError::Inference(format!("Failed to extract result: {}", e))
                })
            })
        })
        .await
        .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))?
    }

    /// Generate tokens as a stream via an mpsc channel.
    ///
    /// Spawns a blocking task that iterates the Python generator and sends
    /// each token through the channel. When `masked_prompt_json` is `Some`,
    /// it is forwarded to the Python worker for masked generation.
    pub fn generate_stream(
        &self,
        prompt: String,
        system_prompt: Option<String>,
        max_tokens: i64,
        temperature: f64,
        top_p: f64,
        masked_prompt_json: Option<String>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>> {
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<ChatChunk, BackendError>>(32);

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let worker = match worker_module(py) {
                    Ok(w) => w,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(BackendError::Inference(format!(
                            "Failed to get worker module: {}",
                            e
                        ))));
                        return;
                    }
                };

                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("prompt", &prompt).unwrap();
                if let Some(ref sys) = system_prompt {
                    kwargs.set_item("system_prompt", sys).unwrap();
                }
                kwargs.set_item("max_tokens", max_tokens).unwrap();
                kwargs.set_item("temperature", temperature).unwrap();
                kwargs.set_item("top_p", top_p).unwrap();
                if let Some(ref mpj) = masked_prompt_json {
                    kwargs.set_item("masked_prompt_json", mpj).unwrap();
                }

                let generator = match worker.call_method("generate_tokens", (), Some(&kwargs)) {
                    Ok(g) => g,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(BackendError::Inference(format!(
                            "Failed to create generator: {}",
                            e
                        ))));
                        return;
                    }
                };

                // Iterate the Python generator
                let iter = match generator.try_iter() {
                    Ok(it) => it,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(BackendError::Inference(format!(
                            "Generator is not iterable: {}",
                            e
                        ))));
                        return;
                    }
                };

                for item in iter {
                    match item {
                        Ok(token_obj) => match token_obj.extract::<String>() {
                            Ok(token) => {
                                if tx
                                    .blocking_send(Ok(ChatChunk {
                                        content: Some(token),
                                        done: false,
                                    }))
                                    .is_err()
                                {
                                    return;
                                }
                            }
                            Err(e) => {
                                let _ = tx.blocking_send(Err(BackendError::Inference(format!(
                                    "Token extraction failed: {}",
                                    e
                                ))));
                                return;
                            }
                        },
                        Err(e) => {
                            let _ = tx.blocking_send(Err(BackendError::Inference(format!(
                                "Generator error: {}",
                                e
                            ))));
                            return;
                        }
                    }
                }

                // Signal completion
                let _ = tx.blocking_send(Ok(ChatChunk {
                    content: None,
                    done: true,
                }));
            });
        });

        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }
}

impl Default for PyTorchBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for PyTorchBackend {
    fn name(&self) -> &'static str {
        "PyTorch"
    }

    fn description(&self) -> &'static str {
        "In-process PyTorch inference for dLLM, Sherry, and HuggingFace models"
    }

    fn capabilities(&self) -> BackendCapabilities {
        Self::static_capabilities()
    }

    async fn start(
        &mut self,
        config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        let was_ready = self.ready;

        // Initialise the Python worker module
        tokio::task::spawn_blocking(|| {
            Python::with_gil(|py| {
                ensure_worker_initialised(py).map_err(|e| {
                    BackendError::StartupFailed(format!(
                        "Failed to initialise Python worker: {}",
                        e
                    ))
                })
            })
        })
        .await
        .map_err(|e| BackendError::StartupFailed(format!("Task join error: {}", e)))??;

        // Log the transformers version for diagnostics
        let tf_version = tokio::task::spawn_blocking(|| {
            Python::with_gil(|py| -> String {
                py.import("transformers")
                    .and_then(|m| m.getattr("__version__"))
                    .and_then(|v| v.extract::<String>())
                    .unwrap_or_else(|_| "unknown".into())
            })
        })
        .await
        .unwrap_or_else(|_| "unknown".into());
        log::info!("PyTorch backend: transformers {}", tf_version);

        // If config includes a model_path, load it immediately
        if let Some(ref model_path) = config.model_path {
            let device = config.device.as_deref().unwrap_or("auto");
            let model_type = config.model_type.as_deref();
            let model_path = model_path.to_string_lossy().to_string();

            if self.can_reuse_loaded_model(&model_path, device, model_type) {
                self.ready = true;
                log::info!("PyTorch backend: reusing loaded model {}", model_path);
                return Ok(BackendStartOutcome {
                    runtime_reused: Some(true),
                    lifecycle_decision_reason: Some("runtime_reused".to_string()),
                });
            }

            self.load_model(&model_path, device, model_type).await?;

            return Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            });
        }

        self.ready = true;
        Ok(BackendStartOutcome {
            runtime_reused: Some(was_ready),
            lifecycle_decision_reason: Some(
                if was_ready {
                    "runtime_reused"
                } else {
                    "runtime_ready"
                }
                .to_string(),
            ),
        })
    }

    fn stop(&mut self) {
        // Best-effort unload — can't await in a sync fn, so use blocking
        let had_model = self.loaded_model.is_some();
        self.loaded_model = None;
        self.ready = false;

        if had_model {
            std::thread::spawn(|| {
                Python::with_gil(|py| {
                    if let Ok(worker) = worker_module(py) {
                        let _ = worker.call_method0("unload_model");
                    }
                });
            });
        }
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    async fn health_check(&self) -> bool {
        if !self.ready {
            return false;
        }
        tokio::task::spawn_blocking(|| Python::with_gil(|py| worker_module(py).is_ok()))
            .await
            .unwrap_or(false)
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        if !self.ready {
            return Err(BackendError::NotReady);
        }

        let request: serde_json::Value = serde_json::from_str(&request_json)
            .map_err(|e| BackendError::Inference(format!("Invalid request JSON: {}", e)))?;

        let prompt = extract_prompt_from_messages(&request)?;
        let system_prompt = extract_system_prompt(&request);
        let max_tokens = request
            .get("max_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(512);
        let temperature = request
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7);
        let top_p = request.get("top_p").and_then(|v| v.as_f64()).unwrap_or(1.0);

        Ok(self.generate_stream(prompt, system_prompt, max_tokens, temperature, top_p, None))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Err(BackendError::Inference(
            "Embeddings not supported by PyTorch backend".to_string(),
        ))
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Err(BackendError::Inference(
            "Reranking not supported by PyTorch backend".to_string(),
        ))
    }

    async fn kv_cache_runtime_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<KvCacheRuntimeFingerprint, BackendError> {
        Ok(Self::kv_cache_runtime_fingerprint_for_loaded_model(
            self.active_loaded_model()?,
        ))
    }

    async fn kv_cache_model_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<ModelFingerprint, BackendError> {
        Ok(Self::kv_cache_model_fingerprint_for_loaded_model(
            self.active_loaded_model()?,
        ))
    }

    async fn save_kv_cache_slot(&self, slot_id: u32, path: &Path) -> Result<(), BackendError> {
        Self::require_live_kv_slot(slot_id)?;
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<(), BackendError> {
                let worker = worker_module(py).map_err(|e| {
                    BackendError::Inference(format!("Failed to get worker module: {}", e))
                })?;
                worker
                    .call_method1("save_live_kv_cache", (path.to_string_lossy().to_string(),))
                    .map_err(|e| {
                        BackendError::Inference(format!("PyTorch KV save failed: {}", e))
                    })?;
                Ok(())
            })
        })
        .await
        .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))?
    }

    async fn restore_kv_cache_slot(&self, slot_id: u32, path: &Path) -> Result<(), BackendError> {
        Self::require_live_kv_slot(slot_id)?;
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<(), BackendError> {
                let worker = worker_module(py).map_err(|e| {
                    BackendError::Inference(format!("Failed to get worker module: {}", e))
                })?;
                worker
                    .call_method1(
                        "restore_live_kv_cache",
                        (path.to_string_lossy().to_string(),),
                    )
                    .map_err(|e| {
                        BackendError::Inference(format!("PyTorch KV restore failed: {}", e))
                    })?;
                Ok(())
            })
        })
        .await
        .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))?
    }

    async fn clear_kv_cache_slot(&self, slot_id: u32) -> Result<(), BackendError> {
        Self::require_live_kv_slot(slot_id)?;
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<(), BackendError> {
                let worker = worker_module(py).map_err(|e| {
                    BackendError::Inference(format!("Failed to get worker module: {}", e))
                })?;
                worker.call_method0("clear_live_kv_cache").map_err(|e| {
                    BackendError::Inference(format!("PyTorch KV clear failed: {}", e))
                })?;
                Ok(())
            })
        })
        .await
        .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))?
    }

    async fn truncate_kv_cache_data(
        &self,
        data: &[u8],
        token_position: usize,
        _active_config: Option<&BackendConfig>,
    ) -> Result<Vec<u8>, BackendError> {
        let temp_path = std::env::temp_dir().join(format!(
            "pantograph-pytorch-kv-truncate-{}.bin",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&temp_path, data)
            .map_err(|e| BackendError::Inference(format!("Failed to write KV temp file: {}", e)))?;
        let truncate_result = tokio::task::spawn_blocking({
            let temp_path = temp_path.clone();
            move || {
                Python::with_gil(|py| -> Result<(), BackendError> {
                    let worker = worker_module(py).map_err(|e| {
                        BackendError::Inference(format!("Failed to get worker module: {}", e))
                    })?;
                    worker
                        .call_method1(
                            "truncate_kv_cache_file",
                            (temp_path.to_string_lossy().to_string(), token_position),
                        )
                        .map_err(|e| {
                            BackendError::Inference(format!("PyTorch KV truncate failed: {}", e))
                        })?;
                    Ok(())
                })
            }
        })
        .await
        .map_err(|e| BackendError::Inference(format!("Task join error: {}", e)))?;
        let read_result = std::fs::read(&temp_path)
            .map_err(|e| BackendError::Inference(format!("Failed to read KV temp file: {}", e)));
        let _ = std::fs::remove_file(&temp_path);
        truncate_result?;
        read_result
    }
}

/// Extract the last user message from OpenAI-format messages array.
fn extract_prompt_from_messages(request: &serde_json::Value) -> Result<String, BackendError> {
    let messages = request
        .get("messages")
        .and_then(|m| m.as_array())
        .ok_or_else(|| BackendError::Inference("Missing 'messages' array".to_string()))?;

    messages
        .iter()
        .rev()
        .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .and_then(|m| m.get("content").and_then(|c| c.as_str()))
        .map(|s| s.to_string())
        .ok_or_else(|| BackendError::Inference("No user message found".to_string()))
}

/// Extract the system prompt from OpenAI-format messages array, if present.
fn extract_system_prompt(request: &serde_json::Value) -> Option<String> {
    request
        .get("messages")
        .and_then(|m| m.as_array())
        .and_then(|msgs| {
            msgs.iter()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
                .and_then(|m| m.get("content").and_then(|c| c.as_str()))
                .map(|s| s.to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_name() {
        let backend = PyTorchBackend::new();
        assert_eq!(backend.name(), "PyTorch");
    }

    #[test]
    fn test_capabilities() {
        let caps = PyTorchBackend::static_capabilities();
        assert!(!caps.vision);
        assert!(!caps.embeddings);
        assert!(caps.gpu);
        assert!(caps.device_selection);
        assert!(caps.streaming);
        assert!(!caps.tool_calling);
    }

    #[test]
    fn test_not_ready_initially() {
        let backend = PyTorchBackend::new();
        assert!(!backend.is_ready());
        assert!(backend.base_url().is_none());
    }

    #[test]
    fn test_no_loaded_model_initially() {
        let backend = PyTorchBackend::new();
        assert!(backend.loaded_model.is_none());
    }

    #[test]
    fn test_can_reuse_loaded_model_requires_matching_request() {
        let mut backend = PyTorchBackend::new();
        backend.loaded_model = Some(LoadedModelInfo {
            model_path: "/models/demo".to_string(),
            model_type: "text-generation".to_string(),
            device: "cuda".to_string(),
        });

        assert!(backend.can_reuse_loaded_model("/models/demo", "cuda", None));
        assert!(backend.can_reuse_loaded_model("/models/demo", "cuda", Some("text-generation")));
        assert!(!backend.can_reuse_loaded_model("/models/other", "cuda", None));
        assert!(!backend.can_reuse_loaded_model("/models/demo", "cpu", None));
        assert!(!backend.can_reuse_loaded_model("/models/demo", "cuda", Some("dllm")));
    }

    #[test]
    fn test_kv_runtime_fingerprint_for_loaded_model_is_stable() {
        let loaded = LoadedModelInfo {
            model_path: "/models/demo".to_string(),
            model_type: "dllm".to_string(),
            device: "cuda".to_string(),
        };

        let fingerprint = PyTorchBackend::kv_cache_runtime_fingerprint_for_loaded_model(&loaded);
        assert_eq!(fingerprint.backend_key, "pytorch");
        assert_eq!(fingerprint.runtime_id, "pytorch");
        assert!(fingerprint.tokenizer_fingerprint.contains("/models/demo"));
        assert_eq!(
            fingerprint.prompt_format_fingerprint.as_deref(),
            Some("pytorch_dllm")
        );
        assert_eq!(
            fingerprint.runtime_build_fingerprint.as_deref(),
            Some("cuda")
        );
    }

    #[test]
    fn test_kv_model_fingerprint_for_loaded_model_tracks_model_identity() {
        let loaded = LoadedModelInfo {
            model_path: "/models/demo".to_string(),
            model_type: "dllm".to_string(),
            device: "cuda".to_string(),
        };

        let fingerprint = PyTorchBackend::kv_cache_model_fingerprint_for_loaded_model(&loaded);
        assert_eq!(fingerprint.model_id, "/models/demo");
        assert_eq!(fingerprint.config_hash, "pytorch:dllm");
    }

    #[test]
    fn test_require_live_kv_slot_rejects_nonzero_slots() {
        assert!(PyTorchBackend::require_live_kv_slot(0).is_ok());
        match PyTorchBackend::require_live_kv_slot(1) {
            Err(BackendError::Config(message)) => {
                assert!(message.contains("slot_id 0"));
            }
            other => panic!("expected Config error, got {other:?}"),
        }
    }

    #[test]
    fn test_live_kv_fingerprint_helpers_match_loaded_model_helpers() {
        let info = PyTorchLiveKvInfo {
            token_count: 42,
            model_path: "/models/demo".to_string(),
            model_type: "dllm".to_string(),
            device: "cuda".to_string(),
        };
        let loaded = LoadedModelInfo {
            model_path: info.model_path.clone(),
            model_type: info.model_type.clone(),
            device: info.device.clone(),
        };

        assert_eq!(
            kv_cache_runtime_fingerprint_for_live_kv(&info),
            PyTorchBackend::kv_cache_runtime_fingerprint_for_loaded_model(&loaded)
        );
        assert_eq!(
            kv_cache_model_fingerprint_for_live_kv(&info),
            PyTorchBackend::kv_cache_model_fingerprint_for_loaded_model(&loaded)
        );
    }

    #[test]
    fn test_in_process_no_base_url() {
        let backend = PyTorchBackend::new();
        assert!(backend.base_url().is_none());
    }

    #[test]
    fn test_extract_prompt() {
        let req = serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello!"}
            ]
        });
        assert_eq!(extract_prompt_from_messages(&req).unwrap(), "Hello!");
    }

    #[test]
    fn test_extract_system_prompt() {
        let req = serde_json::json!({
            "messages": [
                {"role": "system", "content": "Be concise."},
                {"role": "user", "content": "Hi"}
            ]
        });
        assert_eq!(extract_system_prompt(&req), Some("Be concise.".to_string()));
    }

    #[test]
    fn test_extract_system_prompt_missing() {
        let req = serde_json::json!({
            "messages": [{"role": "user", "content": "Hi"}]
        });
        assert_eq!(extract_system_prompt(&req), None);
    }
}

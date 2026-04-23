use std::sync::atomic::{AtomicBool, Ordering};

use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{BackendError, LoadedModelInfo, PyTorchLiveKvInfo};

const WORKER_PY: &str = include_str!("../../torch/worker.py");
const BLOCK_DIFFUSION_PY: &str = include_str!("../../torch/block_diffusion.py");
const AUTOREGRESSIVE_PY: &str = include_str!("../../torch/autoregressive.py");
const WORKER_RUNTIME_PY: &str = include_str!("../../torch/worker_runtime.py");
const WORKER_TRANSFORMERS_PY: &str = include_str!("../../torch/worker_transformers.py");

static WORKER_INITIALISED: AtomicBool = AtomicBool::new(false);

pub(super) fn ensure_worker_initialised(py: Python<'_>) -> PyResult<()> {
    if WORKER_INITIALISED.load(Ordering::Acquire) {
        return Ok(());
    }

    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;

    for (name, source, file_name, module_name) in [
        (
            "block_diffusion",
            BLOCK_DIFFUSION_PY,
            c"block_diffusion.py",
            c"block_diffusion",
        ),
        (
            "autoregressive",
            AUTOREGRESSIVE_PY,
            c"autoregressive.py",
            c"autoregressive",
        ),
        (
            "worker_runtime",
            WORKER_RUNTIME_PY,
            c"worker_runtime.py",
            c"worker_runtime",
        ),
        (
            "worker_transformers",
            WORKER_TRANSFORMERS_PY,
            c"worker_transformers.py",
            c"worker_transformers",
        ),
    ] {
        let code = std::ffi::CString::new(source).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid {} source: {}", name, e))
        })?;
        let module = PyModule::from_code(py, &code, file_name, module_name)?;
        modules.set_item(name, &module)?;
    }

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
    log::info!("PyTorch worker module initialised with embedded sibling modules");
    Ok(())
}

pub(super) fn worker_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    ensure_worker_initialised(py)?;
    py.import("pantograph_torch_worker")
}

pub(super) fn extract_live_kv_info(
    value: &Bound<'_, PyAny>,
) -> Result<PyTorchLiveKvInfo, BackendError> {
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

pub(super) fn extract_loaded_model_info(
    value: &Bound<'_, PyAny>,
) -> Result<LoadedModelInfo, BackendError> {
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

use std::sync::atomic::{AtomicBool, Ordering};

use pyo3::prelude::*;
use pyo3::types::PyModule;

const WORKER_PY: &str = include_str!("../../torch/worker.py");
const BLOCK_DIFFUSION_PY: &str = include_str!("../../torch/block_diffusion.py");
const AUTOREGRESSIVE_PY: &str = include_str!("../../torch/autoregressive.py");
const WORKER_RUNTIME_PY: &str = include_str!("../../torch/worker_runtime.py");
const WORKER_TRANSFORMERS_PY: &str = include_str!("../../torch/worker_transformers.py");
const WORKER_CONTRACT_PY: &str = include_str!("../../torch/worker_contract.py");
const WORKER_IMAGE_CONTRACT_PY: &str = include_str!("../../torch/worker_image_contract.py");

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
        (
            "worker_contract",
            WORKER_CONTRACT_PY,
            c"worker_contract.py",
            c"worker_contract",
        ),
        (
            "worker_image_contract",
            WORKER_IMAGE_CONTRACT_PY,
            c"worker_image_contract.py",
            c"worker_image_contract",
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

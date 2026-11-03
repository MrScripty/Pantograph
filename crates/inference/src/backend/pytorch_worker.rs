use std::sync::atomic::{AtomicBool, Ordering};

use pyo3::prelude::*;
use pyo3::types::PyModule;

const WORKER_PY: &str = include_str!("../../torch/worker.py");
const BLOCK_DIFFUSION_PY: &str = include_str!("../../torch/block_diffusion.py");
const AUTOREGRESSIVE_PY: &str = include_str!("../../torch/autoregressive.py");
const WORKER_DIFFUSION_PY: &str = include_str!("../../torch/worker_diffusion.py");
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
            "worker_diffusion",
            WORKER_DIFFUSION_PY,
            c"worker_diffusion.py",
            c"worker_diffusion",
        ),
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

/// Selected ASR owns module-local model globals, independently of other live
/// embedded runtimes and the inherited generic worker.
pub(super) struct IsolatedAudioWorker {
    name: String,
    initialized: AtomicBool,
}
impl IsolatedAudioWorker {
    pub(super) fn new() -> Self {
        Self {
            name: format!(
                "pantograph_selected_audio_{}",
                uuid::Uuid::new_v4().simple()
            ),
            initialized: AtomicBool::new(false),
        }
    }
    pub(super) fn module<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
        ensure_worker_initialised(py)?;
        match py.import(self.name.as_str()) {
            Ok(module) => {
                self.initialized.store(true, Ordering::Release);
                Ok(module)
            }
            Err(error) if error.is_instance_of::<pyo3::exceptions::PyModuleNotFoundError>(py) => {
                let source = std::ffi::CString::new(WORKER_PY).expect("embedded worker source");
                let name =
                    std::ffi::CString::new(self.name.as_str()).expect("generated module name");
                let module =
                    PyModule::from_code(py, &source, c"pantograph_selected_audio.py", &name)?;
                self.initialized.store(true, Ordering::Release);
                Ok(module)
            }
            Err(error) => Err(error),
        }
    }
}
impl Drop for IsolatedAudioWorker {
    fn drop(&mut self) {
        if !self.initialized.load(Ordering::Acquire) {
            return;
        }
        // Blocking closures retain an Arc to this registration. Retirement is
        // therefore after their actual completion, including caller loss.
        Python::with_gil(|py| {
            if let Ok(sys) = py.import("sys") {
                if let Ok(modules) = sys.getattr("modules") {
                    if let Ok(module) = modules.get_item(self.name.as_str()) {
                        let _ = module.call_method0("shutdown_worker");
                        let _ = modules.del_item(self.name.as_str());
                    }
                }
            }
        });
    }
}

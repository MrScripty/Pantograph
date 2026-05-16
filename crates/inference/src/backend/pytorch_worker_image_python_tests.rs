use std::ffi::CString;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

use super::pytorch_worker_contract::{PyTorchWorkerFailure, PyTorchWorkerResponse};
use super::pytorch_worker_image_contract::PyTorchGenerateImageResult;

fn load_worker_module_with_image_stubs<'py>(py: Python<'py>) -> Bound<'py, PyModule> {
    let setup = CString::new(
        r#"
import sys
import types

for name in ["numpy", "soundfile"]:
    sys.modules[name] = types.ModuleType(name)

torch = types.ModuleType("torch")
torch.float16 = "float16"
torch.float32 = "float32"
torch.bfloat16 = "bfloat16"
torch.cuda = types.SimpleNamespace(is_available=lambda: False)
torch.backends = types.SimpleNamespace(mps=types.SimpleNamespace(is_available=lambda: False))
torch.device = lambda value: types.SimpleNamespace(type=str(value))

class _Generator:
    def __init__(self, device=None):
        self.device = device

    def manual_seed(self, seed):
        self.seed = seed
        return self

torch.Generator = _Generator
sys.modules["torch"] = torch

block_diffusion = types.ModuleType("block_diffusion")
block_diffusion._generate_dllm_masked = lambda *args, **kwargs: None
block_diffusion._generate_dllm_masked_streaming = lambda *args, **kwargs: iter(())
sys.modules["block_diffusion"] = block_diffusion

autoregressive = types.ModuleType("autoregressive")
for attr in [
    "_generate_autoregressive",
    "_continue_sdar_cached",
    "_generate_sdar_cached",
]:
    setattr(autoregressive, attr, lambda *args, **kwargs: None)
autoregressive._generate_autoregressive_streaming = lambda *args, **kwargs: iter(())
sys.modules["autoregressive"] = autoregressive

worker_runtime = types.ModuleType("worker_runtime")
worker_runtime._decode_base64_image = lambda value: value
worker_runtime._detect_diffusion_load_overrides = lambda path: {}
worker_runtime._detect_model_type = lambda path: "text-generation"
worker_runtime._dtype_name = str
worker_runtime._encode_image = lambda image: {
    "data_base64": "iVBORw0KGgo=",
    "mime_type": "image/png",
    "width": getattr(image, "width", None),
    "height": getattr(image, "height", None),
}
worker_runtime._resolve_device = lambda device: torch.device("cpu")
worker_runtime._resolve_model_directory = lambda path: path
worker_runtime._resolve_torch_dtype = lambda device, requested_dtype=None: torch.float32
sys.modules["worker_runtime"] = worker_runtime

worker_transformers = types.ModuleType("worker_transformers")
worker_transformers.apply_compatibility_shims = lambda: None
sys.modules["worker_transformers"] = worker_transformers
"#,
    )
    .expect("stub setup source should not contain nul bytes");
    py.run(&setup, None, None)
        .expect("stubbed worker dependencies should load");

    for (source, file_name, module_name) in [
        (
            include_str!("../../torch/worker_contract.py"),
            c"worker_contract.py",
            c"worker_contract",
        ),
        (
            include_str!("../../torch/worker_image_contract.py"),
            c"worker_image_contract.py",
            c"worker_image_contract",
        ),
    ] {
        let source = CString::new(source).expect("python source should not contain nul bytes");
        let module =
            PyModule::from_code(py, &source, file_name, module_name).expect("module should load");
        py.import("sys")
            .expect("sys should import")
            .getattr("modules")
            .expect("sys.modules should exist")
            .set_item(module_name.to_str().expect("module name"), module)
            .expect("module should register");
    }

    let source = CString::new(include_str!("../../torch/worker.py"))
        .expect("worker source should not contain nul bytes");
    PyModule::from_code(
        py,
        &source,
        c"pantograph_torch_worker_test.py",
        c"pantograph_torch_worker_test",
    )
    .expect("worker module should load with stubs")
}

fn attach_stub_diffusion_pipeline(module: &Bound<'_, PyModule>) {
    let py = module.py();
    let locals = PyDict::new(py);
    let setup = CString::new(
        r#"
import types

class _Image:
    width = 512
    height = 512

class _Pipeline:
    def __call__(self, **kwargs):
        self.last_kwargs = kwargs
        return types.SimpleNamespace(images=[_Image()])

pipeline = _Pipeline()
"#,
    )
    .expect("pipeline setup source should not contain nul bytes");
    py.run(&setup, Some(&locals), Some(&locals))
        .expect("pipeline stub should load");
    module
        .setattr(
            "_diffusion_pipeline",
            locals
                .get_item("pipeline")
                .expect("pipeline lookup should succeed")
                .expect("pipeline should exist"),
        )
        .expect("pipeline should attach");
}

#[test]
fn test_python_worker_generate_image_from_envelope_returns_worker_response() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_image_stubs(py);
        attach_stub_diffusion_pipeline(&module);

        let response_json: String = module
            .call_method1(
                "generate_image_from_envelope",
                (include_str!(
                    "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
                ),),
            )
            .expect("generate_image_from_envelope should return JSON")
            .extract()
            .expect("response should be a string");
        let response: PyTorchWorkerResponse<PyTorchGenerateImageResult> =
            serde_json::from_str(&response_json).expect("worker response should decode");

        let PyTorchWorkerResponse::Ok(success) = response else {
            panic!("expected generate_image worker success, got {response_json}");
        };
        assert_eq!(success.request_id, "req-image-001");
        assert_eq!(success.result.images.len(), 1);
        assert_eq!(success.result.images[0].mime_type, "image/png");
        assert_eq!(success.result.seed_used, Some(42));
        assert_eq!(success.result.metadata["denoising_scheduler"], "euler");
        assert_eq!(success.result.metadata["device"], "cpu");
    });
}

#[test]
fn test_python_worker_generate_image_from_envelope_rejects_unplanned_fields() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_image_stubs(py);
        attach_stub_diffusion_pipeline(&module);
        let mut envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
        ))
        .expect("decode image request fixture");
        envelope["payload"]["trust_remote_code"] = serde_json::json!(true);

        let response_json: String = module
            .call_method1("generate_image_from_envelope", (envelope.to_string(),))
            .expect("generate_image_from_envelope should return JSON")
            .extract()
            .expect("response should be a string");
        let response: PyTorchWorkerResponse<PyTorchGenerateImageResult> =
            serde_json::from_str(&response_json).expect("worker response should decode");

        let PyTorchWorkerResponse::Error(PyTorchWorkerFailure { error, .. }) = response else {
            panic!("expected generate_image worker validation failure");
        };
        assert_eq!(
            error.canonical_code.as_deref(),
            Some("pytorch_worker_invalid_generate_image_request")
        );
        assert!(error
            .message
            .contains("unsupported key(s): trust_remote_code"));
    });
}

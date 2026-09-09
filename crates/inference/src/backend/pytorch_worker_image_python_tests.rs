use std::ffi::CString;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

use crate::resource_observation::{
    InferenceMemoryFailureKind, InferenceResourceObservationSourceKind,
    InferenceResourceObservationUnavailableState,
};

use super::pytorch_worker_contract::{PyTorchWorkerFailure, PyTorchWorkerResponse};
use super::pytorch_worker_image_contract::{
    PyTorchGenerateImageBatchResult, PyTorchGenerateImageResult,
};

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
            include_str!("../../torch/worker_diffusion.py"),
            c"worker_diffusion.py",
            c"worker_diffusion",
        ),
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
        prompt = kwargs.get("prompt")
        count = len(prompt) if isinstance(prompt, list) else 1
        return types.SimpleNamespace(images=[_Image() for _ in range(count)])

pipeline = _Pipeline()
def load_diffusion_model(path, device=None, torch_dtype=None):
    return None
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
    module
        .setattr(
            "load_diffusion_model",
            locals
                .get_item("load_diffusion_model")
                .expect("load_diffusion_model lookup should succeed")
                .expect("load_diffusion_model should exist"),
        )
        .expect("load_diffusion_model should attach");
    module
        .setattr(
            "_admit_diffusion_request",
            locals.get_item("load_diffusion_model").unwrap().unwrap(),
        )
        .unwrap();
}

#[test]
fn test_python_worker_generate_image_batch_from_envelope_returns_worker_response() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_image_stubs(py);
        attach_stub_diffusion_pipeline(&module);

        let response_json: String = module
            .call_method1(
                "generate_image_batch_from_envelope",
                (include_str!(
                    "../../tests/fixtures/pytorch_worker_contract/generate_image_batch_request.json"
                ),),
            )
            .expect("generate_image_batch_from_envelope should return JSON")
            .extract()
            .expect("response should be a string");
        let response: PyTorchWorkerResponse<PyTorchGenerateImageBatchResult> =
            serde_json::from_str(&response_json).expect("worker response should decode");

        let PyTorchWorkerResponse::Ok(success) = response else {
            panic!("expected generate_image_batch worker success, got {response_json}");
        };
        assert_eq!(success.request_id, "req-image-batch-001");
        assert_eq!(success.result.batch_execution_id, "image-batch-001");
        assert_eq!(success.result.members.len(), 2);
        assert_eq!(success.result.members[0].member_id, "member-001");
        assert_eq!(success.result.members[1].member_id, "member-002");
        assert_eq!(
            success.result.members[0]
                .result
                .as_ref()
                .expect("first member result")
                .images
                .len(),
            1
        );
        assert_eq!(
            success.result.members[1]
                .result
                .as_ref()
                .expect("second member result")
                .seed_used,
            Some(43)
        );

        let pipeline = module
            .getattr("_diffusion_pipeline")
            .expect("pipeline should exist");
        let last_kwargs = pipeline
            .getattr("last_kwargs")
            .expect("pipeline should record kwargs");
        let prompts = last_kwargs
            .get_item("prompt")
            .expect("prompt key should exist")
            .extract::<Vec<String>>()
            .expect("batch prompt should be a string list");
        assert_eq!(
            prompts,
            vec![
                "a compact test image".to_string(),
                "a second compact test image".to_string()
            ]
        );
    });
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
        assert!(success.result.metadata["denoising_scheduler"].is_null());
        assert_eq!(success.result.metadata["device"], "cpu");
        assert_eq!(
            success.result.metadata["artifact_load_target"]["local_load_path"],
            "/pumas/models/image/stable-diffusion/tiny-sd"
        );
    });
}

#[test]
fn test_python_worker_generate_image_from_envelope_reports_cuda_peak_vram() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_image_stubs(py);
        attach_stub_diffusion_pipeline(&module);
        let locals = PyDict::new(py);
        locals.set_item("worker", &module).expect("worker binds");
        let cuda_setup = CString::new(
            r#"
class _Cuda:
    def __init__(self):
        self.reset_device = None
        self.max_device = None

    def is_available(self):
        return True

    def reset_peak_memory_stats(self, device=None):
        self.reset_device = device

    def max_memory_allocated(self, device=None):
        self.max_device = device
        return 8192

worker.torch.cuda = _Cuda()
"#,
        )
        .expect("cuda setup source should not contain nul bytes");
        py.run(&cuda_setup, Some(&locals), Some(&locals))
            .expect("cuda telemetry stub should attach");
        let mut envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
        ))
        .expect("decode image request fixture");
        envelope["payload"]["device"] = serde_json::json!("cuda:0");

        let response_json: String = module
            .call_method1("generate_image_from_envelope", (envelope.to_string(),))
            .expect("generate_image_from_envelope should return JSON")
            .extract()
            .expect("response should be a string");
        let response: PyTorchWorkerResponse<PyTorchGenerateImageResult> =
            serde_json::from_str(&response_json).expect("worker response should decode");

        let PyTorchWorkerResponse::Ok(success) = response else {
            panic!("expected generate_image worker success, got {response_json}");
        };
        let observation = success
            .resource_observation
            .expect("CUDA peak VRAM observation should be present");
        assert_eq!(observation.peak_vram_bytes(), Some(8192));
        assert_eq!(observation.sources().len(), 1);
        assert_eq!(
            observation.sources()[0].source_kind(),
            InferenceResourceObservationSourceKind::PytorchCuda
        );
    });
}

#[test]
fn test_python_worker_generate_image_from_envelope_reports_mps_metric_unimplemented() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_image_stubs(py);
        attach_stub_diffusion_pipeline(&module);
        let locals = PyDict::new(py);
        locals.set_item("worker", &module).expect("worker binds");
        let mps_setup = CString::new(
            r#"
import types

worker.torch.backends = types.SimpleNamespace(
    mps=types.SimpleNamespace(is_available=lambda: True)
)
"#,
        )
        .expect("mps setup source should not contain nul bytes");
        py.run(&mps_setup, Some(&locals), Some(&locals))
            .expect("mps telemetry stub should attach");
        let mut envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
        ))
        .expect("decode image request fixture");
        envelope["payload"]["device"] = serde_json::json!("mps");

        let response_json: String = module
            .call_method1("generate_image_from_envelope", (envelope.to_string(),))
            .expect("generate_image_from_envelope should return JSON")
            .extract()
            .expect("response should be a string");
        let response: PyTorchWorkerResponse<PyTorchGenerateImageResult> =
            serde_json::from_str(&response_json).expect("worker response should decode");

        let PyTorchWorkerResponse::Ok(success) = response else {
            panic!("expected generate_image worker success, got {response_json}");
        };
        let observation = success
            .resource_observation
            .expect("MPS availability observation should be present");
        assert_eq!(observation.peak_vram_bytes(), None);
        assert_eq!(observation.availability().len(), 1);
        assert_eq!(
            observation.availability()[0].state(),
            InferenceResourceObservationUnavailableState::NotImplemented
        );
        assert_eq!(
            observation.availability()[0].source_kind(),
            Some(InferenceResourceObservationSourceKind::PytorchMps)
        );
    });
}

#[test]
fn test_python_worker_generate_image_from_envelope_reports_oom_failure() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_image_stubs(py);
        attach_stub_diffusion_pipeline(&module);
        let locals = PyDict::new(py);
        locals.set_item("worker", &module).expect("worker binds");
        let oom_setup = CString::new(
            r#"
class _FailingPipeline:
    def __call__(self, **kwargs):
        raise RuntimeError("CUDA out of memory while allocating tensor")

worker._diffusion_pipeline = _FailingPipeline()
"#,
        )
        .expect("OOM setup source should not contain nul bytes");
        py.run(&oom_setup, Some(&locals), Some(&locals))
            .expect("OOM pipeline stub should attach");
        let mut envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
        ))
        .expect("decode image request fixture");
        envelope["payload"]["device"] = serde_json::json!("cuda:0");

        let response_json: String = module
            .call_method1("generate_image_from_envelope", (envelope.to_string(),))
            .expect("generate_image_from_envelope should return JSON")
            .extract()
            .expect("response should be a string");
        let response: PyTorchWorkerResponse<PyTorchGenerateImageResult> =
            serde_json::from_str(&response_json).expect("worker response should decode");

        let PyTorchWorkerResponse::Error(failure) = response else {
            panic!("expected generate_image worker OOM failure");
        };
        let observation = failure
            .resource_observation
            .expect("OOM resource observation should be present");
        assert_eq!(
            observation.memory_failure_kind(),
            Some(InferenceMemoryFailureKind::OutOfMemory)
        );
    });
}

#[test]
fn test_python_worker_generate_image_from_envelope_rejects_unsupported_denoising_scheduler() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_image_stubs(py);
        attach_stub_diffusion_pipeline(&module);
        let mut envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
        ))
        .expect("decode image request fixture");
        envelope["payload"]["denoising_scheduler"] = serde_json::json!("euler");

        let response_json: String = module
            .call_method1("generate_image_from_envelope", (envelope.to_string(),))
            .expect("generate_image_from_envelope should return JSON")
            .extract()
            .expect("response should be a string");
        let response: PyTorchWorkerResponse<PyTorchGenerateImageResult> =
            serde_json::from_str(&response_json).expect("worker response should decode");

        let PyTorchWorkerResponse::Error(PyTorchWorkerFailure { error, .. }) = response else {
            panic!("expected explicit denoising scheduler to fail");
        };
        assert_eq!(
            error.canonical_code.as_deref(),
            Some("pytorch_worker_invalid_generate_image_request")
        );
        assert!(error.message.contains("denoising_scheduler changes yet"));
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

#[test]
fn test_python_worker_real_diffusion_loader_enforces_closed_bundle_admission() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_image_stubs(py);
        let locals = PyDict::new(py);
        locals.set_item("worker", &module).unwrap();
        locals
            .set_item(
                "single_fixture",
                include_str!(
                    "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
                ),
            )
            .unwrap();
        locals.set_item("batch_fixture", include_str!("../../tests/fixtures/pytorch_worker_contract/generate_image_batch_request.json")).unwrap();
        let source = CString::new(r#"
import copy
import json
from pathlib import Path
import sys
import tempfile
import types
from enum import Enum

calls = []
class Component:
    @classmethod
    def from_pretrained(cls, path, **kwargs):
        calls.append((cls.__name__, Path(path).name, kwargs))
        assert kwargs['local_files_only'] is True
        if Path(path).name in {'unet', 'vae', 'text_encoder', 'safety_checker', 'image_encoder'}:
            assert kwargs['use_safetensors'] is True
            if Path(path).name in {'text_encoder', 'safety_checker', 'image_encoder'}:
                assert kwargs['weights_only'] is True and kwargs['trust_remote_code'] is False
            if (Path(path) / 'only.bin').exists():
                raise OSError('safetensors unavailable')
        result = cls()
        if Path(path).name == 'scheduler':
            result.config = json.loads((Path(path) / 'scheduler_config.json').read_text())
        return result
class SchedulerMixin: pass
class Pipeline:
    def __init__(self, **kwargs): self.components = kwargs
    def set_progress_bar_config(self, **kwargs): pass
    def to(self, device): self.device = device
    def __call__(self, **kwargs): raise AssertionError('denied bundle generated')
    @classmethod
    def from_pretrained(cls, *args, **kwargs): raise AssertionError('generic pipeline loader used')

diffusers = types.ModuleType('diffusers')
diffusers.__version__ = '0.37.0'
safety_capability = types.ModuleType('transformers.utils.import_utils')
safety_capability.check_torch_load_is_safe = lambda: None
sys.modules['transformers.utils.import_utils'] = safety_capability
transformers = types.ModuleType('transformers')
for name in ['UNet2DConditionModel', 'AutoencoderKL']:
    setattr(diffusers, name, type(name, (Component,), {}))
for name in ['CLIPTextModel', 'CLIPTokenizer', 'CLIPTokenizerFast', 'CLIPImageProcessor', 'CLIPFeatureExtractor', 'CLIPVisionModelWithProjection']:
    setattr(transformers, name, type(name, (Component,), {}))
for name in ['EulerDiscreteScheduler', 'DDIMScheduler']:
    setattr(diffusers, name, type(name, (Component, SchedulerMixin), {}))
diffusers.StableDiffusionPipeline = Pipeline
diffusers.DiffusionPipeline = Pipeline
diffusers.OtherBuiltIn = type('OtherBuiltIn', (Component,), {})
scheduling = types.ModuleType('diffusers.schedulers.scheduling_utils')
scheduling.KarrasDiffusionSchedulers = Enum('KarrasDiffusionSchedulers', ['EulerDiscreteScheduler', 'DDIMScheduler'])
scheduling.SchedulerMixin = SchedulerMixin
safety = types.ModuleType('diffusers.pipelines.stable_diffusion.safety_checker')
safety.StableDiffusionSafetyChecker = type('StableDiffusionSafetyChecker', (Component,), {})
for name, module in [('diffusers', diffusers), ('transformers', transformers), ('diffusers.schedulers.scheduling_utils', scheduling), ('diffusers.pipelines.stable_diffusion.safety_checker', safety)]:
    sys.modules[name] = module

base = {'_class_name': 'StableDiffusionPipeline', '_diffusers_version': '0.37.0',
        'unet': ['diffusers', 'UNet2DConditionModel'], 'vae': ['diffusers', 'AutoencoderKL'],
        'text_encoder': ['transformers', 'CLIPTextModel'], 'tokenizer': ['transformers', 'CLIPTokenizer'],
        'scheduler': ['diffusers', 'EulerDiscreteScheduler'], 'safety_checker': [None, None]}
def write(root, config):
    (root / 'model_index.json').write_text(json.dumps(config))
def rejected(root, kind):
    before = len(calls)
    try: worker.load_diffusion_model(str(root))
    except worker.DiffusionLoadError as exc: assert exc.kind == kind, (exc.kind, kind)
    else: raise AssertionError('bundle admitted')
    assert len(calls) == before

with tempfile.TemporaryDirectory(prefix='rt01-loader-') as tmp:
    root = Path(tmp)
    for slot in ['unet', 'vae', 'text_encoder', 'tokenizer', 'scheduler', 'safety_checker', 'feature_extractor', 'image_encoder']:
        (root / slot).mkdir()
    (root / 'scheduler/scheduler_config.json').write_text('{"beta_start":0.001}')
    write(root, base)
    worker._resolve_device = lambda device: device
    worker._resolve_torch_dtype = lambda device, requested_dtype=None: requested_dtype or 'float32'
    worker.load_diffusion_model(str(root), device='cpu')
    first = worker._diffusion_pipeline
    assert type(first.components['scheduler']).__name__ == 'EulerDiscreteScheduler'
    assert first.components['scheduler'].config == {'beta_start': 0.001}
    count = len(calls)
    worker.load_diffusion_model(str(root / '.'), device='cpu')
    assert worker._diffusion_pipeline is first and len(calls) == count
    worker.load_diffusion_model(str(root), device='other')
    assert worker._diffusion_pipeline is not first
    count = len(calls)
    worker.load_diffusion_model(str(root), device='other', torch_dtype='float16')
    assert len(calls) > count
    (root / 'tokenizer/config.json').write_text('{"auto_map":{"AutoConfig":"evil.Config"}}')
    rejected(root, 'trust_policy_rejected')
    assert worker._diffusion_pipeline is None and worker._diffusion_admission is None
    (root / 'tokenizer/config.json').unlink()
    worker.load_diffusion_model(str(root), device='other', torch_dtype='float16')
    worker._diffusion_admission = None
    count = len(calls)
    worker.load_diffusion_model(str(root), device='other', torch_dtype='float16')
    assert len(calls) > count
    moved = root.with_name(root.name + '-moved')
    root.rename(moved)
    root.symlink_to(root, target_is_directory=True)
    try:
        rejected(root, 'model_load_failed')
        assert worker._diffusion_pipeline is None and worker._diffusion_admission is None
    finally:
        root.unlink()
        moved.rename(root)
    worker.load_diffusion_model(str(root))
    config = copy.deepcopy(base)
    config['_class_name'] = ['evil', 'Pipeline']
    write(root, config)
    rejected(root, 'trust_policy_rejected')
    assert worker._diffusion_pipeline is None and worker._diffusion_admission is None
    try: worker.generate_image(prompt='blocked')
    except RuntimeError as exc: assert 'No diffusion pipeline loaded' in str(exc)
    else: raise AssertionError('resident survived rejection')
    for mutation, kind in [
        ({'unet': ['evil', 'CustomModel']}, 'trust_policy_rejected'),
        ({'unet': ['diffusers', 'CustomModel']}, 'trust_policy_rejected'),
        ({'unet': ['diffusers', '../escape']}, 'invalid_request'),
        ({'unet': ['diffusers', '__class__']}, 'invalid_request'),
        ({'unet': ['diffusers', None]}, 'invalid_request'),
        ({'unet': ['diffusers', 'OtherBuiltIn']}, 'unsupported_task'),
        ({'_class_name': 'StableDiffusionXLPipeline'}, 'unsupported_task'),
        ({'requires_safety_checker': 'false'}, 'invalid_request'),
        ({'extra_encoder': ['diffusers', 'OtherBuiltIn']}, 'unsupported_task'),
    ]:
        write(root, dict(base, **mutation)); rejected(root, kind)
    config = dict(base); del config['unet']; write(root, config); rejected(root, 'invalid_request')
    for raw in ['[]', '{', '{"_class_name":"StableDiffusionPipeline","_class_name":"StableDiffusionPipeline"}']:
        (root / 'model_index.json').write_text(raw); rejected(root, 'invalid_request')
    write(root, base)
    (root / 'text_encoder/config.json').write_text('{"auto_map":{"AutoModel":"evil.Model"}}')
    rejected(root, 'trust_policy_rejected')
    (root / 'text_encoder/config.json').unlink()
    config = dict(base, scheduler=['diffusers', 'DDIMScheduler'],
                  safety_checker=['stable_diffusion', 'StableDiffusionSafetyChecker'],
                  feature_extractor=['transformers', 'CLIPImageProcessor'],
                  image_encoder=['transformers', 'CLIPVisionModelWithProjection'])
    write(root, config)
    (root / 'diffusers.py').write_text('raise AssertionError("local code imported")')
    worker.load_diffusion_model(str(root))
    assert type(worker._diffusion_pipeline.components['scheduler']).__name__ == 'DDIMScheduler'
    assert worker._diffusion_pipeline.components['safety_checker'] is not None
    worker.unload_diffusion_model()
    # A stubbed installed loader failure proves typed propagation, not format parsing.
    (root / 'unet/only.bin').write_bytes(b'not a pickle')
    try: worker.load_diffusion_model(str(root))
    except worker.DiffusionLoadError as exc: assert exc.kind == 'model_load_failed'
    else: raise AssertionError('component load failure was swallowed')
    assert worker._diffusion_admission is None
    (root / 'unet/only.bin').unlink()
    write(root, dict(base, _class_name=['evil', 'Pipeline']))
    single = json.loads(single_fixture)
    single['payload']['artifact_load_target']['local_load_path'] = str(root)
    response = json.loads(worker.generate_image_from_envelope(json.dumps(single)))
    assert response['error']['kind'] == 'trust_policy_rejected', response
    safe = root / 'safe'; safe.mkdir(); write(safe, base)
    batch = json.loads(batch_fixture)
    batch['payload']['members'][0]['request']['artifact_load_target']['local_load_path'] = str(safe)
    batch['payload']['members'][1]['request']['artifact_load_target']['local_load_path'] = str(root)
    before = len(calls)
    response = json.loads(worker.generate_image_batch_from_envelope(json.dumps(batch)))
    assert all(member['error']['kind'] == 'trust_policy_rejected' for member in response['result']['members']), response
    assert len(calls) == before
    write(root, base)
    diffusers.__version__ = 'unqualified'
    rejected(root, 'runtime_unavailable')
    diffusers.__version__ = '0.37.0'
    del safety_capability.check_torch_load_is_safe
    rejected(root, 'runtime_unavailable')
    def unsafe_torch(): raise ValueError('Torch below safety floor')
    safety_capability.check_torch_load_is_safe = unsafe_torch
    rejected(root, 'runtime_unavailable')
    safety_capability.check_torch_load_is_safe = lambda: None
    del diffusers.UNet2DConditionModel
    rejected(root, 'runtime_unavailable')
"#).unwrap();
        py.run(&source, Some(&locals), Some(&locals))
            .expect("real loader must enforce closed bundle admission");
    });
}

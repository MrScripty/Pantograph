use std::collections::BTreeSet;
use std::ffi::CString;

use super::pytorch_worker_contract::{
    PyTorchAudioTranscriptionRequest, PyTorchAudioTranscriptionResult, PyTorchClearKvCacheRequest,
    PyTorchGenerateTextRequest, PyTorchGenerateTextResult, PyTorchGetLoadedInfoRequest,
    PyTorchInitWorkerRequest, PyTorchRestoreKvCacheRequest, PyTorchSaveKvCacheRequest,
    PyTorchShutdownWorkerRequest, PyTorchTransformersLoadRequest, PyTorchTransformersModelLoader,
    PyTorchTransformersTrustPolicy, PyTorchTruncateKvCacheRequest, PyTorchUnloadModelRequest,
    PyTorchWorkerEnvelope, PyTorchWorkerError, PyTorchWorkerErrorKind, PyTorchWorkerFailure,
    PyTorchWorkerOperation, PyTorchWorkerResponse, PYTORCH_WORKER_CONTRACT_VERSION,
};
use super::*;
use crate::model_contracts::{
    CacheGenerationOptions, GenerationOptions, LengthGenerationOptions, ModelArtifactKind,
    ModelAuthTokenSource, ModelLoadCachePolicy, ModelLoadNetworkPolicy, ModelLoadSecurityPolicy,
    ModelRemoteCodePolicy, OptionSupportState, OutputGenerationOptions, ProcessorComponentKind,
    PumasModelRef, ResolvedModelPackageFacts, ResolvedModelSourceKind, SamplingGenerationOptions,
    SpecialTokenGenerationOptions, StoppingGenerationOptions, TaskEvidence,
};
use crate::types::{AudioTranscriptionRequest, EncodedAudio};

fn load_worker_contract_module<'py>(py: Python<'py>) -> Bound<'py, pyo3::types::PyModule> {
    let source = CString::new(include_str!("../../torch/worker_contract.py"))
        .expect("worker contract source should not contain nul bytes");
    pyo3::types::PyModule::from_code(py, &source, c"worker_contract.py", c"worker_contract")
        .expect("worker_contract module should load")
}

fn load_worker_module_with_stubbed_dependencies<'py>(
    py: Python<'py>,
) -> Bound<'py, pyo3::types::PyModule> {
    let setup = CString::new(
        r#"
import json
import sys
import types
from pathlib import Path

def _noop(*args, **kwargs):
    return None

for name in ["numpy", "soundfile"]:
    sys.modules[name] = types.ModuleType(name)

torch = types.ModuleType("torch")
torch.float16 = "float16"
torch.float32 = "float32"
torch.bfloat16 = "bfloat16"
torch.cuda = types.SimpleNamespace(is_available=lambda: False)
torch.backends = types.SimpleNamespace(mps=types.SimpleNamespace(is_available=lambda: False))
torch.device = lambda value: types.SimpleNamespace(type=str(value))
sys.modules["torch"] = torch

block_diffusion = types.ModuleType("block_diffusion")
block_diffusion._generate_dllm_masked = _noop
block_diffusion._generate_dllm_masked_streaming = lambda *args, **kwargs: iter(())
sys.modules["block_diffusion"] = block_diffusion

autoregressive = types.ModuleType("autoregressive")
for attr in [
    "_generate_autoregressive",
    "_continue_sdar_cached",
    "_generate_sdar_cached",
]:
    setattr(autoregressive, attr, _noop)
autoregressive._generate_autoregressive_streaming = lambda *args, **kwargs: iter(())
sys.modules["autoregressive"] = autoregressive

worker_runtime = types.ModuleType("worker_runtime")
worker_runtime._decode_base64_image = _noop
worker_runtime._detect_diffusion_load_overrides = lambda config: {}
worker_runtime._detect_model_type = lambda path: "text-generation"
worker_runtime._dtype_name = str
worker_runtime._encode_image = lambda image: "encoded"
worker_runtime._resolve_device = lambda device: torch.device("cpu")
worker_runtime._resolve_model_directory = lambda path: Path(path)
worker_runtime._resolve_torch_dtype = lambda device, requested_dtype=None: torch.float32
sys.modules["worker_runtime"] = worker_runtime

worker_transformers = types.ModuleType("worker_transformers")
worker_transformers.apply_compatibility_shims = _noop
sys.modules["worker_transformers"] = worker_transformers

worker_contract = types.ModuleType("worker_contract")
worker_contract.AUTOMATIC_SPEECH_RECOGNITION_LOADER = "automatic_speech_recognition"
worker_contract.CAUSAL_LM_LOADER = "causal_lm"
worker_contract.GENERATE_TEXT_STREAM_OPERATION = "generate_text_stream"

def _load_kwargs(envelope):
    decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
    payload = decoded.get("payload", {})
    profile = payload.get("task_profile") or {}
    return {
        "model_path": payload.get("entry_path"),
        "loader": profile.get("loader", "causal_lm"),
    }

worker_contract.load_transformers_model_kwargs_from_envelope = _load_kwargs
for attr in [
    "clear_kv_cache_kwargs_from_envelope",
    "generate_text_kwargs_from_envelope",
    "get_loaded_info_kwargs_from_envelope",
    "init_worker_kwargs_from_envelope",
    "restore_kv_cache_kwargs_from_envelope",
    "save_kv_cache_kwargs_from_envelope",
    "shutdown_worker_kwargs_from_envelope",
    "transcribe_audio_kwargs_from_envelope",
    "truncate_kv_cache_kwargs_from_envelope",
    "unload_model_kwargs_from_envelope",
]:
    setattr(worker_contract, attr, lambda envelope: {})
sys.modules["worker_contract"] = worker_contract
"#,
    )
    .expect("stub setup source should not contain nul bytes");
    py.run(&setup, None, None)
        .expect("stubbed worker dependencies should load");

    let source = CString::new(include_str!("../../torch/worker.py"))
        .expect("worker source should not contain nul bytes");
    pyo3::types::PyModule::from_code(
        py,
        &source,
        c"pantograph_torch_worker_test.py",
        c"pantograph_torch_worker_test",
    )
    .expect("worker module should load with stubs")
}

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
    assert!(caps.supports_task(InferenceTaskId::TextGeneration));
    assert!(caps.supports_task(InferenceTaskId::AudioTranscription));
    assert!(!caps.supports_task(InferenceTaskId::Embedding));
    assert!(caps.facts.runtime_variants.iter().any(|variant| {
        variant.runtime_variant_id.as_str() == "pytorch.cpu"
            && variant.device_class == InferenceDeviceClass::Cpu
            && variant.available
    }));
    assert!(caps.facts.runtime_variants.iter().any(|variant| {
        variant.runtime_variant_id.as_str() == "pytorch.cuda"
            && variant.device_class == InferenceDeviceClass::Cuda
            && !variant.available
    }));
    assert_eq!(
        caps.facts.features.kv_cache,
        BackendFeatureSupport::Supported
    );
}

#[test]
fn pytorch_device_probe_projects_cpu_and_cuda_runtime_variants() {
    let cpu_only_variants =
        PyTorchBackend::runtime_variants_from_device_probe(PyTorchDeviceProbeSnapshot::cpu_only());
    assert!(cpu_only_variants.iter().any(|variant| {
        variant.runtime_variant_id.as_str() == "pytorch.cpu"
            && variant.device_class == InferenceDeviceClass::Cpu
            && variant.available
    }));
    let unavailable_cuda = cpu_only_variants
        .iter()
        .find(|variant| variant.runtime_variant_id.as_str() == "pytorch.cuda")
        .expect("cuda variant");
    assert_eq!(unavailable_cuda.device_class, InferenceDeviceClass::Cuda);
    assert!(!unavailable_cuda.available);
    assert_eq!(
        unavailable_cuda.diagnostics[0].code,
        DeviceResolutionDiagnosticCode::CandidateUnavailable
    );

    let cuda_variants =
        PyTorchBackend::runtime_variants_from_device_probe(PyTorchDeviceProbeSnapshot {
            cuda_available: true,
            mps_available: false,
        });
    assert!(cuda_variants.iter().any(|variant| {
        variant.runtime_variant_id.as_str() == "pytorch.cuda"
            && variant.device_class == InferenceDeviceClass::Cuda
            && variant.available
            && variant.diagnostics.is_empty()
    }));
}

#[cfg(target_os = "macos")]
#[test]
fn pytorch_device_probe_projects_macos_mps_runtime_variant() {
    let variants = PyTorchBackend::runtime_variants_from_device_probe(PyTorchDeviceProbeSnapshot {
        cuda_available: false,
        mps_available: true,
    });

    assert!(variants.iter().any(|variant| {
        variant.runtime_variant_id.as_str() == "pytorch.mps"
            && variant.device_class == InferenceDeviceClass::Mps
            && variant.available
    }));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn pytorch_device_probe_excludes_mps_on_non_macos() {
    let variants = PyTorchBackend::runtime_variants_from_device_probe(PyTorchDeviceProbeSnapshot {
        cuda_available: false,
        mps_available: true,
    });

    assert!(!variants
        .iter()
        .any(|variant| variant.runtime_variant_id.as_str() == "pytorch.mps"));
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
fn test_stop_without_loaded_model_clears_ready_state() {
    let mut backend = PyTorchBackend::new();
    backend.ready = true;

    backend.stop();

    assert!(!backend.ready);
    assert!(backend.loaded_model.is_none());
}

#[tokio::test]
async fn test_health_check_returns_false_when_not_ready() {
    let backend = PyTorchBackend::new();

    assert!(!backend.health_check().await);
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

#[test]
fn test_pytorch_worker_load_envelope_decodes_fixture() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/load_transformers_model_request.json"
    );
    let envelope: PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest> =
        serde_json::from_str(fixture).expect("decode worker load fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-load-001");
    assert_eq!(
        envelope.operation,
        PyTorchWorkerOperation::LoadTransformersModel
    );
    assert_eq!(
        envelope
            .payload
            .model_ref
            .as_ref()
            .map(|value| value.model_id.as_str()),
        Some("pumas://models/tiny-causal")
    );
    assert_eq!(
        envelope.payload.artifact_kind,
        ModelArtifactKind::HfCompatibleDirectory
    );
    let model_source = envelope
        .payload
        .model_source
        .as_ref()
        .expect("load fixture should carry resolved model source");
    assert!(model_source.validate_for_backend_load().is_ok());
    assert_eq!(
        model_source.source_kind,
        ResolvedModelSourceKind::PumasResolved
    );
    assert_eq!(
        model_source.model_ref.as_ref(),
        envelope.payload.model_ref.as_ref()
    );
    assert_eq!(envelope.payload.task_id, InferenceTaskId::TextGeneration);
    let task_profile = envelope
        .payload
        .task_profile
        .as_ref()
        .expect("load fixture should carry a canonical task profile");
    assert_eq!(task_profile.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(task_profile.canonical_task_label, "text_generation");
    assert_eq!(
        task_profile.loader,
        PyTorchTransformersModelLoader::CausalLm
    );
    assert!(task_profile
        .required_components
        .contains(&ProcessorComponentKind::Tokenizer));
    assert_eq!(
        envelope.payload.device.as_ref().map(|id| id.as_str()),
        Some("cuda:0")
    );
    assert!(!envelope.payload.trust_policy.allow_remote_code);
    assert!(envelope.payload.trust_policy.local_files_only);
    assert_eq!(
        envelope.payload.trust_policy.cache_policy,
        ModelLoadCachePolicy::BackendDefault
    );
    assert_eq!(
        envelope.payload.trust_policy.auth_token_source,
        ModelAuthTokenSource::None
    );
    assert_eq!(
        envelope.payload.trust_policy.revision.as_deref(),
        Some("rev-1")
    );
    assert!(envelope.payload.trust_policy.code_revision.is_none());
    assert!(envelope.payload.trust_policy.accepted_sources.is_empty());

    PyTorchBackend::validate_transformers_load_envelope(&envelope)
        .expect("load fixture should validate");
}

#[test]
fn test_pytorch_worker_load_envelope_rejects_legacy_device_id() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pytorch_worker_contract/load_transformers_model_request.json"
    ))
    .expect("decode worker load fixture");
    value["payload"]["device"] = serde_json::json!("CUDA0");

    let error =
        serde_json::from_value::<PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest>>(value)
            .expect_err("legacy worker device id should fail contract decoding");
    assert!(error.to_string().contains("invalid identifier shape"));
}

#[test]
fn test_pytorch_worker_load_envelope_rejects_auto_device_field() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pytorch_worker_contract/load_transformers_model_request.json"
    ))
    .expect("decode worker load fixture");
    value["payload"]["device"] = serde_json::json!("auto");

    let error =
        serde_json::from_value::<PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest>>(value)
            .expect_err("auto worker device should be omitted, not sent as a device id");
    assert!(error.to_string().contains("reserved identifier"));
}

#[test]
fn test_pytorch_worker_load_envelope_tolerates_additive_fields() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pytorch_worker_contract/load_transformers_model_request.json"
    ))
    .expect("decode worker load fixture");
    let object = value.as_object_mut().expect("fixture is object");
    object.insert(
        "producer_trace".to_string(),
        serde_json::json!({"span_id": "trace-load"}),
    );
    object
        .get_mut("cancellation")
        .and_then(|value| value.as_object_mut())
        .expect("cancellation is object")
        .insert(
            "future_reason".to_string(),
            serde_json::json!("client_cancel"),
        );
    let payload = object
        .get_mut("payload")
        .and_then(|value| value.as_object_mut())
        .expect("payload is object");
    payload.insert(
        "future_payload_field".to_string(),
        serde_json::json!({"ignored": true}),
    );
    payload
        .get_mut("task_profile")
        .and_then(|value| value.as_object_mut())
        .expect("task profile is object")
        .insert(
            "future_task_profile_field".to_string(),
            serde_json::json!("ignored"),
        );
    payload
        .get_mut("trust_policy")
        .and_then(|value| value.as_object_mut())
        .expect("trust policy is object")
        .insert(
            "future_trust_field".to_string(),
            serde_json::json!("ignored"),
        );

    let envelope: PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest> =
        serde_json::from_value(value).expect("additive fields should be ignored");

    assert_eq!(envelope.request_id, "req-load-001");
    assert_eq!(envelope.payload.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(
        envelope
            .payload
            .task_profile
            .as_ref()
            .map(|profile| profile.loader),
        Some(PyTorchTransformersModelLoader::CausalLm)
    );
}

#[test]
fn test_python_worker_contract_projects_task_profile_loader() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let envelope = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-asr-load",
            "operation": "load_transformers_model",
            "payload": {
                "model_ref": {"model_id": "asr/example/tiny"},
                "artifact_kind": "hf_compatible_directory",
                "entry_path": "/models/asr",
                "task_id": "audio_transcription",
                "task_profile": {
                    "task_id": "audio_transcription",
                    "canonical_task_label": "audio_transcription",
                    "loader": "automatic_speech_recognition",
                    "required_components": ["audio_feature_extractor", "tokenizer"]
                },
                "trust_policy": {
                    "allow_remote_code": false,
                    "local_files_only": true,
                    "cache_policy": "backend_default",
                    "auth_token_source": "none"
                }
            }
        });

        let kwargs = module
            .call_method1(
                "load_transformers_model_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect("worker contract should project load kwargs");
        let loader = kwargs
            .get_item("loader")
            .expect("loader key should be readable")
            .extract::<String>()
            .expect("loader should be a string");

        assert_eq!(loader, "automatic_speech_recognition");
    });
}

#[test]
fn test_python_worker_contract_tolerates_additive_load_fields() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let envelope = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-additive-load",
            "operation": "load_transformers_model",
            "producer_trace": {"span_id": "trace-load"},
            "cancellation": {"token": "cancel-1", "future_reason": "ignored"},
            "payload": {
                "model_ref": {"model_id": "llm/example/tiny"},
                "artifact_kind": "hf_compatible_directory",
                "entry_path": "/models/tiny",
                "task_id": "text_generation",
                "future_payload_field": {"ignored": true},
                "task_profile": {
                    "task_id": "text_generation",
                    "canonical_task_label": "text_generation",
                    "loader": "causal_lm",
                    "required_components": ["tokenizer"],
                    "future_task_profile_field": "ignored"
                },
                "trust_policy": {
                    "allow_remote_code": false,
                    "local_files_only": true,
                    "cache_policy": "backend_default",
                    "auth_token_source": "none",
                    "future_trust_field": "ignored"
                }
            }
        });

        let kwargs = module
            .call_method1(
                "load_transformers_model_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect("additive load fields should be ignored");

        assert_eq!(
            kwargs
                .get_item("model_path")
                .expect("model path key should exist")
                .extract::<String>()
                .expect("model path should be string"),
            "/models/tiny"
        );
        assert_eq!(
            kwargs
                .get_item("loader")
                .expect("loader key should exist")
                .extract::<String>()
                .expect("loader should be string"),
            "causal_lm"
        );
    });
}

#[test]
fn test_python_worker_contract_rejects_unsupported_task_profile_loader() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let envelope = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-load",
            "operation": "load_transformers_model",
            "payload": {
                "model_ref": {"model_id": "vision/example/tiny"},
                "artifact_kind": "hf_compatible_directory",
                "entry_path": "/models/vision",
                "task_id": "image_understanding",
                "task_profile": {
                    "task_id": "image_understanding",
                    "canonical_task_label": "image_understanding",
                    "loader": "image_to_text",
                    "required_components": ["image_processor", "tokenizer"]
                },
                "trust_policy": {
                    "allow_remote_code": false,
                    "local_files_only": true,
                    "cache_policy": "backend_default",
                    "auth_token_source": "none"
                }
            }
        });

        let error = module
            .call_method1(
                "load_transformers_model_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect_err("unsupported loaders should fail validation");

        assert!(error
            .to_string()
            .contains("Unsupported PyTorch worker Transformers loader: image_to_text"));
    });
}

#[test]
fn test_python_worker_contract_rejects_missing_load_entry_path() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let envelope = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-load-missing-entry",
            "operation": "load_transformers_model",
            "payload": {
                "artifact_kind": "hf_compatible_directory",
                "entry_path": " ",
                "task_id": "text_generation",
                "task_profile": {
                    "task_id": "text_generation",
                    "canonical_task_label": "text_generation",
                    "loader": "causal_lm"
                },
                "trust_policy": {
                    "allow_remote_code": false,
                    "local_files_only": true
                }
            }
        });

        let error = module
            .call_method1(
                "load_transformers_model_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect_err("empty load entry_path should fail validation");

        assert!(error
            .to_string()
            .contains("payload.entry_path must be a non-empty string"));
    });
}

#[test]
fn test_python_worker_load_value_error_after_projection_maps_to_model_load_failed() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_stubbed_dependencies(py);
        let patch = CString::new(
            r#"
def fail_load_model(**kwargs):
    raise ValueError("Transformers tokenizer rejected config")
module.load_model = fail_load_model
"#,
        )
        .expect("patch source should not contain nul bytes");
        let globals = pyo3::types::PyDict::new(py);
        globals
            .set_item("module", &module)
            .expect("module global should be set");
        py.run(&patch, Some(&globals), None)
            .expect("worker load_model should be patched");

        let envelope = serde_json::json!({
            "request_id": "req-worker-load-value-error",
            "payload": {
                "entry_path": "/models/tiny",
                "task_profile": {"loader": "causal_lm"}
            }
        });
        let response_json = module
            .call_method1(
                "load_transformers_model_from_envelope",
                (envelope.to_string(),),
            )
            .expect("worker load should return a JSON response")
            .extract::<String>()
            .expect("worker response should be JSON text");
        let response: serde_json::Value =
            serde_json::from_str(&response_json).expect("worker response should decode");

        assert_eq!(response["status"], serde_json::json!("error"));
        assert_eq!(
            response["request_id"],
            serde_json::json!("req-worker-load-value-error")
        );
        assert_eq!(
            response["error"]["kind"],
            serde_json::json!("model_load_failed")
        );
        assert_eq!(
            response["error"]["canonical_code"],
            serde_json::json!("pytorch_worker_model_load_failed")
        );
        assert!(response["error"]["message"]
            .as_str()
            .is_some_and(|message| { message.contains("Transformers tokenizer rejected config") }));
    });
}

#[test]
fn test_python_worker_load_unexpected_loader_exception_maps_to_model_load_failed() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_stubbed_dependencies(py);
        let patch = CString::new(
            r#"
def fail_load_model(**kwargs):
    raise KeyError("missing architectures")
module.load_model = fail_load_model
"#,
        )
        .expect("patch source should not contain nul bytes");
        let globals = pyo3::types::PyDict::new(py);
        globals
            .set_item("module", &module)
            .expect("module global should be set");
        py.run(&patch, Some(&globals), None)
            .expect("worker load_model should be patched");

        let envelope = serde_json::json!({
            "request_id": "req-worker-load-key-error",
            "payload": {
                "entry_path": "/models/tiny",
                "task_profile": {"loader": "causal_lm"}
            }
        });
        let response_json = module
            .call_method1(
                "load_transformers_model_from_envelope",
                (envelope.to_string(),),
            )
            .expect("worker load should return a JSON response")
            .extract::<String>()
            .expect("worker response should be JSON text");
        let response: serde_json::Value =
            serde_json::from_str(&response_json).expect("worker response should decode");

        assert_eq!(response["status"], serde_json::json!("error"));
        assert_eq!(
            response["request_id"],
            serde_json::json!("req-worker-load-key-error")
        );
        assert_eq!(
            response["error"]["kind"],
            serde_json::json!("model_load_failed")
        );
        assert_eq!(
            response["error"]["canonical_code"],
            serde_json::json!("pytorch_worker_model_load_failed")
        );
        assert!(response["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("missing architectures")));
    });
}

#[test]
fn test_python_worker_load_invalid_loader_stays_invalid_request() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_stubbed_dependencies(py);
        let envelope = serde_json::json!({
            "request_id": "req-worker-load-invalid-loader",
            "payload": {
                "entry_path": "/models/tiny",
                "task_profile": {"loader": "image_to_text"}
            }
        });
        let response_json = module
            .call_method1(
                "load_transformers_model_from_envelope",
                (envelope.to_string(),),
            )
            .expect("worker load should return a JSON response")
            .extract::<String>()
            .expect("worker response should be JSON text");
        let response: serde_json::Value =
            serde_json::from_str(&response_json).expect("worker response should decode");

        assert_eq!(response["status"], serde_json::json!("error"));
        assert_eq!(
            response["request_id"],
            serde_json::json!("req-worker-load-invalid-loader")
        );
        assert_eq!(
            response["error"]["kind"],
            serde_json::json!("invalid_request")
        );
        assert_eq!(
            response["error"]["canonical_code"],
            serde_json::json!("pytorch_worker_invalid_load_request")
        );
        assert!(response["error"]["message"]
            .as_str()
            .is_some_and(|message| {
                message.contains("Unsupported PyTorch worker Transformers loader")
            }));
    });
}

#[test]
fn test_pytorch_worker_generate_text_envelope_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_text_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker generate fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-generate-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::GenerateText);
    assert_eq!(
        envelope.payload.prompt,
        "Explain why bounded diagnostics matter."
    );
    assert_eq!(
        envelope.payload.system_prompt.as_deref(),
        Some("Be concise.")
    );
    assert_eq!(envelope.payload.max_tokens, 64);
    assert_eq!(
        envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(40)
    );

    PyTorchBackend::validate_generate_text_envelope(&envelope)
        .expect("generate_text fixture should validate");
}

#[test]
fn test_pytorch_worker_generate_text_envelope_tolerates_additive_fields() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_text_request.json"
    ))
    .expect("decode worker generate fixture");
    let object = value.as_object_mut().expect("fixture is object");
    object.insert(
        "producer_trace".to_string(),
        serde_json::json!("trace-generate"),
    );
    object
        .get_mut("payload")
        .and_then(|value| value.as_object_mut())
        .expect("payload is object")
        .insert(
            "future_payload_field".to_string(),
            serde_json::json!("ignored"),
        );

    let envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_value(value).expect("additive fields should be ignored");

    assert_eq!(envelope.request_id, "req-generate-001");
    assert_eq!(
        envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(40)
    );
    PyTorchBackend::validate_generate_text_envelope(&envelope)
        .expect("additive fields should not affect validation");
}

#[test]
fn test_pytorch_worker_generate_text_dllm_envelope_decodes_backend_local_controls() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_text_dllm_request.json"
    );
    let envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker dLLM generate fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-generate-dllm-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::GenerateText);
    assert_eq!(
        envelope.payload.masked_prompt_json.as_deref(),
        Some(
            "{\"segments\":[{\"kind\":\"known\",\"text\":\"Plan:\"},{\"kind\":\"mask\",\"token_count\":8}]}"
        )
    );
    assert_eq!(envelope.payload.denoising_steps, Some(8));
    assert_eq!(envelope.payload.block_length, Some(64));
    assert_eq!(
        envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(10)
    );

    PyTorchBackend::validate_generate_text_envelope(&envelope)
        .expect("dLLM generate_text fixture should validate");
}

#[test]
fn test_python_worker_contract_projects_dllm_generation_controls() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_text_dllm_request.json"
        );

        let kwargs = module
            .call_method1("generate_text_kwargs_from_envelope", (fixture,))
            .expect("dLLM worker envelope should project to kwargs");

        let masked_prompt_json = kwargs
            .get_item("masked_prompt_json")
            .expect("masked prompt key should exist")
            .extract::<String>()
            .expect("masked prompt should be a string");
        let denoising_steps = kwargs
            .get_item("denoising_steps")
            .expect("denoising steps key should exist")
            .extract::<i64>()
            .expect("denoising steps should be an integer");
        let block_length = kwargs
            .get_item("block_length")
            .expect("block length key should exist")
            .extract::<i64>()
            .expect("block length should be an integer");

        assert!(masked_prompt_json.contains("\"mask\""));
        assert_eq!(denoising_steps, 8);
        assert_eq!(block_length, 64);
    });
}

#[test]
fn test_python_worker_contract_tolerates_additive_generate_fields() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let envelope = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-additive-generate",
            "operation": "generate_text",
            "producer_trace": {"span_id": "trace-generate"},
            "payload": {
                "prompt": "Explain additive worker fields.",
                "max_tokens": 24,
                "temperature": 0.2,
                "top_p": 0.8,
                "future_payload_field": "ignored",
                "transformers_kwargs": {
                    "top_k": 12
                }
            }
        });

        let kwargs = module
            .call_method1(
                "generate_text_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect("additive generate fields should be ignored");

        assert_eq!(
            kwargs
                .get_item("prompt")
                .expect("prompt key should exist")
                .extract::<String>()
                .expect("prompt should be string"),
            "Explain additive worker fields."
        );
        assert_eq!(
            kwargs
                .get_item("top_k")
                .expect("top_k key should exist")
                .extract::<i64>()
                .expect("top_k should be integer"),
            12
        );
    });
}

#[test]
fn test_pytorch_worker_generate_text_stream_envelope_decodes_fixture() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_text_stream_request.json"
    );
    let envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker stream fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-generate-stream-001");
    assert_eq!(
        envelope.operation,
        PyTorchWorkerOperation::GenerateTextStream
    );
    assert!(envelope.cancellation.drop_stream_cancels);
    assert_eq!(
        envelope.payload.prompt,
        "Stream a short answer about adapters."
    );
    assert_eq!(
        envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(20)
    );

    PyTorchBackend::validate_generate_text_stream_envelope(&envelope)
        .expect("generate_text_stream fixture should validate");
}

#[test]
fn test_pytorch_worker_unload_envelope_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/unload_model_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchUnloadModelRequest> =
        serde_json::from_str(fixture).expect("decode worker unload fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-unload-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::UnloadModel);

    PyTorchBackend::validate_unload_model_envelope(&envelope)
        .expect("unload fixture should validate");
}

#[test]
fn test_pytorch_worker_unload_envelope_json_uses_unload_operation() {
    let envelope_json = PyTorchBackend::unload_model_envelope_json("req-stop-unload-001")
        .expect("unload envelope should encode");
    let envelope: PyTorchWorkerEnvelope<PyTorchUnloadModelRequest> =
        serde_json::from_str(&envelope_json).expect("decode encoded unload envelope");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-stop-unload-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::UnloadModel);
    PyTorchBackend::validate_unload_model_envelope(&envelope)
        .expect("encoded unload envelope should validate");
}

#[test]
fn test_pytorch_worker_init_envelope_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/init_worker_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchInitWorkerRequest> =
        serde_json::from_str(fixture).expect("decode worker init fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-init-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::InitWorker);

    validate_init_worker_envelope(&envelope).expect("init fixture should validate");
}

#[test]
fn test_pytorch_worker_init_envelope_json_uses_init_operation() {
    let envelope_json =
        init_worker_envelope_json("req-health-init-001").expect("init envelope should encode");
    let envelope: PyTorchWorkerEnvelope<PyTorchInitWorkerRequest> =
        serde_json::from_str(&envelope_json).expect("decode encoded init envelope");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-health-init-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::InitWorker);
    validate_init_worker_envelope(&envelope).expect("encoded init envelope should validate");
}

#[test]
fn test_pytorch_worker_shutdown_envelope_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/shutdown_worker_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchShutdownWorkerRequest> =
        serde_json::from_str(fixture).expect("decode worker shutdown fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-shutdown-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::ShutdownWorker);

    validate_shutdown_worker_envelope(&envelope).expect("shutdown fixture should validate");
}

#[test]
fn test_pytorch_worker_shutdown_envelope_json_uses_shutdown_operation() {
    let envelope_json = shutdown_worker_envelope_json("req-stop-shutdown-001")
        .expect("shutdown envelope should encode");
    let envelope: PyTorchWorkerEnvelope<PyTorchShutdownWorkerRequest> =
        serde_json::from_str(&envelope_json).expect("decode encoded shutdown envelope");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-stop-shutdown-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::ShutdownWorker);
    validate_shutdown_worker_envelope(&envelope)
        .expect("encoded shutdown envelope should validate");
}

#[test]
fn test_pytorch_worker_init_envelope_rejects_wrong_operation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/init_worker_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchInitWorkerRequest> =
        serde_json::from_str(fixture).expect("decode worker init fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match validate_init_worker_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_init_envelope_rejects_wrong_contract_version() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/init_worker_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchInitWorkerRequest> =
        serde_json::from_str(fixture).expect("decode worker init fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match validate_init_worker_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("init_worker envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_get_loaded_info_envelope_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/get_loaded_info_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchGetLoadedInfoRequest> =
        serde_json::from_str(fixture).expect("decode worker get_loaded_info fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-loaded-info-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::GetLoadedInfo);

    validate_get_loaded_info_envelope(&envelope).expect("get_loaded_info fixture should validate");
}

#[test]
fn test_pytorch_worker_get_loaded_info_envelope_rejects_wrong_operation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/get_loaded_info_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGetLoadedInfoRequest> =
        serde_json::from_str(fixture).expect("decode worker get_loaded_info fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match validate_get_loaded_info_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_get_loaded_info_envelope_rejects_wrong_contract_version() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/get_loaded_info_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGetLoadedInfoRequest> =
        serde_json::from_str(fixture).expect("decode worker get_loaded_info fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match validate_get_loaded_info_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("get_loaded_info envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_clear_kv_cache_envelope_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/clear_kv_cache_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchClearKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker clear_kv_cache fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-clear-kv-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::ClearKvCache);

    validate_clear_kv_cache_envelope(&envelope).expect("clear_kv_cache fixture should validate");
}

#[test]
fn test_pytorch_worker_clear_kv_cache_envelope_rejects_wrong_operation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/clear_kv_cache_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchClearKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker clear_kv_cache fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match validate_clear_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_clear_kv_cache_envelope_rejects_wrong_contract_version() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/clear_kv_cache_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchClearKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker clear_kv_cache fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match validate_clear_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("clear_kv_cache envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_save_kv_cache_envelope_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/save_kv_cache_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchSaveKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker save_kv_cache fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-save-kv-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::SaveKvCache);
    assert_eq!(envelope.payload.path, "/tmp/pantograph-kv-save.bin");

    validate_save_kv_cache_envelope(&envelope).expect("save_kv_cache fixture should validate");
}

#[test]
fn test_pytorch_worker_save_kv_cache_envelope_rejects_wrong_operation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/save_kv_cache_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchSaveKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker save_kv_cache fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match validate_save_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_save_kv_cache_envelope_rejects_wrong_contract_version() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/save_kv_cache_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchSaveKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker save_kv_cache fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match validate_save_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("save_kv_cache envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_save_kv_cache_envelope_rejects_empty_path() {
    let envelope = save_kv_cache_envelope("req-save-kv-empty-path", " ");

    match validate_save_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("save_kv_cache envelope path must be non-empty"));
        }
        other => panic!("expected empty-path config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_restore_kv_cache_envelope_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/restore_kv_cache_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchRestoreKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker restore_kv_cache fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-restore-kv-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::RestoreKvCache);
    assert_eq!(envelope.payload.path, "/tmp/pantograph-kv-restore.bin");

    validate_restore_kv_cache_envelope(&envelope)
        .expect("restore_kv_cache fixture should validate");
}

#[test]
fn test_pytorch_worker_restore_kv_cache_envelope_rejects_wrong_operation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/restore_kv_cache_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchRestoreKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker restore_kv_cache fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match validate_restore_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_restore_kv_cache_envelope_rejects_wrong_contract_version() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/restore_kv_cache_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchRestoreKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker restore_kv_cache fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match validate_restore_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("restore_kv_cache envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_restore_kv_cache_envelope_rejects_empty_path() {
    let envelope = restore_kv_cache_envelope("req-restore-kv-empty-path", " ");

    match validate_restore_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("restore_kv_cache envelope path must be non-empty"));
        }
        other => panic!("expected empty-path config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_truncate_kv_cache_envelope_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/truncate_kv_cache_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchTruncateKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker truncate_kv_cache fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-truncate-kv-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::TruncateKvCache);
    assert_eq!(envelope.payload.path, "/tmp/pantograph-kv-truncate.bin");
    assert_eq!(envelope.payload.token_position, 4);

    validate_truncate_kv_cache_envelope(&envelope)
        .expect("truncate_kv_cache fixture should validate");
}

#[test]
fn test_pytorch_worker_truncate_kv_cache_envelope_rejects_wrong_operation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/truncate_kv_cache_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchTruncateKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker truncate_kv_cache fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match validate_truncate_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_truncate_kv_cache_envelope_rejects_wrong_contract_version() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/truncate_kv_cache_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchTruncateKvCacheRequest> =
        serde_json::from_str(fixture).expect("decode worker truncate_kv_cache fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match validate_truncate_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("truncate_kv_cache envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_truncate_kv_cache_envelope_rejects_empty_path() {
    let envelope = truncate_kv_cache_envelope("req-truncate-kv-empty-path", " ", 4);

    match validate_truncate_kv_cache_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("truncate_kv_cache envelope path must be non-empty"));
        }
        other => panic!("expected empty-path config error, got {other:?}"),
    }
}

#[test]
fn test_python_worker_contract_projects_unload_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture =
            include_str!("../../tests/fixtures/pytorch_worker_contract/unload_model_request.json");

        let kwargs = module
            .call_method1("unload_model_kwargs_from_envelope", (fixture,))
            .expect("unload worker envelope should project to kwargs");
        let len = kwargs.len().expect("kwargs length should be readable");

        assert_eq!(len, 0);
    });
}

#[test]
fn test_python_worker_contract_projects_init_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture =
            include_str!("../../tests/fixtures/pytorch_worker_contract/init_worker_request.json");

        let kwargs = module
            .call_method1("init_worker_kwargs_from_envelope", (fixture,))
            .expect("init worker envelope should project to kwargs");
        let len = kwargs.len().expect("kwargs length should be readable");

        assert_eq!(len, 0);
    });
}

#[test]
fn test_python_worker_contract_projects_shutdown_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/shutdown_worker_request.json"
        );

        let kwargs = module
            .call_method1("shutdown_worker_kwargs_from_envelope", (fixture,))
            .expect("shutdown worker envelope should project to kwargs");
        let len = kwargs.len().expect("kwargs length should be readable");

        assert_eq!(len, 0);
    });
}

#[test]
fn test_python_worker_shutdown_from_envelope_returns_structured_success() {
    Python::with_gil(|py| {
        let module = load_worker_module_with_stubbed_dependencies(py);
        let fixture = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/shutdown_worker_request.json"
        );

        let response_json = module
            .call_method1("shutdown_worker_from_envelope", (fixture,))
            .and_then(|value| value.extract::<String>())
            .expect("shutdown worker envelope should return JSON response");
        let response: serde_json::Value =
            serde_json::from_str(&response_json).expect("shutdown response should be JSON");

        assert_eq!(response["status"], serde_json::json!("ok"));
        assert_eq!(
            response["request_id"],
            serde_json::json!("req-shutdown-001")
        );
        assert_eq!(response["result"]["shutdown"], serde_json::json!(true));
    });
}

#[test]
fn test_python_worker_contract_rejects_invalid_init_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let wrong_operation = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-init-operation",
            "operation": "generate_text",
            "payload": {}
        });

        let error = module
            .call_method1(
                "init_worker_kwargs_from_envelope",
                (wrong_operation.to_string(),),
            )
            .expect_err("wrong init_worker operation should fail validation");

        assert!(error
            .to_string()
            .contains("Unexpected PyTorch worker operation for init_worker"));

        let wrong_version = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION + 1,
            "request_id": "req-invalid-init-version",
            "operation": "init_worker",
            "payload": {}
        });

        let error = module
            .call_method1(
                "init_worker_kwargs_from_envelope",
                (wrong_version.to_string(),),
            )
            .expect_err("wrong init_worker contract version should fail validation");

        assert!(error
            .to_string()
            .contains("Unsupported PyTorch worker contract_version"));
    });
}

#[test]
fn test_python_worker_contract_rejects_invalid_shutdown_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let wrong_operation = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-shutdown-operation",
            "operation": "init_worker",
            "payload": {}
        });

        let error = module
            .call_method1(
                "shutdown_worker_kwargs_from_envelope",
                (wrong_operation.to_string(),),
            )
            .expect_err("wrong shutdown_worker operation should fail validation");

        assert!(error
            .to_string()
            .contains("Unexpected PyTorch worker operation for shutdown_worker"));

        let wrong_version = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION + 1,
            "request_id": "req-invalid-shutdown-version",
            "operation": "shutdown_worker",
            "payload": {}
        });

        let error = module
            .call_method1(
                "shutdown_worker_kwargs_from_envelope",
                (wrong_version.to_string(),),
            )
            .expect_err("wrong shutdown_worker contract version should fail validation");

        assert!(error
            .to_string()
            .contains("Unsupported PyTorch worker contract_version"));
    });
}

#[test]
fn test_python_worker_contract_projects_get_loaded_info_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/get_loaded_info_request.json"
        );

        let kwargs = module
            .call_method1("get_loaded_info_kwargs_from_envelope", (fixture,))
            .expect("get_loaded_info worker envelope should project to kwargs");
        let len = kwargs.len().expect("kwargs length should be readable");

        assert_eq!(len, 0);
    });
}

#[test]
fn test_python_worker_contract_rejects_invalid_get_loaded_info_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let wrong_operation = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-loaded-info-operation",
            "operation": "generate_text",
            "payload": {}
        });

        let error = module
            .call_method1(
                "get_loaded_info_kwargs_from_envelope",
                (wrong_operation.to_string(),),
            )
            .expect_err("wrong get_loaded_info operation should fail validation");

        assert!(error
            .to_string()
            .contains("Unexpected PyTorch worker operation for get_loaded_info"));

        let wrong_version = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION + 1,
            "request_id": "req-invalid-loaded-info-version",
            "operation": "get_loaded_info",
            "payload": {}
        });

        let error = module
            .call_method1(
                "get_loaded_info_kwargs_from_envelope",
                (wrong_version.to_string(),),
            )
            .expect_err("wrong get_loaded_info contract version should fail validation");

        assert!(error
            .to_string()
            .contains("Unsupported PyTorch worker contract_version"));
    });
}

#[test]
fn test_python_worker_contract_projects_clear_kv_cache_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/clear_kv_cache_request.json"
        );

        let kwargs = module
            .call_method1("clear_kv_cache_kwargs_from_envelope", (fixture,))
            .expect("clear_kv_cache worker envelope should project to kwargs");
        let len = kwargs.len().expect("kwargs length should be readable");

        assert_eq!(len, 0);
    });
}

#[test]
fn test_python_worker_contract_rejects_invalid_clear_kv_cache_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let wrong_operation = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-clear-kv-operation",
            "operation": "generate_text",
            "payload": {}
        });

        let error = module
            .call_method1(
                "clear_kv_cache_kwargs_from_envelope",
                (wrong_operation.to_string(),),
            )
            .expect_err("wrong clear_kv_cache operation should fail validation");

        assert!(error
            .to_string()
            .contains("Unexpected PyTorch worker operation for clear_kv_cache"));

        let wrong_version = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION + 1,
            "request_id": "req-invalid-clear-kv-version",
            "operation": "clear_kv_cache",
            "payload": {}
        });

        let error = module
            .call_method1(
                "clear_kv_cache_kwargs_from_envelope",
                (wrong_version.to_string(),),
            )
            .expect_err("wrong clear_kv_cache contract version should fail validation");

        assert!(error
            .to_string()
            .contains("Unsupported PyTorch worker contract_version"));
    });
}

#[test]
fn test_python_worker_contract_projects_save_kv_cache_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture =
            include_str!("../../tests/fixtures/pytorch_worker_contract/save_kv_cache_request.json");

        let kwargs = module
            .call_method1("save_kv_cache_kwargs_from_envelope", (fixture,))
            .expect("save_kv_cache worker envelope should project to kwargs");

        assert_eq!(
            kwargs
                .get_item("path")
                .expect("path item should be readable")
                .extract::<String>()
                .expect("path should extract"),
            "/tmp/pantograph-kv-save.bin"
        );
    });
}

#[test]
fn test_python_worker_contract_rejects_invalid_save_kv_cache_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let wrong_operation = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-save-kv-operation",
            "operation": "generate_text",
            "payload": {"path": "/tmp/pantograph-kv-save.bin"}
        });

        let error = module
            .call_method1(
                "save_kv_cache_kwargs_from_envelope",
                (wrong_operation.to_string(),),
            )
            .expect_err("wrong save_kv_cache operation should fail validation");

        assert!(error
            .to_string()
            .contains("Unexpected PyTorch worker operation for save_kv_cache"));

        let empty_path = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-save-kv-path",
            "operation": "save_kv_cache",
            "payload": {"path": " "}
        });

        let error = module
            .call_method1(
                "save_kv_cache_kwargs_from_envelope",
                (empty_path.to_string(),),
            )
            .expect_err("empty save_kv_cache path should fail validation");

        assert!(error
            .to_string()
            .contains("payload.path must be a non-empty string"));
    });
}

#[test]
fn test_python_worker_contract_projects_restore_kv_cache_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/restore_kv_cache_request.json"
        );

        let kwargs = module
            .call_method1("restore_kv_cache_kwargs_from_envelope", (fixture,))
            .expect("restore_kv_cache worker envelope should project to kwargs");

        assert_eq!(
            kwargs
                .get_item("path")
                .expect("path item should be readable")
                .extract::<String>()
                .expect("path should extract"),
            "/tmp/pantograph-kv-restore.bin"
        );
    });
}

#[test]
fn test_python_worker_contract_rejects_invalid_restore_kv_cache_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let wrong_operation = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-restore-kv-operation",
            "operation": "generate_text",
            "payload": {"path": "/tmp/pantograph-kv-restore.bin"}
        });

        let error = module
            .call_method1(
                "restore_kv_cache_kwargs_from_envelope",
                (wrong_operation.to_string(),),
            )
            .expect_err("wrong restore_kv_cache operation should fail validation");

        assert!(error
            .to_string()
            .contains("Unexpected PyTorch worker operation for restore_kv_cache"));

        let empty_path = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-restore-kv-path",
            "operation": "restore_kv_cache",
            "payload": {"path": " "}
        });

        let error = module
            .call_method1(
                "restore_kv_cache_kwargs_from_envelope",
                (empty_path.to_string(),),
            )
            .expect_err("empty restore_kv_cache path should fail validation");

        assert!(error
            .to_string()
            .contains("payload.path must be a non-empty string"));
    });
}

#[test]
fn test_python_worker_contract_projects_truncate_kv_cache_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/truncate_kv_cache_request.json"
        );

        let kwargs = module
            .call_method1("truncate_kv_cache_kwargs_from_envelope", (fixture,))
            .expect("truncate_kv_cache worker envelope should project to kwargs");

        assert_eq!(
            kwargs
                .get_item("path")
                .expect("path item should be readable")
                .extract::<String>()
                .expect("path should extract"),
            "/tmp/pantograph-kv-truncate.bin"
        );
        assert_eq!(
            kwargs
                .get_item("token_position")
                .expect("token_position item should be readable")
                .extract::<usize>()
                .expect("token_position should extract"),
            4
        );
    });
}

#[test]
fn test_python_worker_contract_rejects_invalid_truncate_kv_cache_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let wrong_operation = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-truncate-kv-operation",
            "operation": "generate_text",
            "payload": {"path": "/tmp/pantograph-kv-truncate.bin", "token_position": 4}
        });

        let error = module
            .call_method1(
                "truncate_kv_cache_kwargs_from_envelope",
                (wrong_operation.to_string(),),
            )
            .expect_err("wrong truncate_kv_cache operation should fail validation");

        assert!(error
            .to_string()
            .contains("Unexpected PyTorch worker operation for truncate_kv_cache"));

        let negative_position = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-truncate-kv-position",
            "operation": "truncate_kv_cache",
            "payload": {"path": "/tmp/pantograph-kv-truncate.bin", "token_position": -1}
        });

        let error = module
            .call_method1(
                "truncate_kv_cache_kwargs_from_envelope",
                (negative_position.to_string(),),
            )
            .expect_err("negative truncate_kv_cache token_position should fail validation");

        assert!(error
            .to_string()
            .contains("payload.token_position must be a non-negative integer"));
    });
}

#[test]
fn test_python_worker_contract_rejects_invalid_unload_envelope() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let wrong_operation = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-unload-operation",
            "operation": "generate_text",
            "payload": {}
        });

        let error = module
            .call_method1(
                "unload_model_kwargs_from_envelope",
                (wrong_operation.to_string(),),
            )
            .expect_err("wrong unload operation should fail validation");

        assert!(error
            .to_string()
            .contains("Unexpected PyTorch worker operation for unload"));

        let wrong_version = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION + 1,
            "request_id": "req-invalid-unload-version",
            "operation": "unload_model",
            "payload": {}
        });

        let error = module
            .call_method1(
                "unload_model_kwargs_from_envelope",
                (wrong_version.to_string(),),
            )
            .expect_err("wrong unload contract version should fail validation");

        assert!(error
            .to_string()
            .contains("Unsupported PyTorch worker contract_version"));
    });
}

#[test]
fn test_pytorch_worker_audio_transcription_envelope_decodes_fixture() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
    );
    let envelope: PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest> =
        serde_json::from_str(fixture).expect("decode worker audio transcription fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-audio-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::TranscribeAudio);
    assert_eq!(envelope.payload.model_path, "asr/example/tiny-whisper");
    assert_eq!(envelope.payload.audio_base64, "UklGRg==");
    assert!(envelope.payload.device.is_none());
    assert_eq!(envelope.payload.language.as_deref(), Some("en"));
    assert_eq!(envelope.payload.prompt.as_deref(), Some("Meeting notes"));
    assert_eq!(envelope.payload.task.as_deref(), Some("transcribe"));
    assert_eq!(envelope.payload.chunk_length_s, Some(30.0));
    assert!(envelope.payload.extra_options.is_null());

    PyTorchBackend::validate_audio_transcription_envelope(&envelope)
        .expect("audio transcription fixture should validate");
}

#[test]
fn test_pytorch_worker_audio_transcription_envelope_rejects_legacy_device_id() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
    ))
    .expect("decode worker audio transcription fixture");
    value["payload"]["device"] = serde_json::json!("CUDA0");

    let error =
        serde_json::from_value::<PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest>>(value)
            .expect_err("legacy audio device id should fail contract decoding");
    assert!(error.to_string().contains("invalid identifier shape"));
}

#[test]
fn test_pytorch_worker_audio_transcription_envelope_rejects_auto_device_field() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
    ))
    .expect("decode worker audio transcription fixture");
    value["payload"]["device"] = serde_json::json!("auto");

    let error =
        serde_json::from_value::<PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest>>(value)
            .expect_err("auto audio device should be omitted, not sent as a device id");
    assert!(error.to_string().contains("reserved identifier"));
}

#[test]
fn test_pytorch_worker_audio_transcription_envelope_tolerates_additive_fields() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
    ))
    .expect("decode worker audio transcription fixture");
    let object = value.as_object_mut().expect("fixture is object");
    object.insert(
        "producer_trace".to_string(),
        serde_json::json!({"span_id": "trace-audio"}),
    );
    object
        .get_mut("payload")
        .and_then(|value| value.as_object_mut())
        .expect("payload is object")
        .insert(
            "future_payload_field".to_string(),
            serde_json::json!("ignored"),
        );

    let envelope: PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest> =
        serde_json::from_value(value).expect("additive fields should be ignored");

    assert_eq!(envelope.request_id, "req-audio-001");
    assert_eq!(envelope.payload.language.as_deref(), Some("en"));
    PyTorchBackend::validate_audio_transcription_envelope(&envelope)
        .expect("additive fields should not affect validation");
}

#[test]
fn test_python_worker_contract_projects_audio_transcription_fields() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
        );

        let kwargs = module
            .call_method1("transcribe_audio_kwargs_from_envelope", (fixture,))
            .expect("audio transcription worker envelope should project to kwargs");

        assert_eq!(
            kwargs
                .get_item("model_path")
                .expect("model path key should exist")
                .extract::<String>()
                .expect("model path should be a string"),
            "asr/example/tiny-whisper"
        );
        assert_eq!(
            kwargs
                .get_item("audio_base64")
                .expect("audio key should exist")
                .extract::<String>()
                .expect("audio payload should be a string"),
            "UklGRg=="
        );
        assert_eq!(
            kwargs
                .get_item("device")
                .expect("device key should exist")
                .extract::<String>()
                .expect("device should be a string"),
            "auto"
        );
        assert_eq!(
            kwargs
                .get_item("language")
                .expect("language key should exist")
                .extract::<String>()
                .expect("language should be a string"),
            "en"
        );
        assert_eq!(
            kwargs
                .get_item("chunk_length_s")
                .expect("chunk length key should exist")
                .extract::<f32>()
                .expect("chunk length should be numeric"),
            30.0
        );
    });
}

#[test]
fn test_python_worker_contract_tolerates_additive_audio_transcription_fields() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let envelope = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-additive-audio",
            "operation": "transcribe_audio",
            "producer_trace": {"span_id": "trace-audio"},
            "payload": {
                "model_path": "/models/asr",
                "audio_base64": " UklGRg== ",
                "language": "en",
                "future_payload_field": "ignored"
            }
        });

        let kwargs = module
            .call_method1(
                "transcribe_audio_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect("additive audio transcription fields should be ignored");

        assert_eq!(
            kwargs
                .get_item("audio_base64")
                .expect("audio key should exist")
                .extract::<String>()
                .expect("audio payload should be a string"),
            "UklGRg=="
        );
    });
}

#[test]
fn test_python_worker_contract_rejects_audio_transcription_legacy_device() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
        ))
        .expect("decode worker audio transcription fixture");
        value["payload"]["device"] = serde_json::json!("CUDA0");

        let error = module
            .call_method1(
                "transcribe_audio_kwargs_from_envelope",
                (value.to_string(),),
            )
            .expect_err("legacy device should fail Python worker validation");
        assert!(error
            .to_string()
            .contains("payload.device must be a canonical device id"));
    });
}

#[test]
fn test_python_worker_contract_rejects_audio_transcription_auto_device_field() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
        ))
        .expect("decode worker audio transcription fixture");
        value["payload"]["device"] = serde_json::json!("auto");

        let error = module
            .call_method1(
                "transcribe_audio_kwargs_from_envelope",
                (value.to_string(),),
            )
            .expect_err("auto device should be omitted for Python worker validation");
        assert!(error
            .to_string()
            .contains("payload.device must omit auto device intent"));
    });
}

#[test]
fn test_pytorch_worker_generate_text_response_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_text_response.json");
    let response: PyTorchWorkerResponse<PyTorchGenerateTextResult> =
        serde_json::from_str(fixture).expect("decode worker generate response fixture");

    match response {
        PyTorchWorkerResponse::Ok(success) => {
            assert_eq!(success.request_id, "req-generate-001");
            assert!(success.result.text.contains("Bounded diagnostics"));
            assert!(success.option_diagnostics.is_empty());
        }
        other => panic!("expected worker success response, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_success_response_returns_text() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-generate-ok",
        "result": {
            "text": "Generated through Transformers."
        }
    });

    let text = PyTorchBackend::generate_text_from_worker_response(
        "req-generate-ok",
        &response.to_string(),
    )
    .expect("generate_text response decodes");

    assert_eq!(text, "Generated through Transformers.");
}

#[test]
fn test_pytorch_worker_response_request_id_mismatch_rejects_generate_success() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-generate-other",
        "result": {
            "text": "Generated through Transformers."
        }
    });

    let error = PyTorchBackend::generate_text_from_worker_response(
        "req-generate-expected",
        &response.to_string(),
    )
    .expect_err("mismatched generate request id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-generate-expected",
        "pytorch_worker_generate_text_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_generate_text_runtime_unavailable_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-generate-no-model",
        "error": {
            "kind": "runtime_unavailable",
            "message": "No model loaded. Call load_model() first.",
            "canonical_code": "pytorch_worker_generate_text_failed"
        }
    });

    match PyTorchBackend::generate_text_from_worker_response(
        "req-generate-no-model",
        &response.to_string(),
    ) {
        Err(BackendError::NotRunning(message)) => {
            assert!(message.contains("pytorch_worker_generate_text_failed"));
            assert!(message.contains("req-generate-no-model"));
            assert!(message.contains("No model loaded"));
        }
        other => panic!("expected NotRunning error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_failure_normalizes_to_inference_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-generate-failed",
        "error": {
            "kind": "generation_failed",
            "message": "Transformers generation failed.",
            "canonical_code": "pytorch_worker_generation_failed"
        }
    });

    match PyTorchBackend::generate_text_from_worker_response(
        "req-generate-failed",
        &response.to_string(),
    ) {
        Err(BackendError::Inference(message)) => {
            assert!(message.contains("pytorch_worker_generation_failed"));
            assert!(message.contains("req-generate-failed"));
            assert!(message.contains("Transformers generation failed"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_cancelled_response_maps_to_inference() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-generate-cancelled",
        "error": {
            "kind": "cancelled",
            "message": "generation was cancelled by client token.",
            "canonical_code": "pytorch_worker_generation_cancelled"
        }
    });

    match PyTorchBackend::generate_text_from_worker_response(
        "req-generate-cancelled",
        &response.to_string(),
    ) {
        Err(error) => assert_worker_backend_error(
            error,
            ExpectedBackendErrorVariant::Inference,
            "req-generate-cancelled",
            "pytorch_worker_generation_cancelled",
            "cancelled by client token",
        ),
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_malformed_response_normalizes_to_inference_error() {
    let malformed = r#"{"status":"ok","secret":"SECRET_RESPONSE""#;

    match PyTorchBackend::generate_text_from_worker_response("req-generate-malformed", malformed) {
        Err(BackendError::Inference(message)) => {
            assert!(message.contains("pytorch_worker_generate_text_failed"));
            assert!(message.contains("req-generate-malformed"));
            assert!(message.contains("Failed to decode PyTorch worker generate_text response"));
            assert!(!message.contains("SECRET_RESPONSE"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_transport_error_normalizes_to_backend_error() {
    match PyTorchBackend::generate_text_worker_failure_from_message(
        "req-generate-transport",
        "PyTorch worker generate_text envelope failed: Python bridge failed.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_generate_text_failed"));
            assert!(message.contains("req-generate-transport"));
            assert!(message.contains("Python bridge failed"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_transport_errors_strip_tracebacks_and_local_paths() {
    let traceback = r#"PyTorch worker generate_text envelope failed:
Traceback (most recent call last):
  File "/home/jeremy/private/model/worker.py", line 42, in generate
    raise RuntimeError("bad prompt")
RuntimeError: failed while reading /home/jeremy/private/model/config.json
"#;

    match PyTorchBackend::generate_text_worker_failure_from_message(
        "req-generate-traceback",
        traceback.to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_generate_text_failed"));
            assert!(message.contains("req-generate-traceback"));
            assert!(message.contains("PyTorch worker generate_text envelope failed"));
            assert!(message.contains("RuntimeError: failed while reading [local-path]"));
            assert!(!message.contains("Traceback"));
            assert!(!message.contains("/home/jeremy/private"));
            assert!(!message.contains("worker.py"));
            assert!(!message.contains("bad prompt"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_kv_worker_transport_errors_strip_local_paths() {
    match kv_worker_failure_from_message(
        "req-kv-save",
        "pytorch_worker_kv_save_failed",
        "PyTorch KV save failed: could not write /tmp/pantograph-cache.bin".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_kv_save_failed"));
            assert!(message.contains("req-kv-save"));
            assert!(message.contains("could not write [local-path]"));
            assert!(!message.contains("/tmp/pantograph-cache.bin"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_task_join_errors_strip_tracebacks_and_local_paths() {
    let message = task_join_error_message(
        "worker panic at /home/jeremy/private/model/worker.py\nTraceback (most recent call last):\n  File \"/home/jeremy/private/model/worker.py\", line 42, in run\nRuntimeError: failed while reading /tmp/pantograph-cache.bin",
    );

    assert!(message.contains("Task join error"));
    assert!(message.contains("worker panic at [local-path]"));
    assert!(message.contains("RuntimeError: failed while reading [local-path]"));
    assert!(!message.contains("Traceback"));
    assert!(!message.contains("/home/jeremy/private"));
    assert!(!message.contains("/tmp/pantograph-cache.bin"));
    assert!(!message.contains("worker.py"));
}

#[test]
fn test_pytorch_worker_generate_text_transport_no_model_normalizes_to_not_running() {
    match PyTorchBackend::generate_text_worker_failure_from_message(
        "req-generate-no-model",
        "PyTorch worker generate_text envelope failed: No model loaded.".to_string(),
    ) {
        BackendError::NotRunning(message) => {
            assert!(message.contains("pytorch_worker_generate_text_failed"));
            assert!(message.contains("req-generate-no-model"));
            assert!(message.contains("No model loaded"));
        }
        other => panic!("expected NotRunning error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_generate_text_request_threads_top_k_as_transformers_kwarg() {
    let request = PyTorchBackend::generate_text_request(
        "Explain adapters.".to_string(),
        Some("Be precise.".to_string()),
        48,
        0.3,
        0.9,
        Some(20),
        None,
    );

    assert_eq!(request.transformers_kwargs["top_k"], serde_json::json!(20));
    assert_eq!(request.prompt, "Explain adapters.");
    assert_eq!(request.system_prompt.as_deref(), Some("Be precise."));
}

#[test]
fn test_pytorch_generate_text_envelopes_thread_top_k_for_generate_and_stream() {
    let generate_envelope = PyTorchBackend::generate_text_envelope(
        "req-generate-top-k",
        PyTorchWorkerOperation::GenerateText,
        "Explain adapters.".to_string(),
        Some("Be precise.".to_string()),
        48,
        0.3,
        0.9,
        Some(33),
        None,
    );
    let stream_envelope = PyTorchBackend::generate_text_envelope(
        "req-stream-top-k",
        PyTorchWorkerOperation::GenerateTextStream,
        "Explain adapters.".to_string(),
        Some("Be precise.".to_string()),
        48,
        0.3,
        0.9,
        Some(33),
        None,
    );

    PyTorchBackend::validate_generate_text_envelope(&generate_envelope)
        .expect("generate envelope validates");
    PyTorchBackend::validate_generate_text_stream_envelope(&stream_envelope)
        .expect("stream envelope validates");
    assert_eq!(
        generate_envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(33)
    );
    assert_eq!(
        stream_envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(33)
    );
}

#[test]
fn test_pytorch_generate_text_envelope_rejects_unscoped_transformers_kwargs() {
    let mut generate_envelope = PyTorchBackend::generate_text_envelope(
        "req-generate-raw-kwarg",
        PyTorchWorkerOperation::GenerateText,
        "Explain adapters.".to_string(),
        None,
        48,
        0.3,
        0.9,
        None,
        None,
    );
    generate_envelope
        .payload
        .transformers_kwargs
        .insert("raw_max_batch_size".to_string(), serde_json::json!(8));

    match PyTorchBackend::validate_generate_text_envelope(&generate_envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("unsupported transformers_kwargs key"));
            assert!(message.contains("raw_max_batch_size"));
        }
        other => panic!("expected unsupported kwargs config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_generate_text_stream_envelope_rejects_policy_transformers_kwargs() {
    let mut stream_envelope = PyTorchBackend::generate_text_envelope(
        "req-stream-policy-kwarg",
        PyTorchWorkerOperation::GenerateTextStream,
        "Explain adapters.".to_string(),
        None,
        48,
        0.3,
        0.9,
        None,
        None,
    );
    stream_envelope
        .payload
        .transformers_kwargs
        .insert("priority".to_string(), serde_json::json!("high"));

    match PyTorchBackend::validate_generate_text_stream_envelope(&stream_envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("unsupported transformers_kwargs key"));
            assert!(message.contains("priority"));
        }
        other => panic!("expected unsupported stream kwargs config error, got {other:?}"),
    }
}

#[test]
fn test_python_worker_contract_rejects_additive_backend_kwargs() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let envelope = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-backend-policy-kwarg",
            "operation": "generate_text",
            "payload": {
                "prompt": "Explain policy kwargs.",
                "transformers_kwargs": {
                    "priority": "high"
                }
            }
        });

        let error = module
            .call_method1(
                "generate_text_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect_err("backend policy kwargs should still reject");

        assert!(error.to_string().contains("unsupported key(s): priority"));
    });
}

#[test]
fn test_pytorch_generate_text_request_omits_absent_top_k_kwarg() {
    let request = PyTorchBackend::generate_text_request(
        "Explain adapters.".to_string(),
        None,
        48,
        0.3,
        0.9,
        None,
        None,
    );

    assert!(request.transformers_kwargs.is_empty());
}

#[test]
fn test_pytorch_worker_generate_text_stream_envelope_rejects_wrong_operation() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_text_stream_request.json"
    );
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker stream fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match PyTorchBackend::validate_generate_text_stream_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_envelope_rejects_wrong_operation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_text_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker generate fixture");
    envelope.operation = PyTorchWorkerOperation::LoadTransformersModel;

    match PyTorchBackend::validate_generate_text_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("LoadTransformersModel"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_envelope_rejects_wrong_contract_version() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_text_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker generate fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match PyTorchBackend::validate_generate_text_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("generate_text envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_unload_envelope_rejects_wrong_operation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/unload_model_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchUnloadModelRequest> =
        serde_json::from_str(fixture).expect("decode worker unload fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match PyTorchBackend::validate_unload_model_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_unload_envelope_rejects_wrong_contract_version() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/unload_model_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchUnloadModelRequest> =
        serde_json::from_str(fixture).expect("decode worker unload fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match PyTorchBackend::validate_unload_model_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("unload envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_audio_transcription_envelope_rejects_wrong_operation() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
    );
    let mut envelope: PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest> =
        serde_json::from_str(fixture).expect("decode worker audio transcription fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match PyTorchBackend::validate_audio_transcription_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_audio_transcription_envelope_rejects_wrong_contract_version() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
    );
    let mut envelope: PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest> =
        serde_json::from_str(fixture).expect("decode worker audio transcription fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match PyTorchBackend::validate_audio_transcription_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("audio_transcription envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_audio_transcription_envelope_rejects_blank_inputs() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
    );
    let mut envelope: PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest> =
        serde_json::from_str(fixture).expect("decode worker audio transcription fixture");
    envelope.payload.model_path = "  ".to_string();

    match PyTorchBackend::validate_audio_transcription_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("requires a model_path"));
        }
        other => panic!("expected blank-model config error, got {other:?}"),
    }

    let mut envelope: PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest> =
        serde_json::from_str(fixture).expect("decode worker audio transcription fixture");
    envelope.payload.audio_base64 = "  ".to_string();

    match PyTorchBackend::validate_audio_transcription_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("requires audio_base64"));
        }
        other => panic!("expected blank-audio config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_audio_transcription_envelope_rejects_extra_options() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/audio_transcription_request.json"
    );
    let mut envelope: PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest> =
        serde_json::from_str(fixture).expect("decode worker audio transcription fixture");
    envelope.payload.extra_options = serde_json::json!({"return_timestamps": true});

    match PyTorchBackend::validate_audio_transcription_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("does not support extra_options yet"));
        }
        other => panic!("expected extra-options config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_audio_transcription_envelope_from_request_trims_audio_and_filters_empty_fields() {
    let request = AudioTranscriptionRequest {
        model: "openai/whisper-tiny".to_string(),
        audio: Some(EncodedAudio {
            data_base64: " UklGRg== ".to_string(),
            mime_type: "audio/wav".to_string(),
            sample_rate_hz: Some(16_000),
        }),
        audio_ref: None,
        language: Some("   ".to_string()),
        prompt: Some("Meeting notes".to_string()),
        task: Some("transcribe".to_string()),
        chunk_length_s: Some(30.0),
        extra_options: serde_json::Value::Null,
    };

    let envelope =
        PyTorchBackend::audio_transcription_envelope_from_request("req-audio-build", request)
            .expect("audio transcription envelope should build");

    assert_eq!(envelope.request_id, "req-audio-build");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::TranscribeAudio);
    assert_eq!(envelope.payload.model_path, "openai/whisper-tiny");
    assert_eq!(envelope.payload.audio_base64, "UklGRg==");
    assert!(envelope.payload.device.is_none());
    assert!(envelope.payload.language.is_none());
    assert_eq!(envelope.payload.prompt.as_deref(), Some("Meeting notes"));
    assert_eq!(envelope.payload.task.as_deref(), Some("transcribe"));
    assert_eq!(envelope.payload.chunk_length_s, Some(30.0));
}

#[test]
fn test_pytorch_worker_trust_policy_defaults_closed() {
    let default_policy = PyTorchBackend::default_transformers_trust_policy();
    assert!(!default_policy.allow_remote_code);
    assert!(default_policy.accepted_sources.is_empty());
    assert!(default_policy.decision_id.is_none());
    assert!(default_policy.local_files_only);
    assert_eq!(default_policy.auth_token_source, ModelAuthTokenSource::None);
    assert_eq!(
        default_policy.cache_policy,
        ModelLoadCachePolicy::BackendDefault
    );

    let request = PyTorchTransformersLoadRequest {
        model_ref: Some(PumasModelRef {
            model_id: "pumas://models/no-custom-code".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }),
        artifact_kind: ModelArtifactKind::HfCompatibleDirectory,
        entry_path: "/models/no-custom-code".to_string(),
        model_source: None,
        task_id: InferenceTaskId::TextGeneration,
        task_profile: None,
        model_type_hint: None,
        device: None,
        trust_policy: PyTorchTransformersTrustPolicy::default(),
        generation_defaults: None,
    };
    let envelope = PyTorchWorkerEnvelope::new(
        "req-default-trust",
        PyTorchWorkerOperation::LoadTransformersModel,
        request,
    );
    let encoded = serde_json::to_value(&envelope).expect("encode envelope");

    assert_eq!(
        encoded["payload"]["trust_policy"]["allow_remote_code"],
        false
    );
    assert!(encoded["payload"]["trust_policy"]
        .get("accepted_sources")
        .is_none());
    assert_eq!(
        encoded["payload"]["trust_policy"]["local_files_only"],
        serde_json::json!(true)
    );
}

#[test]
fn test_pytorch_worker_trust_policy_maps_public_load_security_policy() {
    let policy = ModelLoadSecurityPolicy {
        trust_remote_code: ModelRemoteCodePolicy::Allow,
        network: ModelLoadNetworkPolicy::AllowNetwork,
        cache: ModelLoadCachePolicy::BypassCache,
        auth_token_source: ModelAuthTokenSource::Environment,
        revision: Some("weights-rev".to_string()),
        code_revision: Some("code-rev".to_string()),
        decision_id: Some("trust-001".to_string()),
        accepted_code_sources: vec!["configuration_tiny.py".to_string()],
    };

    let trust_policy = PyTorchTransformersTrustPolicy::from(policy);

    assert!(trust_policy.allow_remote_code);
    assert!(!trust_policy.local_files_only);
    assert_eq!(trust_policy.cache_policy, ModelLoadCachePolicy::BypassCache);
    assert_eq!(
        trust_policy.auth_token_source,
        ModelAuthTokenSource::Environment
    );
    assert_eq!(trust_policy.revision.as_deref(), Some("weights-rev"));
    assert_eq!(trust_policy.code_revision.as_deref(), Some("code-rev"));
    assert_eq!(
        trust_policy.accepted_sources,
        vec!["configuration_tiny.py".to_string()]
    );
}

#[test]
fn test_pytorch_worker_error_response_preserves_request_correlation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/worker_error_response.json");
    let response: PyTorchWorkerResponse<serde_json::Value> =
        serde_json::from_str(fixture).expect("decode worker error fixture");

    match response {
        PyTorchWorkerResponse::Error(PyTorchWorkerFailure { request_id, error }) => {
            assert_eq!(request_id, "req-load-001");
            assert_eq!(error.kind, PyTorchWorkerErrorKind::TrustPolicyRejected);
            assert_eq!(
                error.canonical_code.as_deref(),
                Some("pytorch_transformers_trust_policy_rejected")
            );
        }
        other => panic!("expected worker error response, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_error_response_tolerates_additive_fields() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-load-additive",
        "transport_trace": {"span_id": "trace-error"},
        "error": {
            "kind": "trust_policy_rejected",
            "message": "trust policy is closed",
            "canonical_code": "pytorch_transformers_trust_policy_rejected",
            "future_error_field": "ignored"
        }
    });

    let decoded: PyTorchWorkerResponse<serde_json::Value> =
        serde_json::from_value(response).expect("additive response fields should be ignored");
    let PyTorchWorkerResponse::Error(failure) = decoded else {
        panic!("expected worker error response");
    };

    assert_eq!(failure.request_id, "req-load-additive");
    assert_eq!(
        failure.error.kind,
        PyTorchWorkerErrorKind::TrustPolicyRejected
    );
    match failure.into_backend_error() {
        BackendError::Config(message) => {
            assert!(message.contains("pytorch_transformers_trust_policy_rejected"));
            assert!(message.contains("req-load-additive"));
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_success_response_tolerates_additive_fields() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-generate-additive",
        "response_trace": {"span_id": "trace-ok"},
        "result": {
            "text": "done",
            "future_result_field": "ignored"
        },
        "future_success_field": "ignored"
    });

    let decoded: PyTorchWorkerResponse<PyTorchGenerateTextResult> =
        serde_json::from_value(response).expect("additive success fields should be ignored");
    let PyTorchWorkerResponse::Ok(success) = decoded else {
        panic!("expected worker success response");
    };

    assert_eq!(success.request_id, "req-generate-additive");
    assert_eq!(success.result.text, "done");
}

#[test]
fn test_pytorch_worker_failure_normalizes_to_backend_error() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/worker_error_response.json");
    let response: PyTorchWorkerResponse<serde_json::Value> =
        serde_json::from_str(fixture).expect("decode worker error fixture");
    let PyTorchWorkerResponse::Error(failure) = response else {
        panic!("expected worker error response");
    };

    match failure.into_backend_error() {
        BackendError::Config(message) => {
            assert!(message.contains("pytorch_transformers_trust_policy_rejected"));
            assert!(message.contains("req-load-001"));
            assert!(message.contains("trust policy is closed"));
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_structured_errors_strip_tracebacks_and_local_paths() {
    let failure = PyTorchWorkerFailure {
        request_id: "req-worker-error-hygiene".to_string(),
        error: PyTorchWorkerError {
            kind: PyTorchWorkerErrorKind::GenerationFailed,
            canonical_code: Some("pytorch_worker_generation_failed".to_string()),
            message: r#"Generation failed:
Traceback (most recent call last):
  File "/home/jeremy/private/model/worker.py", line 12, in generate
    raise RuntimeError("SECRET_PROMPT")
RuntimeError: failed while reading /home/jeremy/private/model/config.json
"#
            .to_string(),
        },
    };

    match failure.into_backend_error() {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_generation_failed"));
            assert!(message.contains("req-worker-error-hygiene"));
            assert!(message.contains("Generation failed"));
            assert!(message.contains("RuntimeError: failed while reading [local-path]"));
            assert!(!message.contains("Traceback"));
            assert!(!message.contains("/home/jeremy/private"));
            assert!(!message.contains("worker.py"));
            assert!(!message.contains("SECRET_PROMPT"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[derive(Debug, Clone, Copy)]
enum ExpectedBackendErrorVariant {
    Config,
    Inference,
    NotRunning,
    StartupFailed,
}

fn assert_worker_backend_error(
    error: BackendError,
    expected_variant: ExpectedBackendErrorVariant,
    request_id: &str,
    canonical_code: &str,
    worker_message: &str,
) {
    let message = match (expected_variant, error) {
        (ExpectedBackendErrorVariant::Config, BackendError::Config(message))
        | (ExpectedBackendErrorVariant::Inference, BackendError::Inference(message))
        | (ExpectedBackendErrorVariant::NotRunning, BackendError::NotRunning(message))
        | (ExpectedBackendErrorVariant::StartupFailed, BackendError::StartupFailed(message)) => {
            message
        }
        (expected, other) => panic!("expected {expected:?} error, got {other:?}"),
    };

    assert!(message.contains(request_id));
    assert!(message.contains(canonical_code));
    assert!(message.contains(worker_message));
}

#[test]
fn test_pytorch_worker_error_kind_mapping_matrix_preserves_request_and_code() {
    let cases = [
        ("invalid_request", ExpectedBackendErrorVariant::Config),
        ("unsupported_task", ExpectedBackendErrorVariant::Config),
        ("trust_policy_rejected", ExpectedBackendErrorVariant::Config),
        (
            "runtime_unavailable",
            ExpectedBackendErrorVariant::NotRunning,
        ),
        (
            "model_load_failed",
            ExpectedBackendErrorVariant::StartupFailed,
        ),
        ("generation_failed", ExpectedBackendErrorVariant::Inference),
        ("cancelled", ExpectedBackendErrorVariant::Inference),
        ("internal", ExpectedBackendErrorVariant::Inference),
    ];

    for (kind, expected_variant) in cases {
        let request_id = format!("req-{kind}");
        let canonical_code = format!("pytorch_worker_{kind}");
        let worker_message = format!("worker reported {kind}");
        let response = serde_json::json!({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": kind,
                "message": worker_message,
                "canonical_code": canonical_code
            }
        });

        let decoded: PyTorchWorkerResponse<serde_json::Value> =
            serde_json::from_value(response).expect("decode worker error response");
        let PyTorchWorkerResponse::Error(failure) = decoded else {
            panic!("expected worker error response");
        };

        assert_worker_backend_error(
            failure.into_backend_error(),
            expected_variant,
            &request_id,
            &canonical_code,
            &worker_message,
        );
    }
}

#[test]
fn test_pytorch_worker_load_response_decodes_loaded_model_info() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-load-ok",
        "result": {
            "model_path": "/models/tiny",
            "model_type": "text-generation",
            "device": "cpu"
        }
    });

    let info = PyTorchBackend::load_info_from_worker_response("req-load-ok", &response.to_string())
        .expect("load response decodes");

    assert_eq!(info.model_path, "/models/tiny");
    assert_eq!(info.model_type, "text-generation");
    assert_eq!(info.device, "cpu");
}

#[test]
fn test_pytorch_worker_response_request_id_mismatch_rejects_load_success() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-load-other",
        "result": {
            "model_path": "/models/tiny",
            "model_type": "text-generation",
            "device": "cpu"
        }
    });

    let error =
        PyTorchBackend::load_info_from_worker_response("req-load-expected", &response.to_string())
            .expect_err("mismatched load request id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::StartupFailed,
        "req-load-expected",
        "pytorch_worker_model_load_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_load_error_response_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-load-rejected",
        "error": {
            "kind": "trust_policy_rejected",
            "message": "Model package requires custom Transformers code but trust policy is closed.",
            "canonical_code": "pytorch_transformers_trust_policy_rejected"
        }
    });

    match PyTorchBackend::load_info_from_worker_response("req-load-rejected", &response.to_string())
    {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("pytorch_transformers_trust_policy_rejected"));
            assert!(message.contains("req-load-rejected"));
            assert!(message.contains("trust policy is closed"));
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_load_model_load_failed_response_maps_to_startup_failed() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-load-failed",
        "error": {
            "kind": "model_load_failed",
            "message": "Transformers could not load model weights.",
            "canonical_code": "pytorch_transformers_model_load_failed"
        }
    });

    match PyTorchBackend::load_info_from_worker_response("req-load-failed", &response.to_string()) {
        Err(error) => assert_worker_backend_error(
            error,
            ExpectedBackendErrorVariant::StartupFailed,
            "req-load-failed",
            "pytorch_transformers_model_load_failed",
            "could not load model weights",
        ),
        other => panic!("expected StartupFailed error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_load_malformed_response_normalizes_to_startup_failed() {
    let malformed = r#"{"status":"ok","secret":"SECRET_RESPONSE""#;

    match PyTorchBackend::load_info_from_worker_response("req-load-malformed", malformed) {
        Err(BackendError::StartupFailed(message)) => {
            assert!(message.contains("pytorch_worker_model_load_failed"));
            assert!(message.contains("req-load-malformed"));
            assert!(message.contains("Failed to decode PyTorch worker load response"));
            assert!(!message.contains("SECRET_RESPONSE"));
        }
        other => panic!("expected StartupFailed error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_load_transport_error_normalizes_to_backend_error() {
    match PyTorchBackend::load_worker_failure_from_message(
        "req-load-transport",
        "Transformers envelope model load failed: Python bridge failed.".to_string(),
    ) {
        BackendError::StartupFailed(message) => {
            assert!(message.contains("pytorch_worker_model_load_failed"));
            assert!(message.contains("req-load-transport"));
            assert!(message.contains("Python bridge failed"));
        }
        other => panic!("expected StartupFailed error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_setup_error_response_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-stream-no-model",
        "error": {
            "kind": "runtime_unavailable",
            "message": "No model loaded. Call load_model() first.",
            "canonical_code": "pytorch_worker_generate_text_stream_failed"
        }
    });

    match PyTorchBackend::stream_setup_from_worker_response(
        "req-stream-no-model",
        &response.to_string(),
    ) {
        Err(BackendError::NotRunning(message)) => {
            assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
            assert!(message.contains("req-stream-no-model"));
            assert!(message.contains("No model loaded"));
        }
        other => panic!("expected NotRunning error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_response_request_id_mismatch_rejects_stream_setup_success() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-stream-other",
        "result": {"ready": true}
    });

    let error = PyTorchBackend::stream_setup_from_worker_response(
        "req-stream-expected",
        &response.to_string(),
    )
    .expect_err("mismatched stream setup request id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-stream-expected",
        "pytorch_worker_generate_text_stream_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_stream_invalid_request_response_maps_to_config() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-stream-invalid",
        "error": {
            "kind": "invalid_request",
            "message": "stream request was missing prompt state.",
            "canonical_code": "pytorch_worker_stream_invalid_request"
        }
    });

    match PyTorchBackend::stream_setup_from_worker_response(
        "req-stream-invalid",
        &response.to_string(),
    ) {
        Err(error) => assert_worker_backend_error(
            error,
            ExpectedBackendErrorVariant::Config,
            "req-stream-invalid",
            "pytorch_worker_stream_invalid_request",
            "missing prompt state",
        ),
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_setup_malformed_response_normalizes_to_inference_error() {
    let malformed = r#"{"status":"ok","secret":"SECRET_RESPONSE""#;

    match PyTorchBackend::stream_setup_from_worker_response("req-stream-malformed", malformed) {
        Err(BackendError::Inference(message)) => {
            assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
            assert!(message.contains("req-stream-malformed"));
            assert!(message.contains("Failed to decode PyTorch worker stream setup response"));
            assert!(!message.contains("SECRET_RESPONSE"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_generator_error_normalizes_to_backend_error() {
    match PyTorchBackend::stream_worker_failure_from_message(
        "req-stream-generator",
        "Generator error: Transformers stream failed.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
            assert!(message.contains("req-stream-generator"));
            assert!(message.contains("Transformers stream failed"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_module_error_normalizes_to_backend_error() {
    match PyTorchBackend::stream_worker_failure_from_message(
        "req-stream-module",
        "Failed to get worker module: import failed.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
            assert!(message.contains("req-stream-module"));
            assert!(message.contains("import failed"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_token_extraction_error_normalizes_to_backend_error() {
    match PyTorchBackend::stream_worker_failure_from_message(
        "req-stream-token",
        "Token extraction failed: expected string token.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
            assert!(message.contains("req-stream-token"));
            assert!(message.contains("expected string token"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_token_accepts_string_chunk() {
    Python::with_gil(|py| {
        let token = pyo3::types::PyString::new(py, "hello");
        let chunk =
            PyTorchBackend::stream_chunk_from_python_token("req-stream-token", token.as_any())
                .expect("string stream token should decode");

        assert_eq!(chunk.content.as_deref(), Some("hello"));
        assert!(!chunk.done);
    });
}

#[test]
fn test_pytorch_worker_stream_token_accepts_replace_dict_chunk() {
    Python::with_gil(|py| {
        let token = pyo3::types::PyDict::new(py);
        token.set_item("mode", "replace").expect("set mode");
        token.set_item("text", "final text").expect("set text");
        let usage = pyo3::types::PyDict::new(py);
        usage.set_item("prompt_tokens", 3).expect("set prompt");
        usage
            .set_item("completion_tokens", 5)
            .expect("set completion");
        usage.set_item("total_tokens", 8).expect("set total");
        token.set_item("usage", usage).expect("set usage");

        let chunk =
            PyTorchBackend::stream_chunk_from_python_token("req-stream-token", token.as_any())
                .expect("dict stream token should decode");

        assert_eq!(chunk.content.as_deref(), Some("final text"));
        let usage = chunk.usage.expect("usage should decode");
        assert_eq!(usage.prompt_tokens, Some(3));
        assert_eq!(usage.completion_tokens, Some(5));
        assert_eq!(usage.total_tokens, Some(8));
        assert!(!chunk.done);
    });
}

#[test]
fn test_pytorch_worker_stream_token_accepts_usage_only_dict_chunk() {
    Python::with_gil(|py| {
        let token = pyo3::types::PyDict::new(py);
        let usage = pyo3::types::PyDict::new(py);
        usage.set_item("prompt_tokens", 7).expect("set prompt");
        usage
            .set_item("completion_tokens", 11)
            .expect("set completion");
        usage.set_item("total_tokens", 18).expect("set total");
        token.set_item("usage", usage).expect("set usage");

        let chunk =
            PyTorchBackend::stream_chunk_from_python_token("req-stream-token", token.as_any())
                .expect("usage-only dict stream token should decode");

        assert_eq!(chunk.content, None);
        let usage = chunk.usage.expect("usage should decode");
        assert_eq!(usage.prompt_tokens, Some(7));
        assert_eq!(usage.completion_tokens, Some(11));
        assert_eq!(usage.total_tokens, Some(18));
        assert!(!chunk.done);
    });
}

#[test]
fn test_pytorch_worker_stream_token_bounds_usage_counts() {
    Python::with_gil(|py| {
        let token = pyo3::types::PyDict::new(py);
        let usage = pyo3::types::PyDict::new(py);
        usage.set_item("prompt_tokens", 2).expect("set prompt");
        usage
            .set_item("completion_tokens", u64::from(u32::MAX) + 1)
            .expect("set oversized completion");
        usage
            .set_item("total_tokens", "many")
            .expect("set malformed total");
        token.set_item("usage", usage).expect("set usage");

        let chunk =
            PyTorchBackend::stream_chunk_from_python_token("req-stream-token", token.as_any())
                .expect("bounded usage dict stream token should decode");

        let usage = chunk.usage.expect("usage should decode");
        assert_eq!(usage.prompt_tokens, Some(2));
        assert_eq!(usage.completion_tokens, None);
        assert_eq!(usage.total_tokens, None);
    });
}

#[test]
fn test_pytorch_worker_stream_token_rejects_dict_without_text_or_usage() {
    Python::with_gil(|py| {
        let token = pyo3::types::PyDict::new(py);
        token.set_item("mode", "replace").expect("set mode");

        match PyTorchBackend::stream_chunk_from_python_token("req-stream-token", token.as_any()) {
            Err(BackendError::Inference(message)) => {
                assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
                assert!(message.contains("req-stream-token"));
                assert!(message.contains("missing text or usage"));
            }
            other => panic!("expected Inference error, got {other:?}"),
        }
    });
}

#[test]
fn test_pytorch_worker_stream_runtime_unavailable_normalizes_to_not_running() {
    match PyTorchBackend::stream_worker_failure_from_message(
        "req-stream-no-model",
        "Generator error: No model loaded. Call load_model() first.".to_string(),
    ) {
        BackendError::NotRunning(message) => {
            assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
            assert!(message.contains("req-stream-no-model"));
            assert!(message.contains("No model loaded"));
        }
        other => panic!("expected NotRunning error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_setup_success_response_decodes() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-stream-ready",
        "result": {"ready": true}
    });

    PyTorchBackend::stream_setup_from_worker_response("req-stream-ready", &response.to_string())
        .expect("stream setup success should decode");
}

#[test]
fn test_pytorch_audio_transcription_requires_encoded_audio() {
    let request = AudioTranscriptionRequest {
        model: "openai/whisper-tiny".to_string(),
        audio: None,
        audio_ref: Some("artifact://audio.wav".to_string()),
        language: None,
        prompt: None,
        task: None,
        chunk_length_s: None,
        extra_options: serde_json::Value::Null,
    };

    match PyTorchBackend::audio_base64_from_request(&request) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("audio_ref resolution is owned by the host adapter"));
        }
        other => panic!("expected config error for unresolved audio_ref, got {other:?}"),
    }
}

#[test]
fn test_pytorch_audio_transcription_accepts_encoded_audio() {
    let request = AudioTranscriptionRequest {
        model: "openai/whisper-tiny".to_string(),
        audio: Some(EncodedAudio {
            data_base64: " UklGRg== ".to_string(),
            mime_type: "audio/wav".to_string(),
            sample_rate_hz: Some(16_000),
        }),
        audio_ref: None,
        language: Some("en".to_string()),
        prompt: None,
        task: Some("transcribe".to_string()),
        chunk_length_s: None,
        extra_options: serde_json::Value::Null,
    };

    let audio = PyTorchBackend::audio_base64_from_request(&request)
        .expect("encoded audio should be accepted");
    assert_eq!(audio, "UklGRg==");
}

#[test]
fn test_pytorch_worker_audio_transcription_transport_error_normalizes_to_backend_error() {
    match PyTorchBackend::audio_transcription_worker_failure_from_message(
        "req-audio-transport",
        "PyTorch audio transcription failed: ASR pipeline failed.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_audio_transcription_failed"));
            assert!(message.contains("req-audio-transport"));
            assert!(message.contains("ASR pipeline failed"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_audio_transcription_runtime_unavailable_maps_to_not_running() {
    assert_worker_backend_error(
        PyTorchBackend::audio_transcription_worker_failure_from_message(
            "req-audio-no-model",
            "PyTorch audio transcription failed: No model loaded. Call load_model() first."
                .to_string(),
        ),
        ExpectedBackendErrorVariant::NotRunning,
        "req-audio-no-model",
        "pytorch_worker_audio_transcription_failed",
        "No model loaded",
    );
}

#[test]
fn test_pytorch_audio_transcription_worker_response_decodes() {
    let response =
        PyTorchWorkerResponse::Ok(super::pytorch_worker_contract::PyTorchWorkerSuccess {
            request_id: "req-audio-result-ok".to_string(),
            result: PyTorchAudioTranscriptionResult {
                text: "hello from audio".to_string(),
                language: Some("en".to_string()),
                duration_seconds: Some(1.25_f32),
            },
            option_diagnostics: Vec::new(),
        });
    let response_json = serde_json::to_string(&response).expect("encode audio response");

    let decoded = PyTorchBackend::audio_transcription_result_from_worker_response(
        "req-audio-result-ok",
        &response_json,
    )
    .expect("audio transcription worker response should decode");

    assert_eq!(decoded.text, "hello from audio");
    assert_eq!(decoded.language.as_deref(), Some("en"));
    assert_eq!(decoded.duration_seconds, Some(1.25_f32));
    assert!(decoded.segments.is_empty());
    assert_eq!(decoded.metadata, serde_json::Value::Null);
}

#[test]
fn test_pytorch_worker_response_request_id_mismatch_rejects_audio_transcription_success() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-audio-other",
        "result": {
            "text": "hello from audio"
        }
    });

    let error = PyTorchBackend::audio_transcription_result_from_worker_response(
        "req-audio-expected",
        &response.to_string(),
    )
    .expect_err("mismatched audio transcription request id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-audio-expected",
        "pytorch_worker_audio_transcription_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_response_request_id_mismatch_rejects_structured_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-audio-other",
        "error": {
            "kind": "invalid_request",
            "message": "payload.model_path must be a non-empty string",
            "canonical_code": "pytorch_worker_invalid_audio_transcription_request"
        }
    });

    let error = PyTorchBackend::audio_transcription_result_from_worker_response(
        "req-audio-expected",
        &response.to_string(),
    )
    .expect_err("mismatched structured error request id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-audio-expected",
        "pytorch_worker_audio_transcription_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_audio_transcription_worker_response_rejects_missing_text() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-audio-missing-text",
        "result": {}
    });

    let error = PyTorchBackend::audio_transcription_result_from_worker_response(
        "req-audio-missing-text",
        &response.to_string(),
    )
    .expect_err("missing text should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-audio-missing-text",
        "pytorch_worker_audio_transcription_failed",
        "missing field `text`",
    );
}

#[test]
fn test_pytorch_audio_transcription_worker_response_rejects_non_string_text() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-audio-bad-text",
        "result": {
            "text": 42
        }
    });

    let error = PyTorchBackend::audio_transcription_result_from_worker_response(
        "req-audio-bad-text",
        &response.to_string(),
    )
    .expect_err("non-string text should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-audio-bad-text",
        "pytorch_worker_audio_transcription_failed",
        "invalid type: integer",
    );
}

#[test]
fn test_pytorch_audio_transcription_worker_response_rejects_bad_optional_metadata() {
    let bad_language = serde_json::json!({
        "status": "ok",
        "request_id": "req-audio-bad-language",
        "result": {
            "text": "hello",
            "language": 42
        }
    });

    let error = PyTorchBackend::audio_transcription_result_from_worker_response(
        "req-audio-bad-language",
        &bad_language.to_string(),
    )
    .expect_err("malformed language should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-audio-bad-language",
        "pytorch_worker_audio_transcription_failed",
        "invalid type: integer",
    );

    let bad_duration = serde_json::json!({
        "status": "ok",
        "request_id": "req-audio-bad-duration",
        "result": {
            "text": "hello",
            "duration_seconds": "soon"
        }
    });

    let error = PyTorchBackend::audio_transcription_result_from_worker_response(
        "req-audio-bad-duration",
        &bad_duration.to_string(),
    )
    .expect_err("malformed duration should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-audio-bad-duration",
        "pytorch_worker_audio_transcription_failed",
        "invalid type: string",
    );
}

#[test]
fn test_pytorch_audio_transcription_worker_response_error_maps_to_backend_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-audio-invalid",
        "error": {
            "kind": "invalid_request",
            "message": "PyTorch worker audio_transcription payload.model_path must be a non-empty string",
            "canonical_code": "pytorch_worker_invalid_audio_transcription_request"
        }
    });

    let error = PyTorchBackend::audio_transcription_result_from_worker_response(
        "req-audio-invalid",
        &response.to_string(),
    )
    .expect_err("structured worker error should map to BackendError");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Config,
        "req-audio-invalid",
        "pytorch_worker_invalid_audio_transcription_request",
        "payload.model_path must be a non-empty string",
    );
}

#[test]
fn test_pytorch_worker_unload_response_decodes() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-unload-ok",
        "result": {
            "unloaded": true
        }
    });

    PyTorchBackend::unload_model_result_from_worker_response(
        "req-unload-ok",
        &response.to_string(),
    )
    .expect("unload response should decode");
}

#[test]
fn test_pytorch_worker_unload_response_rejects_request_id_mismatch() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-unload-other",
        "result": {
            "unloaded": true
        }
    });

    let error = PyTorchBackend::unload_model_result_from_worker_response(
        "req-unload-expected",
        &response.to_string(),
    )
    .expect_err("mismatched unload response id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-unload-expected",
        "pytorch_worker_unload_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_unload_malformed_response_normalizes_to_inference_error() {
    let malformed = r#"{"status":"ok","secret":"SECRET_RESPONSE""#;

    match PyTorchBackend::unload_model_result_from_worker_response(
        "req-unload-malformed",
        malformed,
    ) {
        Err(BackendError::Inference(message)) => {
            assert!(message.contains("pytorch_worker_unload_failed"));
            assert!(message.contains("req-unload-malformed"));
            assert!(message.contains("Failed to decode PyTorch worker unload response"));
            assert!(!message.contains("SECRET_RESPONSE"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_unload_invalid_request_response_maps_to_config() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-unload-invalid",
        "error": {
            "kind": "invalid_request",
            "message": "Unexpected PyTorch worker operation for unload: generate_text",
            "canonical_code": "pytorch_worker_invalid_unload_request"
        }
    });

    let error = PyTorchBackend::unload_model_result_from_worker_response(
        "req-unload-invalid",
        &response.to_string(),
    )
    .expect_err("structured unload error should map to BackendError");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Config,
        "req-unload-invalid",
        "pytorch_worker_invalid_unload_request",
        "Unexpected PyTorch worker operation",
    );
}

#[test]
fn test_pytorch_worker_unload_transport_error_normalizes_to_backend_error() {
    match PyTorchBackend::unload_worker_failure_from_message(
        "req-unload",
        "Unload failed: Python bridge failed.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_unload_failed"));
            assert!(message.contains("req-unload"));
            assert!(message.contains("Python bridge failed"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_init_error_normalizes_to_startup_failed() {
    match PyTorchBackend::init_worker_failure_from_message(
        "req-init",
        "Failed to initialise Python worker: import failed.".to_string(),
    ) {
        BackendError::StartupFailed(message) => {
            assert!(message.contains("pytorch_worker_init_failed"));
            assert!(message.contains("req-init"));
            assert!(message.contains("import failed"));
        }
        other => panic!("expected StartupFailed error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_shutdown_error_normalizes_to_inference_error() {
    match PyTorchBackend::shutdown_worker_failure_from_message(
        "req-shutdown",
        "Failed to shutdown Python worker: /tmp/private/model.bin".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_shutdown_failed"));
            assert!(message.contains("req-shutdown"));
            assert!(message.contains("[local-path]"));
            assert!(!message.contains("/tmp/private/model.bin"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_init_response_decodes() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-init-ok",
        "result": {
            "initialized": true
        }
    });

    init_worker_result_from_worker_response("req-init-ok", &response.to_string())
        .expect("init_worker response should decode");
}

#[test]
fn test_pytorch_worker_shutdown_response_decodes() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-shutdown-ok",
        "result": {
            "shutdown": true
        }
    });

    shutdown_worker_result_from_worker_response("req-shutdown-ok", &response.to_string())
        .expect("shutdown_worker response should decode");
}

#[test]
fn test_pytorch_worker_shutdown_response_rejects_false_shutdown() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-shutdown-false",
        "result": {
            "shutdown": false
        }
    });

    let error =
        shutdown_worker_result_from_worker_response("req-shutdown-false", &response.to_string())
            .expect_err("unconfirmed shutdown should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-shutdown-false",
        "pytorch_worker_shutdown_failed",
        "did not confirm shutdown",
    );
}

#[test]
fn test_pytorch_worker_shutdown_invalid_request_maps_to_config_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-shutdown-invalid",
        "error": {
            "kind": "invalid_request",
            "message": "Unexpected PyTorch worker operation for shutdown_worker: init_worker",
            "canonical_code": "pytorch_worker_invalid_shutdown_request"
        }
    });

    let error =
        shutdown_worker_result_from_worker_response("req-shutdown-invalid", &response.to_string())
            .expect_err("invalid shutdown_worker request should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Config,
        "req-shutdown-invalid",
        "pytorch_worker_invalid_shutdown_request",
        "Unexpected PyTorch worker operation for shutdown_worker",
    );
}

#[test]
fn test_pytorch_worker_init_response_rejects_false_initialization() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-init-false",
        "result": {
            "initialized": false
        }
    });

    let error = init_worker_result_from_worker_response("req-init-false", &response.to_string())
        .expect_err("unconfirmed init should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::StartupFailed,
        "req-init-false",
        "pytorch_worker_init_failed",
        "did not confirm initialization",
    );
}

#[test]
fn test_pytorch_worker_init_response_rejects_request_id_mismatch() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-init-other",
        "result": {
            "initialized": true
        }
    });

    let error = init_worker_result_from_worker_response("req-init-expected", &response.to_string())
        .expect_err("mismatched init response id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::StartupFailed,
        "req-init-expected",
        "pytorch_worker_init_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_init_malformed_response_normalizes_to_startup_failed() {
    let malformed = r#"{"status":"ok","secret":"SECRET_RESPONSE""#;

    match init_worker_result_from_worker_response("req-init-malformed", malformed) {
        Err(BackendError::StartupFailed(message)) => {
            assert!(message.contains("pytorch_worker_init_failed"));
            assert!(message.contains("req-init-malformed"));
            assert!(message.contains("Failed to decode PyTorch worker init_worker response"));
            assert!(!message.contains("SECRET_RESPONSE"));
        }
        other => panic!("expected StartupFailed error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_init_invalid_request_maps_to_config_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-init-invalid",
        "error": {
            "kind": "invalid_request",
            "message": "Unexpected PyTorch worker operation for init_worker: generate_text",
            "canonical_code": "pytorch_worker_invalid_init_request"
        }
    });

    let error = init_worker_result_from_worker_response("req-init-invalid", &response.to_string())
        .expect_err("invalid init_worker request should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Config,
        "req-init-invalid",
        "pytorch_worker_invalid_init_request",
        "Unexpected PyTorch worker operation for init_worker",
    );
}

#[test]
fn test_pytorch_worker_live_kv_transport_error_normalizes_to_backend_error() {
    match kv_worker_failure_from_message(
        "req-kv-save",
        "pytorch_worker_kv_save_failed",
        "PyTorch KV save failed: cache export failed.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_kv_save_failed"));
            assert!(message.contains("req-kv-save"));
            assert!(message.contains("cache export failed"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_clear_kv_cache_response_decodes() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-clear-kv-ok",
        "result": {
            "cleared": true
        }
    });

    clear_kv_cache_result_from_worker_response("req-clear-kv-ok", &response.to_string())
        .expect("clear_kv_cache response decodes");
}

#[test]
fn test_pytorch_worker_clear_kv_cache_response_rejects_false_cleanup() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-clear-kv-false",
        "result": {
            "cleared": false
        }
    });

    let error =
        clear_kv_cache_result_from_worker_response("req-clear-kv-false", &response.to_string())
            .expect_err("uncleared KV response should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-clear-kv-false",
        "pytorch_worker_kv_clear_failed",
        "did not confirm cleanup",
    );
}

#[test]
fn test_pytorch_worker_clear_kv_cache_response_rejects_request_id_mismatch() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-clear-kv-other",
        "result": {
            "cleared": true
        }
    });

    let error =
        clear_kv_cache_result_from_worker_response("req-clear-kv-expected", &response.to_string())
            .expect_err("mismatched clear_kv_cache response id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-clear-kv-expected",
        "pytorch_worker_kv_clear_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_clear_kv_cache_malformed_response_normalizes_to_backend_error() {
    let malformed = r#"{"status":"ok","secret":"SECRET_RESPONSE""#;

    match clear_kv_cache_result_from_worker_response("req-clear-kv-malformed", malformed) {
        Err(BackendError::Inference(message)) => {
            assert!(message.contains("pytorch_worker_kv_clear_failed"));
            assert!(message.contains("req-clear-kv-malformed"));
            assert!(message.contains("Failed to decode PyTorch worker clear_kv_cache response"));
            assert!(!message.contains("SECRET_RESPONSE"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_clear_kv_cache_malformed_result_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-clear-kv-bad-result",
        "result": {}
    });

    let error = clear_kv_cache_result_from_worker_response(
        "req-clear-kv-bad-result",
        &response.to_string(),
    )
    .expect_err("missing cleared field should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-clear-kv-bad-result",
        "pytorch_worker_kv_clear_failed",
        "missing field `cleared`",
    );
}

#[test]
fn test_pytorch_worker_clear_kv_cache_invalid_request_maps_to_config_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-clear-kv-invalid",
        "error": {
            "kind": "invalid_request",
            "message": "Unexpected PyTorch worker operation for clear_kv_cache: unload_model",
            "canonical_code": "pytorch_worker_invalid_clear_kv_cache_request"
        }
    });

    let error =
        clear_kv_cache_result_from_worker_response("req-clear-kv-invalid", &response.to_string())
            .expect_err("invalid clear_kv_cache request should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Config,
        "req-clear-kv-invalid",
        "pytorch_worker_invalid_clear_kv_cache_request",
        "Unexpected PyTorch worker operation for clear_kv_cache",
    );
}

#[test]
fn test_pytorch_worker_save_kv_cache_response_decodes_live_info() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-save-kv-ok",
        "result": {
            "token_count": 8,
            "model_path": "/models/tiny",
            "model_type": "dllm",
            "device": "cpu"
        }
    });

    let info = save_kv_cache_result_from_worker_response("req-save-kv-ok", &response.to_string())
        .expect("save_kv_cache response decodes");

    assert_eq!(info.token_count, 8);
    assert_eq!(info.model_path, "/models/tiny");
    assert_eq!(info.model_type, "dllm");
    assert_eq!(info.device, "cpu");
}

#[test]
fn test_pytorch_worker_save_kv_cache_response_rejects_request_id_mismatch() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-save-kv-other",
        "result": {
            "token_count": 8,
            "model_path": "/models/tiny",
            "model_type": "dllm",
            "device": "cpu"
        }
    });

    let error =
        save_kv_cache_result_from_worker_response("req-save-kv-expected", &response.to_string())
            .expect_err("mismatched save_kv_cache response id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-save-kv-expected",
        "pytorch_worker_kv_save_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_save_kv_cache_malformed_response_normalizes_to_backend_error() {
    let malformed = r#"{"status":"ok","secret":"SECRET_RESPONSE""#;

    match save_kv_cache_result_from_worker_response("req-save-kv-malformed", malformed) {
        Err(BackendError::Inference(message)) => {
            assert!(message.contains("pytorch_worker_kv_save_failed"));
            assert!(message.contains("req-save-kv-malformed"));
            assert!(message.contains("Failed to decode PyTorch worker save_kv_cache response"));
            assert!(!message.contains("SECRET_RESPONSE"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_save_kv_cache_malformed_result_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-save-kv-bad-result",
        "result": {
            "token_count": "many",
            "model_path": "/models/tiny",
            "model_type": "dllm",
            "device": "cpu"
        }
    });

    let error =
        save_kv_cache_result_from_worker_response("req-save-kv-bad-result", &response.to_string())
            .expect_err("bad save_kv_cache result should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-save-kv-bad-result",
        "pytorch_worker_kv_save_failed",
        "invalid type",
    );
}

#[test]
fn test_pytorch_worker_save_kv_cache_invalid_request_maps_to_config_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-save-kv-invalid",
        "error": {
            "kind": "invalid_request",
            "message": "PyTorch worker save_kv_cache payload.path must be a non-empty string",
            "canonical_code": "pytorch_worker_invalid_save_kv_cache_request"
        }
    });

    let error =
        save_kv_cache_result_from_worker_response("req-save-kv-invalid", &response.to_string())
            .expect_err("invalid save_kv_cache request should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Config,
        "req-save-kv-invalid",
        "pytorch_worker_invalid_save_kv_cache_request",
        "payload.path must be a non-empty string",
    );
}

#[test]
fn test_pytorch_worker_restore_kv_cache_response_decodes_live_info() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-restore-kv-ok",
        "result": {
            "token_count": 8,
            "model_path": "/models/tiny",
            "model_type": "dllm",
            "device": "cpu"
        }
    });

    let info =
        restore_kv_cache_result_from_worker_response("req-restore-kv-ok", &response.to_string())
            .expect("restore_kv_cache response decodes");

    assert_eq!(info.token_count, 8);
    assert_eq!(info.model_path, "/models/tiny");
    assert_eq!(info.model_type, "dllm");
    assert_eq!(info.device, "cpu");
}

#[test]
fn test_pytorch_worker_restore_kv_cache_response_rejects_request_id_mismatch() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-restore-kv-other",
        "result": {
            "token_count": 8,
            "model_path": "/models/tiny",
            "model_type": "dllm",
            "device": "cpu"
        }
    });

    let error = restore_kv_cache_result_from_worker_response(
        "req-restore-kv-expected",
        &response.to_string(),
    )
    .expect_err("mismatched restore_kv_cache response id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-restore-kv-expected",
        "pytorch_worker_kv_restore_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_restore_kv_cache_malformed_response_normalizes_to_backend_error() {
    let malformed = r#"{"status":"ok","secret":"SECRET_RESPONSE""#;

    match restore_kv_cache_result_from_worker_response("req-restore-kv-malformed", malformed) {
        Err(BackendError::Inference(message)) => {
            assert!(message.contains("pytorch_worker_kv_restore_failed"));
            assert!(message.contains("req-restore-kv-malformed"));
            assert!(message.contains("Failed to decode PyTorch worker restore_kv_cache response"));
            assert!(!message.contains("SECRET_RESPONSE"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_restore_kv_cache_invalid_request_maps_to_config_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-restore-kv-invalid",
        "error": {
            "kind": "invalid_request",
            "message": "PyTorch worker restore_kv_cache payload.path must be a non-empty string",
            "canonical_code": "pytorch_worker_invalid_restore_kv_cache_request"
        }
    });

    let error = restore_kv_cache_result_from_worker_response(
        "req-restore-kv-invalid",
        &response.to_string(),
    )
    .expect_err("invalid restore_kv_cache request should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Config,
        "req-restore-kv-invalid",
        "pytorch_worker_invalid_restore_kv_cache_request",
        "payload.path must be a non-empty string",
    );
}

#[test]
fn test_pytorch_worker_get_loaded_info_response_decodes_loaded_model_info() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-loaded-info-ok",
        "result": {
            "model_path": "/models/tiny",
            "model_type": "dllm",
            "device": "cpu"
        }
    });

    let info = loaded_model_info_from_worker_response("req-loaded-info-ok", &response.to_string())
        .expect("get_loaded_info response decodes");

    assert_eq!(info.model_path, "/models/tiny");
    assert_eq!(info.model_type, "dllm");
    assert_eq!(info.device, "cpu");
}

#[test]
fn test_pytorch_worker_get_loaded_info_unavailable_response_maps_to_not_running() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-loaded-info-empty",
        "error": {
            "kind": "runtime_unavailable",
            "message": "No model loaded. Call load_model() first.",
            "canonical_code": "pytorch_worker_kv_loaded_info_failed"
        }
    });

    match loaded_model_info_from_worker_response("req-loaded-info-empty", &response.to_string()) {
        Err(BackendError::NotRunning(message)) => {
            assert!(message.contains("pytorch_worker_kv_loaded_info_failed"));
            assert!(message.contains("req-loaded-info-empty"));
            assert!(message.contains("No model loaded"));
        }
        other => panic!("expected NotRunning error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_get_loaded_info_invalid_request_maps_to_config_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-loaded-info-invalid",
        "error": {
            "kind": "invalid_request",
            "message": "Unexpected PyTorch worker operation for get_loaded_info: unload_model",
            "canonical_code": "pytorch_worker_invalid_get_loaded_info_request"
        }
    });

    let error =
        loaded_model_info_from_worker_response("req-loaded-info-invalid", &response.to_string())
            .expect_err("invalid get_loaded_info request should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Config,
        "req-loaded-info-invalid",
        "pytorch_worker_invalid_get_loaded_info_request",
        "Unexpected PyTorch worker operation for get_loaded_info",
    );
}

#[test]
fn test_pytorch_worker_get_loaded_info_response_rejects_request_id_mismatch() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-loaded-info-other",
        "result": {
            "model_path": "/models/tiny",
            "model_type": "dllm",
            "device": "cpu"
        }
    });

    let error =
        loaded_model_info_from_worker_response("req-loaded-info-expected", &response.to_string())
            .expect_err("mismatched get_loaded_info response id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-loaded-info-expected",
        "pytorch_worker_kv_loaded_info_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_get_loaded_info_malformed_response_normalizes_to_backend_error() {
    let malformed = r#"{"status":"ok","secret":"SECRET_RESPONSE""#;

    match loaded_model_info_from_worker_response("req-loaded-info-malformed", malformed) {
        Err(BackendError::Inference(message)) => {
            assert!(message.contains("pytorch_worker_kv_loaded_info_failed"));
            assert!(message.contains("req-loaded-info-malformed"));
            assert!(message.contains("Failed to decode PyTorch worker get_loaded_info response"));
            assert!(!message.contains("SECRET_RESPONSE"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_get_loaded_info_malformed_result_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-loaded-info-bad-result",
        "result": {
            "model_path": "/models/tiny",
            "model_type": "dllm"
        }
    });

    let error =
        loaded_model_info_from_worker_response("req-loaded-info-bad-result", &response.to_string())
            .expect_err("missing device should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-loaded-info-bad-result",
        "pytorch_worker_kv_loaded_info_failed",
        "missing field `device`",
    );
}

#[test]
fn test_pytorch_kv_live_info_malformed_result_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-kv-save-malformed",
        "result": {
            "token_count": "many",
            "model_path": "/models/tiny",
            "model_type": "dllm",
            "device": "cpu"
        }
    });

    let error =
        save_kv_cache_result_from_worker_response("req-kv-save-malformed", &response.to_string())
            .expect_err("invalid live KV token_count should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-kv-save-malformed",
        "pytorch_worker_kv_save_failed",
        "invalid type",
    );
}

#[test]
fn test_pytorch_worker_kv_truncate_transport_error_normalizes_to_backend_error() {
    match kv_truncate_worker_failure_from_message(
        "req-kv-truncate",
        "PyTorch KV truncate failed: invalid marker.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_kv_truncate_failed"));
            assert!(message.contains("req-kv-truncate"));
            assert!(message.contains("invalid marker"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_truncate_kv_cache_response_decodes() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-truncate-kv-ok",
        "result": {
            "token_count": 4
        }
    });

    let result =
        truncate_kv_cache_result_from_worker_response("req-truncate-kv-ok", &response.to_string())
            .expect("truncate_kv_cache response decodes");

    assert_eq!(result.token_count, 4);
}

#[test]
fn test_pytorch_worker_truncate_kv_cache_response_rejects_request_id_mismatch() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-truncate-kv-other",
        "result": {
            "token_count": 4
        }
    });

    let error = truncate_kv_cache_result_from_worker_response(
        "req-truncate-kv-expected",
        &response.to_string(),
    )
    .expect_err("mismatched truncate_kv_cache response id should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-truncate-kv-expected",
        "pytorch_worker_kv_truncate_failed",
        "request_id mismatch",
    );
}

#[test]
fn test_pytorch_worker_truncate_kv_cache_malformed_response_normalizes_to_backend_error() {
    let malformed = r#"{"status":"ok","secret":"SECRET_RESPONSE""#;

    match truncate_kv_cache_result_from_worker_response("req-truncate-kv-malformed", malformed) {
        Err(BackendError::Inference(message)) => {
            assert!(message.contains("pytorch_worker_kv_truncate_failed"));
            assert!(message.contains("req-truncate-kv-malformed"));
            assert!(message.contains("Failed to decode PyTorch worker truncate_kv_cache response"));
            assert!(!message.contains("SECRET_RESPONSE"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_truncate_kv_cache_malformed_result_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-truncate-kv-bad-result",
        "result": {
            "token_count": "many"
        }
    });

    let error = truncate_kv_cache_result_from_worker_response(
        "req-truncate-kv-bad-result",
        &response.to_string(),
    )
    .expect_err("bad truncate_kv_cache result should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Inference,
        "req-truncate-kv-bad-result",
        "pytorch_worker_kv_truncate_failed",
        "invalid type",
    );
}

#[test]
fn test_pytorch_worker_truncate_kv_cache_invalid_request_maps_to_config_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-truncate-kv-invalid",
        "error": {
            "kind": "invalid_request",
            "message": "PyTorch worker truncate_kv_cache payload.token_position must be a non-negative integer",
            "canonical_code": "pytorch_worker_invalid_truncate_kv_cache_request"
        }
    });

    let error = truncate_kv_cache_result_from_worker_response(
        "req-truncate-kv-invalid",
        &response.to_string(),
    )
    .expect_err("invalid truncate_kv_cache request should fail closed");

    assert_worker_backend_error(
        error,
        ExpectedBackendErrorVariant::Config,
        "req-truncate-kv-invalid",
        "pytorch_worker_invalid_truncate_kv_cache_request",
        "payload.token_position must be a non-negative integer",
    );
}

#[test]
fn test_pytorch_worker_kv_truncate_temp_file_errors_strip_local_paths() {
    match kv_truncate_worker_failure_from_message(
        "req-kv-truncate-temp",
        "Failed to write KV temp file: Permission denied at /tmp/pantograph-pytorch-kv-truncate-private.bin".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_kv_truncate_failed"));
            assert!(message.contains("req-kv-truncate-temp"));
            assert!(message.contains("Failed to write KV temp file"));
            assert!(message.contains("[local-path]"));
            assert!(!message.contains("/tmp/pantograph-pytorch-kv-truncate-private.bin"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_envelope_rejects_missing_required_fields() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/load_transformers_model_request.json"
    );
    let mut invalid: serde_json::Value =
        serde_json::from_str(fixture).expect("decode worker load fixture");
    invalid
        .as_object_mut()
        .expect("fixture is object")
        .remove("request_id");
    let error =
        serde_json::from_value::<PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest>>(invalid)
            .expect_err("missing request_id and payload fields should reject");

    assert!(error.to_string().contains("request_id"));
}

#[test]
fn test_pytorch_load_envelope_maps_pumas_package_facts() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let trust_policy = PyTorchTransformersTrustPolicy {
        allow_remote_code: true,
        accepted_sources: vec!["configuration_tiny.py".to_string()],
        decision_id: Some("trust-001".to_string()),
        local_files_only: true,
        cache_policy: ModelLoadCachePolicy::BackendDefault,
        auth_token_source: ModelAuthTokenSource::None,
        revision: Some("abc123".to_string()),
        code_revision: None,
    };

    let envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cuda:0"),
        trust_policy,
    )
    .expect("map package facts to worker envelope");

    assert_eq!(envelope.request_id, "req-pumas-load");
    assert_eq!(
        envelope.operation,
        PyTorchWorkerOperation::LoadTransformersModel
    );
    assert_eq!(
        envelope
            .payload
            .model_ref
            .as_ref()
            .map(|value| value.model_id.as_str()),
        Some("llm/example/tiny-transformers")
    );
    assert_eq!(
        envelope.payload.artifact_kind,
        ModelArtifactKind::HfCompatibleDirectory
    );
    assert_eq!(envelope.payload.entry_path, "llm/example/tiny-transformers");
    let model_source = envelope
        .payload
        .model_source
        .as_ref()
        .expect("model source should project from package facts");
    assert_eq!(
        model_source.source_kind,
        ResolvedModelSourceKind::PumasResolved
    );
    assert_eq!(model_source.entry_path, envelope.payload.entry_path);
    assert_eq!(
        model_source.model_ref.as_ref(),
        envelope.payload.model_ref.as_ref()
    );
    assert_eq!(envelope.payload.task_id, InferenceTaskId::TextGeneration);
    let task_profile = envelope
        .payload
        .task_profile
        .as_ref()
        .expect("task profile should project from registry entry");
    assert_eq!(task_profile.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(task_profile.canonical_task_label, "text_generation");
    assert_eq!(
        task_profile.loader,
        PyTorchTransformersModelLoader::CausalLm
    );
    assert!(task_profile
        .required_components
        .contains(&ProcessorComponentKind::Tokenizer));
    assert_eq!(envelope.payload.model_type_hint.as_deref(), Some("llama"));
    assert_eq!(
        envelope.payload.device.as_ref().map(|id| id.as_str()),
        Some("cuda:0")
    );
    assert!(envelope.payload.trust_policy.allow_remote_code);
    assert_eq!(
        envelope.payload.trust_policy.decision_id.as_deref(),
        Some("trust-001")
    );
    assert!(envelope.payload.trust_policy.local_files_only);
    assert_eq!(
        envelope.payload.trust_policy.revision.as_deref(),
        Some("abc123")
    );
    assert_eq!(
        envelope
            .payload
            .generation_defaults
            .as_ref()
            .and_then(|defaults| {
                defaults
                    .get("max_new_tokens")
                    .and_then(serde_json::Value::as_u64)
            }),
        Some(128)
    );
}

#[test]
fn test_pytorch_transformers_load_args_use_worker_envelope_payload() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cuda:0"),
        PyTorchTransformersTrustPolicy::from(ModelLoadSecurityPolicy {
            trust_remote_code: ModelRemoteCodePolicy::Allow,
            network: ModelLoadNetworkPolicy::AllowNetwork,
            cache: ModelLoadCachePolicy::BypassCache,
            auth_token_source: ModelAuthTokenSource::Environment,
            revision: Some("weights-rev".to_string()),
            code_revision: Some("code-rev".to_string()),
            decision_id: Some("trust-001".to_string()),
            accepted_code_sources: vec!["configuration_tiny.py".to_string()],
        }),
    )
    .expect("map package facts to worker envelope");

    PyTorchBackend::validate_transformers_load_envelope(&envelope)
        .expect("envelope should validate");
    let args = PyTorchBackend::transformers_load_args_from_request(&envelope.payload);

    assert_eq!(args.model_path, "llm/example/tiny-transformers");
    assert_eq!(args.device, "cuda:0");
    assert_eq!(args.model_type.as_deref(), Some("llama"));
    assert!(args.trust_policy.allow_remote_code);
    assert!(!args.trust_policy.local_files_only);
    assert_eq!(
        args.trust_policy.cache_policy,
        ModelLoadCachePolicy::BypassCache
    );
    assert_eq!(
        args.trust_policy.auth_token_source,
        ModelAuthTokenSource::Environment
    );
    assert_eq!(args.trust_policy.revision.as_deref(), Some("weights-rev"));
    assert_eq!(args.trust_policy.code_revision.as_deref(), Some("code-rev"));
}

#[test]
fn test_pytorch_direct_load_envelope_uses_transformers_contract() {
    let envelope = PyTorchBackend::transformers_load_envelope_from_direct_path(
        "req-direct-load",
        "/models/direct-hf",
        Some("cpu"),
        Some("dllm"),
        PyTorchTransformersTrustPolicy::default(),
    )
    .expect("direct load envelope should build");

    PyTorchBackend::validate_transformers_load_envelope(&envelope)
        .expect("direct load envelope should validate");
    assert_eq!(envelope.request_id, "req-direct-load");
    assert_eq!(
        envelope.operation,
        PyTorchWorkerOperation::LoadTransformersModel
    );
    assert!(envelope.payload.model_ref.is_none());
    assert_eq!(
        envelope.payload.artifact_kind,
        ModelArtifactKind::HfCompatibleDirectory
    );
    assert_eq!(envelope.payload.entry_path, "/models/direct-hf");
    assert_eq!(envelope.payload.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(envelope.payload.model_type_hint.as_deref(), Some("dllm"));
    assert_eq!(
        envelope.payload.device.as_ref().map(|id| id.as_str()),
        Some("cpu")
    );
    let model_source = envelope
        .payload
        .model_source
        .as_ref()
        .expect("direct load envelope should carry a model source");
    assert_eq!(
        model_source.source_kind,
        ResolvedModelSourceKind::DirectHfCompatibleDirectory
    );
    assert!(model_source.model_ref.is_none());
    assert!(model_source.validate_for_backend_load().is_ok());
    let task_profile = envelope
        .payload
        .task_profile
        .as_ref()
        .expect("direct load envelope should carry a task profile");
    assert_eq!(
        task_profile.loader,
        PyTorchTransformersModelLoader::CausalLm
    );
}

#[test]
fn test_pytorch_direct_load_envelope_rejects_legacy_device_id() {
    match PyTorchBackend::transformers_load_envelope_from_direct_path(
        "req-direct-invalid-device",
        "/models/direct-hf",
        Some("CUDA0"),
        None,
        PyTorchTransformersTrustPolicy::default(),
    ) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Invalid PyTorch worker device id"));
            assert!(message.contains("invalid identifier shape"));
        }
        other => panic!("expected invalid direct device config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_transformers_load_args_default_device_auto() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        None,
        PyTorchTransformersTrustPolicy {
            allow_remote_code: true,
            accepted_sources: vec!["configuration_tiny.py".to_string()],
            decision_id: None,
            local_files_only: true,
            cache_policy: ModelLoadCachePolicy::BackendDefault,
            auth_token_source: ModelAuthTokenSource::None,
            revision: None,
            code_revision: None,
        },
    )
    .expect("map package facts to worker envelope");

    let args = PyTorchBackend::transformers_load_args_from_request(&envelope.payload);

    assert_eq!(args.device, "auto");
}

#[test]
fn test_pytorch_transformers_load_envelope_validation_rejects_contract_version() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let mut envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cpu"),
        PyTorchTransformersTrustPolicy {
            allow_remote_code: true,
            accepted_sources: vec!["configuration_tiny.py".to_string()],
            decision_id: None,
            local_files_only: true,
            cache_policy: ModelLoadCachePolicy::BackendDefault,
            auth_token_source: ModelAuthTokenSource::None,
            revision: None,
            code_revision: None,
        },
    )
    .expect("map package facts to worker envelope");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match PyTorchBackend::validate_transformers_load_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unsupported PyTorch worker load envelope"));
        }
        other => panic!("expected contract-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_transformers_load_envelope_validation_rejects_wrong_operation() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let mut envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cpu"),
        PyTorchTransformersTrustPolicy {
            allow_remote_code: true,
            accepted_sources: vec!["configuration_tiny.py".to_string()],
            decision_id: None,
            local_files_only: true,
            cache_policy: ModelLoadCachePolicy::BackendDefault,
            auth_token_source: ModelAuthTokenSource::None,
            revision: None,
            code_revision: None,
        },
    )
    .expect("map package facts to worker envelope");
    envelope.operation = PyTorchWorkerOperation::InitWorker;

    match PyTorchBackend::validate_transformers_load_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("InitWorker"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_transformers_load_envelope_validation_rejects_invalid_model_source() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let mut envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cpu"),
        PyTorchTransformersTrustPolicy {
            allow_remote_code: true,
            accepted_sources: vec!["configuration_tiny.py".to_string()],
            decision_id: None,
            local_files_only: true,
            cache_policy: ModelLoadCachePolicy::BackendDefault,
            auth_token_source: ModelAuthTokenSource::None,
            revision: None,
            code_revision: None,
        },
    )
    .expect("map package facts to worker envelope");
    envelope
        .payload
        .model_source
        .as_mut()
        .expect("model source")
        .model_ref = None;

    match PyTorchBackend::validate_transformers_load_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Invalid PyTorch worker resolved model source"));
            assert!(message.contains("pumas_resolved_source_missing_model_ref"));
        }
        other => panic!("expected invalid model source config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_transformers_load_envelope_validation_rejects_empty_entry_path() {
    let mut envelope = PyTorchBackend::transformers_load_envelope_from_direct_path(
        "req-empty-entry",
        " ",
        Some("cpu"),
        None,
        PyTorchTransformersTrustPolicy::default(),
    )
    .expect("direct load envelope should build");
    envelope.payload.model_source = None;

    match PyTorchBackend::validate_transformers_load_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("requires a non-empty entry_path"));
        }
        other => panic!("expected empty entry_path config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_transformers_load_envelope_validation_rejects_mismatched_model_source() {
    let mut envelope = PyTorchBackend::transformers_load_envelope_from_direct_path(
        "req-mismatch-source",
        "/models/direct-hf",
        Some("cpu"),
        None,
        PyTorchTransformersTrustPolicy::default(),
    )
    .expect("direct load envelope should build");
    envelope
        .payload
        .model_source
        .as_mut()
        .expect("direct model source")
        .entry_path = "/models/other".to_string();

    match PyTorchBackend::validate_transformers_load_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("model_source entry_path must match"));
        }
        other => panic!("expected mismatched model_source config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_transformers_load_envelope_validation_rejects_mismatched_task_profile() {
    let mut envelope = PyTorchBackend::transformers_load_envelope_from_direct_path(
        "req-mismatch-task",
        "/models/direct-hf",
        Some("cpu"),
        None,
        PyTorchTransformersTrustPolicy::default(),
    )
    .expect("direct load envelope should build");
    envelope
        .payload
        .task_profile
        .as_mut()
        .expect("task profile")
        .task_id = InferenceTaskId::AudioTranscription;

    match PyTorchBackend::validate_transformers_load_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("task_profile task_id"));
            assert!(message.contains("does not match payload task_id"));
        }
        other => panic!("expected mismatched task profile config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_task_profile_uses_canonical_registry_aliases() {
    let profile = PyTorchBackend::transformers_task_profile_from_evidence(&TaskEvidence {
        pipeline_tag: Some("causal-lm".to_string()),
        ..TaskEvidence::default()
    })
    .expect("causal-lm alias should resolve through task registry");

    assert_eq!(profile.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(profile.canonical_task_label, "text_generation");
    assert_eq!(profile.loader, PyTorchTransformersModelLoader::CausalLm);
    assert!(profile
        .required_components
        .contains(&ProcessorComponentKind::Tokenizer));
}

#[test]
fn test_pytorch_task_profile_maps_audio_transcription_aliases() {
    let profile = PyTorchBackend::transformers_task_profile_from_evidence(&TaskEvidence {
        pipeline_tag: Some("automatic-speech-recognition".to_string()),
        task_type_primary: Some("audio_transcription".to_string()),
        input_modalities: vec!["audio".to_string()],
        output_modalities: vec!["text".to_string()],
    })
    .expect("audio transcription aliases should resolve through task registry");

    assert_eq!(profile.task_id, InferenceTaskId::AudioTranscription);
    assert_eq!(profile.canonical_task_label, "audio_transcription");
    assert_eq!(
        profile.loader,
        PyTorchTransformersModelLoader::AutomaticSpeechRecognition
    );
    assert!(profile
        .required_components
        .contains(&ProcessorComponentKind::AudioFeatureExtractor));
}

#[test]
fn test_pytorch_task_profile_rejects_registry_tasks_without_loader() {
    match PyTorchBackend::transformers_task_profile_from_evidence(&TaskEvidence {
        task_type_primary: Some("feature-extraction".to_string()),
        ..TaskEvidence::default()
    }) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("does not support canonical task embedding yet"));
        }
        other => panic!("expected unsupported task config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_task_profile_rejects_unregistered_task_evidence() {
    match PyTorchBackend::transformers_task_profile_from_evidence(&TaskEvidence {
        task_type_primary: Some("object-detection".to_string()),
        pipeline_tag: Some("object-detection".to_string()),
        input_modalities: vec!["image".to_string()],
        output_modalities: vec!["json".to_string()],
    }) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("UnsupportedTaskLabel"));
            assert!(message.contains("task evidence did not resolve"));
        }
        other => panic!("expected unsupported task evidence config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_load_envelope_rejects_custom_code_without_trust_opt_in() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");

    match PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cpu"),
        PyTorchTransformersTrustPolicy::default(),
    ) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("trust policy is closed"));
        }
        other => panic!("expected closed trust policy config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_load_envelope_rejects_unsupported_artifact_kind() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");

    match PyTorchBackend::transformers_load_envelope_from_package(
        "req-gguf-load",
        &facts,
        Some("cpu"),
        PyTorchTransformersTrustPolicy::default(),
    ) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("cannot load Gguf artifacts"));
        }
        other => panic!("expected unsupported artifact config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_generation_options_map_to_transformers_kwargs_and_diagnostics() {
    let options = GenerationOptions {
        length: LengthGenerationOptions {
            max_new_tokens: Some(128),
            ..Default::default()
        },
        sampling: SamplingGenerationOptions {
            temperature: Some(0.6),
            top_p: Some(0.9),
            top_k: Some(40),
            seed: Some(42),
            ..Default::default()
        },
        stopping: StoppingGenerationOptions {
            stop_strings: vec!["END".to_string()],
            eos_token_ids: vec![2, 32000],
        },
        cache: CacheGenerationOptions {
            use_cache: Some(true),
            kv_cache_checkpoint_requested: Some(true),
        },
        output: OutputGenerationOptions {
            return_logprobs: Some(true),
            ..Default::default()
        },
        special_tokens: SpecialTokenGenerationOptions {
            pad_token_id: Some(0),
            ..Default::default()
        },
        backend_extensions: [
            (
                "transformers:renormalize_logits".to_string(),
                serde_json::json!(true),
            ),
            ("raw_top_k".to_string(), serde_json::json!(40)),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    let mapping = PyTorchBackend::transformers_generation_option_mapping(&options);

    assert_eq!(mapping.kwargs["max_new_tokens"], serde_json::json!(128));
    let temperature = mapping.kwargs["temperature"]
        .as_f64()
        .expect("temperature is numeric");
    assert!((temperature - 0.6).abs() < 0.000_001);
    assert_eq!(mapping.kwargs["use_cache"], serde_json::json!(true));
    assert_eq!(mapping.kwargs["pad_token_id"], serde_json::json!(0));
    assert_eq!(
        mapping.kwargs["eos_token_id"],
        serde_json::json!([2, 32000])
    );
    assert_eq!(
        mapping.kwargs["renormalize_logits"],
        serde_json::json!(true)
    );
    assert_eq!(mapping.kwargs["top_k"], serde_json::json!(40));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "sampling.top_k"
            && diagnostic.state == OptionSupportState::Honored
    }));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "sampling.seed"
            && diagnostic.state == OptionSupportState::Unsupported
    }));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "stopping.eos_token_ids"
            && diagnostic.state == OptionSupportState::Mapped
    }));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "cache.kv_cache_checkpoint_requested"
            && diagnostic.state == OptionSupportState::Mapped
    }));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "output.return_logprobs"
            && diagnostic.state == OptionSupportState::Unsupported
    }));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "backend_extensions.raw_top_k"
            && diagnostic.state == OptionSupportState::Rejected
    }));

    let requested_paths: BTreeSet<_> = options.requested_option_paths().into_iter().collect();
    let diagnostic_paths: BTreeSet<_> = mapping
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.option_path.clone())
        .collect();
    assert!(
        requested_paths.is_subset(&diagnostic_paths),
        "missing diagnostics for requested options: {:?}",
        requested_paths
            .difference(&diagnostic_paths)
            .collect::<Vec<_>>()
    );
}

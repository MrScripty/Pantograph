use serde_json::json;

use super::*;
use crate::model_contracts::InferenceTaskId;

fn device_id(value: &str) -> InferenceDeviceId {
    InferenceDeviceId::parse(value).expect("valid device id")
}

fn runtime_variant_id(value: &str) -> RuntimeVariantId {
    RuntimeVariantId::parse(value).expect("valid runtime variant id")
}

fn backend_id(value: &str) -> BackendId {
    BackendId::parse(value).expect("valid backend id")
}

#[test]
fn device_id_parser_accepts_canonical_device_ids() {
    assert_eq!(device_id("cpu").as_str(), "cpu");
    assert_eq!(device_id("cuda:0").as_str(), "cuda:0");
    assert_eq!(device_id("metal:0").as_str(), "metal:0");
    assert_eq!(device_id("mps").as_str(), "mps");
    assert_eq!(device_id("local-gpu_0").as_str(), "local-gpu_0");
}

#[test]
fn device_id_parser_rejects_legacy_or_malformed_values() {
    for value in [
        "", "  ", "auto", "CUDA0", "CUDAx", "cuda:", "cuda::0", "cuda/0",
    ] {
        assert!(
            InferenceDeviceId::parse(value).is_err(),
            "{value:?} should fail validation"
        );
    }
}

#[test]
fn device_id_parser_reports_auto_as_reserved_policy_keyword() {
    let error = InferenceDeviceId::parse("auto").expect_err("auto is policy, not a device id");

    assert_eq!(
        error,
        DeviceContractError::ReservedIdentifier {
            field: "device_id",
            value: "auto".to_string()
        }
    );
}

#[test]
fn runtime_variant_id_parser_uses_lowercase_dotted_ids() {
    assert_eq!(
        runtime_variant_id("llama_cpp.cuda").as_str(),
        "llama_cpp.cuda"
    );
    assert_eq!(
        runtime_variant_id("pytorch.diffusers").as_str(),
        "pytorch.diffusers"
    );

    for value in [
        "LlamaCpp.CUDA",
        "llama_cpp:",
        "llama_cpp..cuda",
        "llama/cpp",
    ] {
        assert!(
            RuntimeVariantId::parse(value).is_err(),
            "{value:?} should fail validation"
        );
    }
}

#[test]
fn backend_id_parser_rejects_variant_or_path_like_values() {
    assert_eq!(backend_id("llama_cpp").as_str(), "llama_cpp");
    assert_eq!(backend_id("pytorch").as_str(), "pytorch");

    for value in ["llama.cpp", "llama:cpp", "llama/cpp", "LlamaCpp"] {
        assert!(
            BackendId::parse(value).is_err(),
            "{value:?} should fail validation"
        );
    }
}

#[test]
fn device_policy_serde_uses_stable_tagged_shape() {
    let policy = InferenceDevicePolicy::Explicit {
        device_class: InferenceDeviceClass::Cuda,
        device_id: Some(device_id("cuda:0")),
    };
    let encoded = serde_json::to_value(&policy).expect("encode policy");
    assert_eq!(
        encoded,
        json!({
            "policy": "explicit",
            "device_class": "cuda",
            "device_id": "cuda:0"
        })
    );

    let decoded: InferenceDevicePolicy = serde_json::from_value(encoded).expect("decode policy");
    assert_eq!(decoded, policy);
    assert_eq!(
        serde_json::to_value(InferenceDevicePolicy::Auto).expect("encode auto"),
        json!({ "policy": "auto" })
    );
}

#[test]
fn invalid_device_id_fails_during_deserialization() {
    let error = serde_json::from_value::<InferenceDevicePolicy>(json!({
        "policy": "explicit",
        "device_class": "cuda",
        "device_id": "CUDA0"
    }))
    .expect_err("legacy CUDA id should not deserialize");

    assert!(error.to_string().contains("invalid identifier shape"));
}

#[test]
fn runtime_variant_capability_serde_roundtrips_with_diagnostics() {
    let capability = RuntimeVariantCapability {
        runtime_variant_id: runtime_variant_id("llama_cpp.cuda"),
        device_class: InferenceDeviceClass::Cuda,
        available: false,
        diagnostics: vec![DeviceResolutionDiagnostic {
            code: DeviceResolutionDiagnosticCode::MissingRuntimeVariant,
            severity: DeviceResolutionDiagnosticSeverity::Error,
            message: "CUDA runtime variant is not installed".to_string(),
            device_class: Some(InferenceDeviceClass::Cuda),
            device_id: Some(device_id("cuda:0")),
            runtime_variant_id: Some(runtime_variant_id("llama_cpp.cuda")),
            backend_id: Some(backend_id("llama_cpp")),
        }],
    };

    let encoded = serde_json::to_value(&capability).expect("encode capability");
    assert_eq!(encoded["runtime_variant_id"], json!("llama_cpp.cuda"));
    assert_eq!(
        encoded["diagnostics"][0]["code"],
        json!("missing_runtime_variant")
    );
    let decoded: RuntimeVariantCapability =
        serde_json::from_value(encoded).expect("decode capability");
    assert_eq!(decoded, capability);
}

#[test]
fn backend_execution_decision_requires_one_selected_candidate() {
    let empty = BackendExecutionDecision::try_from_selected_candidate(
        Vec::new(),
        InferenceDevicePolicy::Auto,
        Some(InferenceTaskId::ImageGeneration),
    )
    .expect_err("empty candidates should fail");
    assert_eq!(empty, DeviceContractError::EmptyBackendCandidates);

    let candidate = BackendExecutionCandidate {
        backend_id: backend_id("pytorch"),
        model_compatible: true,
        model_ref: None,
        supported_task_ids: vec![InferenceTaskId::ImageGeneration],
        runtime_variant_id: runtime_variant_id("pytorch.cuda"),
        device_class: InferenceDeviceClass::Cuda,
        device_id: Some(device_id("cuda:0")),
        resource_estimate: Some(BackendResourceEstimate {
            ram_mb: Some(8192),
            vram_mb: Some(6144),
            context_tokens: None,
        }),
        observed_throughput: None,
        diagnostics: Vec::new(),
    };

    let ambiguous = BackendExecutionDecision::try_from_selected_candidate(
        vec![candidate.clone(), candidate.clone()],
        InferenceDevicePolicy::Auto,
        Some(InferenceTaskId::ImageGeneration),
    )
    .expect_err("ambiguous candidates should fail");
    assert_eq!(
        ambiguous,
        DeviceContractError::AmbiguousBackendCandidates { count: 2 }
    );

    let decision = BackendExecutionDecision::try_from_selected_candidate(
        vec![candidate],
        InferenceDevicePolicy::Explicit {
            device_class: InferenceDeviceClass::Cuda,
            device_id: Some(device_id("cuda:0")),
        },
        Some(InferenceTaskId::ImageGeneration),
    )
    .expect("single selected candidate should produce a decision");

    assert_eq!(decision.selected_backend_id.as_str(), "pytorch");
    assert_eq!(
        decision.selected_runtime_variant_id.as_str(),
        "pytorch.cuda"
    );
    assert_eq!(
        decision.selected_device_id.as_ref(),
        Some(&device_id("cuda:0"))
    );
    assert_eq!(
        decision.device_decision.selected_device_class,
        InferenceDeviceClass::Cuda
    );
}

#[test]
fn explicit_device_policy_rejects_mismatched_selected_candidate() {
    let cpu_candidate = BackendExecutionCandidate {
        backend_id: backend_id("llama_cpp"),
        model_compatible: true,
        model_ref: None,
        supported_task_ids: vec![InferenceTaskId::TextGeneration],
        runtime_variant_id: runtime_variant_id("llama_cpp.cpu"),
        device_class: InferenceDeviceClass::Cpu,
        device_id: Some(device_id("cpu")),
        resource_estimate: None,
        observed_throughput: None,
        diagnostics: Vec::new(),
    };

    let class_error = BackendExecutionDecision::try_from_selected_candidate(
        vec![cpu_candidate.clone()],
        InferenceDevicePolicy::Explicit {
            device_class: InferenceDeviceClass::Cuda,
            device_id: None,
        },
        Some(InferenceTaskId::TextGeneration),
    )
    .expect_err("explicit CUDA policy must not select CPU candidate");
    assert_eq!(
        class_error,
        DeviceContractError::ExplicitDeviceClassUnavailable {
            requested: InferenceDeviceClass::Cuda,
            candidate: InferenceDeviceClass::Cpu,
        }
    );

    let id_error = BackendExecutionDecision::try_from_selected_candidate(
        vec![BackendExecutionCandidate {
            device_class: InferenceDeviceClass::Cuda,
            device_id: Some(device_id("cuda:1")),
            runtime_variant_id: runtime_variant_id("llama_cpp.cuda"),
            ..cpu_candidate
        }],
        InferenceDevicePolicy::Explicit {
            device_class: InferenceDeviceClass::Cuda,
            device_id: Some(device_id("cuda:0")),
        },
        Some(InferenceTaskId::TextGeneration),
    )
    .expect_err("explicit CUDA device id must not select another CUDA device");
    assert_eq!(
        id_error,
        DeviceContractError::ExplicitDeviceIdUnavailable {
            requested: device_id("cuda:0"),
            candidate: Some(device_id("cuda:1")),
        }
    );
}

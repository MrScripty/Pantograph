use crate::backend::{
    BackendCapabilities, BackendCapabilityFacts, BackendComponentCapability,
    BackendDefaultStartMode, BackendFeatureCapabilityFacts, BackendFeatureSupport,
    BackendModelSourceCapabilityFacts, BackendTaskCapability,
};
use crate::device_contracts::{InferenceDeviceClass, RuntimeVariantCapability};
use crate::model_contracts::{InferenceModality, SupportTier, TaskModalitySignature};
use crate::RuntimeVariantId;
use crate::{BackendHintLabel, BackendInfo, ModelArtifactKind, ResolvedModelPackageFacts};

use super::*;

fn fixture(raw: &str) -> ResolvedModelPackageFacts {
    serde_json::from_str(raw).expect("fixture should decode")
}

fn backend_info(backend_key: &str, capabilities: BackendCapabilities) -> BackendInfo {
    BackendInfo {
        name: backend_key.to_string(),
        backend_key: backend_key.to_string(),
        description: format!("{backend_key} test backend"),
        capabilities,
        default_start_mode: BackendDefaultStartMode::Inference,
        active: false,
        available: true,
        unavailable_reason: None,
        can_install: false,
        runtime_binary_id: None,
    }
}

fn available_variant(runtime_variant_id: &str) -> RuntimeVariantCapability {
    RuntimeVariantCapability {
        runtime_variant_id: RuntimeVariantId::parse(runtime_variant_id)
            .expect("test runtime variant id should parse"),
        device_class: InferenceDeviceClass::Cpu,
        available: true,
        diagnostics: Vec::new(),
    }
}

fn pytorch_diffusers_capabilities() -> BackendCapabilities {
    BackendCapabilities {
        image_generation: true,
        facts: BackendCapabilityFacts {
            tasks: vec![BackendTaskCapability {
                task_id: InferenceTaskId::ImageGeneration,
                support_tier: SupportTier::Experimental,
                modality_signature: TaskModalitySignature::new(
                    vec![InferenceModality::Text],
                    vec![InferenceModality::Image],
                ),
            }],
            preprocessing: BackendComponentCapability::RequiresPackageComponent,
            postprocessing: BackendComponentCapability::BackendManaged,
            model_sources: BackendModelSourceCapabilityFacts {
                artifact_kinds: vec![ModelArtifactKind::DiffusersBundle],
                backend_hints: vec![BackendHintLabel::Diffusers],
                custom_code: BackendFeatureSupport::Unsupported,
            },
            features: BackendFeatureCapabilityFacts::default(),
            runtime_variants: vec![available_variant("pytorch.cpu")],
        },
        ..BackendCapabilities::default()
    }
}

fn candle_embedding_capabilities() -> BackendCapabilities {
    BackendCapabilities {
        embeddings: true,
        facts: BackendCapabilityFacts {
            tasks: vec![BackendTaskCapability::stable(
                InferenceTaskId::Embedding,
                vec![InferenceModality::Text],
                vec![InferenceModality::Embedding],
            )],
            preprocessing: BackendComponentCapability::RequiresPackageComponent,
            postprocessing: BackendComponentCapability::BackendManaged,
            model_sources: BackendModelSourceCapabilityFacts {
                artifact_kinds: vec![ModelArtifactKind::Safetensors],
                backend_hints: vec![BackendHintLabel::Candle],
                custom_code: BackendFeatureSupport::Unsupported,
            },
            features: BackendFeatureCapabilityFacts::default(),
            runtime_variants: vec![available_variant("candle.cpu")],
        },
        ..BackendCapabilities::default()
    }
}

#[test]
fn diffusers_package_facts_emit_pytorch_candidate_when_capabilities_support_it() {
    let package = fixture(include_str!(
        "../tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
    ));
    let backends = vec![
        backend_info("pytorch", pytorch_diffusers_capabilities()),
        backend_info("candle", candle_embedding_capabilities()),
    ];
    let report = normalize_execution_evidence(ExecutionEvidenceRequest {
        task_id: InferenceTaskId::ImageGeneration,
        package_facts: &package,
        backends: &backends,
        graph_runtime_requirement: None,
    });

    assert_eq!(report.candidates.len(), 1);
    assert_eq!(report.candidates[0].backend_key, "pytorch");
    assert!(report.records.iter().any(|record| {
        record.role == ExecutionEvidenceRole::DependencyPackageEvidence
            && record.key == "backend_hint"
            && record.value == "diffusers"
            && record.backend_key.is_none()
    }));
    assert!(!report
        .candidates
        .iter()
        .any(|candidate| candidate.backend_key == "diffusers"));
}

#[test]
fn explicit_pytorch_graph_runtime_filters_to_validated_pytorch_candidate() {
    let package = fixture(include_str!(
        "../tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
    ));
    let requirement =
        GraphRuntimeRequirement::parse("pytorch").expect("graph runtime should parse");
    let backends = vec![
        backend_info("pytorch", pytorch_diffusers_capabilities()),
        backend_info("candle", candle_embedding_capabilities()),
    ];
    let report = normalize_execution_evidence(ExecutionEvidenceRequest {
        task_id: InferenceTaskId::ImageGeneration,
        package_facts: &package,
        backends: &backends,
        graph_runtime_requirement: Some(&requirement),
    });

    assert_eq!(report.candidates.len(), 1);
    assert_eq!(report.candidates[0].backend_key, "pytorch");
    assert!(report.records.iter().any(|record| {
        record.role == ExecutionEvidenceRole::GraphRuntimeConstraint
            && record.key == "runtime"
            && record.value == "pytorch"
    }));
}

#[test]
fn explicit_diffusers_graph_runtime_does_not_alias_to_pytorch_candidate() {
    let package = fixture(include_str!(
        "../tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
    ));
    let requirement =
        GraphRuntimeRequirement::parse("diffusers").expect("graph runtime should parse");
    let backends = vec![backend_info("pytorch", pytorch_diffusers_capabilities())];
    let report = normalize_execution_evidence(ExecutionEvidenceRequest {
        task_id: InferenceTaskId::ImageGeneration,
        package_facts: &package,
        backends: &backends,
        graph_runtime_requirement: Some(&requirement),
    });

    assert!(report.candidates.is_empty());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == ExecutionEvidenceDiagnosticCode::GraphRuntimeRequirementUnsatisfied
            && diagnostic.requested_runtime_key.as_deref() == Some("diffusers")
    }));
}

#[test]
fn diffusers_candidate_requires_backend_diffusers_capability_facts() {
    let package = fixture(include_str!(
        "../tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
    ));
    let mut unsupported = pytorch_diffusers_capabilities();
    unsupported
        .facts
        .model_sources
        .artifact_kinds
        .retain(|kind| kind != &ModelArtifactKind::DiffusersBundle);
    let backends = vec![backend_info("pytorch", unsupported)];
    let report = normalize_execution_evidence(ExecutionEvidenceRequest {
        task_id: InferenceTaskId::ImageGeneration,
        package_facts: &package,
        backends: &backends,
        graph_runtime_requirement: None,
    });

    assert!(report.candidates.is_empty());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == ExecutionEvidenceDiagnosticCode::BackendCompatibilityRejected
            && diagnostic.backend_key.as_deref() == Some("pytorch")
    }));
}

#[test]
fn diffusers_candidate_requires_present_diffusers_package_evidence() {
    let mut package = fixture(include_str!(
        "../tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
    ));
    package.diffusers = None;
    let backends = vec![backend_info("pytorch", pytorch_diffusers_capabilities())];
    let report = normalize_execution_evidence(ExecutionEvidenceRequest {
        task_id: InferenceTaskId::ImageGeneration,
        package_facts: &package,
        backends: &backends,
        graph_runtime_requirement: None,
    });

    assert!(report.candidates.is_empty());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == ExecutionEvidenceDiagnosticCode::RequiredPackageEvidenceUnavailable
            && diagnostic.backend_key.as_deref() == Some("pytorch")
    }));
}

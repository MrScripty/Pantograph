//! Backend-owned runtime capability helpers.
//!
//! Hosts may contribute runtime capabilities from producer-specific runtime
//! facts, but the capability-shape mapping belongs in backend Rust rather than
//! adapter modules.

use crate::HostRuntimeModeSnapshot;
use pantograph_runtime_identity::{
    backend_key_aliases, canonical_runtime_backend_key, canonical_runtime_id,
    runtime_backend_key_aliases, runtime_display_name,
};
use pantograph_workflow_service::{
    WorkflowBackendCapabilityFacts, WorkflowBackendComponentCapability,
    WorkflowBackendFeatureCapabilityFacts, WorkflowBackendFeatureSupport, WorkflowBackendHintLabel,
    WorkflowBackendModelSourceCapabilityFacts, WorkflowBackendRequestCancellationSemantics,
    WorkflowBackendRequestCleanupSemantics, WorkflowBackendRequestLifecycleFacts,
    WorkflowBackendRequestLifecyclePhaseFacts, WorkflowBackendTaskCapability,
    WorkflowCapabilitiesResponse, WorkflowInferenceExecutionInputKind,
    WorkflowInferenceExecutionResultKind, WorkflowInferenceLifecyclePhase,
    WorkflowInferenceModality, WorkflowInferenceTaskId, WorkflowModelArtifactKind,
    WorkflowRuntimeCapability, WorkflowRuntimeInstallState, WorkflowRuntimeReadinessState,
    WorkflowRuntimeSourceKind, WorkflowSupportTier, WorkflowTaskModalitySignature,
    WorkflowTaskRequestContract, WorkflowTaskStreamingSupport,
};

const PYTHON_SIDECAR_RUNTIME_IDS: &[&str] =
    &["pytorch", "diffusers", "onnx-runtime", "stable_audio"];

pub fn managed_runtime_capabilities(
    runtimes: &[inference::ManagedRuntimeSnapshot],
    available_backends: &[inference::BackendInfo],
    selected_backend_key: &str,
) -> Vec<WorkflowRuntimeCapability> {
    runtimes
        .iter()
        .map(|runtime| {
            let backend_keys = runtime_backend_keys(runtime.id);
            WorkflowRuntimeCapability {
                runtime_id: runtime.id.key().to_string(),
                display_name: runtime.display_name.clone(),
                install_state: managed_runtime_install_state(runtime.install_state),
                available: runtime.available,
                configured: runtime.readiness_state
                    == inference::ManagedRuntimeReadinessState::Ready,
                can_install: runtime.can_install,
                can_remove: runtime.can_remove,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: runtime_matches_backend(&backend_keys, selected_backend_key),
                readiness_state: Some(managed_runtime_readiness_state(runtime.readiness_state)),
                selected_version: runtime.selection.selected_version.clone(),
                supports_external_connection: runtime_supports_external_connection(
                    available_backends,
                    &backend_keys,
                ),
                backend_capability_facts: runtime_backend_capability_facts(
                    available_backends,
                    &backend_keys,
                ),
                backend_keys,
                missing_files: runtime.missing_files.clone(),
                unavailable_reason: runtime
                    .unavailable_reason
                    .clone()
                    .or_else(|| managed_runtime_unavailable_reason(runtime)),
            }
        })
        .collect()
}

pub fn managed_binary_runtime_capabilities(
    statuses: &[inference::ManagedBinaryStatus],
    available_backends: &[inference::BackendInfo],
    selected_backend_key: &str,
) -> Vec<WorkflowRuntimeCapability> {
    statuses
        .iter()
        .filter(|status| status.category == inference::ManagedBinaryCategory::RuntimeSidecar)
        .map(|status| {
            let runtime_id = managed_binary_runtime_id(status);
            let backend_keys = managed_binary_runtime_backend_keys(&runtime_id);
            WorkflowRuntimeCapability {
                runtime_id,
                display_name: status.display_name.clone(),
                install_state: managed_runtime_install_state(status.install_state),
                available: status.available,
                configured: status.readiness_state
                    == inference::ManagedRuntimeReadinessState::Ready,
                can_install: status.can_install,
                can_remove: status.can_remove,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: runtime_matches_backend(&backend_keys, selected_backend_key),
                readiness_state: Some(managed_runtime_readiness_state(status.readiness_state)),
                selected_version: status.selected_version.clone(),
                supports_external_connection: runtime_supports_external_connection(
                    available_backends,
                    &backend_keys,
                ),
                backend_capability_facts: runtime_backend_capability_facts(
                    available_backends,
                    &backend_keys,
                ),
                backend_keys,
                missing_files: status.missing_files.clone(),
                unavailable_reason: status
                    .unavailable_reason
                    .clone()
                    .or_else(|| managed_binary_unavailable_reason(status)),
            }
        })
        .collect()
}

pub fn host_runtime_capabilities(
    backends: &[inference::BackendInfo],
    selected_backend_key: &str,
) -> Vec<WorkflowRuntimeCapability> {
    backends
        .iter()
        .filter_map(|backend| host_runtime_capability(backend, selected_backend_key))
        .collect()
}

pub fn dedicated_embedding_runtime_capabilities(
    snapshot: Option<inference::RuntimeLifecycleSnapshot>,
) -> Vec<WorkflowRuntimeCapability> {
    let Some(snapshot) = snapshot else {
        return Vec::new();
    };

    vec![WorkflowRuntimeCapability {
        runtime_id: snapshot
            .runtime_id
            .as_deref()
            .map(canonical_runtime_id)
            .unwrap_or_else(|| "llama.cpp.embedding".to_string()),
        display_name: "Dedicated embedding runtime".to_string(),
        install_state: WorkflowRuntimeInstallState::Installed,
        available: snapshot.active,
        configured: snapshot.active,
        can_install: false,
        can_remove: false,
        source_kind: WorkflowRuntimeSourceKind::Host,
        selected: false,
        readiness_state: Some(if snapshot.active {
            WorkflowRuntimeReadinessState::Ready
        } else {
            WorkflowRuntimeReadinessState::Unknown
        }),
        selected_version: None,
        supports_external_connection: false,
        backend_capability_facts: None,
        backend_keys: backend_key_aliases("llama.cpp", "llama_cpp"),
        missing_files: Vec::new(),
        unavailable_reason: snapshot.last_error,
    }]
}

pub fn runtime_capabilities_from_mode_info(
    mode_info: &HostRuntimeModeSnapshot,
) -> Vec<WorkflowRuntimeCapability> {
    let mut capabilities = Vec::new();
    capabilities.extend(dedicated_embedding_runtime_capabilities(
        mode_info.embedding_runtime.clone(),
    ));
    capabilities
}

pub fn python_runtime_capabilities(
    executable_probe: Result<std::path::PathBuf, String>,
    selected_backend_key: &str,
) -> Vec<WorkflowRuntimeCapability> {
    let (available, unavailable_reason) = match executable_probe {
        Ok(_) => (true, None),
        Err(reason) => (false, Some(reason)),
    };
    PYTHON_SIDECAR_RUNTIME_IDS
        .iter()
        .map(|runtime_id| {
            let display_name = runtime_display_name(runtime_id).unwrap_or("Python sidecar runtime");
            let backend_keys = runtime_backend_key_aliases(display_name, runtime_id);
            WorkflowRuntimeCapability {
                runtime_id: runtime_id.to_string(),
                display_name: display_name.to_string(),
                install_state: if available {
                    WorkflowRuntimeInstallState::SystemProvided
                } else {
                    WorkflowRuntimeInstallState::Missing
                },
                available,
                configured: available,
                can_install: false,
                can_remove: false,
                source_kind: WorkflowRuntimeSourceKind::System,
                selected: runtime_matches_backend(&backend_keys, selected_backend_key),
                readiness_state: Some(if available {
                    WorkflowRuntimeReadinessState::Ready
                } else {
                    WorkflowRuntimeReadinessState::Missing
                }),
                selected_version: None,
                supports_external_connection: false,
                backend_capability_facts: None,
                backend_keys,
                missing_files: Vec::new(),
                unavailable_reason: unavailable_reason.clone(),
            }
        })
        .collect()
}

pub fn capability_runtime_lifecycle_snapshot(
    capabilities: Option<&WorkflowCapabilitiesResponse>,
) -> Option<inference::RuntimeLifecycleSnapshot> {
    let capabilities = capabilities?;
    let selected_runtime = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.selected)
        .or_else(|| {
            if capabilities.runtime_requirements.required_backends.len() != 1 {
                return None;
            }

            let required_backend_key = &capabilities.runtime_requirements.required_backends[0];
            capabilities
                .runtime_capabilities
                .iter()
                .filter(|capability| {
                    runtime_capability_matches_required_backend(capability, required_backend_key)
                })
                .max_by(|left, right| {
                    (
                        left.available && left.configured,
                        left.available,
                        left.configured,
                    )
                        .cmp(&(
                            right.available && right.configured,
                            right.available,
                            right.configured,
                        ))
                        .then_with(|| left.runtime_id.cmp(&right.runtime_id))
                })
        })?;
    let lifecycle_decision_reason = if selected_runtime.selected {
        "selected_runtime_reported"
    } else {
        "required_runtime_reported"
    };

    Some(inference::RuntimeLifecycleSnapshot {
        runtime_id: Some(canonical_runtime_id(&selected_runtime.runtime_id))
            .filter(|runtime_id| !runtime_id.is_empty()),
        runtime_instance_id: None,
        warmup_started_at_ms: None,
        warmup_completed_at_ms: None,
        warmup_duration_ms: None,
        runtime_reused: None,
        lifecycle_decision_reason: Some(lifecycle_decision_reason.to_string()),
        active: false,
        last_error: selected_runtime
            .unavailable_reason
            .clone()
            .filter(|reason| !reason.trim().is_empty()),
    })
}

fn runtime_capability_matches_required_backend(
    capability: &WorkflowRuntimeCapability,
    required_backend_key: &str,
) -> bool {
    let required_backend_key = canonical_runtime_backend_key(required_backend_key);
    canonical_runtime_backend_key(&capability.runtime_id) == required_backend_key
        || capability
            .backend_keys
            .iter()
            .any(|backend_key| canonical_runtime_backend_key(backend_key) == required_backend_key)
}

fn runtime_backend_keys(binary_id: inference::ManagedBinaryId) -> Vec<String> {
    match binary_id {
        inference::ManagedBinaryId::LlamaCpp => backend_key_aliases("llama.cpp", "llama_cpp"),
    }
}

fn managed_binary_runtime_id(status: &inference::ManagedBinaryStatus) -> String {
    status
        .key
        .as_str()
        .strip_prefix("runtime:")
        .unwrap_or_else(|| status.key.as_str())
        .to_string()
}

fn managed_binary_runtime_backend_keys(runtime_id: &str) -> Vec<String> {
    match runtime_id {
        "llama_cpp" => backend_key_aliases("llama.cpp", "llama_cpp"),
        _ => runtime_backend_key_aliases(runtime_id, runtime_id),
    }
}

fn runtime_supports_external_connection(
    available_backends: &[inference::BackendInfo],
    backend_keys: &[String],
) -> bool {
    let normalized_backend_keys = backend_keys
        .iter()
        .map(|backend_key| inference::backend::canonical_backend_key(backend_key))
        .collect::<std::collections::HashSet<_>>();

    available_backends.iter().any(|backend| {
        normalized_backend_keys.contains(&backend.backend_key)
            && backend.capabilities.external_connection
    })
}

fn runtime_backend_capability_facts(
    available_backends: &[inference::BackendInfo],
    backend_keys: &[String],
) -> Option<WorkflowBackendCapabilityFacts> {
    let normalized_backend_keys = backend_keys
        .iter()
        .map(|backend_key| inference::backend::canonical_backend_key(backend_key))
        .collect::<std::collections::HashSet<_>>();

    available_backends
        .iter()
        .find(|backend| normalized_backend_keys.contains(&backend.backend_key))
        .map(|backend| project_backend_capability_facts(&backend.capabilities.facts))
}

fn project_backend_capability_facts(
    facts: &inference::BackendCapabilityFacts,
) -> WorkflowBackendCapabilityFacts {
    WorkflowBackendCapabilityFacts {
        tasks: facts
            .tasks
            .iter()
            .map(|task| WorkflowBackendTaskCapability {
                task_id: workflow_task_id(&task.task_id),
                support_tier: workflow_support_tier(&task.support_tier),
                modality_signature: WorkflowTaskModalitySignature {
                    inputs: task
                        .modality_signature
                        .inputs
                        .iter()
                        .map(workflow_modality)
                        .collect(),
                    outputs: task
                        .modality_signature
                        .outputs
                        .iter()
                        .map(workflow_modality)
                        .collect(),
                },
                request_contract: inference::TaskRequestContract::for_task(&task.task_id)
                    .map(|contract| workflow_task_request_contract(&contract)),
            })
            .collect(),
        preprocessing: workflow_component_capability(facts.preprocessing),
        postprocessing: workflow_component_capability(facts.postprocessing),
        model_sources: WorkflowBackendModelSourceCapabilityFacts {
            artifact_kinds: facts
                .model_sources
                .artifact_kinds
                .iter()
                .map(workflow_model_artifact_kind)
                .collect(),
            backend_hints: facts
                .model_sources
                .backend_hints
                .iter()
                .map(workflow_backend_hint_label)
                .collect(),
            custom_code: workflow_feature_support(facts.model_sources.custom_code),
        },
        features: WorkflowBackendFeatureCapabilityFacts {
            streaming: workflow_feature_support(facts.features.streaming),
            device_selection: workflow_feature_support(facts.features.device_selection),
            external_connection: workflow_feature_support(facts.features.external_connection),
            kv_cache: workflow_feature_support(facts.features.kv_cache),
        },
        request_lifecycle: workflow_request_lifecycle_facts(&facts.request_lifecycle_facts()),
    }
}

fn workflow_request_lifecycle_facts(
    facts: &inference::BackendRequestLifecycleFacts,
) -> WorkflowBackendRequestLifecycleFacts {
    WorkflowBackendRequestLifecycleFacts {
        phases: facts
            .phases
            .iter()
            .map(|phase| WorkflowBackendRequestLifecyclePhaseFacts {
                phase: workflow_lifecycle_phase(&phase.phase),
                component: workflow_component_capability(phase.component),
                cancellation: workflow_request_cancellation_semantics(phase.cancellation),
                cleanup: workflow_request_cleanup_semantics(phase.cleanup),
            })
            .collect(),
        kv_cache_publication_cleanup: workflow_request_cleanup_semantics(
            facts.kv_cache_publication_cleanup,
        ),
    }
}

fn workflow_lifecycle_phase(
    phase: &inference::InferenceLifecyclePhase,
) -> WorkflowInferenceLifecyclePhase {
    match phase {
        inference::InferenceLifecyclePhase::ModelPackageResolution => {
            WorkflowInferenceLifecyclePhase::ModelPackageResolution
        }
        inference::InferenceLifecyclePhase::TaskValidation => {
            WorkflowInferenceLifecyclePhase::TaskValidation
        }
        inference::InferenceLifecyclePhase::Preprocessing => {
            WorkflowInferenceLifecyclePhase::Preprocessing
        }
        inference::InferenceLifecyclePhase::BackendExecution => {
            WorkflowInferenceLifecyclePhase::BackendExecution
        }
        inference::InferenceLifecyclePhase::Postprocessing => {
            WorkflowInferenceLifecyclePhase::Postprocessing
        }
        inference::InferenceLifecyclePhase::ResultProjection => {
            WorkflowInferenceLifecyclePhase::ResultProjection
        }
    }
}

fn workflow_request_cancellation_semantics(
    semantics: inference::BackendRequestCancellationSemantics,
) -> WorkflowBackendRequestCancellationSemantics {
    match semantics {
        inference::BackendRequestCancellationSemantics::Unknown => {
            WorkflowBackendRequestCancellationSemantics::Unknown
        }
        inference::BackendRequestCancellationSemantics::NotApplicable => {
            WorkflowBackendRequestCancellationSemantics::NotApplicable
        }
        inference::BackendRequestCancellationSemantics::NotSupported => {
            WorkflowBackendRequestCancellationSemantics::NotSupported
        }
        inference::BackendRequestCancellationSemantics::AdapterManaged => {
            WorkflowBackendRequestCancellationSemantics::AdapterManaged
        }
        inference::BackendRequestCancellationSemantics::DropConsumer => {
            WorkflowBackendRequestCancellationSemantics::DropConsumer
        }
    }
}

fn workflow_request_cleanup_semantics(
    semantics: inference::BackendRequestCleanupSemantics,
) -> WorkflowBackendRequestCleanupSemantics {
    match semantics {
        inference::BackendRequestCleanupSemantics::Unknown => {
            WorkflowBackendRequestCleanupSemantics::Unknown
        }
        inference::BackendRequestCleanupSemantics::NotApplicable => {
            WorkflowBackendRequestCleanupSemantics::NotApplicable
        }
        inference::BackendRequestCleanupSemantics::NotRequired => {
            WorkflowBackendRequestCleanupSemantics::NotRequired
        }
        inference::BackendRequestCleanupSemantics::AdapterManaged => {
            WorkflowBackendRequestCleanupSemantics::AdapterManaged
        }
        inference::BackendRequestCleanupSemantics::DropStream => {
            WorkflowBackendRequestCleanupSemantics::DropStream
        }
        inference::BackendRequestCleanupSemantics::RollbackPublication => {
            WorkflowBackendRequestCleanupSemantics::RollbackPublication
        }
    }
}

fn workflow_model_artifact_kind(
    artifact_kind: &inference::ModelArtifactKind,
) -> WorkflowModelArtifactKind {
    match artifact_kind {
        inference::ModelArtifactKind::Gguf => WorkflowModelArtifactKind::Gguf,
        inference::ModelArtifactKind::HfCompatibleDirectory => {
            WorkflowModelArtifactKind::HfCompatibleDirectory
        }
        inference::ModelArtifactKind::Safetensors => WorkflowModelArtifactKind::Safetensors,
        inference::ModelArtifactKind::DiffusersBundle => WorkflowModelArtifactKind::DiffusersBundle,
        inference::ModelArtifactKind::Onnx => WorkflowModelArtifactKind::Onnx,
        inference::ModelArtifactKind::Adapter => WorkflowModelArtifactKind::Adapter,
        inference::ModelArtifactKind::Shard => WorkflowModelArtifactKind::Shard,
        inference::ModelArtifactKind::Unknown => WorkflowModelArtifactKind::Unknown,
    }
}

fn workflow_backend_hint_label(
    backend_hint: &inference::BackendHintLabel,
) -> WorkflowBackendHintLabel {
    match backend_hint {
        inference::BackendHintLabel::Transformers => WorkflowBackendHintLabel::Transformers,
        inference::BackendHintLabel::LlamaCpp => WorkflowBackendHintLabel::LlamaCpp,
        inference::BackendHintLabel::Vllm => WorkflowBackendHintLabel::Vllm,
        inference::BackendHintLabel::Mlx => WorkflowBackendHintLabel::Mlx,
        inference::BackendHintLabel::Candle => WorkflowBackendHintLabel::Candle,
        inference::BackendHintLabel::Diffusers => WorkflowBackendHintLabel::Diffusers,
        inference::BackendHintLabel::OnnxRuntime => WorkflowBackendHintLabel::OnnxRuntime,
    }
}

fn workflow_feature_support(
    support: inference::BackendFeatureSupport,
) -> WorkflowBackendFeatureSupport {
    match support {
        inference::BackendFeatureSupport::Supported => WorkflowBackendFeatureSupport::Supported,
        inference::BackendFeatureSupport::Unsupported => WorkflowBackendFeatureSupport::Unsupported,
        inference::BackendFeatureSupport::Unknown => WorkflowBackendFeatureSupport::Unknown,
    }
}

fn workflow_component_capability(
    capability: inference::BackendComponentCapability,
) -> WorkflowBackendComponentCapability {
    match capability {
        inference::BackendComponentCapability::Unknown => {
            WorkflowBackendComponentCapability::Unknown
        }
        inference::BackendComponentCapability::NotRequired => {
            WorkflowBackendComponentCapability::NotRequired
        }
        inference::BackendComponentCapability::BackendManaged => {
            WorkflowBackendComponentCapability::BackendManaged
        }
        inference::BackendComponentCapability::RequiresPackageComponent => {
            WorkflowBackendComponentCapability::RequiresPackageComponent
        }
        inference::BackendComponentCapability::Unsupported => {
            WorkflowBackendComponentCapability::Unsupported
        }
    }
}

fn workflow_support_tier(tier: &inference::SupportTier) -> WorkflowSupportTier {
    match tier {
        inference::SupportTier::Stable => WorkflowSupportTier::Stable,
        inference::SupportTier::Experimental => WorkflowSupportTier::Experimental,
        inference::SupportTier::Roadmap => WorkflowSupportTier::Roadmap,
        inference::SupportTier::Unsupported => WorkflowSupportTier::Unsupported,
        inference::SupportTier::Unknown => WorkflowSupportTier::Unknown,
    }
}

fn workflow_task_request_contract(
    contract: &inference::TaskRequestContract,
) -> WorkflowTaskRequestContract {
    WorkflowTaskRequestContract {
        task_id: workflow_task_id(&contract.task_id),
        input_kind: workflow_input_kind(contract.input_kind),
        result_kind: workflow_result_kind(contract.result_kind),
        execution_supported: contract.execution_supported,
        streaming_support: workflow_task_streaming_support(&contract.streaming_support),
        required_input_modalities: contract
            .required_input_modalities
            .iter()
            .map(workflow_modality)
            .collect(),
        output_modalities: contract
            .output_modalities
            .iter()
            .map(workflow_modality)
            .collect(),
    }
}

fn workflow_input_kind(
    kind: inference::InferenceExecutionInputKind,
) -> WorkflowInferenceExecutionInputKind {
    match kind {
        inference::InferenceExecutionInputKind::TextGeneration => {
            WorkflowInferenceExecutionInputKind::TextGeneration
        }
        inference::InferenceExecutionInputKind::Embedding => {
            WorkflowInferenceExecutionInputKind::Embedding
        }
        inference::InferenceExecutionInputKind::Rerank => {
            WorkflowInferenceExecutionInputKind::Rerank
        }
        inference::InferenceExecutionInputKind::ImageGeneration => {
            WorkflowInferenceExecutionInputKind::ImageGeneration
        }
        inference::InferenceExecutionInputKind::ImageUnderstanding => {
            WorkflowInferenceExecutionInputKind::ImageUnderstanding
        }
        inference::InferenceExecutionInputKind::DepthEstimation => {
            WorkflowInferenceExecutionInputKind::DepthEstimation
        }
        inference::InferenceExecutionInputKind::AudioTranscription => {
            WorkflowInferenceExecutionInputKind::AudioTranscription
        }
        inference::InferenceExecutionInputKind::VideoUnderstanding => {
            WorkflowInferenceExecutionInputKind::VideoUnderstanding
        }
        inference::InferenceExecutionInputKind::MultimodalGeneration => {
            WorkflowInferenceExecutionInputKind::MultimodalGeneration
        }
    }
}

fn workflow_result_kind(
    kind: inference::InferenceExecutionResultKind,
) -> WorkflowInferenceExecutionResultKind {
    match kind {
        inference::InferenceExecutionResultKind::TextGeneration => {
            WorkflowInferenceExecutionResultKind::TextGeneration
        }
        inference::InferenceExecutionResultKind::Embedding => {
            WorkflowInferenceExecutionResultKind::Embedding
        }
        inference::InferenceExecutionResultKind::Rerank => {
            WorkflowInferenceExecutionResultKind::Rerank
        }
        inference::InferenceExecutionResultKind::ImageGeneration => {
            WorkflowInferenceExecutionResultKind::ImageGeneration
        }
        inference::InferenceExecutionResultKind::ImageUnderstanding => {
            WorkflowInferenceExecutionResultKind::ImageUnderstanding
        }
        inference::InferenceExecutionResultKind::DepthEstimation => {
            WorkflowInferenceExecutionResultKind::DepthEstimation
        }
        inference::InferenceExecutionResultKind::AudioTranscription => {
            WorkflowInferenceExecutionResultKind::AudioTranscription
        }
        inference::InferenceExecutionResultKind::VideoUnderstanding => {
            WorkflowInferenceExecutionResultKind::VideoUnderstanding
        }
        inference::InferenceExecutionResultKind::MultimodalGeneration => {
            WorkflowInferenceExecutionResultKind::MultimodalGeneration
        }
    }
}

fn workflow_task_streaming_support(
    support: &inference::TaskStreamingSupport,
) -> WorkflowTaskStreamingSupport {
    match support {
        inference::TaskStreamingSupport::Supported => WorkflowTaskStreamingSupport::Supported,
        inference::TaskStreamingSupport::Unsupported => WorkflowTaskStreamingSupport::Unsupported,
        inference::TaskStreamingSupport::BackendDependent => {
            WorkflowTaskStreamingSupport::BackendDependent
        }
        inference::TaskStreamingSupport::Unknown => WorkflowTaskStreamingSupport::Unknown,
    }
}

fn workflow_modality(modality: &inference::InferenceModality) -> WorkflowInferenceModality {
    match modality {
        inference::InferenceModality::Text => WorkflowInferenceModality::Text,
        inference::InferenceModality::Image => WorkflowInferenceModality::Image,
        inference::InferenceModality::Audio => WorkflowInferenceModality::Audio,
        inference::InferenceModality::Video => WorkflowInferenceModality::Video,
        inference::InferenceModality::Embedding => WorkflowInferenceModality::Embedding,
        inference::InferenceModality::Tokens => WorkflowInferenceModality::Tokens,
        inference::InferenceModality::Json => WorkflowInferenceModality::Json,
        inference::InferenceModality::PointCloud => WorkflowInferenceModality::PointCloud,
        inference::InferenceModality::Mesh => WorkflowInferenceModality::Mesh,
        inference::InferenceModality::Other => WorkflowInferenceModality::Other,
    }
}

fn workflow_task_id(task_id: &inference::InferenceTaskId) -> WorkflowInferenceTaskId {
    match task_id {
        inference::InferenceTaskId::TextGeneration => WorkflowInferenceTaskId::TextGeneration,
        inference::InferenceTaskId::ChatCompletion => WorkflowInferenceTaskId::ChatCompletion,
        inference::InferenceTaskId::Embedding => WorkflowInferenceTaskId::Embedding,
        inference::InferenceTaskId::Rerank => WorkflowInferenceTaskId::Rerank,
        inference::InferenceTaskId::ImageGeneration => WorkflowInferenceTaskId::ImageGeneration,
        inference::InferenceTaskId::ImageUnderstanding => {
            WorkflowInferenceTaskId::ImageUnderstanding
        }
        inference::InferenceTaskId::DepthEstimation => WorkflowInferenceTaskId::DepthEstimation,
        inference::InferenceTaskId::AudioTranscription => {
            WorkflowInferenceTaskId::AudioTranscription
        }
        inference::InferenceTaskId::VideoUnderstanding => {
            WorkflowInferenceTaskId::VideoUnderstanding
        }
        inference::InferenceTaskId::MultimodalGeneration => {
            WorkflowInferenceTaskId::MultimodalGeneration
        }
        inference::InferenceTaskId::Unknown => WorkflowInferenceTaskId::Unknown,
    }
}

fn is_python_sidecar_backend(backend: &inference::BackendInfo) -> bool {
    let backend_key = canonical_runtime_backend_key(&backend.backend_key);
    PYTHON_SIDECAR_RUNTIME_IDS
        .iter()
        .any(|runtime_id| canonical_runtime_backend_key(runtime_id) == backend_key)
}

fn host_runtime_capability(
    backend: &inference::BackendInfo,
    selected_backend_key: &str,
) -> Option<WorkflowRuntimeCapability> {
    if backend.runtime_binary_id.is_some() || is_python_sidecar_backend(backend) {
        return None;
    }

    let backend_keys = backend_key_aliases(&backend.name, &backend.backend_key);
    Some(WorkflowRuntimeCapability {
        runtime_id: backend.backend_key.clone(),
        display_name: backend.name.clone(),
        install_state: if backend.available {
            WorkflowRuntimeInstallState::SystemProvided
        } else {
            WorkflowRuntimeInstallState::Missing
        },
        available: backend.available,
        configured: backend.available,
        can_install: backend.can_install,
        can_remove: false,
        source_kind: WorkflowRuntimeSourceKind::Host,
        selected: runtime_matches_backend(&backend_keys, selected_backend_key),
        readiness_state: Some(if backend.available {
            WorkflowRuntimeReadinessState::Ready
        } else {
            WorkflowRuntimeReadinessState::Missing
        }),
        selected_version: None,
        supports_external_connection: backend.capabilities.external_connection,
        backend_capability_facts: Some(project_backend_capability_facts(
            &backend.capabilities.facts,
        )),
        backend_keys,
        missing_files: Vec::new(),
        unavailable_reason: backend.unavailable_reason.clone(),
    })
}

fn managed_runtime_install_state(
    install_state: inference::ManagedBinaryInstallState,
) -> WorkflowRuntimeInstallState {
    match install_state {
        inference::ManagedBinaryInstallState::Installed => WorkflowRuntimeInstallState::Installed,
        inference::ManagedBinaryInstallState::SystemProvided => {
            WorkflowRuntimeInstallState::SystemProvided
        }
        inference::ManagedBinaryInstallState::Missing => WorkflowRuntimeInstallState::Missing,
        inference::ManagedBinaryInstallState::Unsupported => {
            WorkflowRuntimeInstallState::Unsupported
        }
    }
}

fn managed_runtime_readiness_state(
    state: inference::ManagedRuntimeReadinessState,
) -> WorkflowRuntimeReadinessState {
    match state {
        inference::ManagedRuntimeReadinessState::Unknown => WorkflowRuntimeReadinessState::Unknown,
        inference::ManagedRuntimeReadinessState::Missing => WorkflowRuntimeReadinessState::Missing,
        inference::ManagedRuntimeReadinessState::Downloading => {
            WorkflowRuntimeReadinessState::Downloading
        }
        inference::ManagedRuntimeReadinessState::Extracting => {
            WorkflowRuntimeReadinessState::Extracting
        }
        inference::ManagedRuntimeReadinessState::Validating => {
            WorkflowRuntimeReadinessState::Validating
        }
        inference::ManagedRuntimeReadinessState::Ready => WorkflowRuntimeReadinessState::Ready,
        inference::ManagedRuntimeReadinessState::Failed => WorkflowRuntimeReadinessState::Failed,
        inference::ManagedRuntimeReadinessState::Unsupported => {
            WorkflowRuntimeReadinessState::Unsupported
        }
    }
}

fn managed_runtime_unavailable_reason(
    runtime: &inference::ManagedRuntimeSnapshot,
) -> Option<String> {
    match runtime.readiness_state {
        inference::ManagedRuntimeReadinessState::Ready => None,
        inference::ManagedRuntimeReadinessState::Unknown => {
            Some(format!("{} readiness is unknown", runtime.display_name))
        }
        inference::ManagedRuntimeReadinessState::Missing => Some(
            runtime
                .selection
                .selected_version
                .as_ref()
                .map(|version| {
                    format!(
                        "{} selected version '{}' is not installed",
                        runtime.display_name, version
                    )
                })
                .unwrap_or_else(|| format!("{} is not installed", runtime.display_name)),
        ),
        inference::ManagedRuntimeReadinessState::Downloading
        | inference::ManagedRuntimeReadinessState::Extracting
        | inference::ManagedRuntimeReadinessState::Validating => Some(
            runtime
                .active_job
                .as_ref()
                .map(|job| job.status.clone())
                .filter(|status| !status.trim().is_empty())
                .unwrap_or_else(|| {
                    format!(
                        "{} is {}",
                        runtime.display_name,
                        readiness_label(runtime.readiness_state)
                    )
                }),
        ),
        inference::ManagedRuntimeReadinessState::Failed => Some(
            runtime
                .active_job
                .as_ref()
                .and_then(|job| job.error.clone())
                .or_else(|| {
                    runtime.selection.selected_version.as_ref().map(|version| {
                        format!(
                            "{} selected version '{}' failed validation",
                            runtime.display_name, version
                        )
                    })
                })
                .unwrap_or_else(|| format!("{} is not ready", runtime.display_name)),
        ),
        inference::ManagedRuntimeReadinessState::Unsupported => Some(format!(
            "{} is unsupported on this platform",
            runtime.display_name
        )),
    }
}

fn managed_binary_unavailable_reason(status: &inference::ManagedBinaryStatus) -> Option<String> {
    match status.readiness_state {
        inference::ManagedRuntimeReadinessState::Ready => None,
        inference::ManagedRuntimeReadinessState::Unknown => {
            Some(format!("{} readiness is unknown", status.display_name))
        }
        inference::ManagedRuntimeReadinessState::Missing => Some(
            status
                .selected_version
                .as_ref()
                .map(|version| {
                    format!(
                        "{} selected version '{}' is not installed",
                        status.display_name, version
                    )
                })
                .unwrap_or_else(|| format!("{} is not installed", status.display_name)),
        ),
        inference::ManagedRuntimeReadinessState::Downloading
        | inference::ManagedRuntimeReadinessState::Extracting
        | inference::ManagedRuntimeReadinessState::Validating => Some(
            status
                .active_job
                .as_ref()
                .map(|job| job.status.clone())
                .filter(|job_status| !job_status.trim().is_empty())
                .unwrap_or_else(|| {
                    format!(
                        "{} is {}",
                        status.display_name,
                        readiness_label(status.readiness_state)
                    )
                }),
        ),
        inference::ManagedRuntimeReadinessState::Failed => Some(
            status
                .active_job
                .as_ref()
                .and_then(|job| job.error.clone())
                .or_else(|| {
                    status.selected_version.as_ref().map(|version| {
                        format!(
                            "{} selected version '{}' failed validation",
                            status.display_name, version
                        )
                    })
                })
                .unwrap_or_else(|| format!("{} is not ready", status.display_name)),
        ),
        inference::ManagedRuntimeReadinessState::Unsupported => Some(format!(
            "{} is unsupported on this platform",
            status.display_name
        )),
    }
}

fn readiness_label(state: inference::ManagedRuntimeReadinessState) -> &'static str {
    match state {
        inference::ManagedRuntimeReadinessState::Unknown => "unknown",
        inference::ManagedRuntimeReadinessState::Missing => "missing",
        inference::ManagedRuntimeReadinessState::Downloading => "downloading",
        inference::ManagedRuntimeReadinessState::Extracting => "extracting",
        inference::ManagedRuntimeReadinessState::Validating => "validating",
        inference::ManagedRuntimeReadinessState::Ready => "ready",
        inference::ManagedRuntimeReadinessState::Failed => "failed",
        inference::ManagedRuntimeReadinessState::Unsupported => "unsupported",
    }
}

fn runtime_matches_backend(backend_keys: &[String], selected_backend_key: &str) -> bool {
    backend_keys
        .iter()
        .any(|backend_key| canonical_runtime_backend_key(backend_key) == selected_backend_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use inference::backend::{BackendCapabilities, BackendDefaultStartMode};

    fn assert_runtime_capability_contract(
        capability: &WorkflowRuntimeCapability,
        expected_runtime_id: &str,
        expected_source_kind: WorkflowRuntimeSourceKind,
        expected_install_state: WorkflowRuntimeInstallState,
    ) {
        assert_eq!(capability.runtime_id, expected_runtime_id);
        assert_eq!(capability.source_kind, expected_source_kind);
        assert_eq!(capability.install_state, expected_install_state);
        assert!(!capability.display_name.trim().is_empty());
        assert!(capability
            .backend_keys
            .iter()
            .all(|backend_key| !backend_key.trim().is_empty()));
        assert!(!capability.backend_keys.is_empty());
    }

    #[test]
    fn dedicated_embedding_runtime_capability_reports_dedicated_runtime() {
        let capabilities =
            dedicated_embedding_runtime_capabilities(Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                runtime_instance_id: Some("llama-cpp-embedding-9".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }));

        assert_eq!(capabilities.len(), 1);
        let capability = &capabilities[0];
        assert_eq!(capability.runtime_id, "llama.cpp.embedding");
        assert_eq!(capability.display_name, "Dedicated embedding runtime");
        assert_eq!(capability.source_kind, WorkflowRuntimeSourceKind::Host);
        assert!(capability.available);
        assert!(capability.configured);
        assert!(!capability.selected);
        assert!(capability.backend_keys.contains(&"llama_cpp".to_string()));
        assert!(capability.backend_keys.contains(&"llamacpp".to_string()));
    }

    #[test]
    fn dedicated_embedding_runtime_capability_omits_missing_snapshot() {
        assert!(dedicated_embedding_runtime_capabilities(None).is_empty());
    }

    #[test]
    fn runtime_capabilities_from_mode_info_collects_embedding_runtime_capability() {
        let capabilities = runtime_capabilities_from_mode_info(&HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: None,
            embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                runtime_instance_id: Some("llama-cpp-embedding-4".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(15),
                warmup_duration_ms: Some(5),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
        });

        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].runtime_id, "llama.cpp.embedding");
        assert!(capabilities[0].available);
    }

    #[test]
    fn python_runtime_capabilities_report_python_backed_engines() {
        let capabilities = python_runtime_capabilities(
            Ok(std::path::PathBuf::from("/usr/bin/python3")),
            "pytorch",
        );

        assert_eq!(capabilities.len(), 4);

        let pytorch = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "pytorch")
            .expect("pytorch capability");
        assert!(pytorch.available);
        assert!(pytorch.configured);
        assert_eq!(pytorch.source_kind, WorkflowRuntimeSourceKind::System);
        assert!(pytorch.selected);
        assert!(!pytorch.supports_external_connection);
        assert!(pytorch.backend_keys.contains(&"pytorch".to_string()));
        assert!(pytorch.backend_keys.contains(&"torch".to_string()));

        let diffusion = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "diffusers")
            .expect("diffusers capability");
        assert!(diffusion.backend_keys.contains(&"diffusers".to_string()));

        let onnx = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "onnx-runtime")
            .expect("onnx capability");
        assert!(onnx.backend_keys.contains(&"onnx-runtime".to_string()));

        let stable_audio = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "stable_audio")
            .expect("stable audio capability");
        assert!(stable_audio
            .backend_keys
            .contains(&"stable_audio".to_string()));
    }

    #[test]
    fn python_runtime_capabilities_keep_unavailable_reason() {
        let capabilities = python_runtime_capabilities(
            Err("python executable is not configured".to_string()),
            "llama_cpp",
        );

        assert_eq!(capabilities.len(), 4);
        for capability in capabilities {
            assert!(!capability.available);
            assert!(!capability.configured);
            assert_eq!(capability.source_kind, WorkflowRuntimeSourceKind::System);
            assert!(!capability.selected);
            assert_eq!(
                capability.unavailable_reason.as_deref(),
                Some("python executable is not configured")
            );
        }
    }

    #[test]
    fn host_runtime_capabilities_report_candle_backend() {
        let capabilities = host_runtime_capabilities(
            &[inference::BackendInfo {
                name: "Candle".to_string(),
                backend_key: "candle".to_string(),
                description: "In-process Candle inference".to_string(),
                capabilities: BackendCapabilities {
                    external_connection: false,
                    facts: inference::BackendCapabilityFacts {
                        tasks: vec![inference::BackendTaskCapability::stable(
                            inference::InferenceTaskId::Embedding,
                            vec![inference::InferenceModality::Text],
                            vec![inference::InferenceModality::Embedding],
                        )],
                        preprocessing: inference::BackendComponentCapability::NotRequired,
                        postprocessing: inference::BackendComponentCapability::NotRequired,
                        model_sources: inference::BackendModelSourceCapabilityFacts {
                            artifact_kinds: vec![
                                inference::ModelArtifactKind::HfCompatibleDirectory,
                            ],
                            backend_hints: vec![inference::BackendHintLabel::Candle],
                            custom_code: inference::BackendFeatureSupport::Unsupported,
                        },
                        features: inference::BackendFeatureCapabilityFacts {
                            streaming: inference::BackendFeatureSupport::Unsupported,
                            device_selection: inference::BackendFeatureSupport::Unsupported,
                            external_connection: inference::BackendFeatureSupport::Unsupported,
                            kv_cache: inference::BackendFeatureSupport::Unsupported,
                        },
                    },
                    ..BackendCapabilities::default()
                },
                default_start_mode: BackendDefaultStartMode::Embedding,
                active: true,
                available: true,
                unavailable_reason: None,
                can_install: false,
                runtime_binary_id: None,
            }],
            "candle",
        );

        assert_eq!(capabilities.len(), 1);
        let capability = &capabilities[0];
        assert_eq!(capability.runtime_id, "candle");
        assert_eq!(capability.display_name, "Candle");
        assert_eq!(
            capability.install_state,
            WorkflowRuntimeInstallState::SystemProvided
        );
        assert_eq!(capability.source_kind, WorkflowRuntimeSourceKind::Host);
        assert!(capability.selected);
        assert!(capability.backend_keys.contains(&"candle".to_string()));
        assert!(capability.backend_keys.contains(&"Candle".to_string()));
        let facts = capability
            .backend_capability_facts
            .as_ref()
            .expect("backend facts should be projected");
        assert_eq!(facts.tasks.len(), 1);
        assert_eq!(facts.tasks[0].task_id, WorkflowInferenceTaskId::Embedding);
        assert_eq!(
            facts.tasks[0].modality_signature.outputs,
            vec![WorkflowInferenceModality::Embedding]
        );
        let request_contract = facts.tasks[0]
            .request_contract
            .as_ref()
            .expect("task request contract should be projected");
        assert_eq!(
            request_contract.input_kind,
            WorkflowInferenceExecutionInputKind::Embedding
        );
        assert_eq!(
            request_contract.result_kind,
            WorkflowInferenceExecutionResultKind::Embedding
        );
        assert!(request_contract.execution_supported);
        assert_eq!(
            request_contract.streaming_support,
            WorkflowTaskStreamingSupport::Unsupported
        );
        assert_eq!(
            facts.features.kv_cache,
            WorkflowBackendFeatureSupport::Unsupported
        );
        assert_eq!(
            facts.model_sources.artifact_kinds,
            vec![WorkflowModelArtifactKind::HfCompatibleDirectory]
        );
        assert_eq!(
            facts.request_lifecycle.kv_cache_publication_cleanup,
            WorkflowBackendRequestCleanupSemantics::NotApplicable
        );
        assert!(facts.request_lifecycle.phases.iter().any(|phase| {
            phase.phase == WorkflowInferenceLifecyclePhase::Postprocessing
                && phase.cleanup == WorkflowBackendRequestCleanupSemantics::NotRequired
        }));
    }

    #[test]
    fn host_runtime_capabilities_do_not_duplicate_python_sidecar_backends() {
        let capabilities = host_runtime_capabilities(
            &[
                inference::BackendInfo {
                    name: "Torch".to_string(),
                    backend_key: "torch".to_string(),
                    description: "Transformers through Python".to_string(),
                    capabilities: BackendCapabilities::default(),
                    default_start_mode: BackendDefaultStartMode::Inference,
                    active: false,
                    available: true,
                    unavailable_reason: None,
                    can_install: false,
                    runtime_binary_id: None,
                },
                inference::BackendInfo {
                    name: "Diffusers".to_string(),
                    backend_key: "diffusers".to_string(),
                    description: "Diffusers through Python".to_string(),
                    capabilities: BackendCapabilities::default(),
                    default_start_mode: BackendDefaultStartMode::Inference,
                    active: false,
                    available: true,
                    unavailable_reason: None,
                    can_install: false,
                    runtime_binary_id: None,
                },
            ],
            "pytorch",
        );

        assert!(capabilities.is_empty());
    }

    #[test]
    fn backend_capability_projection_preserves_depth_estimation_task_contract() {
        let facts = project_backend_capability_facts(&inference::BackendCapabilityFacts {
            tasks: vec![inference::BackendTaskCapability {
                task_id: inference::InferenceTaskId::DepthEstimation,
                support_tier: inference::SupportTier::Roadmap,
                modality_signature: inference::TaskModalitySignature::new(
                    vec![inference::InferenceModality::Image],
                    vec![
                        inference::InferenceModality::Image,
                        inference::InferenceModality::PointCloud,
                    ],
                ),
            }],
            ..inference::BackendCapabilityFacts::default()
        });

        assert_eq!(facts.tasks.len(), 1);
        assert_eq!(
            facts.tasks[0].task_id,
            WorkflowInferenceTaskId::DepthEstimation
        );
        assert_eq!(
            facts.tasks[0]
                .request_contract
                .as_ref()
                .map(|contract| contract.input_kind),
            Some(WorkflowInferenceExecutionInputKind::DepthEstimation)
        );
        assert_eq!(
            facts.tasks[0]
                .request_contract
                .as_ref()
                .map(|contract| contract.result_kind),
            Some(WorkflowInferenceExecutionResultKind::DepthEstimation)
        );
        assert_eq!(
            facts.tasks[0].modality_signature.outputs,
            vec![
                WorkflowInferenceModality::Image,
                WorkflowInferenceModality::PointCloud
            ]
        );
    }

    #[test]
    fn managed_runtime_capabilities_preserve_external_connection_support() {
        let capabilities = managed_runtime_capabilities(
            &[inference::ManagedRuntimeSnapshot {
                id: inference::ManagedBinaryId::LlamaCpp,
                display_name: "llama.cpp".to_string(),
                install_state: inference::ManagedBinaryInstallState::Installed,
                readiness_state: inference::ManagedRuntimeReadinessState::Ready,
                available: true,
                can_install: false,
                can_remove: true,
                missing_files: Vec::new(),
                unavailable_reason: None,
                versions: Vec::new(),
                selection: inference::ManagedRuntimeSelectionState {
                    selected_version: Some("b8248".to_string()),
                    active_version: Some("b8248".to_string()),
                    default_version: Some("b8248".to_string()),
                },
                active_job: None,
                job_artifact: None,
            }],
            &[inference::BackendInfo {
                name: "llama.cpp".to_string(),
                backend_key: "llama_cpp".to_string(),
                description: "Managed llama.cpp runtime".to_string(),
                capabilities: BackendCapabilities {
                    external_connection: true,
                    facts: inference::BackendCapabilityFacts {
                        tasks: vec![inference::BackendTaskCapability::stable(
                            inference::InferenceTaskId::ChatCompletion,
                            vec![inference::InferenceModality::Text],
                            vec![inference::InferenceModality::Text],
                        )],
                        preprocessing:
                            inference::BackendComponentCapability::RequiresPackageComponent,
                        postprocessing:
                            inference::BackendComponentCapability::RequiresPackageComponent,
                        model_sources: inference::BackendModelSourceCapabilityFacts {
                            artifact_kinds: vec![inference::ModelArtifactKind::Gguf],
                            backend_hints: vec![inference::BackendHintLabel::LlamaCpp],
                            custom_code: inference::BackendFeatureSupport::Unsupported,
                        },
                        features: inference::BackendFeatureCapabilityFacts {
                            streaming: inference::BackendFeatureSupport::Supported,
                            device_selection: inference::BackendFeatureSupport::Supported,
                            external_connection: inference::BackendFeatureSupport::Supported,
                            kv_cache: inference::BackendFeatureSupport::Supported,
                        },
                    },
                    ..BackendCapabilities::default()
                },
                default_start_mode: BackendDefaultStartMode::Inference,
                active: false,
                available: true,
                unavailable_reason: None,
                can_install: true,
                runtime_binary_id: Some(inference::ManagedBinaryId::LlamaCpp),
            }],
            "llama_cpp",
        );

        assert_eq!(capabilities.len(), 1);
        let capability = &capabilities[0];
        assert_eq!(capability.runtime_id, "llama_cpp");
        assert_eq!(capability.source_kind, WorkflowRuntimeSourceKind::Managed);
        assert!(capability.selected);
        assert!(capability.supports_external_connection);
        assert_eq!(
            capability
                .backend_capability_facts
                .as_ref()
                .and_then(|facts| facts.tasks.first())
                .map(|task| task.task_id),
            Some(WorkflowInferenceTaskId::ChatCompletion)
        );
        let facts = capability
            .backend_capability_facts
            .as_ref()
            .expect("backend facts should be projected");
        assert_eq!(
            facts.features.external_connection,
            WorkflowBackendFeatureSupport::Supported
        );
        assert_eq!(
            facts.features.kv_cache,
            WorkflowBackendFeatureSupport::Supported
        );
        assert_eq!(
            facts.model_sources.backend_hints,
            vec![WorkflowBackendHintLabel::LlamaCpp]
        );
        assert_eq!(
            facts.request_lifecycle.kv_cache_publication_cleanup,
            WorkflowBackendRequestCleanupSemantics::RollbackPublication
        );
        assert!(facts.request_lifecycle.phases.iter().any(|phase| {
            phase.phase == WorkflowInferenceLifecyclePhase::BackendExecution
                && phase.cancellation == WorkflowBackendRequestCancellationSemantics::DropConsumer
                && phase.cleanup == WorkflowBackendRequestCleanupSemantics::DropStream
        }));
        assert_eq!(
            capability.install_state,
            WorkflowRuntimeInstallState::Installed
        );
    }

    #[test]
    fn runtime_capability_contract_family_stays_aligned_across_producers() {
        let managed_capability = managed_runtime_capabilities(
            &[inference::ManagedRuntimeSnapshot {
                id: inference::ManagedBinaryId::LlamaCpp,
                display_name: "llama.cpp".to_string(),
                install_state: inference::ManagedBinaryInstallState::Installed,
                readiness_state: inference::ManagedRuntimeReadinessState::Ready,
                available: true,
                can_install: false,
                can_remove: true,
                missing_files: Vec::new(),
                unavailable_reason: None,
                versions: Vec::new(),
                selection: inference::ManagedRuntimeSelectionState {
                    selected_version: Some("b8248".to_string()),
                    active_version: Some("b8248".to_string()),
                    default_version: Some("b8248".to_string()),
                },
                active_job: None,
                job_artifact: None,
            }],
            &[inference::BackendInfo {
                name: "llama.cpp".to_string(),
                backend_key: "llama_cpp".to_string(),
                description: "Managed llama.cpp runtime".to_string(),
                capabilities: BackendCapabilities {
                    external_connection: true,
                    ..BackendCapabilities::default()
                },
                default_start_mode: BackendDefaultStartMode::Inference,
                active: false,
                available: true,
                unavailable_reason: None,
                can_install: true,
                runtime_binary_id: Some(inference::ManagedBinaryId::LlamaCpp),
            }],
            "llama_cpp",
        )
        .remove(0);

        let host_capability = host_runtime_capabilities(
            &[inference::BackendInfo {
                name: "Candle".to_string(),
                backend_key: "candle".to_string(),
                description: "In-process Candle inference".to_string(),
                capabilities: BackendCapabilities {
                    external_connection: false,
                    ..BackendCapabilities::default()
                },
                default_start_mode: BackendDefaultStartMode::Embedding,
                active: true,
                available: true,
                unavailable_reason: None,
                can_install: false,
                runtime_binary_id: None,
            }],
            "candle",
        )
        .remove(0);

        let embedding_capability =
            dedicated_embedding_runtime_capabilities(Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                runtime_instance_id: Some("embedding-runtime-1".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }))
            .remove(0);

        let python_capability = python_runtime_capabilities(
            Ok(std::path::PathBuf::from("/usr/bin/python3")),
            "pytorch",
        )
        .into_iter()
        .find(|capability| capability.runtime_id == "pytorch")
        .expect("pytorch capability");

        assert_runtime_capability_contract(
            &managed_capability,
            "llama_cpp",
            WorkflowRuntimeSourceKind::Managed,
            WorkflowRuntimeInstallState::Installed,
        );
        assert_runtime_capability_contract(
            &host_capability,
            "candle",
            WorkflowRuntimeSourceKind::Host,
            WorkflowRuntimeInstallState::SystemProvided,
        );
        assert_runtime_capability_contract(
            &embedding_capability,
            "llama.cpp.embedding",
            WorkflowRuntimeSourceKind::Host,
            WorkflowRuntimeInstallState::Installed,
        );
        assert_runtime_capability_contract(
            &python_capability,
            "pytorch",
            WorkflowRuntimeSourceKind::System,
            WorkflowRuntimeInstallState::SystemProvided,
        );
    }

    #[test]
    fn managed_runtime_capabilities_preserve_readiness_context_in_reason() {
        let capabilities = managed_runtime_capabilities(
            &[inference::ManagedRuntimeSnapshot {
                id: inference::ManagedBinaryId::LlamaCpp,
                display_name: "llama.cpp".to_string(),
                install_state: inference::ManagedBinaryInstallState::Installed,
                readiness_state: inference::ManagedRuntimeReadinessState::Validating,
                available: false,
                can_install: false,
                can_remove: true,
                missing_files: Vec::new(),
                unavailable_reason: None,
                versions: Vec::new(),
                selection: inference::ManagedRuntimeSelectionState {
                    selected_version: Some("b8248".to_string()),
                    active_version: None,
                    default_version: Some("b8248".to_string()),
                },
                active_job: Some(inference::ManagedRuntimeJobStatus {
                    state: inference::ManagedRuntimeJobState::Validating,
                    status: "Validating b8248".to_string(),
                    current: 1,
                    total: 1,
                    resumable: false,
                    cancellable: false,
                    error: None,
                }),
                job_artifact: None,
            }],
            &[],
            "llama_cpp",
        );

        assert_eq!(capabilities.len(), 1);
        let capability = &capabilities[0];
        assert!(!capability.configured);
        assert_eq!(
            capability.unavailable_reason.as_deref(),
            Some("Validating b8248")
        );
    }

    #[test]
    fn capability_runtime_lifecycle_snapshot_prefers_selected_runtime() {
        let snapshot = capability_runtime_lifecycle_snapshot(Some(&WorkflowCapabilitiesResponse {
            max_input_bindings: 4,
            max_output_targets: 2,
            max_value_bytes: 1000,
            runtime_requirements: pantograph_workflow_service::WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "high".to_string(),
                required_models: vec!["model-a".to_string()],
                required_backends: vec!["pytorch".to_string()],
                required_extensions: Vec::new(),
            },
            models: Vec::new(),
            runtime_capabilities: vec![pantograph_workflow_service::WorkflowRuntimeCapability {
                runtime_id: "PyTorch".to_string(),
                display_name: "PyTorch (Python sidecar)".to_string(),
                install_state:
                    pantograph_workflow_service::WorkflowRuntimeInstallState::SystemProvided,
                available: true,
                configured: true,
                can_install: false,
                can_remove: false,
                source_kind: pantograph_workflow_service::WorkflowRuntimeSourceKind::System,
                selected: true,
                readiness_state: Some(
                    pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready,
                ),
                selected_version: None,
                supports_external_connection: false,
                backend_capability_facts: None,
                backend_keys: vec!["torch".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        }))
        .expect("selected capability snapshot");

        assert_eq!(snapshot.runtime_id.as_deref(), Some("pytorch"));
        assert_eq!(
            snapshot.lifecycle_decision_reason.as_deref(),
            Some("selected_runtime_reported")
        );
        assert!(!snapshot.active);
    }

    #[test]
    fn capability_runtime_lifecycle_snapshot_matches_required_backend_alias() {
        let snapshot = capability_runtime_lifecycle_snapshot(Some(&WorkflowCapabilitiesResponse {
            max_input_bindings: 4,
            max_output_targets: 2,
            max_value_bytes: 1000,
            runtime_requirements: pantograph_workflow_service::WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "high".to_string(),
                required_models: vec!["model-a".to_string()],
                required_backends: vec!["onnxruntime".to_string()],
                required_extensions: Vec::new(),
            },
            models: Vec::new(),
            runtime_capabilities: vec![pantograph_workflow_service::WorkflowRuntimeCapability {
                runtime_id: "onnx-runtime".to_string(),
                display_name: "ONNX Runtime".to_string(),
                install_state:
                    pantograph_workflow_service::WorkflowRuntimeInstallState::SystemProvided,
                available: true,
                configured: true,
                can_install: false,
                can_remove: false,
                source_kind: pantograph_workflow_service::WorkflowRuntimeSourceKind::System,
                selected: false,
                readiness_state: Some(
                    pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready,
                ),
                selected_version: None,
                supports_external_connection: false,
                backend_capability_facts: None,
                backend_keys: vec!["ONNX Runtime".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        }))
        .expect("required backend snapshot");

        assert_eq!(snapshot.runtime_id.as_deref(), Some("onnx-runtime"));
        assert_eq!(
            snapshot.lifecycle_decision_reason.as_deref(),
            Some("required_runtime_reported")
        );
    }
}

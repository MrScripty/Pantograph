use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use async_trait::async_trait;
use inference::{BackendHintLabel, InferenceTaskId, ModelValidationState, TaskRegistryEntry};
use pantograph_inference_interface_contracts::{
    InferenceArtifactType, InferenceAvailability, InferencePortDescriptor, InferencePortDirection,
    InferencePortId, InferencePortOptions, InferencePortRequirement, InferenceScalarType,
    InferenceTaskKind, InferenceValueType, RuntimeIntentId,
};
use pantograph_runtime_registry::RuntimeRegistryStatus;
use pantograph_workflow_service::graph::{
    InferenceCapabilityFacts, InferenceInterfaceFactsProvider,
    InferenceInterfaceFactsProviderError, InferenceInterfaceGraphResolutionInput,
    InferenceInterfaceResolverFacts, InferenceModelResolutionFacts, InferenceModelResolutionState,
    InferenceRuntimeAvailabilityFact, InferenceRuntimeAvailabilityState,
};

use crate::inference_resource_estimator::conservative_estimates_from_package_logical_size;
use crate::pumas_dispatch_package_facts::{
    PumasDispatchPackageFactsBridgeOutcome, PumasDispatchPackageFactsDiagnosticCode,
    PumasDispatchPackageFactsProjection, PumasDispatchPackageFactsSource,
};
use crate::runtime_dispatch_capability_facts::{
    RuntimeDispatchCapabilityFactsOutcome, RuntimeDispatchCapabilityFactsProjection,
    RuntimeDispatchCapabilityFactsSource, RuntimeDispatchRuntimeCapabilityFacts,
};

#[derive(Clone)]
pub(crate) struct EmbeddedInferenceInterfaceFactsProvider {
    pumas_source: PumasDispatchPackageFactsSource,
    runtime_capability_source: RuntimeDispatchCapabilityFactsSource,
}

impl EmbeddedInferenceInterfaceFactsProvider {
    pub(crate) fn new(
        pumas_source: PumasDispatchPackageFactsSource,
        runtime_capability_source: RuntimeDispatchCapabilityFactsSource,
    ) -> Self {
        Self {
            pumas_source,
            runtime_capability_source,
        }
    }
}

impl fmt::Debug for EmbeddedInferenceInterfaceFactsProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmbeddedInferenceInterfaceFactsProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl InferenceInterfaceFactsProvider for EmbeddedInferenceInterfaceFactsProvider {
    async fn facts_for_resolution_inputs(
        &self,
        inputs: &[InferenceInterfaceGraphResolutionInput],
    ) -> Result<
        BTreeMap<String, InferenceInterfaceResolverFacts>,
        InferenceInterfaceFactsProviderError,
    > {
        let runtime_facts = self.runtime_capability_source.collect();
        let mut facts_by_node_id = BTreeMap::new();
        for input in inputs {
            let package_facts = self.pumas_source.collect(&input.request.model_ref).await;
            facts_by_node_id.insert(
                input.node_id.clone(),
                resolver_facts_from_sources(package_facts, &runtime_facts),
            );
        }
        Ok(facts_by_node_id)
    }
}

fn resolver_facts_from_sources(
    package_outcome: PumasDispatchPackageFactsBridgeOutcome,
    runtime_outcome: &RuntimeDispatchCapabilityFactsOutcome,
) -> InferenceInterfaceResolverFacts {
    let PumasDispatchPackageFactsBridgeOutcome::Projected { facts, .. } = package_outcome else {
        return missing_package_facts(package_outcome);
    };

    let model_state = model_resolution_state(&facts);
    if model_state != InferenceModelResolutionState::Ready {
        return InferenceInterfaceResolverFacts {
            model: InferenceModelResolutionFacts { state: model_state },
            capability: None,
            runtimes: Vec::new(),
            estimate_hints: Vec::new(),
        };
    }

    let runtimes = runtime_facts(&facts, runtime_outcome);
    let capability = capability_facts(&facts, &runtimes);
    let estimates = conservative_estimates_from_package_logical_size(&facts.logical_size);

    InferenceInterfaceResolverFacts {
        model: InferenceModelResolutionFacts { state: model_state },
        capability,
        runtimes,
        estimate_hints: estimates.scheduler_hints,
    }
}

fn missing_package_facts(
    outcome: PumasDispatchPackageFactsBridgeOutcome,
) -> InferenceInterfaceResolverFacts {
    InferenceInterfaceResolverFacts {
        model: InferenceModelResolutionFacts {
            state: package_diagnostic_model_state(outcome.diagnostics()),
        },
        capability: None,
        runtimes: Vec::new(),
        estimate_hints: Vec::new(),
    }
}

fn package_diagnostic_model_state(
    diagnostics: &[crate::pumas_dispatch_package_facts::PumasDispatchPackageFactsDiagnostic],
) -> InferenceModelResolutionState {
    if diagnostics.iter().any(|diagnostic| {
        diagnostic.code == PumasDispatchPackageFactsDiagnosticCode::StalePackageFactsContract
    }) {
        return InferenceModelResolutionState::StaleFacts;
    }
    if diagnostics.iter().any(|diagnostic| {
        diagnostic.code == PumasDispatchPackageFactsDiagnosticCode::SelectedArtifactMismatch
    }) {
        return InferenceModelResolutionState::MissingSelectedArtifact;
    }
    if diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.code,
            PumasDispatchPackageFactsDiagnosticCode::InvalidModelRef
                | PumasDispatchPackageFactsDiagnosticCode::PathCarryingModelRef
                | PumasDispatchPackageFactsDiagnosticCode::PackageFactsDecodeFailed
        )
    }) {
        return InferenceModelResolutionState::InvalidArtifact;
    }
    InferenceModelResolutionState::MissingModelFacts
}

fn model_resolution_state(
    facts: &PumasDispatchPackageFactsProjection,
) -> InferenceModelResolutionState {
    match facts.validation_state {
        ModelValidationState::Valid => InferenceModelResolutionState::Ready,
        ModelValidationState::Degraded => InferenceModelResolutionState::StaleFacts,
        ModelValidationState::Invalid => InferenceModelResolutionState::InvalidArtifact,
        ModelValidationState::Unknown => InferenceModelResolutionState::MissingModelFacts,
    }
}

fn capability_facts(
    facts: &PumasDispatchPackageFactsProjection,
    runtimes: &[InferenceRuntimeAvailabilityFact],
) -> Option<InferenceCapabilityFacts> {
    let task_entry = inference::resolve_task_registry_entry_from_evidence(&facts.task).ok()?;
    let task_kind = InferenceTaskKind::parse(task_entry.canonical_label()).ok()?;
    Some(InferenceCapabilityFacts {
        task_kind,
        inputs: input_ports(&task_entry),
        outputs: output_ports(&task_entry),
        runtime_conditions: Vec::new(),
        supported_runtime_ids: runtimes
            .iter()
            .map(|runtime| runtime.runtime_id.clone())
            .collect(),
    })
}

fn runtime_facts(
    facts: &PumasDispatchPackageFactsProjection,
    runtime_outcome: &RuntimeDispatchCapabilityFactsOutcome,
) -> Vec<InferenceRuntimeAvailabilityFact> {
    let RuntimeDispatchCapabilityFactsOutcome::Projected {
        facts: runtimes, ..
    } = runtime_outcome
    else {
        return Vec::new();
    };
    matching_runtime_facts(facts, runtimes)
}

fn matching_runtime_facts(
    package_facts: &PumasDispatchPackageFactsProjection,
    runtime_facts: &RuntimeDispatchCapabilityFactsProjection,
) -> Vec<InferenceRuntimeAvailabilityFact> {
    let backend_keys = backend_hint_keys(&package_facts.backend_hints);
    if backend_keys.is_empty() {
        return Vec::new();
    }
    runtime_facts
        .runtimes
        .iter()
        .filter(|runtime| {
            runtime
                .backend_keys
                .iter()
                .any(|key| backend_keys.contains(&normalize_backend_key(key)))
        })
        .filter_map(runtime_availability_fact)
        .collect()
}

fn runtime_availability_fact(
    runtime: &RuntimeDispatchRuntimeCapabilityFacts,
) -> Option<InferenceRuntimeAvailabilityFact> {
    Some(InferenceRuntimeAvailabilityFact {
        runtime_id: RuntimeIntentId::parse(&runtime.runtime_id).ok()?,
        state: runtime_availability_state(runtime.status),
        device_ids: Vec::new(),
    })
}

fn runtime_availability_state(status: RuntimeRegistryStatus) -> InferenceRuntimeAvailabilityState {
    match status {
        RuntimeRegistryStatus::Ready
        | RuntimeRegistryStatus::Busy
        | RuntimeRegistryStatus::Warming => InferenceRuntimeAvailabilityState::Available,
        RuntimeRegistryStatus::Stopped | RuntimeRegistryStatus::Stopping => {
            InferenceRuntimeAvailabilityState::NotInstalled
        }
        RuntimeRegistryStatus::Unhealthy | RuntimeRegistryStatus::Failed => {
            InferenceRuntimeAvailabilityState::Unsupported
        }
    }
}

fn input_ports(task_entry: &TaskRegistryEntry) -> Vec<InferencePortDescriptor> {
    match task_entry.task_id {
        InferenceTaskId::ImageGeneration
        | InferenceTaskId::TextGeneration
        | InferenceTaskId::ChatCompletion
        | InferenceTaskId::MultimodalGeneration => vec![port(
            "prompt",
            "Prompt",
            InferencePortDirection::Input,
            InferencePortRequirement::Required,
            InferenceValueType::Scalar(InferenceScalarType::String),
        )],
        _ => Vec::new(),
    }
}

fn output_ports(task_entry: &TaskRegistryEntry) -> Vec<InferencePortDescriptor> {
    match task_entry.task_id {
        InferenceTaskId::ImageGeneration => vec![port(
            "image",
            "Image",
            InferencePortDirection::Output,
            InferencePortRequirement::Required,
            InferenceValueType::Artifact(InferenceArtifactType::Image),
        )],
        InferenceTaskId::TextGeneration
        | InferenceTaskId::ChatCompletion
        | InferenceTaskId::MultimodalGeneration => vec![port(
            "text",
            "Text",
            InferencePortDirection::Output,
            InferencePortRequirement::Required,
            InferenceValueType::Scalar(InferenceScalarType::String),
        )],
        _ => Vec::new(),
    }
}

fn port(
    port_id: &str,
    label: &str,
    direction: InferencePortDirection,
    requirement: InferencePortRequirement,
    value_type: InferenceValueType,
) -> InferencePortDescriptor {
    InferencePortDescriptor {
        port_id: InferencePortId::parse(port_id).expect("static port ids are valid"),
        label: label.to_string(),
        direction,
        requirement,
        value_type,
        default: None,
        options: InferencePortOptions::None,
        availability: InferenceAvailability::available(),
        runtime_conditions: Vec::new(),
        diagnostics: Vec::new(),
    }
}

fn backend_hint_keys(backend_hints: &inference::BackendHintFacts) -> BTreeSet<String> {
    backend_hints
        .accepted
        .iter()
        .map(|hint| normalize_backend_key(backend_hint_label_key(*hint)))
        .chain(
            backend_hints
                .raw
                .iter()
                .map(|hint| normalize_backend_key(hint)),
        )
        .filter(|key| !key.is_empty())
        .collect()
}

fn backend_hint_label_key(label: BackendHintLabel) -> &'static str {
    match label {
        BackendHintLabel::Transformers => "transformers",
        BackendHintLabel::LlamaCpp => "llama.cpp",
        BackendHintLabel::Vllm => "vllm",
        BackendHintLabel::Mlx => "mlx",
        BackendHintLabel::Candle => "candle",
        BackendHintLabel::Diffusers => "diffusers",
        BackendHintLabel::OnnxRuntime => "onnxruntime",
    }
}

fn normalize_backend_key(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use inference::{
        BackendHintFacts, PackageFactValueSource, PackageLogicalSizeFacts, TaskEvidence,
    };
    use pantograph_dependency_planning::PumasModelRef;
    use pantograph_scheduler::SchedulerEstimateHintKind;

    use super::*;

    #[test]
    fn projected_package_and_runtime_facts_publish_descriptor_inputs_and_estimates() {
        let package = projected_package_facts();
        let runtime = RuntimeDispatchCapabilityFactsOutcome::Projected {
            facts: RuntimeDispatchCapabilityFactsProjection {
                generated_at_ms: 1,
                runtimes: vec![RuntimeDispatchRuntimeCapabilityFacts {
                    runtime_id: "pytorch".to_string(),
                    backend_keys: vec!["pytorch".to_string()],
                    status: RuntimeRegistryStatus::Ready,
                    runtime_instance_id: Some("runtime.1".to_string()),
                    loaded_model_ids: Vec::new(),
                    active_reservation_ids: Vec::new(),
                    has_admission_budget: true,
                }],
            },
            diagnostics: Vec::new(),
        };

        let facts = resolver_facts_from_sources(
            PumasDispatchPackageFactsBridgeOutcome::Projected {
                facts: package,
                diagnostics: Vec::new(),
            },
            &runtime,
        );

        assert_eq!(facts.model.state, InferenceModelResolutionState::Ready);
        let capability = facts.capability.expect("capability facts");
        assert_eq!(capability.task_kind.as_str(), "image_generation");
        assert_eq!(capability.inputs[0].port_id.as_str(), "prompt");
        assert_eq!(capability.outputs[0].port_id.as_str(), "image");
        assert_eq!(capability.supported_runtime_ids.len(), 1);
        assert_eq!(capability.supported_runtime_ids[0].as_str(), "pytorch");
        assert_eq!(facts.runtimes.len(), 1);
        assert_eq!(facts.runtimes[0].runtime_id.as_str(), "pytorch");
        assert_eq!(
            facts.runtimes[0].state,
            InferenceRuntimeAvailabilityState::Available
        );
        assert!(facts.estimate_hints.iter().any(|hint| {
            hint.kind == SchedulerEstimateHintKind::PeakRamBytes && hint.value > 0
        }));
        assert!(facts.estimate_hints.iter().any(|hint| {
            hint.kind == SchedulerEstimateHintKind::PeakVramBytes && hint.value > 0
        }));
    }

    #[test]
    fn missing_runtime_facts_keep_capability_but_publish_no_runtime_availability() {
        let facts = resolver_facts_from_sources(
            PumasDispatchPackageFactsBridgeOutcome::Projected {
                facts: projected_package_facts(),
                diagnostics: Vec::new(),
            },
            &RuntimeDispatchCapabilityFactsOutcome::Unavailable {
                diagnostics: Vec::new(),
            },
        );

        assert_eq!(facts.model.state, InferenceModelResolutionState::Ready);
        assert!(facts.capability.is_some());
        assert!(facts.runtimes.is_empty());
        assert!(!facts.estimate_hints.is_empty());
    }

    #[test]
    fn missing_package_facts_fail_closed_without_capability_or_estimates() {
        let facts = resolver_facts_from_sources(
            PumasDispatchPackageFactsBridgeOutcome::Unavailable {
                diagnostics: vec![
                    crate::pumas_dispatch_package_facts::PumasDispatchPackageFactsDiagnostic {
                        code: PumasDispatchPackageFactsDiagnosticCode::MissingLogicalSizeFacts,
                        message: "missing logical size".to_string(),
                    },
                ],
            },
            &RuntimeDispatchCapabilityFactsOutcome::Unavailable {
                diagnostics: Vec::new(),
            },
        );

        assert_eq!(
            facts.model.state,
            InferenceModelResolutionState::MissingModelFacts
        );
        assert!(facts.capability.is_none());
        assert!(facts.runtimes.is_empty());
        assert!(facts.estimate_hints.is_empty());
    }

    fn projected_package_facts() -> PumasDispatchPackageFactsProjection {
        PumasDispatchPackageFactsProjection {
            model_ref: PumasModelRef {
                model_id: "image/example".to_string(),
                revision: None,
                selected_artifact_id: Some("diffusers".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            artifact_kind: inference::ModelArtifactKind::DiffusersBundle,
            validation_state: ModelValidationState::Valid,
            task: TaskEvidence {
                pipeline_tag: Some("text-to-image".to_string()),
                task_type_primary: Some("image_generation".to_string()),
                input_modalities: vec!["text".to_string()],
                output_modalities: vec!["image".to_string()],
            },
            backend_hints: BackendHintFacts {
                accepted: vec![BackendHintLabel::Diffusers],
                raw: vec!["pytorch".to_string()],
                unsupported: Vec::new(),
            },
            requires_custom_code: false,
            logical_size: PackageLogicalSizeFacts {
                total_size_bytes: Some(1024),
                value_source: PackageFactValueSource::FilesystemMetadata,
                files: Vec::new(),
                diagnostics: Vec::new(),
            },
            diffusers: None,
        }
    }
}

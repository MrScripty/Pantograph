use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use async_trait::async_trait;
use inference::{BackendHintLabel, InferenceTaskId, ModelValidationState, TaskRegistryEntry};
use pantograph_inference_interface_contracts::{
    InferenceArtifactType, InferenceAvailability, InferenceNumericRange, InferencePortDescriptor,
    InferencePortDirection, InferencePortId, InferencePortOptions, InferencePortRequirement,
    InferenceScalarType, InferenceStructuredType, InferenceTaskKind, InferenceValueType,
    RuntimeIntentId,
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
use crate::runtime_host_image_execution as image;

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
        InferenceTaskId::TextGeneration | InferenceTaskId::ChatCompletion => {
            vec![
                port(
                    "prompt",
                    "Prompt",
                    InferencePortDirection::Input,
                    InferencePortRequirement::Required,
                    InferenceValueType::Scalar(InferenceScalarType::String),
                ),
                positive_u32_input_port(
                    crate::runtime_host_text_execution::MAX_NEW_TOKENS_PORT,
                    "Max new tokens",
                ),
                port(
                    crate::runtime_host_text_execution::SYSTEM_PROMPT_PORT,
                    "System prompt",
                    InferencePortDirection::Input,
                    InferencePortRequirement::Optional,
                    InferenceValueType::Scalar(InferenceScalarType::String),
                ),
                u32_input_port(crate::runtime_host_text_execution::TOP_K_PORT, "Top k", 0),
                temperature_input_port(),
                top_p_input_port(),
                repetition_penalty_input_port(),
                u32_input_port(
                    crate::runtime_host_text_execution::MIN_NEW_TOKENS_PORT,
                    "Min new tokens",
                    0,
                ),
                port(
                    crate::runtime_host_text_execution::SEED_PORT,
                    "Seed",
                    InferencePortDirection::Input,
                    InferencePortRequirement::Optional,
                    InferenceValueType::Scalar(InferenceScalarType::U64),
                ),
                port(
                    crate::runtime_host_text_execution::STOP_PORT,
                    "Stop string",
                    InferencePortDirection::Input,
                    InferencePortRequirement::Optional,
                    InferenceValueType::Scalar(InferenceScalarType::String),
                ),
            ]
        }
        InferenceTaskId::ImageGeneration => vec![
            port(
                image::PROMPT_PORT,
                "Prompt",
                InferencePortDirection::Input,
                InferencePortRequirement::Required,
                InferenceValueType::Scalar(InferenceScalarType::String),
            ),
            port(
                image::NEGATIVE_PROMPT_PORT,
                "Negative prompt",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Scalar(InferenceScalarType::String),
            ),
            positive_u32_input_port(image::WIDTH_PORT, "Width"),
            positive_u32_input_port(image::HEIGHT_PORT, "Height"),
            positive_u32_input_port(image::STEPS_PORT, "Inference steps"),
            port(
                image::SEED_PORT,
                "Seed",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Scalar(InferenceScalarType::U64),
            ),
            guidance_scale_input_port(),
            image_count_input_port(),
            denoising_scheduler_input_port(),
        ],
        InferenceTaskId::AudioTranscription => vec![
            port(
                "audio",
                "Small WAV audio",
                InferencePortDirection::Input,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "language",
                "Language",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Scalar(InferenceScalarType::String),
            ),
            port(
                "prompt",
                "Prompt",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Scalar(InferenceScalarType::String),
            ),
            port(
                "asr_task",
                "ASR task",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Scalar(InferenceScalarType::String),
            ),
            port(
                "chunk_length_s",
                "Chunk length (seconds)",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Scalar(InferenceScalarType::F64),
            ),
            port(
                "extra_options",
                "Backend options (empty only)",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
        ],
        InferenceTaskId::Rerank => vec![
            port(
                "query",
                "Query",
                InferencePortDirection::Input,
                InferencePortRequirement::Required,
                InferenceValueType::Scalar(InferenceScalarType::String),
            ),
            port(
                "documents",
                "Documents",
                InferencePortDirection::Input,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "top_n",
                "Top n",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Scalar(InferenceScalarType::U64),
            ),
            port(
                "return_documents",
                "Return documents",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Scalar(InferenceScalarType::Bool),
            ),
            port(
                "task_options",
                "Task options",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "extra_options",
                "Backend options",
                InferencePortDirection::Input,
                InferencePortRequirement::Optional,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
        ],
        InferenceTaskId::Embedding => vec![port(
            "text",
            "Text",
            InferencePortDirection::Input,
            InferencePortRequirement::Required,
            InferenceValueType::Scalar(InferenceScalarType::String),
        )],
        InferenceTaskId::MultimodalGeneration => vec![port(
            "prompt",
            "Prompt",
            InferencePortDirection::Input,
            InferencePortRequirement::Required,
            InferenceValueType::Scalar(InferenceScalarType::String),
        )],
        _ => Vec::new(),
    }
}

fn denoising_scheduler_input_port() -> InferencePortDescriptor {
    let mut descriptor = port(
        image::DENOISING_SCHEDULER_PORT,
        "Denoising scheduler",
        InferencePortDirection::Input,
        InferencePortRequirement::Optional,
        InferenceValueType::Scalar(InferenceScalarType::String),
    );
    descriptor.options = InferencePortOptions::Enum {
        values: inference::STABLE_DIFFUSION_DENOISING_SCHEDULERS
            .iter()
            .map(
                |id| pantograph_inference_interface_contracts::InferenceOptionValue {
                    option_id: pantograph_inference_interface_contracts::InferenceOptionId::parse(
                        id,
                    )
                    .expect("static scheduler id"),
                    label: match *id {
                        "ddim" => "DDIM",
                        "euler" => "Euler",
                        _ => unreachable!("closed scheduler choices"),
                    }
                    .into(),
                    value: pantograph_inference_interface_contracts::InferenceOptionScalar::String(
                        (*id).into(),
                    ),
                    availability: InferenceAvailability::available(),
                    diagnostics: Vec::new(),
                },
            )
            .collect(),
    };
    descriptor
}

fn image_count_input_port() -> InferencePortDescriptor {
    let mut descriptor = positive_u32_input_port(image::NUM_IMAGES_PORT, "Images per prompt");
    let InferencePortOptions::NumericRange { range } = &mut descriptor.options else {
        unreachable!("integer port has numeric bounds");
    };
    range.max = pantograph_runtime_host_contracts::MAX_RUNTIME_HOST_OUTPUTS as f64;
    descriptor
}

fn guidance_scale_input_port() -> InferencePortDescriptor {
    let mut descriptor = port(
        image::GUIDANCE_SCALE_PORT,
        "Guidance scale",
        InferencePortDirection::Input,
        InferencePortRequirement::Optional,
        InferenceValueType::Scalar(InferenceScalarType::F64),
    );
    descriptor.options = InferencePortOptions::NumericRange {
        range: InferenceNumericRange {
            min: f64::from(f32::MIN),
            max: f64::from(f32::MAX),
            step: None,
            default: None,
        },
    };
    descriptor
}

fn positive_u32_input_port(port_id: &str, label: &str) -> InferencePortDescriptor {
    u32_input_port(port_id, label, 1)
}

fn u32_input_port(port_id: &str, label: &str, min: u32) -> InferencePortDescriptor {
    let mut descriptor = port(
        port_id,
        label,
        InferencePortDirection::Input,
        InferencePortRequirement::Optional,
        InferenceValueType::Scalar(InferenceScalarType::U64),
    );
    descriptor.options = InferencePortOptions::NumericRange {
        range: InferenceNumericRange {
            min: f64::from(min),
            max: f64::from(u32::MAX),
            step: Some(1.0),
            default: None,
        },
    };
    descriptor
}

fn temperature_input_port() -> InferencePortDescriptor {
    sampling_number_input_port(
        crate::runtime_host_text_execution::TEMPERATURE_PORT,
        "Temperature",
        f64::from(f32::MAX),
    )
}

fn top_p_input_port() -> InferencePortDescriptor {
    sampling_number_input_port(crate::runtime_host_text_execution::TOP_P_PORT, "Top p", 1.0)
}

fn repetition_penalty_input_port() -> InferencePortDescriptor {
    let mut descriptor = sampling_number_input_port(
        crate::runtime_host_text_execution::REPETITION_PENALTY_PORT,
        "Repetition penalty",
        f64::from(f32::MAX),
    );
    if let InferencePortOptions::NumericRange { range } = &mut descriptor.options {
        range.min = f64::from(f32::from_bits(1));
    }
    descriptor
}

fn sampling_number_input_port(port_id: &str, label: &str, max: f64) -> InferencePortDescriptor {
    let mut descriptor = port(
        port_id,
        label,
        InferencePortDirection::Input,
        InferencePortRequirement::Optional,
        InferenceValueType::Scalar(InferenceScalarType::F64),
    );
    descriptor.options = InferencePortOptions::NumericRange {
        range: InferenceNumericRange {
            min: 0.0,
            max,
            step: None,
            default: None,
        },
    };
    descriptor
}

fn output_ports(task_entry: &TaskRegistryEntry) -> Vec<InferencePortDescriptor> {
    match task_entry.task_id {
        InferenceTaskId::AudioTranscription => vec![
            port(
                "response",
                "Response",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Scalar(InferenceScalarType::String),
            ),
            port(
                "stream",
                "Stream",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "text",
                "Text",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Scalar(InferenceScalarType::String),
            ),
            port(
                "language",
                "Language",
                InferencePortDirection::Output,
                InferencePortRequirement::Optional,
                InferenceValueType::Scalar(InferenceScalarType::String),
            ),
            port(
                "duration_seconds",
                "Duration seconds",
                InferencePortDirection::Output,
                InferencePortRequirement::Optional,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "segments",
                "Segments",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "metadata",
                "Metadata",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "diagnostics",
                "Diagnostics",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
        ],
        InferenceTaskId::Rerank => vec![
            port(
                "results",
                "Results",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "scores",
                "Scores",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "top_document",
                "Top document",
                InferencePortDirection::Output,
                InferencePortRequirement::Optional,
                InferenceValueType::Scalar(InferenceScalarType::String),
            ),
            port(
                "top_score",
                "Top score",
                InferencePortDirection::Output,
                InferencePortRequirement::Optional,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "diagnostics",
                "Diagnostics",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
        ],
        InferenceTaskId::Embedding => vec![
            port(
                "embedding",
                "Embedding",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Embedding),
            ),
            port(
                "metadata",
                "Metadata",
                InferencePortDirection::Output,
                InferencePortRequirement::Required,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
            port(
                "usage",
                "Usage",
                InferencePortDirection::Output,
                InferencePortRequirement::Optional,
                InferenceValueType::Structured(InferenceStructuredType::Json),
            ),
        ],
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
    fn text_generation_descriptor_exposes_optional_numeric_controls_without_defaults() {
        let task = inference::resolve_task_registry_entry("text_generation").expect("task entry");
        let inputs = input_ports(&task);
        assert_eq!(
            inputs
                .iter()
                .map(|input| input.port_id.as_str())
                .collect::<Vec<_>>(),
            [
                "prompt",
                "max_new_tokens",
                "system_prompt",
                "top_k",
                "temperature",
                "top_p",
                "repetition_penalty",
                "min_new_tokens",
                "seed",
                "stop",
            ]
        );
        for (port_id, scalar_type, min, max, step) in [
            (
                "max_new_tokens",
                InferenceScalarType::U64,
                1.0,
                f64::from(u32::MAX),
                Some(1.0),
            ),
            (
                "top_k",
                InferenceScalarType::U64,
                0.0,
                f64::from(u32::MAX),
                Some(1.0),
            ),
            (
                "temperature",
                InferenceScalarType::F64,
                0.0,
                f64::from(f32::MAX),
                None,
            ),
            ("top_p", InferenceScalarType::F64, 0.0, 1.0, None),
            (
                "repetition_penalty",
                InferenceScalarType::F64,
                f64::from(f32::from_bits(1)),
                f64::from(f32::MAX),
                None,
            ),
            (
                "min_new_tokens",
                InferenceScalarType::U64,
                0.0,
                f64::from(u32::MAX),
                Some(1.0),
            ),
        ] {
            let control = inputs
                .iter()
                .find(|input| input.port_id.as_str() == port_id)
                .expect("numeric control port");
            assert_eq!(
                control.direction,
                InferencePortDirection::Input,
                "{port_id}"
            );
            assert_eq!(
                control.requirement,
                InferencePortRequirement::Optional,
                "{port_id}"
            );
            assert_eq!(
                control.value_type,
                InferenceValueType::Scalar(scalar_type),
                "{port_id}"
            );
            assert!(control.default.is_none(), "{port_id} must have no default");
            assert_eq!(
                control.options,
                InferencePortOptions::NumericRange {
                    range: InferenceNumericRange {
                        min,
                        max,
                        step,
                        default: None,
                    },
                },
                "{port_id}"
            );
            control.validate().expect("numeric control port contract");
        }
        let chat = inference::resolve_task_registry_entry("chat_completion").expect("chat task");
        assert_eq!(input_ports(&chat), input_ports(&task));
    }

    #[test]
    fn text_generation_inputs_match_shared_system_prompt_contract() {
        let task = inference::resolve_task_registry_entry("text_generation").unwrap();
        let expected: Vec<InferencePortDescriptor> = serde_json::from_str(include_str!(
            "../../pantograph-inference-interface-contracts/tests/fixtures/text_generation_system_prompt_inputs.json"
        )).unwrap();
        assert_eq!(input_ports(&task), expected);
        let chat = inference::resolve_task_registry_entry("chat_completion").unwrap();
        assert_eq!(input_ports(&chat), expected);
    }

    #[test]
    fn image_generation_inputs_match_the_shared_basic_controls_contract() {
        let task = inference::resolve_task_registry_entry("image_generation").expect("image task");
        let expected: Vec<InferencePortDescriptor> = serde_json::from_str(include_str!(
            "../../pantograph-inference-interface-contracts/tests/fixtures/image_generation_basic_inputs.json"
        )).expect("image input contract fixture");
        let actual = input_ports(&task);
        assert_eq!(actual, expected);
        for input in actual {
            input.validate().expect("image input contract");
        }
    }

    #[test]
    fn projected_package_and_runtime_facts_publish_descriptor_inputs_and_estimates() {
        let package = projected_package_facts();
        let runtime = RuntimeDispatchCapabilityFactsOutcome::Projected {
            facts: RuntimeDispatchCapabilityFactsProjection {
                generated_at_ms: 1,
                runtimes: vec![RuntimeDispatchRuntimeCapabilityFacts {
                    runtime_id: "pytorch".to_string(),
                    backend_keys: vec!["pytorch".to_string()],
                    runtime_family: "diffusers".to_string(),
                    runtime_residency_key: "runtime.diffusers.pytorch.shared".to_string(),
                    status: RuntimeRegistryStatus::Ready,
                    runtime_instance_id: Some("runtime.1".to_string()),
                    loaded_model_ids: Vec::new(),
                    active_reservation_ids: Vec::new(),
                    has_admission_budget: true,
                    automatic_device_candidates: Vec::new(),
                }],
            },
            diagnostics: Vec::new(),
        };

        let facts = resolver_facts_from_sources(
            PumasDispatchPackageFactsBridgeOutcome::Projected {
                facts: Box::new(package),
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
                facts: Box::new(projected_package_facts()),
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

    #[test]
    fn rerank_descriptor_exposes_typed_parent_inputs_outputs_without_streaming_or_defaults() {
        let mut package = projected_package_facts();
        let rerank: inference::ResolvedModelPackageFacts = serde_json::from_str(include_str!(
            "../../inference/tests/fixtures/inference_package_facts/rerank_package_facts.json"
        ))
        .unwrap();
        package.task = rerank.task;
        let runtime = InferenceRuntimeAvailabilityFact {
            runtime_id: "llama_cpp".parse().unwrap(),
            state: InferenceRuntimeAvailabilityState::Available,
            device_ids: vec!["cpu".parse().unwrap()],
        };
        let capability = capability_facts(&package, &[runtime]).unwrap();
        let descriptor: pantograph_inference_interface_contracts::InferenceInterfaceDescriptor = serde_json::from_str(include_str!("../../pantograph-inference-interface-contracts/tests/fixtures/descriptor_rerank_ready.json")).unwrap();
        descriptor.validate().unwrap();
        assert_eq!(capability.task_kind.as_str(), "rerank");
        assert_eq!(capability.inputs, descriptor.inputs);
        assert_eq!(capability.outputs, descriptor.outputs);
        assert!(!capability
            .inputs
            .iter()
            .any(|port| port.port_id.as_str() == "stream"));
        assert!(capability
            .inputs
            .iter()
            .all(|port| matches!(port.options, InferencePortOptions::None)));
    }

    #[test]
    fn selected_audio_descriptor_exposes_all_eight_parent_outputs_without_defaults() {
        let mut package = projected_package_facts();
        let audio: inference::ResolvedModelPackageFacts = serde_json::from_str(include_str!("../../inference/tests/fixtures/inference_package_facts/hf_audio_transcription_package_facts.json")).unwrap();
        package.task = audio.task;
        let runtime = InferenceRuntimeAvailabilityFact {
            runtime_id: "pytorch".parse().unwrap(),
            state: InferenceRuntimeAvailabilityState::Available,
            device_ids: vec!["cpu".parse().unwrap()],
        };
        let capability = capability_facts(&package, &[runtime]).unwrap();
        let descriptor:pantograph_inference_interface_contracts::InferenceInterfaceDescriptor = serde_json::from_str(include_str!("../../pantograph-inference-interface-contracts/tests/fixtures/descriptor_audio_ready.json")).unwrap();
        descriptor.validate().unwrap();
        assert_eq!(capability.task_kind.as_str(), "audio_transcription");
        assert_eq!(capability.inputs, descriptor.inputs);
        assert_eq!(capability.outputs, descriptor.outputs);
        assert_eq!(capability.outputs.len(), 8);
        assert!(capability
            .inputs
            .iter()
            .all(|port| matches!(port.options, InferencePortOptions::None)));
    }

    #[test]
    fn embedding_descriptor_types_have_stable_fingerprints_and_saved_snapshots() {
        use pantograph_inference_interface_contracts::ResolveInferenceInterfaceRequest;
        use pantograph_workflow_service::graph::{
            authored_snapshot_from_descriptor, resolve_inference_interface_from_facts,
        };
        let mut package = projected_package_facts();
        package.task = TaskEvidence {
            pipeline_tag: Some("feature-extraction".into()),
            task_type_primary: Some("embedding".into()),
            input_modalities: vec!["text".into()],
            output_modalities: vec!["embedding".into()],
        };
        let runtime = InferenceRuntimeAvailabilityFact {
            runtime_id: "candle".parse().unwrap(),
            state: InferenceRuntimeAvailabilityState::Available,
            device_ids: vec!["cpu".parse().unwrap()],
        };
        let capability = capability_facts(&package, std::slice::from_ref(&runtime)).unwrap();
        assert_eq!(capability.task_kind.as_str(), "embedding");
        let fixture: pantograph_inference_interface_contracts::InferenceInterfaceDescriptor = serde_json::from_str(include_str!("../../pantograph-inference-interface-contracts/tests/fixtures/descriptor_embedding_ready.json")).unwrap();
        assert_eq!(capability.inputs, fixture.inputs);
        assert_eq!(capability.outputs, fixture.outputs);
        let request: ResolveInferenceInterfaceRequest = serde_json::from_value(serde_json::json!({
            "model_ref": {"model_id": "embedding/example/tiny-cpu", "selected_artifact_id": "candle-safetensors"},
            "task_kind": "embedding", "runtime_constraint": "candle", "device_constraint": "cpu",
        })).unwrap();
        let facts = InferenceInterfaceResolverFacts {
            model: InferenceModelResolutionFacts {
                state: InferenceModelResolutionState::Ready,
            },
            capability: Some(capability),
            runtimes: vec![runtime],
            estimate_hints: Vec::new(),
        };
        let descriptor =
            resolve_inference_interface_from_facts(request.clone(), facts.clone()).unwrap();
        let repeated =
            resolve_inference_interface_from_facts(request.clone(), facts.clone()).unwrap();
        assert_eq!(
            descriptor.descriptor_fingerprint,
            repeated.descriptor_fingerprint
        );
        let snapshot = authored_snapshot_from_descriptor(&descriptor).unwrap();
        let restored: pantograph_inference_interface_contracts::AuthoredInferenceInterfaceSnapshot =
            serde_json::from_str(&serde_json::to_string(&snapshot).unwrap()).unwrap();
        assert_eq!(restored, snapshot);
        assert_eq!(
            restored.descriptor_fingerprint,
            descriptor.descriptor_fingerprint
        );
        let mut tensor_facts = facts;
        tensor_facts.capability.as_mut().unwrap().outputs[0].value_type =
            InferenceValueType::Artifact(InferenceArtifactType::Tensor);
        let tensor_descriptor =
            resolve_inference_interface_from_facts(request, tensor_facts).unwrap();
        assert_ne!(
            descriptor.descriptor_fingerprint,
            tensor_descriptor.descriptor_fingerprint
        );
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

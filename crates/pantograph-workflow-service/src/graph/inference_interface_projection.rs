use pantograph_inference_interface_contracts::{
    AuthoredInferenceInterfaceSnapshot, AuthoredInferencePortSnapshot,
    DraftGraphEnqueueDisabledReason, DraftGraphValidationStatus, DraftGraphValidationSummary,
    InferenceAvailabilityReason, InferenceAvailabilityStatus, InferenceDiagnosticSeverity,
    InferenceInterfaceContractError, InferenceInterfaceDescriptor,
    INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    resolve_inference_interface_from_facts, InferenceInterfaceResolverError,
    InferenceInterfaceResolverFacts,
};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferenceInterfaceProjectionError {
    #[error("inference interface resolver failed: {0}")]
    Resolver(#[from] InferenceInterfaceResolverError),
    #[error("inference interface contract error: {0}")]
    Contract(#[from] InferenceInterfaceContractError),
    #[error("inference interface validation count exceeds u32: {field}={count}")]
    CountOverflow { field: &'static str, count: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceResolutionProjection {
    pub descriptor: InferenceInterfaceDescriptor,
    pub authored_snapshot: AuthoredInferenceInterfaceSnapshot,
    pub validation_summary: DraftGraphValidationSummary,
}

pub fn resolve_inference_interface_projection(
    request: pantograph_inference_interface_contracts::ResolveInferenceInterfaceRequest,
    facts: InferenceInterfaceResolverFacts,
) -> Result<InferenceInterfaceResolutionProjection, InferenceInterfaceProjectionError> {
    let descriptor = resolve_inference_interface_from_facts(request, facts)?;
    let authored_snapshot = authored_snapshot_from_descriptor(&descriptor)?;
    let validation_summary = validation_summary_from_descriptor(&descriptor)?;
    Ok(InferenceInterfaceResolutionProjection {
        descriptor,
        authored_snapshot,
        validation_summary,
    })
}

pub fn authored_snapshot_from_descriptor(
    descriptor: &InferenceInterfaceDescriptor,
) -> Result<AuthoredInferenceInterfaceSnapshot, InferenceInterfaceProjectionError> {
    descriptor.validate()?;
    let snapshot = AuthoredInferenceInterfaceSnapshot {
        contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
        descriptor_fingerprint: descriptor.descriptor_fingerprint.clone(),
        task_kind: descriptor.task_kind.clone(),
        inputs: descriptor
            .inputs
            .iter()
            .map(authored_port_snapshot_from_descriptor)
            .collect(),
        outputs: descriptor
            .outputs
            .iter()
            .map(authored_port_snapshot_from_descriptor)
            .collect(),
    };
    snapshot.validate()?;
    Ok(snapshot)
}

pub fn validation_summary_from_descriptor(
    descriptor: &InferenceInterfaceDescriptor,
) -> Result<DraftGraphValidationSummary, InferenceInterfaceProjectionError> {
    descriptor.validate()?;
    let diagnostics_count = checked_u32("diagnostics_count", descriptor.diagnostics.len())?;
    let blocking_diagnostics_count = checked_u32(
        "blocking_diagnostics_count",
        descriptor
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == InferenceDiagnosticSeverity::Error)
            .count(),
    )?;
    let mut enqueue_disabled_reasons = Vec::new();
    extend_availability_reasons(&mut enqueue_disabled_reasons, descriptor);
    if blocking_diagnostics_count > 0 {
        push_unique(
            &mut enqueue_disabled_reasons,
            DraftGraphEnqueueDisabledReason::BlockingDiagnostics,
        );
    }

    let status = validation_status(descriptor, blocking_diagnostics_count);
    let executable =
        status == DraftGraphValidationStatus::Executable && enqueue_disabled_reasons.is_empty();
    let summary = DraftGraphValidationSummary {
        status,
        executable,
        enqueue_disabled_reasons,
        diagnostics_count,
        blocking_diagnostics_count,
    };
    summary.validate()?;
    Ok(summary)
}

fn authored_port_snapshot_from_descriptor(
    port: &pantograph_inference_interface_contracts::InferencePortDescriptor,
) -> AuthoredInferencePortSnapshot {
    AuthoredInferencePortSnapshot {
        port_id: port.port_id.clone(),
        label: port.label.clone(),
        direction: port.direction,
        requirement: port.requirement,
        value_type: port.value_type.clone(),
        default: port.default.clone(),
        availability: port.availability.clone(),
    }
}

fn validation_status(
    descriptor: &InferenceInterfaceDescriptor,
    blocking_diagnostics_count: u32,
) -> DraftGraphValidationStatus {
    if descriptor.availability.status == InferenceAvailabilityStatus::Pending {
        return DraftGraphValidationStatus::Pending;
    }
    if descriptor.availability.status == InferenceAvailabilityStatus::Stale {
        return DraftGraphValidationStatus::Stale;
    }
    if descriptor.availability.reasons.iter().any(|reason| {
        matches!(
            reason,
            InferenceAvailabilityReason::ExplicitRuntimeInvalid
                | InferenceAvailabilityReason::ExplicitDeviceInvalid
                | InferenceAvailabilityReason::MissingRequiredInput
                | InferenceAvailabilityReason::InvalidOption
                | InferenceAvailabilityReason::DriftDetected
        )
    }) {
        return DraftGraphValidationStatus::Blocked;
    }
    match descriptor.availability.status {
        InferenceAvailabilityStatus::Available if blocking_diagnostics_count == 0 => {
            DraftGraphValidationStatus::Executable
        }
        InferenceAvailabilityStatus::Available => DraftGraphValidationStatus::Blocked,
        InferenceAvailabilityStatus::Unavailable
        | InferenceAvailabilityStatus::NotImplemented
        | InferenceAvailabilityStatus::Unsupported => DraftGraphValidationStatus::Unavailable,
        _ => DraftGraphValidationStatus::Blocked,
    }
}

fn extend_availability_reasons(
    reasons: &mut Vec<DraftGraphEnqueueDisabledReason>,
    descriptor: &InferenceInterfaceDescriptor,
) {
    match descriptor.availability.status {
        InferenceAvailabilityStatus::Available => {}
        InferenceAvailabilityStatus::Pending => {
            push_unique(reasons, DraftGraphEnqueueDisabledReason::ValidationPending);
        }
        InferenceAvailabilityStatus::Stale => {
            push_unique(reasons, DraftGraphEnqueueDisabledReason::ValidationStale);
        }
        InferenceAvailabilityStatus::Unavailable
        | InferenceAvailabilityStatus::NotImplemented
        | InferenceAvailabilityStatus::Unsupported => {
            push_unique(
                reasons,
                DraftGraphEnqueueDisabledReason::DescriptorUnavailable,
            );
        }
        _ => {
            push_unique(
                reasons,
                DraftGraphEnqueueDisabledReason::BlockingDiagnostics,
            );
        }
    }

    for reason in &descriptor.availability.reasons {
        match reason {
            InferenceAvailabilityReason::ExplicitRuntimeInvalid => push_unique(
                reasons,
                DraftGraphEnqueueDisabledReason::InvalidRuntimeConstraint,
            ),
            InferenceAvailabilityReason::ExplicitDeviceInvalid => push_unique(
                reasons,
                DraftGraphEnqueueDisabledReason::InvalidDeviceConstraint,
            ),
            InferenceAvailabilityReason::MissingRequiredInput => {
                push_unique(
                    reasons,
                    DraftGraphEnqueueDisabledReason::MissingRequiredInput,
                );
            }
            InferenceAvailabilityReason::InvalidOption => {
                push_unique(reasons, DraftGraphEnqueueDisabledReason::InvalidPortBinding);
            }
            InferenceAvailabilityReason::DriftDetected => {
                push_unique(
                    reasons,
                    DraftGraphEnqueueDisabledReason::DriftRequiresReview,
                );
            }
            InferenceAvailabilityReason::BackendValidationPending => {
                push_unique(reasons, DraftGraphEnqueueDisabledReason::ValidationPending);
            }
            _ => {}
        }
    }
}

fn push_unique(
    reasons: &mut Vec<DraftGraphEnqueueDisabledReason>,
    reason: DraftGraphEnqueueDisabledReason,
) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn checked_u32(
    field: &'static str,
    count: usize,
) -> Result<u32, InferenceInterfaceProjectionError> {
    u32::try_from(count)
        .map_err(|_| InferenceInterfaceProjectionError::CountOverflow { field, count })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_inference_interface_contracts::{
        DeviceIntentId, InferenceArtifactType, InferenceAvailability, InferencePortDescriptor,
        InferencePortDirection, InferencePortId, InferencePortOptions, InferencePortRequirement,
        InferenceRuntimeCondition, InferenceRuntimeConditionKind, InferenceScalarType,
        InferenceTaskKind, InferenceValueType, PumasModelRef, ResolveInferenceInterfaceRequest,
        RuntimeIntentId,
    };

    use crate::graph::{
        InferenceCapabilityFacts, InferenceModelResolutionFacts, InferenceModelResolutionState,
        InferenceRuntimeAvailabilityFact, InferenceRuntimeAvailabilityState,
    };

    #[test]
    fn projection_turns_ready_descriptor_into_authored_snapshot_and_executable_summary() {
        let projection = resolve_inference_interface_projection(request(None, None), ready_facts())
            .expect("projection");

        assert_eq!(projection.authored_snapshot.inputs.len(), 1);
        assert_eq!(
            projection.authored_snapshot.inputs[0].port_id.as_str(),
            "prompt"
        );
        assert_eq!(
            projection.validation_summary.status,
            DraftGraphValidationStatus::Executable
        );
        assert!(projection.validation_summary.executable);
        assert!(projection
            .validation_summary
            .enqueue_disabled_reasons
            .is_empty());
    }

    #[test]
    fn projection_blocks_invalid_runtime_without_changing_scheduler_choice() {
        let projection = resolve_inference_interface_projection(
            request(Some(runtime_id("vllm")), None),
            ready_facts(),
        )
        .expect("projection");

        assert_eq!(
            projection.validation_summary.status,
            DraftGraphValidationStatus::Blocked
        );
        assert!(!projection.validation_summary.executable);
        assert!(projection
            .validation_summary
            .enqueue_disabled_reasons
            .contains(&DraftGraphEnqueueDisabledReason::InvalidRuntimeConstraint));
        assert!(projection
            .validation_summary
            .enqueue_disabled_reasons
            .contains(&DraftGraphEnqueueDisabledReason::BlockingDiagnostics));
        assert_eq!(projection.validation_summary.blocking_diagnostics_count, 1);
    }

    #[test]
    fn projection_reports_unavailable_descriptor_without_ports() {
        let mut facts = ready_facts();
        facts.model.state = InferenceModelResolutionState::MissingSelectedArtifact;
        facts.capability = None;

        let projection =
            resolve_inference_interface_projection(request(None, None), facts).expect("projection");

        assert!(projection.authored_snapshot.inputs.is_empty());
        assert!(projection.authored_snapshot.outputs.is_empty());
        assert_eq!(
            projection.validation_summary.status,
            DraftGraphValidationStatus::Unavailable
        );
        assert!(!projection.validation_summary.executable);
        assert!(projection
            .validation_summary
            .enqueue_disabled_reasons
            .contains(&DraftGraphEnqueueDisabledReason::DescriptorUnavailable));
    }

    fn request(
        runtime_constraint: Option<RuntimeIntentId>,
        device_constraint: Option<DeviceIntentId>,
    ) -> ResolveInferenceInterfaceRequest {
        ResolveInferenceInterfaceRequest {
            contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            model_ref: PumasModelRef {
                model_id: "diffusion/imported/tiny-sd".to_string(),
                revision: None,
                selected_artifact_id: Some("artifact.diffusers".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            task_kind: Some(InferenceTaskKind::parse("image_generation").unwrap()),
            runtime_constraint,
            device_constraint,
        }
    }

    fn ready_facts() -> InferenceInterfaceResolverFacts {
        InferenceInterfaceResolverFacts {
            model: InferenceModelResolutionFacts {
                state: InferenceModelResolutionState::Ready,
            },
            capability: Some(InferenceCapabilityFacts {
                task_kind: InferenceTaskKind::parse("image_generation").unwrap(),
                inputs: vec![port(
                    "prompt",
                    "Prompt",
                    InferencePortDirection::Input,
                    InferencePortRequirement::Required,
                    InferenceValueType::Scalar(InferenceScalarType::String),
                )],
                outputs: vec![port(
                    "image",
                    "Image",
                    InferencePortDirection::Output,
                    InferencePortRequirement::Required,
                    InferenceValueType::Artifact(InferenceArtifactType::Image),
                )],
                runtime_conditions: vec![InferenceRuntimeCondition {
                    condition: InferenceRuntimeConditionKind::ArtifactKind,
                    value: "diffusers_bundle".to_string(),
                }],
                supported_runtime_ids: vec![runtime_id("pytorch")],
            }),
            runtimes: vec![InferenceRuntimeAvailabilityFact {
                runtime_id: runtime_id("pytorch"),
                state: InferenceRuntimeAvailabilityState::Available,
                device_ids: vec![device_id("cuda.0")],
            }],
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
            port_id: InferencePortId::parse(port_id).unwrap(),
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

    fn runtime_id(value: &str) -> RuntimeIntentId {
        RuntimeIntentId::parse(value).unwrap()
    }

    fn device_id(value: &str) -> DeviceIntentId {
        DeviceIntentId::parse(value).unwrap()
    }
}

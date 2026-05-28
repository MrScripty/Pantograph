use pantograph_inference_interface_contracts::{
    DeviceIntentId, InferenceAvailability, InferenceAvailabilityReason,
    InferenceAvailabilityStatus, InferenceDiagnosticCode, InferenceDiagnosticSeverity,
    InferenceInterfaceContractError, InferenceInterfaceDescriptor, InferenceInterfaceDiagnostic,
    InferenceInterfaceFingerprint, InferencePortDescriptor, InferenceRuntimeCondition,
    InferenceTaskKind, ResolveInferenceInterfaceRequest, RuntimeIntentId,
    INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DESCRIPTOR_FINGERPRINT_PREFIX: &str = "iface.";
const UNKNOWN_TASK_KIND: &str = "unknown";

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferenceInterfaceResolverError {
    #[error("inference interface contract error: {0}")]
    Contract(#[from] InferenceInterfaceContractError),
    #[error("failed to encode inference interface facts: {0}")]
    EncodeFacts(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceResolverFacts {
    pub model: InferenceModelResolutionFacts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<InferenceCapabilityFacts>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtimes: Vec<InferenceRuntimeAvailabilityFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceModelResolutionFacts {
    pub state: InferenceModelResolutionState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceModelResolutionState {
    Ready,
    MissingModelFacts,
    MissingSelectedArtifact,
    InvalidArtifact,
    StaleFacts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceCapabilityFacts {
    pub task_kind: InferenceTaskKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<InferencePortDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<InferencePortDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_conditions: Vec<InferenceRuntimeCondition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_runtime_ids: Vec<RuntimeIntentId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceRuntimeAvailabilityFact {
    pub runtime_id: RuntimeIntentId,
    pub state: InferenceRuntimeAvailabilityState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_ids: Vec<DeviceIntentId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceRuntimeAvailabilityState {
    Available,
    NotInstalled,
    NotImplemented,
    Unsupported,
}

pub fn resolve_inference_interface_from_facts(
    request: ResolveInferenceInterfaceRequest,
    facts: InferenceInterfaceResolverFacts,
) -> Result<InferenceInterfaceDescriptor, InferenceInterfaceResolverError> {
    request.validate()?;
    validate_capability(&facts.capability)?;

    let task_kind = resolved_task_kind(&request, facts.capability.as_ref())?;
    let mut diagnostics = Vec::new();
    let mut availability = InferenceAvailability::available();

    apply_model_state_diagnostics(facts.model.state, &mut availability, &mut diagnostics);
    apply_capability_diagnostics(
        request.task_kind.as_ref(),
        facts.capability.as_ref(),
        &task_kind,
        &mut availability,
        &mut diagnostics,
    );
    apply_runtime_diagnostics(
        &request,
        facts.capability.as_ref(),
        &facts.runtimes,
        &mut availability,
        &mut diagnostics,
    );

    let descriptor_fingerprint = descriptor_fingerprint(&request, &facts, &availability)?;
    let (inputs, outputs, runtime_conditions) = facts
        .capability
        .map(|capability| {
            (
                capability.inputs,
                capability.outputs,
                capability.runtime_conditions,
            )
        })
        .unwrap_or_default();

    let descriptor = InferenceInterfaceDescriptor {
        contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
        model_ref: request.model_ref,
        task_kind,
        descriptor_fingerprint,
        runtime_conditions,
        inputs,
        outputs,
        availability,
        diagnostics,
    };
    descriptor.validate()?;
    Ok(descriptor)
}

fn validate_capability(
    capability: &Option<InferenceCapabilityFacts>,
) -> Result<(), InferenceInterfaceContractError> {
    if let Some(capability) = capability {
        for port in capability.inputs.iter().chain(capability.outputs.iter()) {
            port.validate()?;
        }
        for condition in &capability.runtime_conditions {
            condition.validate()?;
        }
    }
    Ok(())
}

fn resolved_task_kind(
    request: &ResolveInferenceInterfaceRequest,
    capability: Option<&InferenceCapabilityFacts>,
) -> Result<InferenceTaskKind, InferenceInterfaceContractError> {
    if let Some(task_kind) = &request.task_kind {
        return Ok(task_kind.clone());
    }
    if let Some(capability) = capability {
        return Ok(capability.task_kind.clone());
    }
    InferenceTaskKind::parse(UNKNOWN_TASK_KIND)
}

fn apply_model_state_diagnostics(
    state: InferenceModelResolutionState,
    availability: &mut InferenceAvailability,
    diagnostics: &mut Vec<InferenceInterfaceDiagnostic>,
) {
    match state {
        InferenceModelResolutionState::Ready => {}
        InferenceModelResolutionState::MissingModelFacts => mark_unavailable(
            availability,
            diagnostics,
            InferenceAvailabilityReason::MissingModelFacts,
            InferenceDiagnosticCode::DescriptorUnavailable,
            "Pumas model facts are not available for inference interface resolution",
            Some("Refresh Pumas model facts before validating this inference node"),
        ),
        InferenceModelResolutionState::MissingSelectedArtifact => mark_unavailable(
            availability,
            diagnostics,
            InferenceAvailabilityReason::MissingSelectedArtifact,
            InferenceDiagnosticCode::DescriptorUnavailable,
            "Pumas did not resolve a selected artifact for this model reference",
            Some("Select a concrete model artifact in the Pumas model reference"),
        ),
        InferenceModelResolutionState::InvalidArtifact => mark_unavailable(
            availability,
            diagnostics,
            InferenceAvailabilityReason::MissingSelectedArtifact,
            InferenceDiagnosticCode::DescriptorUnavailable,
            "Pumas reported the selected model artifact as invalid",
            Some("Repair or replace the selected Pumas artifact before validation"),
        ),
        InferenceModelResolutionState::StaleFacts => mark_unavailable(
            availability,
            diagnostics,
            InferenceAvailabilityReason::StaleFacts,
            InferenceDiagnosticCode::DescriptorStale,
            "Pumas model facts are stale for this inference interface",
            Some("Refresh model facts and re-run descriptor resolution"),
        ),
    }
}

fn apply_capability_diagnostics(
    requested_task_kind: Option<&InferenceTaskKind>,
    capability: Option<&InferenceCapabilityFacts>,
    task_kind: &InferenceTaskKind,
    availability: &mut InferenceAvailability,
    diagnostics: &mut Vec<InferenceInterfaceDiagnostic>,
) {
    let Some(capability) = capability else {
        mark_unavailable(
            availability,
            diagnostics,
            InferenceAvailabilityReason::MissingRuntimeCapability,
            InferenceDiagnosticCode::DescriptorUnavailable,
            "No inference capability facts were provided for this model",
            Some("Wait for the inference capability resolver to report supported task ports"),
        );
        return;
    };

    if requested_task_kind.is_some() && capability.task_kind != *task_kind {
        mark_unavailable(
            availability,
            diagnostics,
            InferenceAvailabilityReason::UnsupportedTaskKind,
            InferenceDiagnosticCode::UnsupportedTaskKind,
            "The requested inference task kind is not supported by the resolved model capability",
            Some("Select a task kind supported by the connected Pumas model"),
        );
    }
}

fn apply_runtime_diagnostics(
    request: &ResolveInferenceInterfaceRequest,
    capability: Option<&InferenceCapabilityFacts>,
    runtimes: &[InferenceRuntimeAvailabilityFact],
    availability: &mut InferenceAvailability,
    diagnostics: &mut Vec<InferenceInterfaceDiagnostic>,
) {
    let eligible_available_runtimes = eligible_available_runtimes(capability, runtimes);
    let eligible_available_runtime_ids = eligible_available_runtimes
        .iter()
        .map(|runtime| runtime.runtime_id.as_str())
        .collect::<Vec<_>>();

    if let Some(runtime_constraint) = &request.runtime_constraint {
        let runtime_fact = runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == *runtime_constraint);
        let supported_by_capability = capability
            .map(|capability| {
                capability.supported_runtime_ids.is_empty()
                    || capability
                        .supported_runtime_ids
                        .iter()
                        .any(|runtime_id| runtime_id == runtime_constraint)
            })
            .unwrap_or(false);
        let runtime_available = runtime_fact
            .map(|runtime| runtime.state == InferenceRuntimeAvailabilityState::Available)
            .unwrap_or(false);

        if !supported_by_capability || !runtime_available {
            let hint = if eligible_available_runtime_ids.is_empty() {
                "No alternative runtime is currently available".to_string()
            } else {
                format!(
                    "Available alternatives: {}",
                    eligible_available_runtime_ids.join(", ")
                )
            };
            mark_unavailable(
                availability,
                diagnostics,
                InferenceAvailabilityReason::ExplicitRuntimeInvalid,
                InferenceDiagnosticCode::InvalidRuntimeConstraint,
                "The explicit runtime constraint cannot execute this inference interface",
                Some(&hint),
            );
        }
    } else if capability.is_some() && eligible_available_runtime_ids.is_empty() {
        mark_unavailable(
            availability,
            diagnostics,
            InferenceAvailabilityReason::RuntimeNotInstalled,
            InferenceDiagnosticCode::DescriptorUnavailable,
            "No available runtime can execute this inference interface",
            Some("Install or enable a supported runtime before submitting the workflow"),
        );
    }

    if let (Some(_), Some(device_constraint)) = (capability, request.device_constraint.as_ref()) {
        let device_runtimes = eligible_available_runtimes
            .iter()
            .filter(|runtime| {
                request
                    .runtime_constraint
                    .as_ref()
                    .map(|runtime_constraint| runtime.runtime_id == *runtime_constraint)
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        let device_available = device_runtimes.iter().any(|runtime| {
            runtime
                .device_ids
                .iter()
                .any(|device_id| device_id == device_constraint)
        });

        if !device_available {
            let alternative_devices = eligible_available_runtimes
                .iter()
                .flat_map(|runtime| runtime.device_ids.iter().map(|device| device.as_str()))
                .collect::<Vec<_>>();
            let hint = if alternative_devices.is_empty() {
                "No alternative device is currently available".to_string()
            } else {
                format!(
                    "Available device alternatives: {}",
                    alternative_devices.join(", ")
                )
            };
            mark_unavailable(
                availability,
                diagnostics,
                InferenceAvailabilityReason::ExplicitDeviceInvalid,
                InferenceDiagnosticCode::InvalidDeviceConstraint,
                "The explicit device constraint is not available for this inference interface",
                Some(&hint),
            );
        }
    }
}

fn eligible_available_runtimes<'a>(
    capability: Option<&InferenceCapabilityFacts>,
    runtimes: &'a [InferenceRuntimeAvailabilityFact],
) -> Vec<&'a InferenceRuntimeAvailabilityFact> {
    runtimes
        .iter()
        .filter(|runtime| runtime.state == InferenceRuntimeAvailabilityState::Available)
        .filter(|runtime| {
            capability
                .map(|capability| {
                    capability.supported_runtime_ids.is_empty()
                        || capability
                            .supported_runtime_ids
                            .iter()
                            .any(|runtime_id| runtime_id == &runtime.runtime_id)
                })
                .unwrap_or(false)
        })
        .collect()
}

fn mark_unavailable(
    availability: &mut InferenceAvailability,
    diagnostics: &mut Vec<InferenceInterfaceDiagnostic>,
    reason: InferenceAvailabilityReason,
    code: InferenceDiagnosticCode,
    message: impl Into<String>,
    hint: Option<&str>,
) {
    availability.status = InferenceAvailabilityStatus::Unavailable;
    if !availability.reasons.contains(&reason) {
        availability.reasons.push(reason);
    }
    diagnostics.push(InferenceInterfaceDiagnostic {
        severity: InferenceDiagnosticSeverity::Error,
        code,
        message: message.into(),
        hint: hint.map(str::to_string),
        port_id: None,
    });
}

fn descriptor_fingerprint(
    request: &ResolveInferenceInterfaceRequest,
    facts: &InferenceInterfaceResolverFacts,
    availability: &InferenceAvailability,
) -> Result<InferenceInterfaceFingerprint, InferenceInterfaceResolverError> {
    let encoded = serde_json::to_vec(&(request, facts, availability))
        .map_err(|error| InferenceInterfaceResolverError::EncodeFacts(error.to_string()))?;
    let digest = blake3::hash(&encoded);
    InferenceInterfaceFingerprint::parse(format!("{DESCRIPTOR_FINGERPRINT_PREFIX}{digest}"))
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_inference_interface_contracts::{
        InferenceArtifactType, InferencePortDirection, InferencePortId, InferencePortOptions,
        InferencePortRequirement, InferenceRuntimeConditionKind, InferenceScalarType,
        InferenceValueType, PumasModelRef,
    };

    #[test]
    fn resolver_projects_capability_ports_when_facts_are_ready() {
        let descriptor = resolve_inference_interface_from_facts(request(None, None), ready_facts())
            .expect("descriptor");

        assert_eq!(
            descriptor.availability.status,
            InferenceAvailabilityStatus::Available
        );
        assert_eq!(descriptor.inputs[0].port_id.as_str(), "prompt");
        assert_eq!(descriptor.outputs[0].port_id.as_str(), "image");
        assert_eq!(descriptor.diagnostics, Vec::new());
        assert!(descriptor
            .descriptor_fingerprint
            .as_str()
            .starts_with(DESCRIPTOR_FINGERPRINT_PREFIX));
    }

    #[test]
    fn resolver_reports_missing_selected_artifact_without_guessing_ports() {
        let mut facts = ready_facts();
        facts.model.state = InferenceModelResolutionState::MissingSelectedArtifact;
        facts.capability = None;

        let descriptor =
            resolve_inference_interface_from_facts(request(None, None), facts).expect("descriptor");

        assert_eq!(
            descriptor.availability.status,
            InferenceAvailabilityStatus::Unavailable
        );
        assert!(descriptor.inputs.is_empty());
        assert!(descriptor.outputs.is_empty());
        assert!(descriptor
            .availability
            .reasons
            .contains(&InferenceAvailabilityReason::MissingSelectedArtifact));
        assert!(descriptor.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == InferenceDiagnosticCode::DescriptorUnavailable
                && diagnostic.message.contains("selected artifact")
        }));
    }

    #[test]
    fn explicit_invalid_runtime_blocks_with_advisory_alternative() {
        let descriptor = resolve_inference_interface_from_facts(
            request(Some(runtime_id("vllm")), None),
            ready_facts(),
        )
        .expect("descriptor");

        assert_eq!(
            descriptor.availability.status,
            InferenceAvailabilityStatus::Unavailable
        );
        assert!(descriptor
            .availability
            .reasons
            .contains(&InferenceAvailabilityReason::ExplicitRuntimeInvalid));
        assert!(descriptor.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == InferenceDiagnosticCode::InvalidRuntimeConstraint
                && diagnostic
                    .hint
                    .as_deref()
                    .is_some_and(|hint| hint.contains("pytorch"))
        }));
    }

    #[test]
    fn explicit_invalid_runtime_advisory_excludes_unsupported_available_runtimes() {
        let mut facts = ready_facts();
        facts.runtimes.push(InferenceRuntimeAvailabilityFact {
            runtime_id: runtime_id("onnx"),
            state: InferenceRuntimeAvailabilityState::Available,
            device_ids: vec![device_id("cpu")],
        });

        let descriptor =
            resolve_inference_interface_from_facts(request(Some(runtime_id("vllm")), None), facts)
                .expect("descriptor");

        let diagnostic = descriptor
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == InferenceDiagnosticCode::InvalidRuntimeConstraint)
            .expect("runtime diagnostic");
        let hint = diagnostic.hint.as_deref().expect("runtime hint");
        assert!(hint.contains("pytorch"));
        assert!(!hint.contains("onnx"));
    }

    #[test]
    fn explicit_device_constraint_is_scoped_to_explicit_runtime() {
        let mut facts = ready_facts();
        facts
            .capability
            .as_mut()
            .expect("capability")
            .supported_runtime_ids
            .push(runtime_id("vllm"));
        facts.runtimes.push(InferenceRuntimeAvailabilityFact {
            runtime_id: runtime_id("vllm"),
            state: InferenceRuntimeAvailabilityState::Available,
            device_ids: vec![device_id("cpu")],
        });

        let descriptor = resolve_inference_interface_from_facts(
            request(Some(runtime_id("vllm")), Some(device_id("cuda.0"))),
            facts,
        )
        .expect("descriptor");

        assert_eq!(
            descriptor.availability.status,
            InferenceAvailabilityStatus::Unavailable
        );
        assert!(descriptor
            .availability
            .reasons
            .contains(&InferenceAvailabilityReason::ExplicitDeviceInvalid));
        assert!(descriptor.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == InferenceDiagnosticCode::InvalidDeviceConstraint
                && diagnostic
                    .hint
                    .as_deref()
                    .is_some_and(|hint| hint.contains("cuda.0") && hint.contains("cpu"))
        }));
    }

    #[test]
    fn explicit_runtime_and_device_pass_when_device_belongs_to_runtime() {
        let descriptor = resolve_inference_interface_from_facts(
            request(Some(runtime_id("pytorch")), Some(device_id("cuda.0"))),
            ready_facts(),
        )
        .expect("descriptor");

        assert_eq!(
            descriptor.availability.status,
            InferenceAvailabilityStatus::Available
        );
        assert!(descriptor.diagnostics.is_empty());
    }

    #[test]
    fn resolver_descriptor_does_not_serialize_paths_or_package_facts() {
        let descriptor = resolve_inference_interface_from_facts(request(None, None), ready_facts())
            .expect("descriptor");

        let encoded = serde_json::to_string(&descriptor).expect("descriptor json");
        assert!(!encoded.contains("model_path"));
        assert!(!encoded.contains("local_load_path"));
        assert!(!encoded.contains("package_facts"));
        assert!(!encoded.contains("runtime_host"));
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

use std::collections::BTreeSet;

use pantograph_dependency_planning::{
    DependencyEnvironmentRef, DependencyPlanningContractError, DeviceIntentId,
};

use crate::dispatch_selection::{
    SchedulerDispatchCandidate, SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
};
use crate::error::SchedulerContractError;
use crate::intent::SchedulableTaskIntent;
use crate::readiness::SchedulerDependencyReadinessProof;
use crate::resource::{SchedulerResourceFitAssessment, SchedulerResourceReservation};
use crate::resource_types::{SchedulerResourceDiagnosticSeverity, SchedulerResourceFitState};

const MAX_ID_LEN: usize = 128;
const MAX_TEXT_LEN: usize = 1024;

pub(crate) fn validate_selected_device_ids(
    selected_device_ids: &[DeviceIntentId],
) -> Result<(), SchedulerContractError> {
    if selected_device_ids.is_empty() {
        return Err(SchedulerContractError::MissingField {
            field: "dispatch_candidate.selected_device_ids",
        });
    }
    let mut seen = BTreeSet::new();
    for device_id in selected_device_ids {
        if !seen.insert(device_id) {
            return Err(SchedulerContractError::InvalidField {
                field: "dispatch_candidate.selected_device_ids",
                reason: "selected device ids must not contain duplicates",
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_candidate_selected_model_ref(
    candidate: &SchedulerDispatchCandidate,
    intent: &SchedulableTaskIntent,
) -> Result<(), SchedulerContractError> {
    if candidate.selected_model_ref.model_id != intent.model_ref.model_id {
        return Err(SchedulerContractError::InvalidField {
            field: "dispatch_candidate.selected_model_ref.model_id",
            reason: "selected model id must match task intent model id",
        });
    }
    if let Some(requested_artifact_id) = &intent.model_ref.selected_artifact_id {
        if Some(requested_artifact_id) != candidate.selected_model_ref.selected_artifact_id.as_ref()
        {
            return Err(SchedulerContractError::InvalidField {
                field: "dispatch_candidate.selected_model_ref.selected_artifact_id",
                reason: "selected artifact id must satisfy task intent artifact requirement",
            });
        }
    }
    if let Some(requested_artifact_path) = &intent.model_ref.selected_artifact_path {
        if Some(requested_artifact_path)
            != candidate.selected_model_ref.selected_artifact_path.as_ref()
        {
            return Err(SchedulerContractError::InvalidField {
                field: "dispatch_candidate.selected_model_ref.selected_artifact_path",
                reason: "selected artifact path must satisfy task intent artifact requirement",
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_reservation(
    candidate: &SchedulerDispatchCandidate,
    intent: &SchedulableTaskIntent,
    reservation: &SchedulerResourceReservation,
) -> Result<(), SchedulerContractError> {
    if reservation.workflow_run_id != intent.workflow_run_id {
        return Err(SchedulerContractError::InvalidField {
            field: "dispatch_candidate.reservation.workflow_run_id",
            reason: "reservation workflow run id must match task intent",
        });
    }
    if reservation.task_id != intent.task_id {
        return Err(SchedulerContractError::InvalidField {
            field: "dispatch_candidate.reservation.task_id",
            reason: "reservation task id must match task intent",
        });
    }
    if reservation.reserved_bytes == 0 {
        return Err(SchedulerContractError::InvalidField {
            field: "dispatch_candidate.reservation.reserved_bytes",
            reason: "reserved bytes must be greater than zero",
        });
    }
    if !candidate
        .selected_device_ids
        .contains(&reservation.device_id)
    {
        return Err(SchedulerContractError::InvalidField {
            field: "dispatch_candidate.reservation.device_id",
            reason: "reservation device must be selected by the candidate",
        });
    }
    Ok(())
}

pub(crate) fn validate_resource_fit(
    intent: &SchedulableTaskIntent,
    resource_fit_assessment: &SchedulerResourceFitAssessment,
) -> Result<(), SchedulerContractError> {
    if resource_fit_assessment.workflow_run_id != intent.workflow_run_id {
        return Err(SchedulerContractError::InvalidField {
            field: "dispatch_candidate.resource_fit_assessment.workflow_run_id",
            reason: "resource fit workflow run id must match task intent",
        });
    }
    if resource_fit_assessment.task_id != intent.task_id {
        return Err(SchedulerContractError::InvalidField {
            field: "dispatch_candidate.resource_fit_assessment.task_id",
            reason: "resource fit task id must match task intent",
        });
    }
    if matches!(
        resource_fit_assessment.state,
        SchedulerResourceFitState::WaitingForResources
            | SchedulerResourceFitState::ImpossibleFit
            | SchedulerResourceFitState::Unknown
    ) && resource_fit_assessment.diagnostics.is_empty()
    {
        return Err(SchedulerContractError::MissingField {
            field: "dispatch_candidate.resource_fit_assessment.diagnostics",
        });
    }
    for diagnostic in &resource_fit_assessment.diagnostics {
        if matches!(
            diagnostic.severity,
            SchedulerResourceDiagnosticSeverity::Error
        ) && resource_fit_assessment.state == SchedulerResourceFitState::Fits
        {
            return Err(SchedulerContractError::InvalidField {
                field: "dispatch_candidate.resource_fit_assessment.diagnostics",
                reason: "fitting resource assessment must not carry error diagnostics",
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_environment_ref(
    readiness_proof: &SchedulerDependencyReadinessProof,
    environment_ref: &DependencyEnvironmentRef,
) -> Result<(), SchedulerContractError> {
    let Some(proof_environment_ref) = &readiness_proof.preflight_result.environment_ref else {
        return Err(SchedulerContractError::MissingField {
            field: "readiness_proof.preflight_result.environment_ref",
        });
    };
    if proof_environment_ref != environment_ref {
        return Err(SchedulerContractError::InvalidField {
            field: "environment_ref",
            reason: "dispatch selection environment ref must match readiness proof",
        });
    }
    Ok(())
}

pub(crate) fn default_scheduler_dispatch_selection_contract_version() -> u16 {
    SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION
}

pub(crate) fn validate_contract_version(value: u16) -> Result<(), SchedulerContractError> {
    if value == SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler dispatch selection contract version",
        })
    }
}

pub(crate) fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField { field });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(SchedulerContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

pub(crate) fn validate_text(
    field: &'static str,
    value: &str,
) -> Result<(), SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField { field });
    }
    if trimmed.len() > MAX_TEXT_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field,
            max_len: MAX_TEXT_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(SchedulerContractError::InvalidText { field });
    }
    Ok(())
}

pub(crate) fn map_dependency_error(
    error: DependencyPlanningContractError,
) -> SchedulerContractError {
    match error {
        DependencyPlanningContractError::MissingField { field } => {
            SchedulerContractError::MissingField { field }
        }
        DependencyPlanningContractError::FieldTooLong { field, max_len } => {
            SchedulerContractError::FieldTooLong { field, max_len }
        }
        DependencyPlanningContractError::InvalidIdentifier { field } => {
            SchedulerContractError::InvalidIdentifier { field }
        }
        DependencyPlanningContractError::InvalidText { field } => {
            SchedulerContractError::InvalidText { field }
        }
        DependencyPlanningContractError::InvalidField { field, reason } => {
            SchedulerContractError::InvalidField { field, reason }
        }
        _ => SchedulerContractError::InvalidField {
            field: "dependency_planning",
            reason: "dependency planning contract value is invalid",
        },
    }
}

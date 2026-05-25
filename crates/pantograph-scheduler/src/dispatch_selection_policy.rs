use std::collections::BTreeSet;

use crate::dispatch::{SchedulerDispatchDecision, SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION};
use crate::dispatch_selection::{
    SchedulerDispatchCandidate, SchedulerDispatchSelectionDecision,
    SchedulerDispatchSelectionDiagnostic, SchedulerDispatchSelectionDiagnosticCode,
    SchedulerDispatchSelectionRequest, SchedulerDispatchSelectionState,
    ValidatedSchedulerDispatchSelectionDecision, ValidatedSchedulerDispatchSelectionRequest,
    SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
};
use crate::error::SchedulerContractError;
use crate::intent::SchedulableTaskIntent;
use crate::resource_types::SchedulerResourceFitState;

#[must_use]
pub fn select_scheduler_dispatch(
    request: ValidatedSchedulerDispatchSelectionRequest,
) -> Result<ValidatedSchedulerDispatchSelectionDecision, SchedulerContractError> {
    let request = request.into_inner();
    let diagnostics = duplicate_candidate_diagnostics(&request.candidates);
    if !diagnostics.is_empty() {
        return validate_decision(no_selection_decision(request.task_intent, diagnostics));
    }

    let mut eligible = Vec::new();
    let mut diagnostics = Vec::new();
    for candidate in &request.candidates {
        match candidate_eligibility(&request, candidate) {
            Ok(()) => eligible.push(candidate.clone()),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    match eligible.as_slice() {
        [candidate] => validate_decision(selected_decision(request, candidate)),
        [] => {
            if diagnostics.is_empty() {
                diagnostics.push(SchedulerDispatchSelectionDiagnostic::error(
                    SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
                    None,
                    "No scheduler dispatch candidates were supplied.",
                ));
            }
            validate_decision(no_selection_decision(request.task_intent, diagnostics))
        }
        _ => validate_decision(no_selection_decision(
            request.task_intent,
            vec![SchedulerDispatchSelectionDiagnostic::error(
                SchedulerDispatchSelectionDiagnosticCode::AmbiguousRanking,
                None,
                "Multiple dispatch candidates are eligible and no ranking policy resolved one.",
            )],
        )),
    }
}

fn selected_decision(
    request: SchedulerDispatchSelectionRequest,
    candidate: &SchedulerDispatchCandidate,
) -> SchedulerDispatchSelectionDecision {
    let Some(reservation) = candidate.reservation.as_ref() else {
        return no_selection_decision(
            request.task_intent,
            vec![SchedulerDispatchSelectionDiagnostic::error(
                SchedulerDispatchSelectionDiagnosticCode::MissingReservation,
                Some(&candidate.candidate_id),
                "Dispatch candidate is missing a resource reservation fact.",
            )],
        );
    };
    let dispatch_decision = SchedulerDispatchDecision {
        contract_version: SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION,
        workflow_id: request.task_intent.workflow_id.clone(),
        workflow_run_id: request.task_intent.workflow_run_id.clone(),
        node_id: request.task_intent.node_id.clone(),
        task_id: request.task_intent.task_id.clone(),
        task_intent: request.task_intent.clone(),
        selected_runtime_id: candidate.selected_runtime_id.clone(),
        selected_runtime_variant_id: candidate.selected_runtime_variant_id.clone(),
        selected_device_ids: candidate.selected_device_ids.clone(),
        selected_model_ref: candidate.selected_model_ref.clone(),
        readiness_proof: request.readiness_proof,
        environment_ref: request.environment_ref,
        batching_group_id: candidate.batching_group_id.clone(),
        reservation_lease_id: reservation.reservation_lease_id.clone(),
        runtime_trait_settings: candidate.runtime_trait_settings.clone(),
        diagnostics: Vec::new(),
    };
    SchedulerDispatchSelectionDecision {
        contract_version: SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
        task_intent: request.task_intent,
        state: SchedulerDispatchSelectionState::Selected,
        dispatch_decision: Some(dispatch_decision),
        diagnostics: vec![SchedulerDispatchSelectionDiagnostic::info(
            SchedulerDispatchSelectionDiagnosticCode::CandidateSelected,
            Some(&candidate.candidate_id),
            "Scheduler dispatch selection chose the only eligible candidate.",
        )],
    }
}

fn no_selection_decision(
    task_intent: SchedulableTaskIntent,
    diagnostics: Vec<SchedulerDispatchSelectionDiagnostic>,
) -> SchedulerDispatchSelectionDecision {
    SchedulerDispatchSelectionDecision {
        contract_version: SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
        task_intent,
        state: SchedulerDispatchSelectionState::NoSelection,
        dispatch_decision: None,
        diagnostics,
    }
}

fn validate_decision(
    decision: SchedulerDispatchSelectionDecision,
) -> Result<ValidatedSchedulerDispatchSelectionDecision, SchedulerContractError> {
    ValidatedSchedulerDispatchSelectionDecision::try_from(decision)
}

fn candidate_eligibility(
    request: &SchedulerDispatchSelectionRequest,
    candidate: &SchedulerDispatchCandidate,
) -> Result<(), SchedulerDispatchSelectionDiagnostic> {
    if let Some(requested_runtime_id) = &request.task_intent.constraints.requested_runtime_id {
        if requested_runtime_id != &candidate.selected_runtime_id {
            return Err(SchedulerDispatchSelectionDiagnostic::error(
                SchedulerDispatchSelectionDiagnosticCode::IncompatibleRuntimeRequirement,
                Some(&candidate.candidate_id),
                "Dispatch candidate does not satisfy the explicit runtime requirement.",
            ));
        }
    }
    if let Some(requested_device_id) = &request.task_intent.constraints.requested_device_id {
        if !candidate.selected_device_ids.contains(requested_device_id) {
            return Err(SchedulerDispatchSelectionDiagnostic::error(
                SchedulerDispatchSelectionDiagnosticCode::IncompatibleDeviceRequirement,
                Some(&candidate.candidate_id),
                "Dispatch candidate does not satisfy the explicit device requirement.",
            ));
        }
    }
    let Some(reservation) = &candidate.reservation else {
        return Err(SchedulerDispatchSelectionDiagnostic::error(
            SchedulerDispatchSelectionDiagnosticCode::MissingReservation,
            Some(&candidate.candidate_id),
            "Dispatch candidate is missing a resource reservation fact.",
        ));
    };
    if !candidate
        .selected_device_ids
        .contains(&reservation.device_id)
    {
        return Err(SchedulerDispatchSelectionDiagnostic::error(
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
            Some(&candidate.candidate_id),
            "Dispatch candidate reservation device is not selected by the candidate.",
        ));
    }
    let Some(resource_fit_assessment) = &candidate.resource_fit_assessment else {
        return Err(SchedulerDispatchSelectionDiagnostic::error(
            SchedulerDispatchSelectionDiagnosticCode::MissingResourceFit,
            Some(&candidate.candidate_id),
            "Dispatch candidate is missing a resource fit assessment.",
        ));
    };
    if resource_fit_assessment.state != SchedulerResourceFitState::Fits {
        return Err(SchedulerDispatchSelectionDiagnostic::error(
            SchedulerDispatchSelectionDiagnosticCode::ResourceFitRejected,
            Some(&candidate.candidate_id),
            "Dispatch candidate resource fit assessment is not runnable.",
        ));
    }
    Ok(())
}

fn duplicate_candidate_diagnostics(
    candidates: &[SchedulerDispatchCandidate],
) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    let mut seen = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for candidate in candidates {
        if !seen.insert(candidate.candidate_id.as_str()) {
            diagnostics.push(SchedulerDispatchSelectionDiagnostic::error(
                SchedulerDispatchSelectionDiagnosticCode::DuplicateCandidateId,
                Some(&candidate.candidate_id),
                "Dispatch candidate ids must be unique.",
            ));
        }
    }
    diagnostics
}

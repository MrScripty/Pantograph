use std::cmp::Ordering;

use crate::snapshot::RuntimeRegistryRuntimeSnapshot;
use crate::state::RuntimeRegistryStatus;
use crate::technical_fit::{
    compare_candidate_ids, decision_from_candidate_with_trace,
    explicit_device_unavailable_diagnostics, unselected_decision_with_device_diagnostics,
    RuntimeTechnicalFitCandidate, RuntimeTechnicalFitCandidateSetSummary,
    RuntimeTechnicalFitDecision, RuntimeTechnicalFitDeviceDiagnostic,
    RuntimeTechnicalFitDeviceDiagnosticCode, RuntimeTechnicalFitDeviceDiagnosticSeverity,
    RuntimeTechnicalFitDevicePolicy, RuntimeTechnicalFitFactor, RuntimeTechnicalFitReason,
    RuntimeTechnicalFitReasonCode, RuntimeTechnicalFitRequest, RuntimeTechnicalFitResidencyState,
    RuntimeTechnicalFitSelectionMode, RuntimeTechnicalFitSelectionPolicyTrace,
    RuntimeTechnicalFitWarmupState,
};

const MAX_HEADROOM_RANKABLE_ACTIVE_RESERVATIONS: usize = u16::MAX as usize;
const TECHNICAL_FIT_SELECTION_POLICY_VERSION: u32 = 1;
const FNV_OFFSET_BASIS_64: u64 = 0xcbf29ce484222325;
const FNV_PRIME_64: u64 = 0x100000001b3;

pub(crate) fn select_runtime_technical_fit_automatically(
    normalized: &RuntimeTechnicalFitRequest,
    candidates: &[RuntimeTechnicalFitCandidate],
) -> RuntimeTechnicalFitDecision {
    let mut reasons = Vec::new();
    let mut eligible_candidates = candidates
        .iter()
        .filter(|candidate| candidate_is_eligible(candidate, normalized))
        .collect::<Vec<_>>();

    if headroom_ranking_applies(normalized) {
        if let Some(unrankable_candidate) = eligible_candidates.iter().find(|candidate| {
            candidate_active_reservation_count_exceeds_rankable_range(candidate, normalized)
        }) {
            let mut unrankable_reasons = Vec::new();
            if queue_pressure_applies(normalized) {
                unrankable_reasons.push(RuntimeTechnicalFitReason::new(
                    RuntimeTechnicalFitReasonCode::QueuePressure,
                    Some(unrankable_candidate.candidate_id.as_str()),
                ));
            }
            if budget_pressure_applies(normalized) {
                unrankable_reasons.push(RuntimeTechnicalFitReason::new(
                    RuntimeTechnicalFitReasonCode::BudgetPressure,
                    Some(unrankable_candidate.candidate_id.as_str()),
                ));
            }

            return unselected_decision_with_device_diagnostics(
                RuntimeTechnicalFitSelectionMode::Automatic,
                unrankable_reasons,
                vec![unrankable_headroom_candidate_diagnostic(
                    unrankable_candidate,
                )],
            );
        }
    }

    eligible_candidates.sort_by(|left, right| compare_candidates(left, right, normalized));

    if let Some(selected_candidate) = eligible_candidates.first().copied() {
        let tied_candidates = eligible_candidates
            .iter()
            .copied()
            .filter(|candidate| {
                compare_candidate_priority(selected_candidate, candidate, normalized).is_eq()
            })
            .collect::<Vec<_>>();
        let (selected_candidate, controlled_exploration_seed_basis) = if tied_candidates.len() > 1 {
            let seed_basis = controlled_exploration_seed_basis(normalized, &tied_candidates);
            let selected_candidate =
                controlled_exploration_candidate(&tied_candidates, &seed_basis);
            (selected_candidate, Some(seed_basis))
        } else {
            (selected_candidate, None)
        };

        let selection_policy_trace = match automatic_selection_policy_trace(
            normalized,
            &eligible_candidates,
            selected_candidate,
            controlled_exploration_seed_basis.as_deref(),
        ) {
            Ok(trace) => trace,
            Err(diagnostic) => {
                return unselected_decision_with_device_diagnostics(
                    RuntimeTechnicalFitSelectionMode::Automatic,
                    Vec::new(),
                    vec![diagnostic],
                );
            }
        };

        reasons.push(RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::AutomaticRanking,
            Some(selected_candidate.candidate_id.as_str()),
        ));
        if controlled_exploration_seed_basis.is_some() {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::ControlledExploration,
                Some(selected_candidate.candidate_id.as_str()),
            ));
        }

        reasons.push(RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::RuntimeRequirements,
            Some(selected_candidate.candidate_id.as_str()),
        ));

        if uses_factor(normalized, RuntimeTechnicalFitFactor::ResidencyReuse)
            && candidate_residency_rank(selected_candidate, normalized) > 0
        {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::ResidencyReuse,
                Some(selected_candidate.candidate_id.as_str()),
            ));
        }

        if uses_factor(normalized, RuntimeTechnicalFitFactor::WarmupCost)
            && candidate_warmup_rank(selected_candidate, normalized) > 0
        {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::WarmupCost,
                Some(selected_candidate.candidate_id.as_str()),
            ));
        }

        if queue_pressure_applies(normalized)
            && eligible_candidates.iter().skip(1).any(|candidate| {
                candidate_queue_pressure_rank(selected_candidate, normalized)
                    > candidate_queue_pressure_rank(candidate, normalized)
            })
        {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::QueuePressure,
                Some(selected_candidate.candidate_id.as_str()),
            ));
        }

        if budget_pressure_applies(normalized)
            && eligible_candidates.iter().skip(1).any(|candidate| {
                candidate_budget_pressure_rank(selected_candidate, normalized)
                    > candidate_budget_pressure_rank(candidate, normalized)
            })
        {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::BudgetPressure,
                Some(selected_candidate.candidate_id.as_str()),
            ));
        }

        return decision_from_candidate_with_trace(
            RuntimeTechnicalFitSelectionMode::Automatic,
            selected_candidate,
            reasons,
            Some(selection_policy_trace),
        );
    }

    let scoped_diagnostic_candidate = diagnostic_candidate(candidates, normalized);
    if candidates.is_empty() {
        reasons.push(RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::MissingCandidateData,
            None,
        ));
    } else {
        if candidates
            .iter()
            .any(|candidate| candidate_has_missing_state(candidate, normalized))
        {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::MissingRuntimeState,
                scoped_diagnostic_candidate.map(|candidate| candidate.candidate_id.as_str()),
            ));
        }
        reasons.push(RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::MissingCandidateData,
            scoped_diagnostic_candidate.map(|candidate| candidate.candidate_id.as_str()),
        ));
    }

    unselected_decision_with_device_diagnostics(
        RuntimeTechnicalFitSelectionMode::Automatic,
        reasons,
        automatic_no_valid_candidate_diagnostics(normalized),
    )
}

fn automatic_selection_policy_trace(
    request: &RuntimeTechnicalFitRequest,
    eligible_candidates: &[&RuntimeTechnicalFitCandidate],
    selected_candidate: &RuntimeTechnicalFitCandidate,
    controlled_exploration_seed_basis: Option<&str>,
) -> Result<RuntimeTechnicalFitSelectionPolicyTrace, RuntimeTechnicalFitDeviceDiagnostic> {
    let candidate_set_summary = automatic_candidate_set_summary(request, eligible_candidates)?;
    Ok(RuntimeTechnicalFitSelectionPolicyTrace {
        policy_version: TECHNICAL_FIT_SELECTION_POLICY_VERSION,
        candidate_set_summary: Some(candidate_set_summary),
        ranking_reason: Some("candidate_priority".to_string()),
        exploration_reason: controlled_exploration_seed_basis
            .map(|_| "equal_priority_seeded_choice".to_string()),
        seed_basis: controlled_exploration_seed_basis
            .map(ToOwned::to_owned)
            .or_else(|| {
                Some(format!(
                    "workflow:{}|snapshot:{}|candidate:{}",
                    request.workflow_id.as_deref().unwrap_or("unknown"),
                    request.runtime_snapshot.generated_at_ms,
                    selected_candidate.candidate_id
                ))
            }),
    }
    .normalized())
}

fn automatic_candidate_set_summary(
    request: &RuntimeTechnicalFitRequest,
    eligible_candidates: &[&RuntimeTechnicalFitCandidate],
) -> Result<RuntimeTechnicalFitCandidateSetSummary, RuntimeTechnicalFitDeviceDiagnostic> {
    let total_candidate_count = checked_candidate_count(request.candidates.len())?;
    let eligible_candidate_count = checked_candidate_count(eligible_candidates.len())?;
    let rejected_candidate_count = total_candidate_count
        .checked_sub(eligible_candidate_count)
        .ok_or_else(candidate_summary_count_diagnostic)?;

    Ok(RuntimeTechnicalFitCandidateSetSummary {
        total_candidate_count,
        eligible_candidate_count,
        rejected_candidate_count,
        eligible_candidate_ids: eligible_candidates
            .iter()
            .map(|candidate| candidate.candidate_id.clone())
            .collect(),
    }
    .normalized())
}

fn checked_candidate_count(count: usize) -> Result<u32, RuntimeTechnicalFitDeviceDiagnostic> {
    u32::try_from(count).map_err(|_| candidate_summary_count_diagnostic())
}

fn candidate_summary_count_diagnostic() -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: "technical-fit candidate set is too large to summarize exactly".to_string(),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: None,
    }
}

fn controlled_exploration_seed_basis(
    request: &RuntimeTechnicalFitRequest,
    tied_candidates: &[&RuntimeTechnicalFitCandidate],
) -> String {
    let candidate_ids = tied_candidates
        .iter()
        .map(|candidate| candidate.candidate_id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "workflow:{}|snapshot:{}|candidates:{}",
        request.workflow_id.as_deref().unwrap_or("unknown"),
        request.runtime_snapshot.generated_at_ms,
        candidate_ids
    )
}

fn controlled_exploration_candidate<'a>(
    tied_candidates: &[&'a RuntimeTechnicalFitCandidate],
    seed_basis: &str,
) -> &'a RuntimeTechnicalFitCandidate {
    let index = (stable_policy_hash(seed_basis) as usize) % tied_candidates.len();
    tied_candidates[index]
}

fn stable_policy_hash(seed_basis: &str) -> u64 {
    seed_basis
        .as_bytes()
        .iter()
        .fold(FNV_OFFSET_BASIS_64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME_64)
        })
}

fn diagnostic_candidate<'a>(
    candidates: &'a [RuntimeTechnicalFitCandidate],
    normalized: &RuntimeTechnicalFitRequest,
) -> Option<&'a RuntimeTechnicalFitCandidate> {
    candidates
        .iter()
        .filter(|candidate| {
            let runtime_snapshot = candidate_runtime_snapshot(candidate, normalized);
            candidate_matches_required_models(candidate, runtime_snapshot, normalized)
                && candidate_matches_required_backends(candidate, runtime_snapshot, normalized)
        })
        .min_by(|left, right| compare_candidate_ids(left, right))
        .or_else(|| {
            candidates
                .iter()
                .min_by(|left, right| compare_candidate_ids(left, right))
        })
}

fn automatic_no_valid_candidate_diagnostics(
    request: &RuntimeTechnicalFitRequest,
) -> Vec<RuntimeTechnicalFitDeviceDiagnostic> {
    let explicit_device_diagnostics = explicit_device_unavailable_diagnostics(request);
    if !explicit_device_diagnostics.is_empty() {
        return explicit_device_diagnostics;
    }

    vec![RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: "technical-fit auto policy found no valid candidate".to_string(),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: None,
    }]
}

fn unrankable_headroom_candidate_diagnostic(
    candidate: &RuntimeTechnicalFitCandidate,
) -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message:
            "technical-fit cannot rank candidate headroom because active reservation count exceeds the supported range"
                .to_string(),
        device_class: candidate.device_class,
        device_id: candidate.selected_device_id.clone(),
        runtime_variant_id: candidate.runtime_variant_id.clone(),
        backend_key: candidate.backend_key.clone(),
    }
}

fn compare_candidates(
    left: &RuntimeTechnicalFitCandidate,
    right: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> Ordering {
    compare_candidate_priority(left, right, request)
        .then_with(|| compare_candidate_ids(left, right))
}

fn compare_candidate_priority(
    left: &RuntimeTechnicalFitCandidate,
    right: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> Ordering {
    candidate_residency_rank(left, request)
        .cmp(&candidate_residency_rank(right, request))
        .reverse()
        .then_with(|| {
            candidate_warmup_rank(left, request)
                .cmp(&candidate_warmup_rank(right, request))
                .reverse()
        })
        .then_with(|| {
            candidate_queue_pressure_rank(left, request)
                .cmp(&candidate_queue_pressure_rank(right, request))
                .reverse()
        })
        .then_with(|| {
            candidate_budget_pressure_rank(left, request)
                .cmp(&candidate_budget_pressure_rank(right, request))
                .reverse()
        })
}

pub(crate) fn candidate_is_eligible(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    let runtime_snapshot = candidate_runtime_snapshot(candidate, request);

    (!uses_factor(request, RuntimeTechnicalFitFactor::RuntimeRequirements)
        || candidate.supports_runtime_requirements)
        && candidate_matches_required_models(candidate, runtime_snapshot, request)
        && candidate_matches_required_backends(candidate, runtime_snapshot, request)
        && candidate_matches_device_policy(candidate, request)
        && candidate_meets_context_length(candidate, request)
}

pub(crate) fn candidate_matches_device_policy(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    let Some(RuntimeTechnicalFitDevicePolicy::Explicit {
        device_class,
        device_id,
    }) = request.device_policy.as_ref()
    else {
        return true;
    };

    if candidate.device_class != Some(*device_class) {
        return false;
    }

    let Some(device_id) = device_id.as_deref() else {
        return true;
    };

    candidate.selected_device_id.as_deref() == Some(device_id)
}

fn candidate_matches_required_models(
    candidate: &RuntimeTechnicalFitCandidate,
    runtime_snapshot: Option<&RuntimeRegistryRuntimeSnapshot>,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    if request.required_model_ids.is_empty() {
        return true;
    }

    if let Some(model_id) = candidate.model_id.as_deref() {
        return request
            .required_model_ids
            .iter()
            .any(|required| required == model_id);
    }

    let Some(runtime_snapshot) = runtime_snapshot else {
        return false;
    };

    request.required_model_ids.iter().all(|required| {
        runtime_snapshot
            .models
            .iter()
            .any(|model| model.model_id == *required)
    })
}

fn candidate_matches_required_backends(
    candidate: &RuntimeTechnicalFitCandidate,
    runtime_snapshot: Option<&RuntimeRegistryRuntimeSnapshot>,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    if request.required_backend_keys.is_empty() {
        return true;
    }

    let candidate_backend_matches = candidate.backend_key.as_deref().map(|backend_key| {
        request
            .required_backend_keys
            .iter()
            .any(|required| required == backend_key)
    });

    if candidate_backend_matches == Some(true) {
        return true;
    }

    let Some(runtime_snapshot) = runtime_snapshot else {
        return false;
    };

    request.required_backend_keys.iter().all(|required| {
        runtime_snapshot
            .backend_keys
            .iter()
            .any(|backend_key| backend_key == required)
    })
}

fn candidate_meets_context_length(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    let Some(required_context_window_tokens) = request.required_context_window_tokens else {
        return true;
    };

    let Some(context_window_tokens) = candidate.context_window_tokens else {
        return false;
    };

    context_window_tokens >= required_context_window_tokens
}

fn candidate_has_missing_state(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    candidate_runtime_snapshot(candidate, request).is_none()
        && candidate.runtime_id.is_some()
        && (candidate.residency_state.is_none() || candidate.warmup_state.is_none())
}

fn candidate_residency_rank(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> u8 {
    if !uses_factor(request, RuntimeTechnicalFitFactor::ResidencyReuse) {
        return 0;
    }

    match candidate
        .residency_state
        .or_else(|| snapshot_residency_state(candidate_runtime_snapshot(candidate, request)))
    {
        Some(RuntimeTechnicalFitResidencyState::Active) => 3,
        Some(RuntimeTechnicalFitResidencyState::Reserved) => 2,
        Some(RuntimeTechnicalFitResidencyState::Loaded) => 1,
        Some(RuntimeTechnicalFitResidencyState::Unloaded) | None => 0,
    }
}

fn candidate_warmup_rank(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> u8 {
    if !uses_factor(request, RuntimeTechnicalFitFactor::WarmupCost) {
        return 0;
    }

    match candidate
        .warmup_state
        .or_else(|| snapshot_warmup_state(candidate_runtime_snapshot(candidate, request)))
    {
        Some(RuntimeTechnicalFitWarmupState::Ready) => 2,
        Some(RuntimeTechnicalFitWarmupState::Warm) => 1,
        Some(RuntimeTechnicalFitWarmupState::Cold) | None => 0,
    }
}

fn candidate_queue_pressure_rank(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> u16 {
    if !queue_pressure_applies(request) {
        return 0;
    }

    runtime_headroom_rank(candidate, request)
}

fn candidate_budget_pressure_rank(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> u16 {
    if !budget_pressure_applies(request) {
        return 0;
    }

    runtime_headroom_rank(candidate, request)
}

fn headroom_ranking_applies(request: &RuntimeTechnicalFitRequest) -> bool {
    queue_pressure_applies(request) || budget_pressure_applies(request)
}

fn candidate_active_reservation_count_exceeds_rankable_range(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    candidate_runtime_snapshot(candidate, request)
        .map(|runtime| {
            runtime.active_reservation_ids.len() > MAX_HEADROOM_RANKABLE_ACTIVE_RESERVATIONS
        })
        .unwrap_or(false)
}

fn runtime_headroom_rank(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> u16 {
    let active_reservation_count = candidate_runtime_snapshot(candidate, request)
        .map(|runtime| runtime.active_reservation_ids.len())
        .unwrap_or(usize::MAX);
    u16::MAX - active_reservation_count.min(MAX_HEADROOM_RANKABLE_ACTIVE_RESERVATIONS) as u16
}

fn candidate_runtime_snapshot<'a>(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &'a RuntimeTechnicalFitRequest,
) -> Option<&'a RuntimeRegistryRuntimeSnapshot> {
    let runtime_id = candidate.runtime_id.as_deref()?;
    request
        .runtime_snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == runtime_id)
}

fn snapshot_residency_state(
    runtime_snapshot: Option<&RuntimeRegistryRuntimeSnapshot>,
) -> Option<RuntimeTechnicalFitResidencyState> {
    let runtime_snapshot = runtime_snapshot?;
    match runtime_snapshot.status {
        RuntimeRegistryStatus::Busy => Some(RuntimeTechnicalFitResidencyState::Active),
        RuntimeRegistryStatus::Ready => {
            if runtime_snapshot.active_reservation_ids.is_empty() {
                Some(RuntimeTechnicalFitResidencyState::Loaded)
            } else {
                Some(RuntimeTechnicalFitResidencyState::Reserved)
            }
        }
        RuntimeRegistryStatus::Warming => Some(RuntimeTechnicalFitResidencyState::Reserved),
        RuntimeRegistryStatus::Stopped
        | RuntimeRegistryStatus::Stopping
        | RuntimeRegistryStatus::Unhealthy
        | RuntimeRegistryStatus::Failed => Some(RuntimeTechnicalFitResidencyState::Unloaded),
    }
}

fn snapshot_warmup_state(
    runtime_snapshot: Option<&RuntimeRegistryRuntimeSnapshot>,
) -> Option<RuntimeTechnicalFitWarmupState> {
    let runtime_snapshot = runtime_snapshot?;
    match runtime_snapshot.status {
        RuntimeRegistryStatus::Busy | RuntimeRegistryStatus::Ready => {
            Some(RuntimeTechnicalFitWarmupState::Ready)
        }
        RuntimeRegistryStatus::Warming => Some(RuntimeTechnicalFitWarmupState::Warm),
        RuntimeRegistryStatus::Stopped
        | RuntimeRegistryStatus::Stopping
        | RuntimeRegistryStatus::Unhealthy
        | RuntimeRegistryStatus::Failed => Some(RuntimeTechnicalFitWarmupState::Cold),
    }
}

fn uses_factor(request: &RuntimeTechnicalFitRequest, factor: RuntimeTechnicalFitFactor) -> bool {
    request.legal_factors.contains(&factor)
}

fn queue_pressure_applies(request: &RuntimeTechnicalFitRequest) -> bool {
    uses_factor(request, RuntimeTechnicalFitFactor::QueuePressure)
        && request
            .resource_pressure
            .as_ref()
            .and_then(|pressure| pressure.queued_run_count)
            .unwrap_or(0)
            > 0
}

fn budget_pressure_applies(request: &RuntimeTechnicalFitRequest) -> bool {
    uses_factor(request, RuntimeTechnicalFitFactor::BudgetPressure)
        && request.resource_pressure.as_ref().is_some_and(|pressure| {
            pressure.estimated_peak_vram_mb.is_some()
                || pressure.estimated_peak_ram_mb.is_some()
                || pressure
                    .loaded_runtime_count
                    .zip(pressure.loaded_runtime_capacity)
                    .is_some_and(|(count, capacity)| count >= capacity)
        })
}

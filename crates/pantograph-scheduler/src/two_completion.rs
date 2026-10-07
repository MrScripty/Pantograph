//! Pure advisory search for a declared serialized two-task prefix.
//! The caller owns projected capacity and transition-conditioned timing evidence.
//! This module never invents release/retention facts or obtains reservations.
use crate::dispatch_selection_policy::candidate_eligibility;
use crate::{
    completion_diagnostics_bounded, select_scheduler_candidate_with_completion,
    SchedulerCompletionContext, SchedulerCompletionEvidence, SchedulerCompletionEvidenceSource,
    SchedulerCompletionRankingDiagnostic, SchedulerCompletionRankingPolicy,
    SchedulerCompletionRankingResult, SchedulerDispatchCandidate, SchedulerDispatchCandidateId,
    SchedulerDispatchReservationSelection, SchedulerDispatchSelectionDiagnostic,
    SchedulerDispatchSelectionDiagnosticCode, SchedulerResourceFitState, SchedulerTraitSetting,
    SchedulerTraitValue, ValidatedSchedulerDispatchSelectionRequest,
};

pub const SCHEDULER_TWO_COMPLETION_MAX_OFFERS: usize = 4;
pub const SCHEDULER_TWO_COMPLETION_MAX_PLANS: usize = 16;
pub const SCHEDULER_TWO_COMPLETION_MAX_EVENTS: usize = 32;

/// Complete successor offer universe, shared by every first placement. Declares
/// only this serialized prefix; it does not certify the whole workflow/queue.
pub struct SchedulerTwoCompletionPrefix<'a> {
    pub identity: &'a str,
    pub successor_universe: &'a ValidatedSchedulerDispatchSelectionRequest,
}

/// An owner-qualified projection AFTER first completion, cleanup, reservation
/// release and reconciliation. The transition fingerprint must identify those
/// conditions, including producer retention/reload. Every successor offer stays
/// present. Only resource fit may change; Unknown is incomplete, never infeasible.
/// Evidence binds to this exact projected request and names its transition as
/// resource_condition_fingerprint. Matching labels do not authenticate history.
pub struct SchedulerTwoCompletionContinuation<'a> {
    pub first_candidate: &'a SchedulerDispatchCandidate,
    pub first_context: SchedulerCompletionContext<'a>,
    pub transition_fingerprint: &'a str,
    pub projected_request: &'a ValidatedSchedulerDispatchSelectionRequest,
    pub evidence: &'a [SchedulerCompletionEvidence<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct SchedulerTwoCompletionBudget {
    pub max_plans: usize,
    pub max_events: usize,
}
impl Default for SchedulerTwoCompletionBudget {
    fn default() -> Self {
        Self {
            max_plans: SCHEDULER_TWO_COMPLETION_MAX_PLANS,
            max_events: SCHEDULER_TWO_COMPLETION_MAX_EVENTS,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerTwoCompletionFallback {
    InvalidPrefix,
    FirstEvidenceIncomplete,
    MissingOrForeignBranch,
    ChangedSuccessorUniverse,
    UnknownProjectedCapacity,
    ConditionalEvidenceIncomplete,
    NoCompletePlan,
    DurationOverflow,
    BudgetExhausted,
    WorkLimitExceeded,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerTwoCompletionScore {
    pub prefix_identity: String,
    pub successor_candidate_id: SchedulerDispatchCandidateId,
    pub first_completion_us: u64,
    pub terminal_completion_us: u64,
    pub completion_sum_us: u64,
    pub source: SchedulerCompletionEvidenceSource,
    pub plans_evaluated: usize,
    pub completion_events_evaluated: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerTwoCompletionDiagnostic {
    Ranked(SchedulerTwoCompletionScore),
    /// Preserve the exact one-decision selection, including its fallback.
    Fallback(SchedulerTwoCompletionFallback),
    /// Refuse before scanning/cloning an oversized comparison or fallback.
    Refused(SchedulerTwoCompletionFallback),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerTwoCompletionResult {
    pub selection: SchedulerDispatchReservationSelection,
    pub diagnostic: SchedulerTwoCompletionDiagnostic,
}

/// Exhaustively compare <=4x4 legal serialized paths. Primary objective is
/// terminal prefix completion; secondary is the two represented completion sum.
/// Only a first advisory ID is returned. Actual dispatch must revalidate current
/// capacity, Ready/workload versions, dependency proof and cancellation/custody.
/// No partially evaluated population wins; incomplete branches fall back.
/// Bounds cover consumed fields after contract validation, not validation itself.
pub fn select_scheduler_candidate_with_two_completions(
    first: &ValidatedSchedulerDispatchSelectionRequest,
    first_evidence: &[SchedulerCompletionEvidence<'_>],
    prefix: SchedulerTwoCompletionPrefix<'_>,
    continuations: &[SchedulerTwoCompletionContinuation<'_>],
    policy: SchedulerCompletionRankingPolicy,
    budget: SchedulerTwoCompletionBudget,
) -> SchedulerTwoCompletionResult {
    use SchedulerTwoCompletionFallback as Reason;
    if !bounded(first)
        || !bounded(prefix.successor_universe)
        || first_evidence.len() > 4
        || continuations.len() > 4
        || continuations
            .iter()
            .any(|b| !bounded(b.projected_request) || b.evidence.len() > 4)
        || budget.max_plans > 16
        || budget.max_events > 32
    {
        return SchedulerTwoCompletionResult {
            selection: SchedulerDispatchReservationSelection::NoSelection {
                diagnostics: vec![SchedulerDispatchSelectionDiagnostic::error(
                    SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
                    None,
                    "Two-completion comparison exceeded its fixed work bounds.",
                )],
            },
            diagnostic: SchedulerTwoCompletionDiagnostic::Refused(Reason::WorkLimitExceeded),
        };
    }
    let baseline = select_scheduler_candidate_with_completion(first, first_evidence, policy);
    let fallback = |reason| SchedulerTwoCompletionResult {
        selection: baseline.selection.clone(),
        diagnostic: SchedulerTwoCompletionDiagnostic::Fallback(reason),
    };
    let SchedulerCompletionRankingDiagnostic::Ranked { source, .. } = baseline.diagnostic else {
        return fallback(Reason::FirstEvidenceIncomplete);
    };
    let a = &first.as_ref().task_intent;
    let b = &prefix.successor_universe.as_ref().task_intent;
    if !text(prefix.identity)
        || a.workflow_id != b.workflow_id
        || a.workflow_run_id != b.workflow_run_id
        || a.task_id == b.task_id
        || a.model_ref != b.model_ref
        || budget.max_plans == 0
        || budget.max_events == 0
    {
        return fallback(Reason::InvalidPrefix);
    }
    let eligible: Vec<_> = first
        .as_ref()
        .candidates
        .iter()
        .filter(|c| candidate_eligibility(first.as_ref(), c, false).is_ok())
        .collect();
    // Validate ALL branches, including duplicates/extras, before exploring paths.
    if continuations.len() != eligible.len()
        || continuations.iter().enumerate().any(|(i, branch)| {
            !eligible
                .iter()
                .any(|c| std::ptr::eq(*c, branch.first_candidate))
                || continuations[..i]
                    .iter()
                    .any(|old| std::ptr::eq(old.first_candidate, branch.first_candidate))
        })
    {
        return fallback(Reason::MissingOrForeignBranch);
    }
    let mut successor_workload = None;
    let mut plans = 0;
    let mut events = 0;
    let mut winner: Option<(&SchedulerDispatchCandidate, SchedulerTwoCompletionScore)> = None;
    for branch in continuations {
        let row = first_evidence
            .iter()
            .find(|r| std::ptr::eq(r.candidate, branch.first_candidate))
            .expect("baseline checked evidence");
        if row.current_context != branch.first_context || !text(branch.transition_fingerprint) {
            return fallback(Reason::MissingOrForeignBranch);
        }
        if !same_universe(prefix.successor_universe, branch.projected_request) {
            return fallback(Reason::ChangedSuccessorUniverse);
        }
        let projected = branch.projected_request.as_ref();
        // Current capacity is deliberately NOT substituted for conditional facts.
        if projected.candidates.iter().any(|c| {
            c.resource_fit_assessment
                .as_ref()
                .is_none_or(|fit| fit.state == SchedulerResourceFitState::Unknown)
        }) {
            return fallback(Reason::UnknownProjectedCapacity);
        }
        if branch.evidence.iter().any(|r| {
            !r.current_context.valid()
                || !r.sample.context.valid()
                || !std::ptr::eq(r.request, branch.projected_request)
                || r.current_context.host_id != row.current_context.host_id
                || r.current_context.artifact_fingerprint
                    != row.current_context.artifact_fingerprint
                || r.current_context.timing_convention != row.current_context.timing_convention
                || r.current_context.resource_condition_fingerprint != branch.transition_fingerprint
                || r.current_context.workload_fingerprint
                    == row.current_context.workload_fingerprint
                || r.sample.source != source
        }) {
            return fallback(Reason::ConditionalEvidenceIncomplete);
        }
        for evidence in branch.evidence {
            if successor_workload
                .is_some_and(|old| old != evidence.current_context.workload_fingerprint)
            {
                return fallback(Reason::ConditionalEvidenceIncomplete);
            }
            successor_workload = Some(evidence.current_context.workload_fingerprint);
        }
        let next = select_scheduler_candidate_with_completion(
            branch.projected_request,
            branch.evidence,
            policy,
        );
        let next_eligible: Vec<_> = projected
            .candidates
            .iter()
            .filter(|c| candidate_eligibility(projected, c, false).is_ok())
            .collect();
        // A fully qualified impossible continuation is a valid dead-end branch.
        // Otherwise the inner selector must qualify every legal alternative.
        if next_eligible.is_empty() {
            if !branch.evidence.is_empty() {
                return fallback(Reason::ConditionalEvidenceIncomplete);
            }
            continue;
        }
        if !matches!(
            next.diagnostic,
            SchedulerCompletionRankingDiagnostic::Ranked { .. }
        ) {
            return fallback(Reason::ConditionalEvidenceIncomplete);
        }
        let first_cost = cost(row).expect("baseline checked durations");
        for candidate in next_eligible {
            let sample = branch
                .evidence
                .iter()
                .find(|r| std::ptr::eq(r.candidate, candidate))
                .expect("inner selector checked evidence");
            plans += 1;
            events += 2;
            if plans > budget.max_plans || events > budget.max_events {
                return fallback(Reason::BudgetExhausted);
            }
            let Some(terminal) =
                first_cost.checked_add(cost(sample).expect("inner selector checked durations"))
            else {
                return fallback(Reason::DurationOverflow);
            };
            let Some(sum) = first_cost.checked_add(terminal) else {
                return fallback(Reason::DurationOverflow);
            };
            let score = SchedulerTwoCompletionScore {
                prefix_identity: prefix.identity.to_owned(),
                successor_candidate_id: candidate.candidate_id.clone(),
                first_completion_us: first_cost,
                terminal_completion_us: terminal,
                completion_sum_us: sum,
                source,
                plans_evaluated: 0,
                completion_events_evaluated: 0,
            };
            if winner.as_ref().is_none_or(|(old, best)| {
                (
                    terminal,
                    sum,
                    &branch.first_candidate.candidate_id,
                    &candidate.candidate_id,
                ) < (
                    best.terminal_completion_us,
                    best.completion_sum_us,
                    &old.candidate_id,
                    &best.successor_candidate_id,
                )
            }) {
                winner = Some((branch.first_candidate, score));
            }
        }
    }
    let Some((candidate, mut score)) = winner else {
        return fallback(Reason::NoCompletePlan);
    };
    score.plans_evaluated = plans;
    score.completion_events_evaluated = events;
    let SchedulerCompletionRankingResult { mut selection, .. } = baseline;
    if let SchedulerDispatchReservationSelection::Selected {
        candidate_id,
        diagnostics,
    } = &mut selection
    {
        *candidate_id = candidate.candidate_id.clone();
        // The one-task diagnostic would otherwise describe the old winner.
        diagnostics
            .retain(|d| d.code != SchedulerDispatchSelectionDiagnosticCode::CandidateSelected);
        diagnostics.push(SchedulerDispatchSelectionDiagnostic::info(
            SchedulerDispatchSelectionDiagnosticCode::CandidateSelected, Some(candidate_id),
            "Scheduler chose terminal completion of a declared serialized two-task prefix; current commit remains required.",
        ));
    }
    SchedulerTwoCompletionResult {
        selection,
        diagnostic: SchedulerTwoCompletionDiagnostic::Ranked(score),
    }
}
fn cost(row: &SchedulerCompletionEvidence<'_>) -> Option<u64> {
    row.sample
        .preparation_us?
        .checked_add(row.sample.required_transfer_us?)?
        .checked_add(row.sample.execution_us?)
}
fn text(value: &str) -> bool {
    value.len() <= 128 && !value.trim().is_empty() && !value.chars().any(char::is_control)
}
fn traits_bounded(settings: &[SchedulerTraitSetting]) -> bool {
    settings.len() <= 32
        && settings.iter().all(|s| match &s.value {
            SchedulerTraitValue::String(s) => s.len() <= 1024,
            _ => true,
        })
}
fn model_bounded(model: &pantograph_dependency_planning::PumasModelRef) -> bool {
    model.model_id.len() <= 128
        && model.revision.as_ref().is_none_or(|s| s.len() <= 128)
        && model
            .selected_artifact_id
            .as_ref()
            .is_none_or(|s| s.len() <= 128)
        && model.selected_artifact_path.is_none()
        && model.migration_diagnostics.is_empty()
}
fn bounded(request: &ValidatedSchedulerDispatchSelectionRequest) -> bool {
    let input = request.as_ref();
    let intent = &input.task_intent;
    let proof = &input.readiness_proof;
    let preflight = &proof.preflight_result;
    input.candidates.len() <= 4
        && completion_diagnostics_bounded(&input.diagnostics)
        && model_bounded(&intent.model_ref)
        && traits_bounded(&intent.trait_settings)
        && intent.dependency_override_patches.is_empty()
        && intent.estimate_hints.len() <= 32
        && proof.execution_context.selected_binding_ids.len() <= 32
        && preflight.identity_key.selected_binding_ids.len() <= 32
        && model_bounded(&preflight.identity_key.model_ref)
        && preflight.diagnostics.len() <= 32
        && preflight.diagnostics.iter().all(|d| {
            d.message.len() <= 1024
                && d.model_id.as_ref().is_none_or(|s| s.len() <= 128)
                && d.field_path.as_ref().is_none_or(|s| s.len() <= 1024)
        })
        && input.candidates.iter().all(|c| {
            c.selected_device_ids.len() <= 8
                && model_bounded(&c.selected_model_ref)
                && traits_bounded(&c.runtime_trait_settings)
                && c.reservations.is_empty()
                && c.candidate_source_diagnostics.is_empty()
        })
}
fn same_universe(
    base: &ValidatedSchedulerDispatchSelectionRequest,
    projected: &ValidatedSchedulerDispatchSelectionRequest,
) -> bool {
    let base = base.as_ref();
    let projected = projected.as_ref();
    base.task_intent == projected.task_intent
        && base.readiness_proof == projected.readiness_proof
        && base.environment_ref == projected.environment_ref
        && base.candidates.len() == projected.candidates.len()
        && projected.candidates.iter().enumerate().all(|(i, c)| {
            !projected.candidates[..i]
                .iter()
                .any(|other| other.candidate_id == c.candidate_id)
                && base.candidates.iter().any(|old| {
                    old.candidate_id == c.candidate_id
                        && old.selected_runtime_id == c.selected_runtime_id
                        && old.selected_runtime_variant_id == c.selected_runtime_variant_id
                        && old.selected_device_ids == c.selected_device_ids
                        && old.selected_model_ref == c.selected_model_ref
                        && old.runtime_trait_settings == c.runtime_trait_settings
                        && old.batching_group_id == c.batching_group_id
                })
        })
}

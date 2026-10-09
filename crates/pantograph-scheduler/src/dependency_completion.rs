//! An advisory successor forecast deliberately carries no dispatch/Ready proof.
//! Actual successor admission is required after real output and acknowledged cleanup.
use crate::dispatch_selection_policy::{candidate_constraint_eligibility, candidate_eligibility};
use crate::two_completion::{bounded, model_bounded, traits_bounded};
use crate::*;

#[derive(Debug, Clone)]
pub struct SchedulerCompletionSuccessorRequest {
    pub task_intent: SchedulableTaskIntent,
    pub snapshot_identity: String,
    pub candidates: Vec<SchedulerDispatchCandidate>,
}
#[derive(Debug, Clone)]
pub struct ValidatedSchedulerCompletionSuccessor(SchedulerCompletionSuccessorRequest);
impl AsRef<SchedulerCompletionSuccessorRequest> for ValidatedSchedulerCompletionSuccessor {
    fn as_ref(&self) -> &SchedulerCompletionSuccessorRequest {
        &self.0
    }
}
impl TryFrom<SchedulerCompletionSuccessorRequest> for ValidatedSchedulerCompletionSuccessor {
    type Error = SchedulerContractError;
    fn try_from(value: SchedulerCompletionSuccessorRequest) -> Result<Self, Self::Error> {
        if !successor_bounded(&value) {
            return Err(SchedulerContractError::InvalidField {
                field: "completion_successor",
                reason: "advisory snapshot exceeds bounds or contains executable reservations",
            });
        }
        value.task_intent.validate()?;
        for (i, c) in value.candidates.iter().enumerate() {
            c.validate(&value.task_intent)?;
            if value.candidates[..i]
                .iter()
                .any(|old| old.candidate_id == c.candidate_id)
            {
                return Err(SchedulerContractError::InvalidField {
                    field: "completion_successor",
                    reason: "duplicate successor placement",
                });
            }
        }
        Ok(Self(value))
    }
}

/// Exactly one qualified conditional row for every legal first/successor pair.
/// Capacity is hypothetical owner evidence, never an executable reservation.
/// Unknown state or unqualified costs invalidate the entire comparison.
pub struct SchedulerDependencyCompletionEvidence<'a> {
    pub successor: &'a ValidatedSchedulerCompletionSuccessor,
    pub predecessor_candidate: &'a SchedulerDispatchCandidate,
    pub predecessor_context: SchedulerCompletionContext<'a>,
    pub candidate: &'a SchedulerDispatchCandidate,
    pub context: SchedulerCompletionContext<'a>,
    pub transition_fingerprint: &'a str,
    pub resource_fit: &'a SchedulerResourceFitAssessment,
    pub sample: Option<SchedulerCompletionSample<'a>>,
}

pub fn select_scheduler_candidate_with_dependency_completion(
    first: &ValidatedSchedulerDispatchSelectionRequest,
    first_evidence: &[SchedulerCompletionEvidence<'_>],
    successor: &ValidatedSchedulerCompletionSuccessor,
    evidence: &[SchedulerDependencyCompletionEvidence<'_>],
    policy: SchedulerCompletionRankingPolicy,
) -> SchedulerTwoCompletionResult {
    use SchedulerTwoCompletionFallback as R;
    if !bounded(first) || first_evidence.len() > 4 || evidence.len() > 16 {
        return SchedulerTwoCompletionResult {
            selection: SchedulerDispatchReservationSelection::NoSelection {
                diagnostics: vec![SchedulerDispatchSelectionDiagnostic::error(
                    SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
                    None,
                    "Dependency completion forecast exceeded fixed work limits.",
                )],
            },
            diagnostic: SchedulerTwoCompletionDiagnostic::Refused(R::WorkLimitExceeded),
        };
    }
    let baseline = select_scheduler_candidate_with_completion(first, first_evidence, policy);
    let fallback = |reason| SchedulerTwoCompletionResult {
        selection: baseline.selection.clone(),
        diagnostic: SchedulerTwoCompletionDiagnostic::Fallback(reason),
    };
    let SchedulerCompletionRankingDiagnostic::Ranked { source, .. } = baseline.diagnostic else {
        return fallback(R::FirstEvidenceIncomplete);
    };
    let future = successor.as_ref();
    let current = &first.as_ref().task_intent;
    if current.workflow_id != future.task_intent.workflow_id
        || current.workflow_run_id != future.task_intent.workflow_run_id
        || current.task_id == future.task_intent.task_id
        || current.model_ref != future.task_intent.model_ref
    {
        return fallback(R::InvalidPrefix);
    }
    let firsts: Vec<_> = first
        .as_ref()
        .candidates
        .iter()
        .filter(|c| candidate_eligibility(first.as_ref(), c, false).is_ok())
        .collect();
    let nexts: Vec<_> = future
        .candidates
        .iter()
        .filter(|c| candidate_constraint_eligibility(&future.task_intent, c).is_ok())
        .collect();
    if evidence.len() != firsts.len() * nexts.len() {
        return fallback(R::MissingOrForeignBranch);
    }
    // Prevalidate ALL associations/contexts before equality or scoring.
    for (i, row) in evidence.iter().enumerate() {
        if !fit_bounded(row.resource_fit)
            || !row.context.valid()
            || !row.predecessor_context.valid()
            || !text(row.transition_fingerprint)
            || !std::ptr::eq(row.successor, successor)
            || !firsts
                .iter()
                .any(|c| std::ptr::eq(*c, row.predecessor_candidate))
            || !nexts.iter().any(|c| std::ptr::eq(*c, row.candidate))
            || evidence[..i].iter().any(|old| {
                std::ptr::eq(old.predecessor_candidate, row.predecessor_candidate)
                    && std::ptr::eq(old.candidate, row.candidate)
            })
        {
            return fallback(R::MissingOrForeignBranch);
        }
    }
    let mut plans = 0;
    let mut winner: Option<(&SchedulerDispatchCandidate, SchedulerTwoCompletionScore)> = None;
    for row in evidence {
        let first_row = first_evidence
            .iter()
            .find(|e| std::ptr::eq(e.candidate, row.predecessor_candidate))
            .expect("baseline qualified all first offers");
        if evidence.iter().any(|other| {
            std::ptr::eq(other.predecessor_candidate, row.predecessor_candidate)
                && other.transition_fingerprint != row.transition_fingerprint
        }) {
            return fallback(R::ConditionalEvidenceIncomplete);
        }
        if first_row.current_context != row.predecessor_context
            || row.context.host_id != first_row.current_context.host_id
            || row.context.artifact_fingerprint != first_row.current_context.artifact_fingerprint
            || row.context.timing_convention != first_row.current_context.timing_convention
            || row.context.workload_fingerprint != future.snapshot_identity
            || row.context.resource_condition_fingerprint != row.transition_fingerprint
            || row.resource_fit.workflow_run_id != future.task_intent.workflow_run_id
            || row.resource_fit.task_id != future.task_intent.task_id
        {
            return fallback(R::ConditionalEvidenceIncomplete);
        }
        match row.resource_fit.state {
            SchedulerResourceFitState::Unknown => return fallback(R::UnknownProjectedCapacity),
            SchedulerResourceFitState::WaitingForResources
            | SchedulerResourceFitState::ImpossibleFit => {
                if row.sample.is_some() {
                    return fallback(R::ConditionalEvidenceIncomplete);
                }
                continue;
            }
            SchedulerResourceFitState::Fits => {}
        }
        if row.candidate.selected_device_ids.len() != 1 || row.candidate.batching_group_id.is_some()
        {
            return fallback(R::ConditionalEvidenceIncomplete);
        }
        let Some(sample) = row.sample else {
            return fallback(R::ConditionalEvidenceIncomplete);
        };
        if !sample.context.valid()
            || !std::ptr::eq(sample.candidate, row.candidate)
            || sample.context != row.context
            || sample.source != source
            || sample.sample_count < policy.minimum_samples
            || (source == SchedulerCompletionEvidenceSource::Synthetic && !policy.allow_synthetic)
            || policy
                .now_ms
                .checked_sub(sample.observed_at_ms)
                .is_none_or(|age| age > policy.max_sample_age_ms)
        {
            return fallback(R::ConditionalEvidenceIncomplete);
        }
        let Some(next_cost) = stages(sample) else {
            return fallback(R::ConditionalEvidenceIncomplete);
        };
        let first_cost = stages(first_row.sample).expect("baseline qualified duration");
        let Some(terminal) = first_cost.checked_add(next_cost) else {
            return fallback(R::DurationOverflow);
        };
        let Some(sum) = first_cost.checked_add(terminal) else {
            return fallback(R::DurationOverflow);
        };
        plans += 1;
        let score = SchedulerTwoCompletionScore {
            prefix_identity: future.snapshot_identity.clone(),
            successor_candidate_id: row.candidate.candidate_id.clone(),
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
                &row.predecessor_candidate.candidate_id,
                &row.candidate.candidate_id,
            ) < (
                best.terminal_completion_us,
                best.completion_sum_us,
                &old.candidate_id,
                &best.successor_candidate_id,
            )
        }) {
            winner = Some((row.predecessor_candidate, score));
        }
    }
    let Some((chosen, mut score)) = winner else {
        return fallback(R::NoCompletePlan);
    };
    score.plans_evaluated = plans;
    score.completion_events_evaluated = plans * 2;
    let mut selection = baseline.selection;
    if let SchedulerDispatchReservationSelection::Selected {
        candidate_id,
        diagnostics,
    } = &mut selection
    {
        *candidate_id = chosen.candidate_id.clone();
        diagnostics
            .retain(|d| d.code != SchedulerDispatchSelectionDiagnosticCode::CandidateSelected);
        diagnostics.push(SchedulerDispatchSelectionDiagnostic::info(SchedulerDispatchSelectionDiagnosticCode::CandidateSelected, Some(candidate_id), "Scheduler chose completion of an owned dependency prefix; successor still requires real outputs, cleanup and admission."));
    }
    SchedulerTwoCompletionResult {
        selection,
        diagnostic: SchedulerTwoCompletionDiagnostic::Ranked(score),
    }
}
fn text(s: &str) -> bool {
    s.len() <= 128 && !s.trim().is_empty() && !s.chars().any(char::is_control)
}
fn stages(s: SchedulerCompletionSample<'_>) -> Option<u64> {
    s.preparation_us?
        .checked_add(s.required_transfer_us?)?
        .checked_add(s.execution_us?)
}
fn successor_bounded(s: &SchedulerCompletionSuccessorRequest) -> bool {
    text(&s.snapshot_identity)
        && s.candidates.len() <= 4
        && model_bounded(&s.task_intent.model_ref)
        && traits_bounded(&s.task_intent.trait_settings)
        && s.task_intent.dependency_override_patches.is_empty()
        && s.task_intent.estimate_hints.len() <= 32
        && s.candidates.iter().all(|c| {
            c.selected_device_ids.len() <= 8
                && model_bounded(&c.selected_model_ref)
                && traits_bounded(&c.runtime_trait_settings)
                && c.reservations.is_empty()
                && c.candidate_source_diagnostics.is_empty()
                && c.resource_fit_assessment.as_ref().is_none_or(fit_bounded)
        })
}

fn fit_bounded(f: &SchedulerResourceFitAssessment) -> bool {
    f.diagnostics.len() <= 32
        && f.diagnostics
            .iter()
            .all(|d| d.message.len() <= 1024 && d.hint.as_ref().is_none_or(|s| s.len() <= 1024))
}

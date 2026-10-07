//! Opt-in earliest completion for one serialized, single-device Ready task.
//! No leases, runtime calls, wall clock reads, search or persistent profile store.

use crate::dispatch_selection_policy::candidate_eligibility;
use crate::{
    select_scheduler_candidate_for_reservation, SchedulerDispatchCandidate,
    SchedulerDispatchReservationSelection, SchedulerDispatchSelectionDiagnostic,
    SchedulerDispatchSelectionDiagnosticCode, ValidatedSchedulerDispatchSelectionRequest,
};

pub const SCHEDULER_COMPLETION_MAX_CANDIDATES: usize = 64;
const MAX_DEVICES: usize = 8;
const MAX_DIAGNOSTICS: usize = 32;
const MAX_CONTEXT_TEXT_BYTES: usize = 128;

/// Caller-supplied comparison conditions, never inferred from model size or warmth.
/// The profile producer must verify these against runtime/artifact/workload facts.
/// Matching strings alone does not authenticate a historical sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerCompletionContext<'a> {
    pub host_id: &'a str,
    pub runtime_instance_id: &'a str,
    pub artifact_fingerprint: &'a str,
    pub workload_fingerprint: &'a str,
    pub resource_condition_fingerprint: &'a str,
    pub residency_fingerprint: &'a str,
    /// Identical units, aggregation (e.g. mean), version and serialized stage scope.
    pub timing_convention: &'a str,
}

impl SchedulerCompletionContext<'_> {
    fn valid(&self) -> bool {
        [
            self.host_id,
            self.runtime_instance_id,
            self.artifact_fingerprint,
            self.workload_fingerprint,
            self.resource_condition_fingerprint,
            self.residency_fingerprint,
            self.timing_convention,
        ]
        .into_iter()
        .all(|text| {
            text.len() <= MAX_CONTEXT_TEXT_BYTES
                && !text.trim().is_empty()
                && !text.chars().any(char::is_control)
        })
    }

    fn comparable(&self, other: &Self) -> bool {
        self.host_id == other.host_id
            && self.workload_fingerprint == other.workload_fingerprint
            && self.resource_condition_fingerprint == other.resource_condition_fingerprint
            && self.timing_convention == other.timing_convention
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerCompletionEvidenceSource {
    Measured,
    Synthetic,
}

#[derive(Debug, Clone, Copy)]
pub struct SchedulerCompletionSample<'a> {
    /// Exact current immutable offer to which a trusted producer bound the sample.
    pub candidate: &'a SchedulerDispatchCandidate,
    pub context: SchedulerCompletionContext<'a>,
    pub source: SchedulerCompletionEvidenceSource,
    /// Successful complete samples covering all three stages under this convention.
    pub sample_count: u32,
    /// Same clock domain as policy.now_ms. No clock is read by the policy.
    pub observed_at_ms: u64,
    /// Includes all required serialized preparation/load/warmup/owner delay.
    pub preparation_us: Option<u64>,
    pub required_transfer_us: Option<u64>,
    pub execution_us: Option<u64>,
}

/// Ephemeral binding to an immutable decision snapshot. Cloned or old requests
/// and offers are deliberately rejected, even if structurally equal. This is
/// local snapshot association, not proof of historical profile authenticity.
/// Rebinding profiles requires a trusted producer, which this crate does not add.
#[derive(Debug, Clone, Copy)]
pub struct SchedulerCompletionEvidence<'a> {
    pub request: &'a ValidatedSchedulerDispatchSelectionRequest,
    pub candidate: &'a SchedulerDispatchCandidate,
    pub current_context: SchedulerCompletionContext<'a>,
    pub sample: SchedulerCompletionSample<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct SchedulerCompletionRankingPolicy {
    pub now_ms: u64,
    pub max_sample_age_ms: u64,
    pub minimum_samples: u32,
    /// Synthetic fixtures require explicit opt-in; source remains in the result.
    pub allow_synthetic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerCompletionRefusalReason {
    WorkLimitExceeded,
    InvalidPolicy,
    IneligibleOrDuplicateCandidates,
    MissingEvidence,
    ForeignSnapshot,
    DuplicateEvidence,
    InvalidContext,
    IdentityMismatch,
    IncomparableEvidence,
    SyntheticEvidenceDisabled,
    InsufficientSamples,
    StaleOrFutureSample,
    MissingStage,
    DurationOverflow,
    UnsupportedExecutionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerCompletionRankingDiagnostic {
    Ranked {
        predicted_completion_us: u64,
        source: SchedulerCompletionEvidenceSource,
        eligible_candidates: usize,
    },
    /// Existing sole-eligible/otherwise-ambiguous selection is retained.
    Fallback(SchedulerCompletionRefusalReason),
    /// Input exceeds fixed work bounds; even the legacy scan is refused.
    Refused(SchedulerCompletionRefusalReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerCompletionRankingResult {
    pub selection: SchedulerDispatchReservationSelection,
    pub diagnostic: SchedulerCompletionRankingDiagnostic,
}

/// Rank all eligible alternatives by checked serialized completion cost.
///
/// Bounds apply after request contract validation, which remains the caller's
/// responsibility: at most 64 offers/evidence rows, 8 devices per offer, 32
/// request diagnostics and 128 bytes per context field. At most 4096 membership
/// comparisons per pass; no recursive search, partial-population truncation or
/// elapsed-time-dependent decisions. Invalid timing never eliminates an eligible
/// alternative. Only the ordinary eligibility rules can eliminate an offer.
///
/// A selected ID is advisory. Commit the selected observation against current
/// capacity/instance/owner state, then use ordinary dispatch validation. This
/// function does not acquire or release capacity or alter session/output custody.
pub fn select_scheduler_candidate_with_completion(
    request: &ValidatedSchedulerDispatchSelectionRequest,
    evidence: &[SchedulerCompletionEvidence<'_>],
    policy: SchedulerCompletionRankingPolicy,
) -> SchedulerCompletionRankingResult {
    use SchedulerCompletionRefusalReason as Reason;
    let input = request.as_ref();
    if input.candidates.len() > SCHEDULER_COMPLETION_MAX_CANDIDATES
        || evidence.len() > SCHEDULER_COMPLETION_MAX_CANDIDATES
        || input.diagnostics.len() > MAX_DIAGNOSTICS
        || input
            .candidates
            .iter()
            .any(|c| c.selected_device_ids.len() > MAX_DEVICES)
    {
        return SchedulerCompletionRankingResult {
            selection: SchedulerDispatchReservationSelection::NoSelection {
                diagnostics: vec![SchedulerDispatchSelectionDiagnostic::error(
                    SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
                    None,
                    "Completion ranking refused input exceeding its fixed work bounds.",
                )],
            },
            diagnostic: SchedulerCompletionRankingDiagnostic::Refused(Reason::WorkLimitExceeded),
        };
    }
    let fallback = |reason| SchedulerCompletionRankingResult {
        selection: select_scheduler_candidate_for_reservation(request),
        diagnostic: SchedulerCompletionRankingDiagnostic::Fallback(reason),
    };
    if policy.max_sample_age_ms == 0 || policy.minimum_samples == 0 {
        return fallback(Reason::InvalidPolicy);
    }
    // Do not resolve duplicate offer IDs using timing evidence.
    for (index, candidate) in input.candidates.iter().enumerate() {
        if input.candidates[..index]
            .iter()
            .any(|other| other.candidate_id == candidate.candidate_id)
        {
            return fallback(Reason::IneligibleOrDuplicateCandidates);
        }
    }
    let mut diagnostics = input.diagnostics.clone();
    let mut eligible = Vec::new();
    for candidate in &input.candidates {
        match candidate_eligibility(input, candidate, false) {
            Ok(()) => eligible.push(candidate),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    if eligible.is_empty() {
        return fallback(Reason::IneligibleOrDuplicateCandidates);
    }
    // Inspect every supplied row, including extra rows, before selecting anything.
    for (index, row) in evidence.iter().enumerate() {
        if !std::ptr::eq(row.request, request)
            || !input
                .candidates
                .iter()
                .any(|candidate| std::ptr::eq(row.candidate, candidate))
            || !std::ptr::eq(row.sample.candidate, row.candidate)
        {
            return fallback(Reason::ForeignSnapshot);
        }
        if evidence[..index]
            .iter()
            .any(|other| std::ptr::eq(row.candidate, other.candidate))
        {
            return fallback(Reason::DuplicateEvidence);
        }
    }
    let mut comparison = None;
    let mut winner: Option<(&SchedulerDispatchCandidate, u64)> = None;
    for candidate in &eligible {
        if candidate.selected_device_ids.len() != 1 || candidate.batching_group_id.is_some() {
            return fallback(Reason::UnsupportedExecutionMode);
        }
        let Some(row) = evidence
            .iter()
            .find(|row| std::ptr::eq(row.candidate, *candidate))
        else {
            return fallback(Reason::MissingEvidence);
        };
        let sample = row.sample;
        if !row.current_context.valid() || !sample.context.valid() {
            return fallback(Reason::InvalidContext);
        }
        if row.current_context != sample.context {
            return fallback(Reason::IdentityMismatch);
        }
        if let Some((context, source)) = comparison {
            if !row.current_context.comparable(&context) || source != sample.source {
                return fallback(Reason::IncomparableEvidence);
            }
        } else {
            comparison = Some((row.current_context, sample.source));
        }
        if sample.source == SchedulerCompletionEvidenceSource::Synthetic && !policy.allow_synthetic
        {
            return fallback(Reason::SyntheticEvidenceDisabled);
        }
        if sample.sample_count < policy.minimum_samples {
            return fallback(Reason::InsufficientSamples);
        }
        if policy
            .now_ms
            .checked_sub(sample.observed_at_ms)
            .is_none_or(|age| age > policy.max_sample_age_ms)
        {
            return fallback(Reason::StaleOrFutureSample);
        }
        let (Some(preparation), Some(transfer), Some(execution)) = (
            sample.preparation_us,
            sample.required_transfer_us,
            sample.execution_us,
        ) else {
            return fallback(Reason::MissingStage);
        };
        let Some(total) = preparation
            .checked_add(transfer)
            .and_then(|value| value.checked_add(execution))
        else {
            return fallback(Reason::DurationOverflow);
        };
        if winner.is_none_or(|(best, cost)| {
            (total, &candidate.candidate_id) < (cost, &best.candidate_id)
        }) {
            winner = Some((candidate, total));
        }
    }
    let (candidate, total) = winner.expect("eligible population is nonempty");
    diagnostics.push(SchedulerDispatchSelectionDiagnostic::info(
        SchedulerDispatchSelectionDiagnosticCode::CandidateSelected,
        Some(&candidate.candidate_id),
        "Scheduler chose earliest predicted serialized completion; lease commit is required.",
    ));
    SchedulerCompletionRankingResult {
        selection: SchedulerDispatchReservationSelection::Selected {
            candidate_id: candidate.candidate_id.clone(),
            diagnostics,
        },
        diagnostic: SchedulerCompletionRankingDiagnostic::Ranked {
            predicted_completion_us: total,
            source: comparison.expect("eligible evidence checked").1,
            eligible_candidates: eligible.len(),
        },
    }
}

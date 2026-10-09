//! Pure comparable-history quantiles; no live instance, residual, lease or store.

use crate::empirical_timing::{estimate_kernel, KernelContext};
use crate::{
    SchedulerCompletionEvidenceSource, SchedulerCompletionRankingPolicy,
    SchedulerEmpiricalQuantile, SchedulerEmpiricalServiceBudget,
    SchedulerEmpiricalServiceCondition, SchedulerEmpiricalServiceIncomplete,
    SchedulerEmpiricalServiceOutcome, SchedulerEmpiricalServiceWork,
};

/// Whole custody-through-worker-drain totals under a comparable history key.
/// The interval scope matches selected text; the context contract is separate
/// from legacy exact-instance evidence and the six-stage convention.
pub const SCHEDULER_EMPIRICAL_HISTORY_TOTAL_CONVENTION: &str =
    "selected-text-comparable-history-custody-through-worker-drain-total-us.v1";

/// Trusted producer's stable comparison conditions within one owner/clock domain.
/// This has no runtime-instance field and cannot bind a live execution or lease.
/// The joint fingerprint must cover actual content, implementation, configuration,
/// CPU domain and exact workload. Matching labels alone authenticate nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerEmpiricalHistoryContext<'a> {
    pub owner_epoch: &'a str,
    pub identity_fingerprint: &'a str,
    pub clock_epoch: &'a str,
    pub residency_fingerprint: &'a str,
    pub timing_convention: &'a str,
}
impl SchedulerEmpiricalHistoryContext<'_> {
    pub(crate) fn valid(&self) -> bool {
        [
            self.owner_epoch,
            self.clock_epoch,
            self.residency_fingerprint,
            self.timing_convention,
        ]
        .into_iter()
        .all(|text| {
            text.len() <= 128 && !text.trim().is_empty() && !text.chars().any(char::is_control)
        }) && self.identity_fingerprint.len() == 64
            && self
                .identity_fingerprint
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}

/// One individually qualified, acknowledged-drain whole interval. The producer
/// retains and checks the raw instance and load/drain fence before projection;
/// those custody proofs are not erased from its raw observations.
#[derive(Debug, Clone, Copy)]
pub struct SchedulerEmpiricalHistoryTotalObservation<'a> {
    pub attempt_id: &'a str,
    pub context: SchedulerEmpiricalHistoryContext<'a>,
    pub source: SchedulerCompletionEvidenceSource,
    pub observed_at_ms: u64,
    pub outcome: SchedulerEmpiricalServiceOutcome,
    pub duration_us: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerEmpiricalHistoryServiceEstimate<'a> {
    pub context: SchedulerEmpiricalHistoryContext<'a>,
    pub source: SchedulerCompletionEvidenceSource,
    pub quantile: SchedulerEmpiricalQuantile,
    pub duration_us: u64,
    pub total_observations: usize,
    pub support_count: usize,
    pub rank: usize,
    pub work: SchedulerEmpiricalServiceWork,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerEmpiricalHistoryServiceResult<'a> {
    Estimated(SchedulerEmpiricalHistoryServiceEstimate<'a>),
    Incomplete {
        reason: SchedulerEmpiricalServiceIncomplete,
        work: SchedulerEmpiricalServiceWork,
    },
}

/// Exact nearest-rank, success-conditioned not-started quantile. Every supplied
/// row must qualify; failures/censoring are refused rather than filtered out.
/// Existing 128-row/32,768-unit limits and metered kernel apply. No live elapsed
/// origin, running residual, prediction authority or default selection is added.
pub fn estimate_scheduler_empirical_history_total_duration<'a>(
    current_context: SchedulerEmpiricalHistoryContext<'a>,
    observations: &[SchedulerEmpiricalHistoryTotalObservation<'_>],
    quantile: SchedulerEmpiricalQuantile,
    policy: SchedulerCompletionRankingPolicy,
    budget: SchedulerEmpiricalServiceBudget,
) -> SchedulerEmpiricalHistoryServiceResult<'a> {
    match estimate_kernel(
        KernelContext::History(current_context),
        observations,
        quantile,
        SchedulerEmpiricalServiceCondition::NotStarted,
        policy,
        budget,
        SCHEDULER_EMPIRICAL_HISTORY_TOTAL_CONVENTION,
    ) {
        Ok(value) => SchedulerEmpiricalHistoryServiceResult::Estimated(
            SchedulerEmpiricalHistoryServiceEstimate {
                context: current_context,
                source: value.source,
                quantile,
                duration_us: value.duration_us,
                total_observations: observations.len(),
                support_count: value.support_count,
                rank: value.rank,
                work: value.work,
            },
        ),
        Err((reason, work)) => SchedulerEmpiricalHistoryServiceResult::Incomplete { reason, work },
    }
}

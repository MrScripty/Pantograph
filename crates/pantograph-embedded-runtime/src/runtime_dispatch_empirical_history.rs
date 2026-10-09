//! Pure bridge for individually fenced, comparable selected-text history.
//! No recorder/store, running-origin binding, runtime call or selector is added.

use crate::SelectedTextEmpiricalOutcomeCounts;
use inference::{
    RuntimeServiceTimingAttempt, RuntimeServiceTimingClockSnapshot,
    RuntimeServiceTimingHistoryProfile, RuntimeServiceTimingLoadDisposition as Load,
    RuntimeServiceTimingOutcome, RuntimeServiceTimingOwnerProvenance,
    RuntimeServiceTimingTermination as Termination,
};
use pantograph_scheduler::{
    estimate_scheduler_empirical_history_total_duration, SchedulerCompletionEvidenceSource,
    SchedulerCompletionRankingPolicy, SchedulerEmpiricalHistoryContext,
    SchedulerEmpiricalHistoryServiceResult, SchedulerEmpiricalHistoryTotalObservation,
    SchedulerEmpiricalQuantile, SchedulerEmpiricalServiceBudget,
    SchedulerEmpiricalServiceIncomplete as Reason, SchedulerEmpiricalServiceOutcome as Outcome,
    SCHEDULER_EMPIRICAL_HISTORY_TOTAL_CONVENTION, SCHEDULER_EMPIRICAL_MAX_OBSERVATIONS,
    SCHEDULER_EMPIRICAL_MAX_WORK,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedTextEmpiricalHistoryServiceReport<'a> {
    pub records_examined: usize,
    pub outcomes: SelectedTextEmpiricalOutcomeCounts,
    pub estimate: SchedulerEmpiricalHistoryServiceResult<'a>,
}

/// Strict built-in history proof for EVERY supplied attempt, including failed or
/// censored rows; no successful-only filtering. Each row keeps its original raw
/// instance and native load/drain fence. Different genuine reload instances may
/// share a stable history key, but mixed reload/reuse, owner/clock, content,
/// configuration, workload and CPU-domain conditions refuse comparison.
///
/// Actual capture and drain age are checked in ns before projecting drain to ms;
/// the full interval is rounded up to us once. Results are not-started successful
/// empirical quantiles only, never live predictions or failure-risk/coverage
/// claims. The best-effort recorder can omit any outcome under saturation.
///
/// Existing hard 128-row/32,768-unit guards precede all row work. Prequalification
/// has a separate fixed 128-row/four-phase/bounded-metadata limit; returned work
/// counters describe only the shared quantile kernel, as in the legacy bridge.
pub fn estimate_selected_text_empirical_history_duration<'a>(
    profile: &'a RuntimeServiceTimingHistoryProfile,
    clock: &'a RuntimeServiceTimingClockSnapshot,
    attempts: &[RuntimeServiceTimingAttempt],
    quantile: SchedulerEmpiricalQuantile,
    mut policy: SchedulerCompletionRankingPolicy,
    budget: SchedulerEmpiricalServiceBudget,
) -> SelectedTextEmpiricalHistoryServiceReport<'a> {
    let incomplete = |reason| SelectedTextEmpiricalHistoryServiceReport {
        records_examined: 0,
        outcomes: SelectedTextEmpiricalOutcomeCounts::default(),
        estimate: SchedulerEmpiricalHistoryServiceResult::Incomplete {
            reason,
            work: Default::default(),
        },
    };
    if attempts.len() > SCHEDULER_EMPIRICAL_MAX_OBSERVATIONS
        || budget.max_work_units > SCHEDULER_EMPIRICAL_MAX_WORK
    {
        return incomplete(Reason::WorkLimitExceeded);
    }
    let Some(max_age_ns) = policy.max_sample_age_ms.checked_mul(1_000_000) else {
        return incomplete(Reason::InvalidPolicy);
    };
    policy.now_ms = clock.now_ns / 1_000_000;
    let current_context = context(
        Some(profile),
        &clock.clock_epoch,
        attempts.first().map(residency).unwrap_or("unknown"),
    );
    let mut outcomes = SelectedTextEmpiricalOutcomeCounts::default();
    let mut samples = Vec::with_capacity(attempts.len());
    for row in attempts {
        let termination = row.lifecycle.as_ref().map(|l| l.termination);
        match termination {
            Some(Termination::Completed) => outcomes.completed += 1,
            Some(Termination::Failed) => outcomes.failed += 1,
            Some(Termination::CancellationRequested) => outcomes.cancellation_requested += 1,
            Some(Termination::ShutdownRequested) => outcomes.shutdown_requested += 1,
            Some(Termination::Abandoned) => outcomes.abandoned += 1,
            None => outcomes.legacy_unknown += 1,
        }
        let outcome = match (row.outcome, termination) {
            (RuntimeServiceTimingOutcome::Abandoned, _) | (_, Some(Termination::Abandoned)) => {
                Outcome::Incomplete
            }
            (_, Some(Termination::CancellationRequested | Termination::ShutdownRequested)) => {
                Outcome::Cancelled
            }
            (RuntimeServiceTimingOutcome::Failed, _) | (_, Some(Termination::Failed)) => {
                Outcome::Failed
            }
            _ => Outcome::Completed,
        };
        let source = if row
            .capture
            .as_ref()
            .is_some_and(|c| c.owner_provenance == RuntimeServiceTimingOwnerProvenance::BuiltIn)
        {
            SchedulerCompletionEvidenceSource::Measured
        } else {
            SchedulerCompletionEvidenceSource::Synthetic
        };
        let row_context = context(
            row.history.as_ref().map(|h| &h.profile),
            row.capture
                .as_ref()
                .map(|c| c.clock_epoch.as_str())
                .unwrap_or(""),
            residency(row),
        );
        samples.push(SchedulerEmpiricalHistoryTotalObservation {
            attempt_id: &row.attempt_id,
            context: row_context,
            source,
            observed_at_ms: row
                .lifecycle
                .as_ref()
                .and_then(|l| l.interval.as_ref())
                .and_then(|i| i.worker_drained_at_ns)
                .map(|ns| ns / 1_000_000)
                .unwrap_or(0),
            outcome,
            duration_us: row
                .fresh_production_history_interval_ns(profile, clock, max_age_ns)
                .map(|ns| ns.div_ceil(1_000)),
        });
    }
    SelectedTextEmpiricalHistoryServiceReport {
        records_examined: attempts.len(),
        outcomes,
        estimate: estimate_scheduler_empirical_history_total_duration(
            current_context,
            &samples,
            quantile,
            policy,
            budget,
        ),
    }
}

fn residency(row: &RuntimeServiceTimingAttempt) -> &'static str {
    match row.capture.as_ref().map(|c| c.load_disposition) {
        Some(Load::Reloaded) => "reloaded",
        Some(Load::Reused) => "reused",
        _ => "unknown",
    }
}
fn context<'a>(
    profile: Option<&'a RuntimeServiceTimingHistoryProfile>,
    clock_epoch: &'a str,
    residency: &'static str,
) -> SchedulerEmpiricalHistoryContext<'a> {
    // Absent facts remain invalid fields/missing duration. Never invent a stable
    // runtime-instance label or change the original raw custody evidence.
    SchedulerEmpiricalHistoryContext {
        owner_epoch: profile.map(|p| p.owner_epoch()).unwrap_or(""),
        identity_fingerprint: profile.map(|p| p.identity_fingerprint()).unwrap_or(""),
        clock_epoch,
        residency_fingerprint: residency,
        timing_convention: SCHEDULER_EMPIRICAL_HISTORY_TOTAL_CONVENTION,
    }
}

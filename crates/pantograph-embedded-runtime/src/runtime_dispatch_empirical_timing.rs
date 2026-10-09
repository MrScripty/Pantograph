//! Pure bridge from existing opt-in selected-text observations. No selector,
//! recorder/store, runtime lease or phase observer is created by this helper.

use inference::{
    RuntimeServiceTimingAttempt, RuntimeServiceTimingClockSnapshot, RuntimeServiceTimingIdentity,
    RuntimeServiceTimingLoadDisposition as Load, RuntimeServiceTimingOutcome,
    RuntimeServiceTimingOwnerProvenance, RuntimeServiceTimingProfile,
    RuntimeServiceTimingTermination as Termination,
};
use pantograph_scheduler::{
    estimate_scheduler_empirical_service_total_duration, SchedulerCompletionContext,
    SchedulerCompletionEvidenceSource, SchedulerCompletionRankingPolicy,
    SchedulerEmpiricalQuantile, SchedulerEmpiricalServiceBudget,
    SchedulerEmpiricalServiceCondition, SchedulerEmpiricalServiceIncomplete as Reason,
    SchedulerEmpiricalServiceOutcome as Outcome, SchedulerEmpiricalServiceResult,
    SchedulerEmpiricalServiceTotalObservation, SCHEDULER_EMPIRICAL_MAX_OBSERVATIONS,
    SCHEDULER_EMPIRICAL_MAX_WORK, SCHEDULER_EMPIRICAL_SERVICE_TOTAL_CONVENTION,
};

/// Counts of termination labels in the supplied window, not population rates.
/// The recorder remains best-effort; saturation can omit any outcome. Keep raw
/// attempts/failures/censoring separately; these counts do not imply coverage.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SelectedTextEmpiricalOutcomeCounts {
    pub completed: usize,
    pub failed: usize,
    pub cancellation_requested: usize,
    pub shutdown_requested: usize,
    pub abandoned: usize,
    pub legacy_unknown: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedTextEmpiricalServiceReport<'a> {
    pub records_examined: usize,
    pub outcomes: SelectedTextEmpiricalOutcomeCounts,
    pub estimate: SchedulerEmpiricalServiceResult<'a>,
}

/// Every supplied row is retained for qualification; no success-only filtering.
/// Failed/canceled/abandoned rows refuse the window's estimate. The bounded raw
/// recorder still receives partial observations independently of this helper.
///
/// Only exact built-in owner evidence qualifies. Unknown native owner facts,
/// per-load generation changes, missing legacy intervals and injected fixtures
/// cannot authorize production history. Caller association is trusted in-process.
/// Capture AND worker-drain freshness are checked in nanoseconds BEFORE
/// projecting the actual drain timestamp to ms; a
/// total is rounded upward to us once, never by summing rounded phase durations.
/// No running residual is exposed: live elapsed/origin association is unqualified.
///
/// Hard guards precede all row work. Prequalification inspects at most128 rows
/// with exactly4 phases and bounded clock/profile text; returned work counters
/// cover the subsequent shared quantile kernel, not this fixed prequalification.
pub fn estimate_selected_text_empirical_service_duration<'a>(
    profile: &'a RuntimeServiceTimingProfile,
    clock: &RuntimeServiceTimingClockSnapshot,
    attempts: &[RuntimeServiceTimingAttempt],
    quantile: SchedulerEmpiricalQuantile,
    mut policy: SchedulerCompletionRankingPolicy,
    budget: SchedulerEmpiricalServiceBudget,
) -> SelectedTextEmpiricalServiceReport<'a> {
    let incomplete = |reason| SelectedTextEmpiricalServiceReport {
        records_examined: 0,
        outcomes: SelectedTextEmpiricalOutcomeCounts::default(),
        estimate: SchedulerEmpiricalServiceResult::Incomplete {
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
    let residency = attempts.first().map(residency).unwrap_or("unknown");
    let current_context = context(profile, residency);
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
        let row_profile = match &row.identity {
            RuntimeServiceTimingIdentity::Exact { profile } => profile,
            RuntimeServiceTimingIdentity::Unknown { .. } => profile,
        };
        samples.push(SchedulerEmpiricalServiceTotalObservation {
            attempt_id: &row.attempt_id,
            context: context(row_profile, self::residency(row)),
            source,
            observed_at_ms: drain(row).map(|ns| ns / 1_000_000).unwrap_or(0),
            outcome,
            duration_us: row
                .fresh_production_service_interval_ns(profile, clock, max_age_ns)
                .filter(|_| {
                    drain(row)
                        .and_then(|ns| clock.now_ns.checked_sub(ns))
                        .is_some_and(|age| age <= max_age_ns)
                })
                .map(|ns| ns.div_ceil(1_000)),
        });
    }
    SelectedTextEmpiricalServiceReport {
        records_examined: attempts.len(),
        outcomes,
        estimate: estimate_scheduler_empirical_service_total_duration(
            current_context,
            &samples,
            quantile,
            SchedulerEmpiricalServiceCondition::NotStarted,
            policy,
            budget,
        ),
    }
}

fn drain(row: &RuntimeServiceTimingAttempt) -> Option<u64> {
    row.lifecycle
        .as_ref()?
        .interval
        .as_ref()?
        .worker_drained_at_ns
}

fn residency(row: &RuntimeServiceTimingAttempt) -> &'static str {
    match row.capture.as_ref().map(|c| c.load_disposition) {
        Some(Load::Reloaded) => "reloaded",
        Some(Load::Reused) => "reused",
        _ => "unknown",
    }
}
fn context<'a>(
    profile: &'a RuntimeServiceTimingProfile,
    residency: &'static str,
) -> SchedulerCompletionContext<'a> {
    // The existing joint digest binds all three identities together. It cannot
    // support independent hardware/workload/artifact substitution or matching.
    SchedulerCompletionContext {
        host_id: profile.owner_epoch(),
        runtime_instance_id: profile.runtime_instance_id(),
        artifact_fingerprint: profile.identity_fingerprint(),
        workload_fingerprint: profile.identity_fingerprint(),
        resource_condition_fingerprint: profile.identity_fingerprint(),
        residency_fingerprint: residency,
        timing_convention: SCHEDULER_EMPIRICAL_SERVICE_TOTAL_CONVENTION,
    }
}

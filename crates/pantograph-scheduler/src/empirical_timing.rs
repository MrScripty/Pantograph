//! Bounded empirical joint service quantiles and successful-population residuals.
//! No clocks, calibration store, runtime calls, reservations or policy activation.

use crate::{
    SchedulerCohortCosts, SchedulerCompletionContext, SchedulerCompletionEvidenceSource,
    SchedulerCompletionRankingPolicy,
};

pub const SCHEDULER_EMPIRICAL_MAX_OBSERVATIONS: usize = 128;
pub const SCHEDULER_EMPIRICAL_MAX_WORK: usize = 32_768;
/// Individual disjoint serialized stages ending at acknowledged service drain.
/// Retention means serialized owner work, not a resident model's cache lifetime.
pub const SCHEDULER_EMPIRICAL_SERVICE_CONVENTION: &str =
    "serialized-six-stage-successful-drain-samples-us.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerEmpiricalServiceOutcome {
    Completed,
    Failed,
    Cancelled,
    Incomplete,
}

/// One distinct attempt, never an aggregated mean/quantile or a repeated weight.
/// A trusted producer must qualify configuration, clock domain, accepted success,
/// all phase boundaries and actual drain. Matching labels do not authenticate it.
#[derive(Debug, Clone, Copy)]
pub struct SchedulerEmpiricalServiceObservation<'a> {
    pub attempt_id: &'a str,
    pub context: SchedulerCompletionContext<'a>,
    pub source: SchedulerCompletionEvidenceSource,
    pub observed_at_ms: u64,
    pub outcome: SchedulerEmpiricalServiceOutcome,
    pub costs: SchedulerCohortCosts,
}

/// Whole observed selected-text custody-through-worker-drain total. This distinct
/// scope includes inter-phase gaps and is not the six-stage serialized convention.
pub const SCHEDULER_EMPIRICAL_SERVICE_TOTAL_CONVENTION: &str =
    "selected-text-custody-through-worker-drain-total-us.v1";

#[derive(Debug, Clone, Copy)]
pub struct SchedulerEmpiricalServiceTotalObservation<'a> {
    pub attempt_id: &'a str,
    pub context: SchedulerCompletionContext<'a>,
    pub source: SchedulerCompletionEvidenceSource,
    pub observed_at_ms: u64,
    pub outcome: SchedulerEmpiricalServiceOutcome,
    pub duration_us: Option<u64>,
}

trait Observation {
    fn sample(&self) -> SchedulerEmpiricalServiceObservation<'_>;
}
impl Observation for SchedulerEmpiricalServiceObservation<'_> {
    fn sample(&self) -> SchedulerEmpiricalServiceObservation<'_> {
        *self
    }
}
impl Observation for SchedulerEmpiricalServiceTotalObservation<'_> {
    fn sample(&self) -> SchedulerEmpiricalServiceObservation<'_> {
        // Private arithmetic encoding of ONE known total under its own convention.
        // These additive identities do not assert any observed phase was zero.
        SchedulerEmpiricalServiceObservation {
            attempt_id: self.attempt_id,
            context: self.context,
            source: self.source,
            observed_at_ms: self.observed_at_ms,
            outcome: self.outcome,
            costs: SchedulerCohortCosts {
                execution_us: self.duration_us,
                setup_us: Some(0),
                transfer_us: Some(0),
                cleanup_us: Some(0),
                retention_us: Some(0),
                reload_us: Some(0),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerEmpiricalServiceCondition {
    NotStarted,
    /// Elapsed from the SAME full six-stage service origin to the current read.
    /// Compute-only elapsed, pre-service queue delay and another attempt's age
    /// are not interchangeable. The trusted owner must verify this association.
    Running {
        elapsed_us: u64,
    },
}

/// Exact probability in (0, 1]; no floating-point interpolation or tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerEmpiricalQuantile {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct SchedulerEmpiricalServiceBudget {
    pub max_work_units: usize,
}
impl Default for SchedulerEmpiricalServiceBudget {
    fn default() -> Self {
        Self {
            max_work_units: SCHEDULER_EMPIRICAL_MAX_WORK,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerEmpiricalServiceWork {
    pub work_units: usize,
    pub observations_validated: usize,
    pub identity_comparisons: usize,
    pub order_comparisons: usize,
    pub order_swaps: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerEmpiricalServiceIncomplete {
    WorkLimitExceeded,
    BudgetExhausted,
    InvalidPolicy,
    InvalidQuantile,
    InvalidContext,
    UnsupportedConvention,
    InvalidAttemptIdentity,
    DuplicateAttempt,
    IncomparableEvidence,
    SyntheticEvidenceDisabled,
    UnsuccessfulAttempt,
    StaleOrFutureSample,
    MissingStage,
    DurationOverflow,
    InsufficientSamples,
    InsufficientSurvivors,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerEmpiricalServiceEstimate<'a> {
    pub context: SchedulerCompletionContext<'a>,
    pub source: SchedulerCompletionEvidenceSource,
    pub condition: SchedulerEmpiricalServiceCondition,
    pub quantile: SchedulerEmpiricalQuantile,
    /// Full service duration, or remaining service duration when Running.
    pub duration_us: u64,
    pub total_observations: usize,
    pub support_count: usize,
    pub rank: usize,
    pub work: SchedulerEmpiricalServiceWork,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerEmpiricalServiceResult<'a> {
    Estimated(SchedulerEmpiricalServiceEstimate<'a>),
    /// No partial estimate. Consumers retain their existing safe fallback.
    Incomplete {
        reason: SchedulerEmpiricalServiceIncomplete,
        work: SchedulerEmpiricalServiceWork,
    },
}

struct Meter {
    limit: usize,
    work: SchedulerEmpiricalServiceWork,
}
impl Meter {
    fn charge(&mut self) -> Result<(), SchedulerEmpiricalServiceIncomplete> {
        if self.work.work_units == self.limit {
            return Err(SchedulerEmpiricalServiceIncomplete::BudgetExhausted);
        }
        self.work.work_units += 1;
        Ok(())
    }
}

/// Sum paired phases within EACH attempt, then take the nearest-rank empirical
/// quantile. Running(e) conditions on strictly D>e and uses D-e; it is not the
/// unconditional quantile minus elapsed, nor a sum of marginal phase quantiles.
/// All rows are validated before survival filtering can yield an estimate.
/// Original and survivor populations independently require minimum_samples.
///
/// This describes successful samples only. Failures/censoring must be retained
/// elsewhere by the producer; this function rejects them rather than estimating
/// unconditional survival, failure risk, confidence or calibrated coverage.
/// Overlapping stages need a different model. Zero/missing stages are distinct.
/// A timing estimate never establishes physical release, readiness or a lease.
///
/// Hard guards precede row scans. Units charge bounded context/row validation,
/// each stage addition, attempt comparison, order comparison, adjacent swap and
/// rank evaluation. Each text field is <=128 bytes; storage is <=128 durations.
/// Worst-case duplicate scans and adjacent-swap insertion sort are quadratic and
/// fully metered. Limits describe computation, not measured dispatch latency.
pub fn estimate_scheduler_empirical_service_duration<'a>(
    current_context: SchedulerCompletionContext<'a>,
    observations: &[SchedulerEmpiricalServiceObservation<'_>],
    quantile: SchedulerEmpiricalQuantile,
    condition: SchedulerEmpiricalServiceCondition,
    policy: SchedulerCompletionRankingPolicy,
    budget: SchedulerEmpiricalServiceBudget,
) -> SchedulerEmpiricalServiceResult<'a> {
    estimate_duration(
        current_context,
        observations,
        quantile,
        condition,
        policy,
        budget,
        SCHEDULER_EMPIRICAL_SERVICE_CONVENTION,
    )
}

/// Bounded quantile/residual of explicit whole service totals. Same admission,
/// identity, support, work and success-conditioned semantics as the six-stage API.
/// The total convention is distinct; elapsed must share its custody origin.
/// Missing totals refuse; no missing native phase is invented or zero-filled.
pub fn estimate_scheduler_empirical_service_total_duration<'a>(
    current_context: SchedulerCompletionContext<'a>,
    observations: &[SchedulerEmpiricalServiceTotalObservation<'_>],
    quantile: SchedulerEmpiricalQuantile,
    condition: SchedulerEmpiricalServiceCondition,
    policy: SchedulerCompletionRankingPolicy,
    budget: SchedulerEmpiricalServiceBudget,
) -> SchedulerEmpiricalServiceResult<'a> {
    estimate_duration(
        current_context,
        observations,
        quantile,
        condition,
        policy,
        budget,
        SCHEDULER_EMPIRICAL_SERVICE_TOTAL_CONVENTION,
    )
}

fn estimate_duration<'a, O: Observation>(
    current_context: SchedulerCompletionContext<'a>,
    observations: &[O],
    quantile: SchedulerEmpiricalQuantile,
    condition: SchedulerEmpiricalServiceCondition,
    policy: SchedulerCompletionRankingPolicy,
    budget: SchedulerEmpiricalServiceBudget,
    convention: &str,
) -> SchedulerEmpiricalServiceResult<'a> {
    use SchedulerEmpiricalServiceIncomplete as R;
    let mut meter = Meter {
        limit: budget.max_work_units,
        work: SchedulerEmpiricalServiceWork::default(),
    };
    let result = (|| {
        if observations.len() > SCHEDULER_EMPIRICAL_MAX_OBSERVATIONS
            || budget.max_work_units > SCHEDULER_EMPIRICAL_MAX_WORK
        {
            return Err(R::WorkLimitExceeded);
        }
        meter.charge()?;
        if policy.minimum_samples == 0 || policy.max_sample_age_ms == 0 {
            return Err(R::InvalidPolicy);
        }
        if quantile.numerator == 0
            || quantile.denominator == 0
            || quantile.numerator > quantile.denominator
        {
            return Err(R::InvalidQuantile);
        }
        if !current_context.valid() {
            return Err(R::InvalidContext);
        }
        if current_context.timing_convention != convention {
            return Err(R::UnsupportedConvention);
        }
        let source = observations
            .first()
            .ok_or(R::InsufficientSamples)?
            .sample()
            .source;
        if source == SchedulerCompletionEvidenceSource::Synthetic && !policy.allow_synthetic {
            return Err(R::SyntheticEvidenceDisabled);
        }
        let mut durations = Vec::with_capacity(observations.len());
        for (index, observation) in observations.iter().enumerate() {
            meter.charge()?;
            let observation = observation.sample();
            if !bounded_identity(observation.attempt_id) {
                return Err(R::InvalidAttemptIdentity);
            }
            if !observation.context.valid() {
                return Err(R::InvalidContext);
            }
            if observation.context != current_context || observation.source != source {
                return Err(R::IncomparableEvidence);
            }
            if observation.outcome != SchedulerEmpiricalServiceOutcome::Completed {
                return Err(R::UnsuccessfulAttempt);
            }
            if policy
                .now_ms
                .checked_sub(observation.observed_at_ms)
                .is_none_or(|age| age > policy.max_sample_age_ms)
            {
                return Err(R::StaleOrFutureSample);
            }
            for prior in &observations[..index] {
                meter.charge()?;
                meter.work.identity_comparisons += 1;
                if prior.sample().attempt_id == observation.attempt_id {
                    return Err(R::DuplicateAttempt);
                }
            }
            let c = observation.costs;
            let mut total = 0_u64;
            for stage in [
                c.setup_us,
                c.transfer_us,
                c.execution_us,
                c.cleanup_us,
                c.retention_us,
                c.reload_us,
            ] {
                meter.charge()?;
                total = total
                    .checked_add(stage.ok_or(R::MissingStage)?)
                    .ok_or(R::DurationOverflow)?;
            }
            meter.work.observations_validated += 1;
            match condition {
                SchedulerEmpiricalServiceCondition::NotStarted => durations.push(total),
                SchedulerEmpiricalServiceCondition::Running { elapsed_us }
                    if total > elapsed_us =>
                {
                    durations.push(total - elapsed_us);
                }
                SchedulerEmpiricalServiceCondition::Running { .. } => {}
            }
        }
        if observations.len() < policy.minimum_samples as usize {
            return Err(R::InsufficientSamples);
        }
        if durations.len() < policy.minimum_samples as usize {
            return Err(R::InsufficientSurvivors);
        }
        for index in 1..durations.len() {
            let mut position = index;
            while position > 0 {
                meter.charge()?;
                meter.work.order_comparisons += 1;
                if durations[position - 1] <= durations[position] {
                    break;
                }
                meter.charge()?;
                meter.work.order_swaps += 1;
                durations.swap(position - 1, position);
                position -= 1;
            }
        }
        meter.charge()?;
        let rank = (u128::from(quantile.numerator) * durations.len() as u128)
            .div_ceil(u128::from(quantile.denominator)) as usize;
        Ok((source, durations[rank - 1], durations.len(), rank))
    })();
    match result {
        Ok((source, duration_us, support_count, rank)) => {
            SchedulerEmpiricalServiceResult::Estimated(SchedulerEmpiricalServiceEstimate {
                context: current_context,
                source,
                condition,
                quantile,
                duration_us,
                total_observations: observations.len(),
                support_count,
                rank,
                work: meter.work,
            })
        }
        Err(reason) => SchedulerEmpiricalServiceResult::Incomplete {
            reason,
            work: meter.work,
        },
    }
}

fn bounded_identity(value: &str) -> bool {
    value.len() <= 128 && !value.trim().is_empty() && !value.chars().any(char::is_control)
}

//! Read-only diagnostic projections from the existing cleanup-qualified ring.
//! A historical elapsed maximum is neither an end-to-end forecast nor authority.
use super::{
    same_owner, SerialReadyHistory, SerialReadyMode, WorkflowSerialReadyConfig,
    WorkflowSerialReadyMember,
};
use pantograph_runtime_host_contracts::SerialRuntimeHostCpuOwnerEvidence;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum ForecastReason {
    Available,
    OwnerUnavailable,
    OwnerChanged,
    HistoryUnavailable,
    InvalidClock,
    NoFreshSamples,
    InsufficientFreshSamples,
}

#[derive(Debug, serde::Serialize)]
struct WarmServiceForecast {
    workload_key: [u8; 32],
    owner_generation: Option<u64>,
    reason: ForecastReason,
    eligible_samples: usize,
    minimum_samples: usize,
    oldest_age_ns: Option<u64>,
    historical_max_elapsed_ns: Option<u64>,
}

impl WarmServiceForecast {
    fn unavailable(
        key: [u8; 32],
        config: WorkflowSerialReadyConfig,
        reason: ForecastReason,
    ) -> Self {
        Self {
            workload_key: key,
            owner_generation: None,
            reason,
            eligible_samples: 0,
            minimum_samples: config.minimum_samples,
            oldest_age_ns: None,
            historical_max_elapsed_ns: None,
        }
    }
}

fn project_history(
    history: &SerialReadyHistory,
    key: [u8; 32],
    owner: &SerialRuntimeHostCpuOwnerEvidence,
    config: WorkflowSerialReadyConfig,
    now: Instant,
) -> WarmServiceForecast {
    let mut forecast =
        WarmServiceForecast::unavailable(key, config, ForecastReason::NoFreshSamples);
    forecast.owner_generation = Some(owner.snapshot.generation);
    if !owner.is_current() {
        forecast.reason = ForecastReason::OwnerChanged;
        return forecast;
    }
    let mut maximum = 0;
    for sample in &history.samples {
        if sample.key != key || !same_owner(&sample.owner, owner) {
            continue;
        }
        let Some(age) = now.checked_duration_since(sample.at) else {
            forecast.reason = ForecastReason::InvalidClock;
            return forecast;
        };
        if age > config.maximum_age {
            continue;
        }
        // Fresh ages are bounded by the constructor to at most one hour.
        let Ok(age_ns) = u64::try_from(age.as_nanos()) else {
            forecast.reason = ForecastReason::InvalidClock;
            return forecast;
        };
        forecast.eligible_samples += 1;
        maximum = maximum.max(sample.cost_ns);
        forecast.oldest_age_ns = Some(forecast.oldest_age_ns.unwrap_or(0).max(age_ns));
    }
    if forecast.eligible_samples >= config.minimum_samples {
        forecast.reason = ForecastReason::Available;
        forecast.historical_max_elapsed_ns = Some(maximum);
    } else if forecast.eligible_samples > 0 {
        forecast.reason = ForecastReason::InsufficientFreshSamples;
    }
    forecast
}

impl SerialReadyMode {
    fn warm_service_forecast(
        &self,
        member: &WorkflowSerialReadyMember,
        key: [u8; 32],
    ) -> WarmServiceForecast {
        // Provider callbacks never run under history/cache/store guards.
        let Some(owner) = self.owner_for(&member.task) else {
            return WarmServiceForecast::unavailable(
                key,
                self.config,
                ForecastReason::OwnerUnavailable,
            );
        };
        let mut forecast = match self.history.lock() {
            Ok(history) => project_history(&history, key, &owner, self.config, Instant::now()),
            Err(_) => WarmServiceForecast::unavailable(
                key,
                self.config,
                ForecastReason::HistoryUnavailable,
            ),
        };
        // This is a point-in-time observation, never an execution capability.
        if !owner.is_current() {
            forecast.reason = ForecastReason::OwnerChanged;
            forecast.historical_max_elapsed_ns = None;
        }
        forecast
    }

    pub(super) fn log_warm_service_forecasts(
        &self,
        entries: &[(WorkflowSerialReadyMember, [u8; 32])],
    ) {
        let scope = self.grouping_observation_scope();
        let forecasts: Vec<_> = entries
            .iter()
            .map(|(member, key)| self.warm_service_forecast(member, *key))
            .collect();
        // entries comes only from bounded fill: no new key calculation, cache
        // access, profile population, native work or dispatch result is created.
        log::info!(target: "workflow::serial_observations", "{}", serde_json::json!({
            "phase": "warm_service_forecasts",
            "scope": scope.map(|scope| scope.to_string()),
            "coverage": "captured_compatible_prefix",
            "source": "observed_dispatch_through_drain_and_matched_cleanup",
            "supported_backend": "candle",
            "supported_device": "cpu",
            "conditioning": "unconditioned_current_owner_exact_workload",
            "units": "ns",
            "future_bound": false,
            "includes_queue_or_cold_preparation": false,
            "forecasts": forecasts,
        }));
    }
}

#[cfg(test)]
#[path = "observations_tests.rs"]
pub(super) mod tests;

use super::*;
use crate::workflow::serial_ready::{Observation, WorkflowSerialReadyPair};
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;

pub(in crate::workflow) fn assert_warm_forecast_oracle(
    mode: &SerialReadyMode,
    pair: &WorkflowSerialReadyPair,
) {
    let member = &pair.members[0];
    let key = crate::workflow::serial_ready::observation_key(member).unwrap();
    let owner = mode.owner_for(&member.task).unwrap();
    let now = Instant::now();
    let mut history = SerialReadyHistory::default();
    for (seconds, cost_ns) in [(1, 7), (2, 3), (3, 11)] {
        history.samples.push_back(Observation {
            key,
            owner: owner.clone(),
            at: now.checked_sub(Duration::from_secs(seconds)).unwrap(),
            cost_ns,
            predecessor_key: None,
            task_id: "controlled unit observation".into(),
        });
    }
    let projected = project_history(&history, key, &owner, mode.config, now);
    assert_eq!(projected.reason, ForecastReason::Available);
    assert_eq!(projected.historical_max_elapsed_ns, Some(11));
    assert_eq!(projected.eligible_samples, 3);
    assert_eq!(projected.oldest_age_ns, Some(3_000_000_000));
    assert_eq!(projected.owner_generation, Some(owner.snapshot.generation));

    let expired = project_history(
        &history,
        key,
        &owner,
        mode.config,
        now + Duration::from_secs(61),
    );
    assert_eq!(expired.reason, ForecastReason::NoFreshSamples);
    assert_eq!(expired.historical_max_elapsed_ns, None);
    assert_eq!(expired.oldest_age_ns, None);
    assert_eq!(
        project_history(&history, [9; 32], &owner, mode.config, now).reason,
        ForecastReason::NoFreshSamples
    );

    let mut alias = owner.clone();
    alias.generation = Arc::new(std::sync::atomic::AtomicU64::new(owner.snapshot.generation));
    assert_eq!(
        project_history(&history, key, &alias, mode.config, now).reason,
        ForecastReason::NoFreshSamples
    );
    let mut different = owner.clone();
    different.snapshot.loaded_instance = [8; 16];
    assert_eq!(
        project_history(&history, key, &different, mode.config, now).reason,
        ForecastReason::NoFreshSamples
    );

    let saved = history.samples.pop_back().unwrap();
    let insufficient = project_history(&history, key, &owner, mode.config, now);
    assert_eq!(
        insufficient.reason,
        ForecastReason::InsufficientFreshSamples
    );
    assert_eq!(insufficient.eligible_samples, 2);
    assert_eq!(insufficient.historical_max_elapsed_ns, None);
    history.samples.push_back(saved);
    let saved_at = history.samples[0].at;
    history.samples[0].at = now + Duration::from_nanos(1);
    let invalid = project_history(&history, key, &owner, mode.config, now);
    assert_eq!(invalid.reason, ForecastReason::InvalidClock);
    assert_eq!(invalid.historical_max_elapsed_ns, None);
    history.samples[0].at = saved_at;

    for sample in &mut history.samples {
        sample.cost_ns = 0;
        sample.at = now.checked_sub(mode.config.maximum_age).unwrap();
    }
    let boundary = project_history(&history, key, &owner, mode.config, now);
    assert_eq!(boundary.reason, ForecastReason::Available);
    assert_eq!(boundary.historical_max_elapsed_ns, Some(0));
    assert_eq!(boundary.oldest_age_ns, Some(60_000_000_000));
    owner
        .generation
        .store(owner.snapshot.generation + 2, Ordering::Release);
    assert_eq!(
        project_history(&history, key, &owner, mode.config, now).reason,
        ForecastReason::OwnerChanged
    );
    owner
        .generation
        .store(owner.snapshot.generation, Ordering::Release);

    // Exercise the production owner/query/lock path without encodings, cache
    // hits, profile mutation, execution or cleanup. Hand data is unit evidence.
    for sample in &mut history.samples {
        sample.at = now
            .checked_sub(mode.config.maximum_age + Duration::from_secs(1))
            .unwrap();
    }
    *mode.history.lock().unwrap() = history;
    let work_before = (
        mode.key_encodings.load(Ordering::Relaxed),
        mode.key_cache_hits.load(Ordering::Relaxed),
    );
    let costs_before: Vec<_> = mode
        .history
        .lock()
        .unwrap()
        .samples
        .iter()
        .map(|s| s.cost_ns)
        .collect();
    let forecast = mode.warm_service_forecast(member, key);
    // The production path must refuse hand observations beyond the freshness bound.
    assert_eq!(forecast.reason, ForecastReason::NoFreshSamples);
    for sample in &mut mode.history.lock().unwrap().samples {
        sample.at = Instant::now();
    }
    let forecast = mode.warm_service_forecast(member, key);
    assert_eq!(forecast.reason, ForecastReason::Available);
    assert_eq!(forecast.historical_max_elapsed_ns, Some(0));
    mode.log_warm_service_forecasts(&[(member.clone(), key)]);
    assert_eq!(
        mode.history
            .lock()
            .unwrap()
            .samples
            .iter()
            .map(|s| s.cost_ns)
            .collect::<Vec<_>>(),
        costs_before
    );
    assert_eq!(
        work_before,
        (
            mode.key_encodings.load(Ordering::Relaxed),
            mode.key_cache_hits.load(Ordering::Relaxed)
        )
    );
    let mut unsupported = member.clone();
    unsupported.task.schedulable_intent = None;
    let refused = mode.warm_service_forecast(&unsupported, key);
    assert_eq!(refused.reason, ForecastReason::OwnerUnavailable);
    assert_eq!(refused.historical_max_elapsed_ns, None);
    let json = serde_json::to_string(&forecast).unwrap();
    assert!(!json.contains(member.task.task_id.as_str()));
    assert!(!json.contains("controlled unit observation"));
    *mode.history.lock().unwrap() = SerialReadyHistory::default();

    let poisoned = SerialReadyMode::new(mode.port.clone(), mode.config);
    let cloned = poisoned.clone();
    assert!(std::thread::spawn(move || {
        let _guard = cloned.history.lock().unwrap();
        panic!("controlled history poison");
    })
    .join()
    .is_err());
    let refused = poisoned.warm_service_forecast(member, key);
    assert_eq!(refused.reason, ForecastReason::HistoryUnavailable);
    assert_eq!(refused.historical_max_elapsed_ns, None);
}

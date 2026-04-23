use super::*;

impl RuntimeRegistry {
    pub fn snapshot(&self) -> RuntimeRegistrySnapshot {
        let generated_at_ms = unix_timestamp_ms();
        let guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let mut runtimes = guard
            .runtimes
            .values()
            .map(runtime_snapshot)
            .collect::<Vec<_>>();
        runtimes.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));

        let mut reservations = guard
            .reservations
            .values()
            .cloned()
            .map(RuntimeReservationRecord::into_lease)
            .collect::<Vec<_>>();
        reservations.sort_by(|left, right| {
            left.runtime_id
                .cmp(&right.runtime_id)
                .then_with(|| left.reservation_id.cmp(&right.reservation_id))
        });

        RuntimeRegistrySnapshot {
            generated_at_ms,
            runtimes,
            reservations,
        }
    }

    pub fn eviction_candidates(&self) -> Vec<RuntimeRegistryRuntimeSnapshot> {
        let guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let mut candidates = guard
            .runtimes
            .values()
            .filter(|record| runtime_is_evictable(record))
            .map(runtime_snapshot)
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            eviction_status_rank(left.status)
                .cmp(&eviction_status_rank(right.status))
                .then_with(|| left.last_transition_at_ms.cmp(&right.last_transition_at_ms))
                .then_with(|| left.runtime_id.cmp(&right.runtime_id))
        });
        candidates
    }

    pub fn eviction_reservation_candidates(&self) -> Vec<RuntimeReservationLease> {
        let guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let ordered_runtime_ids = guard
            .runtimes
            .values()
            .filter(|record| runtime_is_reservation_evictable(record))
            .map(|record| record.runtime_id.clone())
            .collect::<Vec<_>>();

        let mut candidates = ordered_runtime_ids
            .into_iter()
            .flat_map(|runtime_id| {
                let mut reservations = guard
                    .reservations
                    .values()
                    .filter(|reservation| reservation.runtime_id == runtime_id)
                    .cloned()
                    .collect::<Vec<_>>();
                reservations.sort_by(|left, right| {
                    retention_hint_rank(left.retention_hint)
                        .cmp(&retention_hint_rank(right.retention_hint))
                        .then_with(|| left.created_at_ms.cmp(&right.created_at_ms))
                        .then_with(|| left.reservation_id.cmp(&right.reservation_id))
                });
                reservations
                    .into_iter()
                    .map(RuntimeReservationRecord::into_lease)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            runtime_eviction_rank(&left.runtime_id, &guard)
                .cmp(&runtime_eviction_rank(&right.runtime_id, &guard))
                .then_with(|| {
                    retention_hint_rank(left.retention_hint)
                        .cmp(&retention_hint_rank(right.retention_hint))
                })
                .then_with(|| left.created_at_ms.cmp(&right.created_at_ms))
                .then_with(|| left.reservation_id.cmp(&right.reservation_id))
        });
        candidates
    }

    pub fn eviction_reservation_candidate_for_owners(
        &self,
        owner_ids: &[&str],
    ) -> Option<RuntimeReservationLease> {
        let owner_ids = owner_ids.iter().copied().collect::<BTreeSet<_>>();
        self.eviction_reservation_candidates()
            .into_iter()
            .find(|reservation| {
                reservation
                    .reservation_owner_id
                    .as_deref()
                    .map(|owner_id| owner_ids.contains(owner_id))
                    .unwrap_or(false)
            })
    }
}

pub(super) fn runtime_snapshot(record: &RuntimeRegistryRecord) -> RuntimeRegistryRuntimeSnapshot {
    let mut backend_keys = record.backend_keys.iter().cloned().collect::<Vec<_>>();
    backend_keys.sort();

    let mut active_reservation_ids = record
        .active_reservations
        .iter()
        .copied()
        .collect::<Vec<_>>();
    active_reservation_ids.sort();

    let mut models = record.models.values().cloned().collect::<Vec<_>>();
    models.sort_by(|left, right| left.model_id.cmp(&right.model_id));

    RuntimeRegistryRuntimeSnapshot {
        runtime_id: record.runtime_id.clone(),
        display_name: record.display_name.clone(),
        backend_keys,
        status: record.status,
        runtime_instance_id: record.runtime_instance_id.clone(),
        last_error: record.last_error.clone(),
        last_transition_at_ms: record.last_transition_at_ms,
        active_reservation_ids,
        models,
    }
}

pub(super) fn runtime_is_evictable(record: &RuntimeRegistryRecord) -> bool {
    if !record.active_reservations.is_empty() {
        return false;
    }

    if record.models.values().any(|model| model.pinned) {
        return false;
    }

    matches!(
        record.status,
        RuntimeRegistryStatus::Warming
            | RuntimeRegistryStatus::Ready
            | RuntimeRegistryStatus::Unhealthy
            | RuntimeRegistryStatus::Stopping
    )
}

fn runtime_is_reservation_evictable(record: &RuntimeRegistryRecord) -> bool {
    if record.models.values().any(|model| model.pinned) {
        return false;
    }

    matches!(
        record.status,
        RuntimeRegistryStatus::Warming
            | RuntimeRegistryStatus::Ready
            | RuntimeRegistryStatus::Unhealthy
            | RuntimeRegistryStatus::Stopping
    )
}

fn eviction_status_rank(status: RuntimeRegistryStatus) -> u8 {
    match status {
        RuntimeRegistryStatus::Unhealthy => 0,
        RuntimeRegistryStatus::Ready => 1,
        RuntimeRegistryStatus::Warming => 2,
        RuntimeRegistryStatus::Stopping => 3,
        RuntimeRegistryStatus::Busy
        | RuntimeRegistryStatus::Stopped
        | RuntimeRegistryStatus::Failed => 4,
    }
}

fn retention_hint_rank(retention_hint: RuntimeRetentionHint) -> u8 {
    match retention_hint {
        RuntimeRetentionHint::Ephemeral => 0,
        RuntimeRetentionHint::KeepAlive => 1,
    }
}

fn runtime_eviction_rank(runtime_id: &str, state: &RuntimeRegistryState) -> (u8, u64, String) {
    let runtime_id = canonical_runtime_id(runtime_id);
    let Some(record) = state.runtimes.get(&runtime_id) else {
        return (u8::MAX, u64::MAX, runtime_id);
    };
    (
        eviction_status_rank(record.status),
        record.last_transition_at_ms,
        record.runtime_id.clone(),
    )
}

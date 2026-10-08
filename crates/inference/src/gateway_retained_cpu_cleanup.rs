//! Opt-in retained-only cleanup under the actual native backend writer.
use super::{canonical_backend_key, config_model_target, GatewayError, InferenceGateway};
use crate::CandleCpuCleanupOwner;
use pantograph_runtime_registry::{
    RuntimeObservation, RuntimeProducerAllocationState, RuntimeProducerObservation,
    RuntimeRegistry, RuntimeRegistryStatus, RuntimeReservationLease, RuntimeRetainedOwnerIdentity,
    RuntimeRetentionDisposition,
};
use std::sync::atomic::Ordering;

impl InferenceGateway {
    /// Nonblocking advisory query. Actual dispatch revalidates under its writer.
    pub fn resident_cpu_serial_owner(
        &self,
        model: &crate::PumasModelRef,
    ) -> Option<CandleCpuCleanupOwner> {
        let backend = self.backend.try_read().ok()?;
        if canonical_backend_key(backend.name()) != "candle" || !backend.is_ready() {
            return None;
        }
        let instance = backend.resident_cpu_calibration_instance()?;
        let owner = self
            .candle_cpu_calibration
            .as_ref()?
            .cleanup_owner(instance)?;
        let lifecycle = self.runtime_lifecycle.try_read().ok()?;
        let config = self.current_runtime_config.try_read().ok()?;
        (owner.model_ref() == model
            && lifecycle.runtime_id.as_deref() == Some("candle")
            && lifecycle.active
            && config.as_ref()?.model_name.as_deref() == Some(model.model_id.as_str()))
        .then_some(owner)
    }

    /// Publish actual loaded CPU identity to a separately configured registry
    /// envelope, and issue an opaque current stamp. Missing estimates stay
    /// unknown; publication alone does not enable retained cleanup or scheduling.
    pub async fn publish_resident_cpu_cleanup_owner(
        &self,
        registry: &RuntimeRegistry,
    ) -> Result<CandleCpuCleanupOwner, GatewayError> {
        let backend = self.backend.write().await;
        if canonical_backend_key(backend.name()) != "candle" || !backend.is_ready() {
            return Err(cleanup_refusal());
        }
        let instance = backend
            .resident_cpu_calibration_instance()
            .ok_or_else(cleanup_refusal)?;
        let owner = self
            .candle_cpu_calibration
            .as_ref()
            .and_then(|owner| owner.cleanup_owner(instance))
            .ok_or_else(cleanup_refusal)?;
        let lifecycle = self.runtime_lifecycle.read().await;
        let runtime_instance_id = lifecycle
            .runtime_instance_id
            .clone()
            .ok_or_else(cleanup_refusal)?;
        let config = self.current_runtime_config.read().await;
        if lifecycle.runtime_id.as_deref() != Some("candle")
            || !lifecycle.active
            || config.as_ref().and_then(|c| c.model_name.as_deref())
                != Some(owner.model_ref().model_id.as_str())
        {
            return Err(cleanup_refusal());
        }
        let model_target = config
            .as_ref()
            .and_then(config_model_target)
            .ok_or_else(cleanup_refusal)?;
        if !self
            .candle_cpu_calibration
            .as_ref()
            .is_some_and(|calibration| calibration.matches_cleanup_owner(&owner, instance))
        {
            return Err(cleanup_refusal());
        }
        registry
            .observe_runtime_producer(RuntimeProducerObservation {
                source_id: self.resident_source_id.clone(),
                sequence: self
                    .resident_observation_sequence
                    .fetch_add(1, Ordering::Relaxed)
                    + 1,
                allocation_state: RuntimeProducerAllocationState::Resident,
                observation: RuntimeObservation {
                    runtime_id: "candle".into(),
                    display_name: "Candle".into(),
                    backend_keys: vec!["candle".into()],
                    model_id: Some(model_target),
                    runtime_instance_id: Some(runtime_instance_id),
                    status: RuntimeRegistryStatus::Ready,
                    last_error: None,
                },
            })
            .map_err(|error| GatewayError::SwitchFailed(error.to_string()))?;
        Ok(owner)
    }

    /// Validate actual authority/epoch and release only a completed task lease
    /// kept resident by another lease. The backend writer spans every registry
    /// check and mutation; there is no await, stop or callback after preflight.
    /// A stale stamp or last-lease eviction refuses without freeing the claim.
    /// The caller asserts that this particular lease is completed; this method
    /// does not mint an attempt, drained-task or serial cleanup proof.
    pub async fn release_retained_cpu_reservation(
        &self,
        registry: &RuntimeRegistry,
        expected_lease: &RuntimeReservationLease,
        expected_owner: &CandleCpuCleanupOwner,
    ) -> Result<RuntimeRetentionDisposition, GatewayError> {
        self.release_retained_cpu_reservation_owned(registry, expected_lease, expected_owner, None)
            .await
    }

    pub(super) async fn release_retained_cpu_reservation_owned(
        &self,
        registry: &RuntimeRegistry,
        expected_lease: &RuntimeReservationLease,
        expected_owner: &CandleCpuCleanupOwner,
        custody: Option<pantograph_runtime_registry::RuntimeReservationExecutionCustody>,
    ) -> Result<RuntimeRetentionDisposition, GatewayError> {
        let backend = self.backend.write().await;
        if canonical_backend_key(backend.name()) != "candle" || !backend.is_ready() {
            return Err(cleanup_refusal());
        }
        let instance = backend
            .resident_cpu_calibration_instance()
            .ok_or_else(cleanup_refusal)?;
        if !self
            .candle_cpu_calibration
            .as_ref()
            .is_some_and(|owner| owner.matches_cleanup_owner(expected_owner, instance))
            || expected_lease.model_id.as_deref()
                != Some(expected_owner.model_ref().model_id.as_str())
        {
            return Err(cleanup_refusal());
        }
        let lifecycle = self.runtime_lifecycle.read().await;
        let runtime_instance_id = lifecycle
            .runtime_instance_id
            .as_deref()
            .ok_or_else(cleanup_refusal)?;
        let config = self.current_runtime_config.read().await;
        if lifecycle.runtime_id.as_deref() != Some("candle")
            || !lifecycle.active
            || config.as_ref().and_then(|c| c.model_name.as_deref())
                != Some(expected_owner.model_ref().model_id.as_str())
        {
            return Err(cleanup_refusal());
        }
        let model_target = config
            .as_ref()
            .and_then(config_model_target)
            .ok_or_else(cleanup_refusal)?;
        if !self
            .candle_cpu_calibration
            .as_ref()
            .is_some_and(|calibration| calibration.matches_cleanup_owner(expected_owner, instance))
        {
            return Err(cleanup_refusal());
        }
        let identity = RuntimeRetainedOwnerIdentity {
            runtime_id: "candle",
            source_id: &self.resident_source_id,
            runtime_instance_id,
            model_target: &model_target,
        };
        match custody {
            Some(custody) => custody.release_retained(identity),
            None => registry.release_retained_reservation_for_owner(expected_lease, identity),
        }
        .map_err(|error| GatewayError::SwitchFailed(error.to_string()))
    }
}

fn cleanup_refusal() -> GatewayError {
    GatewayError::SwitchFailed(
        "retained CPU cleanup requires the current actual loaded owner and exact task lease".into(),
    )
}

#[cfg(test)]
#[path = "gateway_retained_cpu_cleanup_tests.rs"]
mod tests;

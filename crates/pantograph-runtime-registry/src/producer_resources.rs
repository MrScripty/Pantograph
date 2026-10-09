//! Ordered observations from the existing lifecycle owner, with explicit estimates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    apply_producer_runtime_observation, canonical_runtime_id, model_resources::declare_locked,
    reservation_claim_from_requirements, runtime_snapshot, sync_observed_models, unix_timestamp_ms,
    RuntimeObservation, RuntimeRegistry, RuntimeRegistryError, RuntimeRegistryRuntimeSnapshot,
    RuntimeRegistryStatus, RuntimeReservationRequirements,
};

/// An operator estimate for an exact observed model target. Zero claims mean
/// known zero; omitted kinds mean unknown. These are not allocator measurements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeModelResidentEstimate {
    pub runtime_id: String,
    pub model_id: String,
    pub requirements: RuntimeReservationRequirements,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProducerAllocationState {
    Resident,
    Released,
    Unknown,
}

/// Sequence is allocated while sampling under the owner's lifecycle lock,
/// before delivery. A producer source is bound for this registry's lifetime;
/// replacing the owner requires fresh startup composition, never implicit reuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProducerObservation {
    pub source_id: String,
    pub sequence: u64,
    pub allocation_state: RuntimeProducerAllocationState,
    pub observation: RuntimeObservation,
}

impl RuntimeRegistry {
    /// Install immutable startup estimates. Configuration never implies residency.
    pub fn configure_model_resident_estimates(
        &self,
        estimates: Vec<RuntimeModelResidentEstimate>,
    ) -> Result<(), RuntimeRegistryError> {
        let mut configured = BTreeMap::new();
        for estimate in estimates {
            let runtime_id = canonical_runtime_id(&estimate.runtime_id);
            if pantograph_runtime_identity::runtime_display_name(&runtime_id).is_none()
                || estimate.model_id.trim().is_empty()
                || estimate.requirements.claims.is_empty()
                || configured.contains_key(&(runtime_id.clone(), estimate.model_id.clone()))
            {
                return Err(RuntimeRegistryError::InvalidModelResidencyResources {
                    runtime_id,
                    reason:
                        "known runtime, exact model, nonempty claims and unique estimate required",
                });
            }
            reservation_claim_from_requirements(&runtime_id, Some(&estimate.requirements))?;
            configured.insert((runtime_id, estimate.model_id), estimate.requirements);
        }
        let mut state = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        if !state.resident_estimates.is_empty() || !state.producer_observations.is_empty() {
            return Err(RuntimeRegistryError::InvalidModelResidencyResources {
                runtime_id: "configuration".into(),
                reason: "resident estimates are immutable startup configuration",
            });
        }
        for (runtime_id, _) in configured.keys() {
            state.runtimes.entry(runtime_id.clone()).or_insert_with(|| {
                crate::RuntimeRegistryRecord::new(runtime_id, runtime_id, unix_timestamp_ms())
            });
        }
        state.resident_estimates = configured;
        Ok(())
    }

    /// Apply owner facts and their configured resident envelope under the same
    /// admission lock. A rejected estimate leaves the actual new identity unknown
    /// and thus blocks shared admission; it never rolls back lifecycle reality.
    pub fn observe_runtime_producer(
        &self,
        frame: RuntimeProducerObservation,
    ) -> Result<RuntimeRegistryRuntimeSnapshot, RuntimeRegistryError> {
        let runtime_id = canonical_runtime_id(&frame.observation.runtime_id);
        let invalid = || RuntimeRegistryError::ModelResidencyObservationChanged(runtime_id.clone());
        if frame.source_id.trim().is_empty() || frame.sequence == 0 {
            return Err(invalid());
        }
        match frame.allocation_state {
            RuntimeProducerAllocationState::Resident => {
                if frame
                    .observation
                    .model_id
                    .as_ref()
                    .is_none_or(|id| id.trim().is_empty())
                    || frame
                        .observation
                        .runtime_instance_id
                        .as_ref()
                        .is_none_or(|id| id.trim().is_empty())
                    || frame.observation.status == RuntimeRegistryStatus::Stopped
                {
                    return Err(invalid());
                }
            }
            RuntimeProducerAllocationState::Released if frame.observation.model_id.is_some() => {
                return Err(invalid());
            }
            _ => {}
        }
        let mut state = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let configured = state
            .resident_estimates
            .keys()
            .any(|(runtime, _)| runtime == &runtime_id)
            || state.resource_domains.values().any(|domain| {
                domain
                    .bindings
                    .iter()
                    .any(|binding| binding.runtime_id == runtime_id)
            });
        if !configured && !state.producer_observations.contains_key(&runtime_id) {
            // Preserve unconfigured legacy admission/lifecycle behavior.
            crate::apply_runtime_observation(&mut state, frame.observation, unix_timestamp_ms());
            return Ok(runtime_snapshot(
                state.runtimes.get(&runtime_id).expect("observed runtime"),
            ));
        }
        if let Some((source, sequence)) = state.producer_observations.get(&runtime_id) {
            if *source != frame.source_id || *sequence >= frame.sequence {
                return Err(invalid());
            }
        }
        let mut observation = frame.observation;
        match frame.allocation_state {
            RuntimeProducerAllocationState::Unknown => {
                // Neither a missing model nor a health failure proves release.
                // Preserve the last allocation while recording owner uncertainty.
                let record = state.runtimes.entry(runtime_id.clone()).or_insert_with(|| {
                    crate::RuntimeRegistryRecord::new(
                        &runtime_id,
                        &observation.display_name,
                        unix_timestamp_ms(),
                    )
                });
                record.resident_resources_uncertain = true;
                record.status = RuntimeRegistryStatus::Failed;
                record.last_error = observation.last_error;
            }
            RuntimeProducerAllocationState::Released => {
                let health_status = observation.status;
                observation.status = RuntimeRegistryStatus::Stopped;
                apply_producer_runtime_observation(&mut state, observation, unix_timestamp_ms());
                // Confirmed logical absence releases allocation even when health
                // remains failed. Task custody is independent and stays charged.
                let record = state
                    .runtimes
                    .get_mut(&runtime_id)
                    .expect("observed runtime");
                record.status = health_status;
                record.resident_resources_uncertain = false;
            }
            RuntimeProducerAllocationState::Resident => {
                let model_id = observation.model_id.clone().expect("validated model");
                let instance_id = observation
                    .runtime_instance_id
                    .clone()
                    .expect("validated instance");
                apply_producer_runtime_observation(&mut state, observation, unix_timestamp_ms());
                let record = state
                    .runtimes
                    .get_mut(&runtime_id)
                    .expect("observed runtime");
                record.resident_resources_uncertain = false;
                sync_observed_models(
                    record,
                    Some(model_id.clone()),
                    RuntimeRegistryStatus::Ready,
                    unix_timestamp_ms(),
                );
                // Advance even on rejected growth: the owner identity is real.
                state.producer_observations.insert(
                    runtime_id.clone(),
                    (frame.source_id.clone(), frame.sequence),
                );
                if let Some(requirements) = state
                    .resident_estimates
                    .get(&(runtime_id.clone(), model_id.clone()))
                    .cloned()
                {
                    return declare_locked(
                        &mut state,
                        &runtime_id,
                        &model_id,
                        &instance_id,
                        requirements,
                    );
                }
            }
        }
        state
            .producer_observations
            .insert(runtime_id.clone(), (frame.source_id, frame.sequence));
        Ok(runtime_snapshot(
            state.runtimes.get(&runtime_id).expect("observed runtime"),
        ))
    }
}

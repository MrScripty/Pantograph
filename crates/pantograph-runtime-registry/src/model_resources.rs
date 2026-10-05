//! Explicit resident allocation estimates owned by the runtime registry.

use serde::{Deserialize, Serialize};

use crate::{
    admission::RuntimeReservationClaim, admission_failure, canonical_runtime_id,
    reservation_claim_from_requirements, runtime_snapshot, RuntimeAdmissionResourceKind,
    RuntimeObservation, RuntimeRegistry, RuntimeRegistryError, RuntimeRegistryRecord,
    RuntimeRegistryRuntimeSnapshot, RuntimeRegistryStatus, RuntimeReservationRequirements,
};

/// An observed model/producer identity and its separately declared resident
/// envelope. Missing declarations are unknown, never measured zero. A task's
/// existing peak envelope is still charged in full; aliases are not inferred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeModelResourceResidency {
    pub model_id: String,
    pub runtime_instance_id: Option<String>,
    pub requirements: Option<RuntimeReservationRequirements>,
}

impl RuntimeRegistry {
    /// Publish explicit resident estimates for the current observed identity.
    /// This does not load a model, measure an allocator, or infer a device split.
    /// Declarations grow componentwise while this identity remains resident:
    /// smaller or partial updates cannot release its previously held envelope.
    /// Growth must fit alongside all live task claims; rejection keeps the
    /// previous declaration. After model/instance changes a producer must
    /// publish fresh estimates. A missing per-kind declaration blocks shared
    /// admission for that kind until provided or the producer confirms stop.
    pub fn declare_model_residency_resources(
        &self,
        runtime_id: &str,
        model_id: &str,
        runtime_instance_id: &str,
        requirements: RuntimeReservationRequirements,
    ) -> Result<RuntimeRegistryRuntimeSnapshot, RuntimeRegistryError> {
        let runtime_id = canonical_runtime_id(runtime_id);
        if requirements.claims.is_empty()
            || requirements.claims.iter().any(|claim| claim.bytes == 0)
            || model_id.trim().is_empty()
            || runtime_instance_id.trim().is_empty()
        {
            return Err(RuntimeRegistryError::InvalidModelResidencyResources {
                runtime_id,
                reason: "nonempty positive resident estimates are required",
            });
        }
        let mut claim = reservation_claim_from_requirements(&runtime_id, Some(&requirements))?;
        let mut state = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let record = state
            .runtimes
            .get_mut(&runtime_id)
            .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.clone()))?;
        let Some(residency) = record.model_resource_residency.as_mut() else {
            return Err(RuntimeRegistryError::ModelResidencyObservationChanged(
                runtime_id,
            ));
        };
        if residency.model_id != model_id
            || !record.models.contains_key(model_id)
            || residency.runtime_instance_id.as_deref() != Some(runtime_instance_id)
            || record.runtime_instance_id.as_deref() != Some(runtime_instance_id)
            || record.status == RuntimeRegistryStatus::Stopped
        {
            return Err(RuntimeRegistryError::ModelResidencyObservationChanged(
                runtime_id,
            ));
        }
        let previous_claim =
            reservation_claim_from_requirements(&runtime_id, residency.requirements.as_ref())?;
        claim.ram_bytes = claim.ram_bytes.max(previous_claim.ram_bytes);
        claim.vram_bytes = claim.vram_bytes.max(previous_claim.vram_bytes);
        let mut held_claims = Vec::new();
        if let Some(bytes) = claim.ram_bytes {
            held_claims.push(crate::RuntimeReservationResourceClaim::ram_bytes(bytes));
        }
        if let Some(bytes) = claim.vram_bytes {
            held_claims.push(crate::RuntimeReservationResourceClaim::vram_bytes(bytes));
        }
        let previous = residency
            .requirements
            .replace(RuntimeReservationRequirements::from_claims(held_claims));
        let result = (|| {
            let record = state.runtimes.get(&runtime_id).expect("validated runtime");
            if let Some(failure) = admission_failure(
                record,
                RuntimeReservationClaim {
                    ram_bytes: Some(0),
                    vram_bytes: Some(0),
                },
                &state.reservations,
                None,
            )? {
                return Err(RuntimeRegistryError::AdmissionRejected {
                    runtime_id: runtime_id.clone(),
                    failure,
                });
            }
            crate::resource_domain::validate_resident_domain_capacity(&state, &runtime_id, claim)
        })();
        if let Err(error) = result {
            state
                .runtimes
                .get_mut(&runtime_id)
                .expect("validated runtime")
                .model_resource_residency
                .as_mut()
                .expect("validated residency")
                .requirements = previous;
            return Err(error);
        }
        Ok(runtime_snapshot(
            state.runtimes.get(&runtime_id).expect("validated runtime"),
        ))
    }
}

pub(crate) fn observe_model_resources(
    record: &mut RuntimeRegistryRecord,
    observation: &RuntimeObservation,
) {
    if observation.status == RuntimeRegistryStatus::Stopped {
        record.model_resource_residency = None;
        return;
    }
    if let Some(resident) = record.model_resource_residency.as_mut() {
        if observation
            .runtime_instance_id
            .as_ref()
            .is_some_and(|instance| resident.runtime_instance_id.as_ref() != Some(instance))
        {
            resident.runtime_instance_id = observation.runtime_instance_id.clone();
            resident.requirements = None;
        }
    }
    let Some(model_id) = observation.model_id.as_ref() else {
        // Missing model metadata and health failures do not confirm deallocation.
        return;
    };
    if record
        .model_resource_residency
        .as_ref()
        .is_some_and(|resident| {
            resident.model_id == *model_id
                && (observation.runtime_instance_id.is_none()
                    || resident.runtime_instance_id == observation.runtime_instance_id)
        })
    {
        return;
    }
    record.model_resource_residency = Some(RuntimeModelResourceResidency {
        model_id: model_id.clone(),
        runtime_instance_id: observation.runtime_instance_id.clone(),
        requirements: None,
    });
}

pub(crate) fn resident_bytes(
    record: &RuntimeRegistryRecord,
    kind: RuntimeAdmissionResourceKind,
    require_known: bool,
) -> Result<u64, RuntimeRegistryError> {
    let Some(resident) = record.model_resource_residency.as_ref() else {
        return Ok(0);
    };
    let claim =
        reservation_claim_from_requirements(&record.runtime_id, resident.requirements.as_ref())?;
    let bytes = match kind {
        RuntimeAdmissionResourceKind::RamBytes => claim.ram_bytes,
        RuntimeAdmissionResourceKind::VramBytes => claim.vram_bytes,
    };
    if require_known && bytes.is_none() {
        return Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable {
            runtime_id: record.runtime_id.clone(),
            model_id: resident.model_id.clone(),
            resource_kind: kind.resource_label(),
        });
    }
    Ok(bytes.unwrap_or(0))
}

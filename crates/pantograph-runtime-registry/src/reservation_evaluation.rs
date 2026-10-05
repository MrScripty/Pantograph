//! Read-only candidate admission observations, followed by authoritative commit.

use crate::{
    available_budget_bytes, reservation_claim_from_requirements, total_reserved_resource_bytes,
    validate_reservation_request, RuntimeAdmissionResourceKind, RuntimeRegistry,
    RuntimeRegistryError, RuntimeRegistryStatus, RuntimeReservationLease,
    RuntimeReservationRequest,
};
use pantograph_runtime_identity::canonical_runtime_id;

/// Advisory capacity observations for one request, not a resource lease.
///
/// The registry's configured runtime budgets remain the accounting domain. These
/// observations do not certify physical host-wide capacity or model capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeReservationAdmissionObservation {
    pub runtime_id: String,
    pub runtime_status: RuntimeRegistryStatus,
    pub runtime_instance_id: Option<String>,
    pub observed_at_ms: u64,
    /// An existing same-owner lease excluded while evaluating its replacement.
    pub replaces_reservation_id: Option<u64>,
    pub resources: Vec<RuntimeReservationResourceObservation>,
    /// Explicit shared backing capacities, in addition to runtime-local budgets.
    pub resource_domains: Vec<crate::RuntimeResourceDomainObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeReservationResourceObservation {
    pub kind: RuntimeAdmissionResourceKind,
    pub requested_bytes: u64,
    pub reserved_bytes: u64,
    /// Missing capacity is unknown, never a measurement of unlimited memory.
    pub capacity_bytes: Option<u64>,
    pub safety_margin_bytes: u64,
    pub available_bytes: Option<u64>,
}

/// An immutable request and its advisory observation, tied to its registry.
///
/// Evaluation does not allocate a lease or consume a lease ID. It can be dropped
/// freely. Commit consumes the evaluation and rechecks current registry state;
/// the observation never grants authority to use capacity that has since changed.
#[must_use]
pub struct RuntimeReservationEvaluation<'a> {
    registry: &'a RuntimeRegistry,
    request: RuntimeReservationRequest,
    observation: RuntimeReservationAdmissionObservation,
}

impl std::fmt::Debug for RuntimeReservationEvaluation<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeReservationEvaluation")
            .field("request", &self.request)
            .field("observation", &self.observation)
            .finish_non_exhaustive()
    }
}

impl RuntimeReservationEvaluation<'_> {
    pub fn observation(&self) -> &RuntimeReservationAdmissionObservation {
        &self.observation
    }

    pub fn request(&self) -> &RuntimeReservationRequest {
        &self.request
    }

    /// Revalidate capacity, owner, status and observed runtime instance under the lock.
    pub fn commit(self) -> Result<RuntimeReservationLease, RuntimeRegistryError> {
        self.registry
            .acquire_reservation_observed(self.request, Some(&self.observation))
    }
}

impl RuntimeRegistry {
    pub fn evaluate_reservation(
        &self,
        request: RuntimeReservationRequest,
    ) -> Result<RuntimeReservationEvaluation<'_>, RuntimeRegistryError> {
        let runtime_id = canonical_runtime_id(&request.runtime_id);
        let guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let existing = request.reservation_owner_id.as_deref().and_then(|owner| {
            guard
                .reservations
                .values()
                .find(|lease| lease.reservation_owner_id.as_deref() == Some(owner))
        });
        if let Some(existing) = existing {
            if existing.runtime_id != runtime_id {
                return Err(RuntimeRegistryError::ReservationOwnerConflict {
                    owner_id: existing
                        .reservation_owner_id
                        .clone()
                        .expect("matched owner"),
                    existing_runtime_id: existing.runtime_id.clone(),
                    requested_runtime_id: runtime_id,
                });
            }
        }
        let replaces_reservation_id = existing.map(|lease| lease.reservation_id);
        validate_reservation_request(
            &guard,
            &runtime_id,
            request.requirements.as_ref(),
            replaces_reservation_id,
        )?;
        let record = guard.runtimes.get(&runtime_id).expect("validated runtime");
        let claim =
            reservation_claim_from_requirements(&runtime_id, request.requirements.as_ref())?;
        let mut resources = Vec::new();
        for (kind, requested) in [
            (RuntimeAdmissionResourceKind::RamBytes, claim.ram_bytes),
            (RuntimeAdmissionResourceKind::VramBytes, claim.vram_bytes),
        ] {
            let Some(requested_bytes) = requested else {
                continue;
            };
            let reserved_bytes = total_reserved_resource_bytes(
                &runtime_id,
                kind.resource_label(),
                &guard.reservations,
                replaces_reservation_id,
                |lease| match kind {
                    RuntimeAdmissionResourceKind::RamBytes => lease.claim.ram_bytes,
                    RuntimeAdmissionResourceKind::VramBytes => lease.claim.vram_bytes,
                },
            )?;
            let budget = record
                .admission_budget
                .as_ref()
                .and_then(|budget| budget.resource_budget(kind));
            let capacity_bytes = budget.and_then(|budget| budget.total_bytes);
            let available_bytes = capacity_bytes
                .map(|_| {
                    available_budget_bytes(
                        &runtime_id,
                        kind.resource_label(),
                        budget,
                        reserved_bytes,
                    )
                })
                .transpose()?;
            resources.push(RuntimeReservationResourceObservation {
                kind,
                requested_bytes,
                reserved_bytes,
                capacity_bytes,
                safety_margin_bytes: budget.map(|budget| budget.safety_margin_bytes).unwrap_or(0),
                available_bytes,
            });
        }
        let resource_domains = crate::resource_domain::domain_observations(
            &guard,
            &runtime_id,
            claim,
            replaces_reservation_id,
        )?;
        Ok(RuntimeReservationEvaluation {
            registry: self,
            request,
            observation: RuntimeReservationAdmissionObservation {
                runtime_id,
                runtime_status: record.status,
                runtime_instance_id: record.runtime_instance_id.clone(),
                observed_at_ms: crate::unix_timestamp_ms(),
                replaces_reservation_id,
                resources,
                resource_domains,
            },
        })
    }
}

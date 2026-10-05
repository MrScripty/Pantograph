//! Explicit shared backing capacities for runtime reservation admission.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::admission::RuntimeReservationClaim;
use crate::{
    canonical_runtime_id, RuntimeAdmissionResourceKind, RuntimeRegistry, RuntimeRegistryError,
    RuntimeRegistryState,
};

/// One logical resource charged against a physical backing pool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct RuntimeResourceDomainBinding {
    pub runtime_id: String,
    pub resource_kind: RuntimeAdmissionResourceKind,
}

/// Operator-supplied capacity; membership does not imply device discovery.
///
/// Bind RAM from multiple runtimes to a host pool, VRAM to a shared device pool,
/// or both kinds to one unified-memory pool. Distinct copies are charged fully;
/// this contract does not infer aliases or deduplicate model content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeResourceDomain {
    pub domain_id: String,
    pub total_bytes: u64,
    #[serde(default)]
    pub safety_margin_bytes: u64,
    pub bindings: Vec<RuntimeResourceDomainBinding>,
}

/// Optional persisted application configuration projected into startup composition.
/// Other application settings are owned by the enclosing application config.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeResourceDomainConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_resource_domains: Vec<RuntimeResourceDomain>,
}

impl RuntimeResourceDomainConfig {
    /// Build the registry consumed by host composition. No partial configuration
    /// is published if a declaration is invalid. Declared known runtime IDs are
    /// registered without capabilities, readiness, model facts or backend keys;
    /// ordinary producer reconciliation remains authoritative for those facts.
    pub fn compose_registry(&self) -> Result<RuntimeRegistry, RuntimeRegistryError> {
        let registry = RuntimeRegistry::new();
        let mut ids = BTreeSet::new();
        for domain in &self.runtime_resource_domains {
            if !ids.insert(domain.domain_id.as_str()) {
                return Err(invalid(domain, "duplicate declared domain id"));
            }
            for binding in &domain.bindings {
                let runtime_id = canonical_runtime_id(&binding.runtime_id);
                let Some(display_name) =
                    pantograph_runtime_identity::runtime_display_name(&runtime_id)
                else {
                    return Err(invalid(domain, "unknown declared runtime id"));
                };
                registry
                    .register_runtime(crate::RuntimeRegistration::new(runtime_id, display_name));
            }
            registry.configure_resource_domain(domain.clone())?;
        }
        Ok(registry)
    }
}

/// Read-only accounting at evaluation time, never execution authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeResourceDomainObservation {
    pub domain_id: String,
    pub requested_bytes: u64,
    pub reserved_bytes: u64,
    /// Resident estimates are part of reserved_bytes, separate from task leases.
    pub resident_bytes: u64,
    pub total_bytes: u64,
    pub safety_margin_bytes: u64,
    pub available_bytes: u64,
}

impl RuntimeRegistry {
    /// Add a shared capacity or change its budget under the admission lock.
    ///
    /// Bindings are canonicalized and remain fixed once configured. Each logical
    /// runtime resource belongs to at most one domain. Budget changes must cover
    /// every live claim, including capacity held for provisional rollback.
    /// Unconfigured resources retain the existing runtime-local behavior.
    pub fn configure_resource_domain(
        &self,
        mut domain: RuntimeResourceDomain,
    ) -> Result<(), RuntimeRegistryError> {
        if domain.domain_id.trim().is_empty()
            || domain.domain_id.len() > 256
            || domain.domain_id.chars().any(char::is_control)
        {
            return Err(invalid(&domain, "a bounded nonempty domain id is required"));
        }
        if domain.bindings.is_empty() {
            return Err(invalid(
                &domain,
                "at least one resource binding is required",
            ));
        }
        if domain.safety_margin_bytes > domain.total_bytes {
            return Err(invalid(&domain, "safety margin exceeds capacity"));
        }
        let mut state = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let mut bindings = BTreeSet::new();
        for binding in &mut domain.bindings {
            binding.runtime_id = canonical_runtime_id(&binding.runtime_id);
            if !state.runtimes.contains_key(&binding.runtime_id) {
                return Err(RuntimeRegistryError::RuntimeNotFound(
                    binding.runtime_id.clone(),
                ));
            }
            if !bindings.insert(binding.clone()) {
                return Err(invalid(&domain, "duplicate logical resource binding"));
            }
        }
        domain.bindings = bindings.into_iter().collect();
        if let Some(previous) = state.resource_domains.get(&domain.domain_id) {
            if previous.bindings != domain.bindings {
                return Err(invalid(
                    &domain,
                    "configured resource bindings cannot change",
                ));
            }
        }
        if state.resource_domains.values().any(|other| {
            other.domain_id != domain.domain_id
                && other
                    .bindings
                    .iter()
                    .any(|binding| domain.bindings.contains(binding))
        }) {
            return Err(invalid(
                &domain,
                "logical resource already belongs to another domain",
            ));
        }
        let reserved = reserved_bytes(&state, &domain, None, false)?;
        if reserved > domain.total_bytes - domain.safety_margin_bytes {
            return Err(invalid(
                &domain,
                "capacity does not cover live reservations",
            ));
        }
        state
            .resource_domains
            .insert(domain.domain_id.clone(), domain);
        Ok(())
    }
}

fn invalid(domain: &RuntimeResourceDomain, reason: &'static str) -> RuntimeRegistryError {
    RuntimeRegistryError::InvalidResourceDomain {
        domain_id: domain.domain_id.clone(),
        reason,
    }
}

fn add(
    domain: &RuntimeResourceDomain,
    total: u64,
    bytes: u64,
) -> Result<u64, RuntimeRegistryError> {
    total
        .checked_add(bytes)
        .ok_or_else(|| RuntimeRegistryError::ResourceDomainAccountingOverflow {
            domain_id: domain.domain_id.clone(),
        })
}

fn claim_bytes(
    domain: &RuntimeResourceDomain,
    runtime_id: &str,
    claim: RuntimeReservationClaim,
) -> Result<u64, RuntimeRegistryError> {
    domain
        .bindings
        .iter()
        .filter(|binding| binding.runtime_id == runtime_id)
        .try_fold(0, |total, binding| {
            let bytes = match binding.resource_kind {
                RuntimeAdmissionResourceKind::RamBytes => claim.ram_bytes,
                RuntimeAdmissionResourceKind::VramBytes => claim.vram_bytes,
            };
            add(domain, total, bytes.unwrap_or(0))
        })
}

fn reserved_bytes(
    state: &RuntimeRegistryState,
    domain: &RuntimeResourceDomain,
    excluded: Option<u64>,
    require_known: bool,
) -> Result<u64, RuntimeRegistryError> {
    let resident = resident_domain_bytes(state, domain, require_known)?;
    state
        .reservations
        .values()
        .filter(|lease| Some(lease.reservation_id) != excluded)
        .try_fold(resident, |total, lease| {
            add(
                domain,
                total,
                claim_bytes(domain, &lease.runtime_id, lease.claim)?,
            )
        })
}

fn resident_domain_bytes(
    state: &RuntimeRegistryState,
    domain: &RuntimeResourceDomain,
    require_known: bool,
) -> Result<u64, RuntimeRegistryError> {
    domain.bindings.iter().try_fold(0, |total, binding| {
        let record = state
            .runtimes
            .get(&binding.runtime_id)
            .expect("configured runtime");
        add(
            domain,
            total,
            crate::model_resources::resident_bytes(record, binding.resource_kind, require_known)?,
        )
    })
}

pub(crate) fn validate_resident_domain_capacity(
    state: &RuntimeRegistryState,
    runtime_id: &str,
    claim: RuntimeReservationClaim,
) -> Result<(), RuntimeRegistryError> {
    for domain in state.resource_domains.values() {
        // Other loaded members may not yet have declarations. Their unknown
        // state blocks admission, but must not prevent publishing known facts.
        let reserved = reserved_bytes(state, domain, None, false)?;
        let capacity = domain.total_bytes - domain.safety_margin_bytes;
        if reserved > capacity {
            let requested = claim_bytes(domain, runtime_id, claim)?;
            return Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
                runtime_id: runtime_id.to_string(),
                domain_id: domain.domain_id.clone(),
                requested_bytes: requested,
                available_bytes: capacity.saturating_sub(reserved - requested),
            });
        }
    }
    Ok(())
}

pub(crate) fn domain_observations(
    state: &RuntimeRegistryState,
    runtime_id: &str,
    claim: RuntimeReservationClaim,
    excluded: Option<u64>,
) -> Result<Vec<RuntimeResourceDomainObservation>, RuntimeRegistryError> {
    state
        .resource_domains
        .values()
        .filter(|domain| {
            domain
                .bindings
                .iter()
                .any(|binding| binding.runtime_id == runtime_id)
        })
        .map(|domain| {
            let reserved_bytes = reserved_bytes(state, domain, excluded, true)?;
            Ok(RuntimeResourceDomainObservation {
                domain_id: domain.domain_id.clone(),
                requested_bytes: claim_bytes(domain, runtime_id, claim)?,
                reserved_bytes,
                resident_bytes: resident_domain_bytes(state, domain, true)?,
                total_bytes: domain.total_bytes,
                safety_margin_bytes: domain.safety_margin_bytes,
                // Configuration and every mutation uphold this subtraction.
                available_bytes: domain.total_bytes - domain.safety_margin_bytes - reserved_bytes,
            })
        })
        .collect()
}

pub(crate) fn validate_domain_admission(
    state: &RuntimeRegistryState,
    runtime_id: &str,
    claim: RuntimeReservationClaim,
    excluded: Option<u64>,
) -> Result<(), RuntimeRegistryError> {
    for observation in domain_observations(state, runtime_id, claim, excluded)? {
        if observation.requested_bytes > observation.available_bytes {
            return Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
                runtime_id: runtime_id.to_string(),
                domain_id: observation.domain_id,
                requested_bytes: observation.requested_bytes,
                available_bytes: observation.available_bytes,
            });
        }
    }
    Ok(())
}

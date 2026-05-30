use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use pantograph_dependency_planning::{
    DependencyBindingId, DependencyPlanningContractError, DependencyPlanningIdentityKey,
    DependencyRequirement, DependencyRequirementBinding, DependencyRequirementName,
    DependencyRequirementsId, ValidatedDependencyEnvironmentRequest,
    ValidatedDependencyEnvironmentResult,
};

/// Concrete dependency requirements and bindings needed by host readiness probes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyRequirementsPayload {
    pub dependency_requirements_id: DependencyRequirementsId,
    pub identity_key: DependencyPlanningIdentityKey,
    pub requirements: Vec<DependencyRequirement>,
    pub bindings: Vec<DependencyRequirementBinding>,
    pub selected_binding_ids: Vec<DependencyBindingId>,
}

impl DependencyRequirementsPayload {
    /// Creates a validated path-free requirements payload.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the payload cannot drive host readiness
    /// probes without reconstructing planning policy from legacy state.
    pub fn new(
        dependency_requirements_id: DependencyRequirementsId,
        identity_key: DependencyPlanningIdentityKey,
        requirements: Vec<DependencyRequirement>,
        bindings: Vec<DependencyRequirementBinding>,
        selected_binding_ids: Vec<DependencyBindingId>,
    ) -> Result<Self, DependencyRequirementsRegistryError> {
        let payload = Self {
            dependency_requirements_id,
            identity_key,
            requirements,
            bindings,
            selected_binding_ids,
        };
        payload.validate()?;
        Ok(payload)
    }

    /// Extracts a requirements payload from a validated environment result.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the result does not contain the concrete
    /// requirement/binding rows required by readiness probes.
    pub fn from_result(
        result: &ValidatedDependencyEnvironmentResult,
    ) -> Result<Self, DependencyRequirementsRegistryError> {
        let result = result.as_result();
        let Some(dependency_requirements_id) = result.dependency_requirements_id.clone() else {
            return Err(DependencyRequirementsRegistryError::MissingRequirementsId);
        };
        Self::new(
            dependency_requirements_id,
            result.identity_key.clone(),
            result.requirements.clone(),
            result.bindings.clone(),
            result.selected_binding_ids.clone(),
        )
    }

    fn validate(&self) -> Result<(), DependencyRequirementsRegistryError> {
        self.identity_key
            .validate()
            .map_err(DependencyRequirementsRegistryError::InvalidContract)?;
        if self.requirements.is_empty() {
            return Err(DependencyRequirementsRegistryError::InvalidPayload {
                field: "dependency_requirements_payload.requirements",
                reason: "requirements payload must include at least one requirement",
            });
        }
        if self.bindings.is_empty() {
            return Err(DependencyRequirementsRegistryError::InvalidPayload {
                field: "dependency_requirements_payload.bindings",
                reason: "requirements payload must include at least one binding",
            });
        }
        if self.selected_binding_ids.is_empty() {
            return Err(DependencyRequirementsRegistryError::InvalidPayload {
                field: "dependency_requirements_payload.selected_binding_ids",
                reason: "requirements payload must include at least one selected binding",
            });
        }

        let requirement_names = self
            .requirements
            .iter()
            .map(|requirement| {
                requirement
                    .validate()
                    .map_err(DependencyRequirementsRegistryError::InvalidContract)?;
                Ok(requirement.name.clone())
            })
            .collect::<Result<BTreeSet<DependencyRequirementName>, _>>()?;

        let binding_ids = self
            .bindings
            .iter()
            .map(|binding| {
                binding
                    .validate()
                    .map_err(DependencyRequirementsRegistryError::InvalidContract)?;
                if !requirement_names.contains(&binding.requirement_name) {
                    return Err(DependencyRequirementsRegistryError::InvalidPayload {
                        field: "dependency_requirements_payload.bindings.requirement_name",
                        reason: "binding references an unknown requirement",
                    });
                }
                Ok(binding.binding_id.clone())
            })
            .collect::<Result<BTreeSet<DependencyBindingId>, _>>()?;

        for selected_binding_id in &self.selected_binding_ids {
            if !binding_ids.contains(selected_binding_id) {
                return Err(DependencyRequirementsRegistryError::InvalidPayload {
                    field: "dependency_requirements_payload.selected_binding_ids",
                    reason: "selected binding id is not present in bindings",
                });
            }
        }

        Ok(())
    }
}

/// Freshness state for dependency requirements registry entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyRequirementsRegistryStatus {
    Fresh,
    Stale,
}

/// Backend registry entry keyed by `DependencyRequirementsId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyRequirementsRegistryEntry {
    pub payload: DependencyRequirementsPayload,
    pub status: DependencyRequirementsRegistryStatus,
}

impl DependencyRequirementsRegistryEntry {
    #[must_use]
    pub fn fresh(payload: DependencyRequirementsPayload) -> Self {
        Self {
            payload,
            status: DependencyRequirementsRegistryStatus::Fresh,
        }
    }

    #[must_use]
    pub fn stale(payload: DependencyRequirementsPayload) -> Self {
        Self {
            payload,
            status: DependencyRequirementsRegistryStatus::Stale,
        }
    }
}

/// Narrow lookup contract used by readiness producers before host probes.
pub trait DependencyRequirementsRegistry: Send + Sync {
    fn lookup_requirements(
        &self,
        dependency_requirements_id: &DependencyRequirementsId,
    ) -> Option<DependencyRequirementsRegistryEntry>;
}

impl<T> DependencyRequirementsRegistry for Arc<T>
where
    T: DependencyRequirementsRegistry + ?Sized,
{
    fn lookup_requirements(
        &self,
        dependency_requirements_id: &DependencyRequirementsId,
    ) -> Option<DependencyRequirementsRegistryEntry> {
        (**self).lookup_requirements(dependency_requirements_id)
    }
}

pub type SharedDependencyRequirementsRegistry = Arc<dyn DependencyRequirementsRegistry>;

/// Synchronous in-memory dependency requirements registry.
#[derive(Debug, Clone, Default)]
pub struct InMemoryDependencyRequirementsRegistry {
    entries: Arc<Mutex<BTreeMap<DependencyRequirementsId, DependencyRequirementsRegistryEntry>>>,
}

impl InMemoryDependencyRequirementsRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces a fresh registry payload.
    pub fn insert_payload(&self, payload: DependencyRequirementsPayload) {
        self.insert_entry(DependencyRequirementsRegistryEntry::fresh(payload));
    }

    /// Inserts or replaces a registry entry.
    pub fn insert_entry(&self, entry: DependencyRequirementsRegistryEntry) {
        self.entries
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(entry.payload.dependency_requirements_id.clone(), entry);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl DependencyRequirementsRegistry for InMemoryDependencyRequirementsRegistry {
    fn lookup_requirements(
        &self,
        dependency_requirements_id: &DependencyRequirementsId,
    ) -> Option<DependencyRequirementsRegistryEntry> {
        self.entries
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(dependency_requirements_id)
            .cloned()
    }
}

/// Resolves a validated request to a concrete, fresh requirements payload.
///
/// # Errors
///
/// Returns typed diagnostics for missing ids, missing registry entries, stale
/// entries, or identity mismatches. Callers must not reconstruct this payload
/// from graph, frontend, preview, load-target, or legacy path data.
pub fn resolve_dependency_requirements_payload(
    registry: &dyn DependencyRequirementsRegistry,
    request: &ValidatedDependencyEnvironmentRequest,
) -> Result<DependencyRequirementsPayload, DependencyRequirementsRegistryError> {
    let request = request.as_request();
    let Some(dependency_requirements_id) = request.dependency_requirements_id.as_ref() else {
        return Err(DependencyRequirementsRegistryError::MissingRequirementsId);
    };
    let Some(entry) = registry.lookup_requirements(dependency_requirements_id) else {
        return Err(DependencyRequirementsRegistryError::MissingPayload {
            dependency_requirements_id: dependency_requirements_id.clone(),
        });
    };
    if entry.status == DependencyRequirementsRegistryStatus::Stale {
        return Err(DependencyRequirementsRegistryError::StalePayload {
            dependency_requirements_id: dependency_requirements_id.clone(),
        });
    }
    if entry.payload.identity_key != request.identity_key {
        return Err(DependencyRequirementsRegistryError::MismatchedPayload {
            dependency_requirements_id: dependency_requirements_id.clone(),
            field: "dependency_requirements_payload.identity_key",
        });
    }
    Ok(entry.payload)
}

/// Errors returned by dependency requirements registry contracts.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DependencyRequirementsRegistryError {
    #[error("dependency requirements id is required")]
    MissingRequirementsId,
    #[error("dependency requirements payload is missing for {dependency_requirements_id}")]
    MissingPayload {
        dependency_requirements_id: DependencyRequirementsId,
    },
    #[error("dependency requirements payload is stale for {dependency_requirements_id}")]
    StalePayload {
        dependency_requirements_id: DependencyRequirementsId,
    },
    #[error(
        "dependency requirements payload for {dependency_requirements_id} does not match request: {field}"
    )]
    MismatchedPayload {
        dependency_requirements_id: DependencyRequirementsId,
        field: &'static str,
    },
    #[error("dependency requirements payload field is invalid: {field}: {reason}")]
    InvalidPayload {
        field: &'static str,
        reason: &'static str,
    },
    #[error("dependency requirements payload contract is invalid: {0}")]
    InvalidContract(DependencyPlanningContractError),
}

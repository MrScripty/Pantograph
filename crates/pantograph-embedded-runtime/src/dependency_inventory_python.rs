//! Python package dependency inventory provider.
//!
//! The provider keeps Python probe request shaping and probe-outcome projection
//! out of the inventory dispatcher so dispatch remains provider-agnostic.

use std::sync::Arc;

use async_trait::async_trait;

use crate::dependency_environment_probe_selector::python_probe_request_for_payload;
use crate::dependency_environment_probe_snapshot::{
    dependency_inventory_observations_from_probe_outcome, invalid_probe_shape_observations,
};
use crate::dependency_inventory::{
    DependencyInventoryObservation, DependencyInventoryProvider, DependencyInventoryRequest,
};
use crate::package_readiness_provider::PackageReadinessProbeRunner;

/// Python package inventory provider backed by the existing no-shell probe.
pub(crate) struct PythonPackageDependencyInventoryProvider {
    package_probe_runner: Arc<dyn PackageReadinessProbeRunner>,
}

impl PythonPackageDependencyInventoryProvider {
    #[must_use]
    pub(crate) fn new(package_probe_runner: Arc<dyn PackageReadinessProbeRunner>) -> Self {
        Self {
            package_probe_runner,
        }
    }
}

#[async_trait]
impl DependencyInventoryProvider for PythonPackageDependencyInventoryProvider {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation {
        let result = match python_probe_request_for_payload(&request.item.request, &request.payload)
        {
            Ok(probe_request) => {
                let outcome = self.package_probe_runner.probe(probe_request).await;
                dependency_inventory_observations_from_probe_outcome(
                    &request.item,
                    &request.payload,
                    outcome,
                )
            }
            Err(error) => invalid_probe_shape_observations(&request.item, &request.payload, error),
        };
        DependencyInventoryObservation::new(result.0, result.1)
    }
}

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
        #[cfg(feature = "backend-pytorch")]
        if crate::inference_text_dependency_requirements::targets_text_environment(
            request.item.request.as_request(),
            &request.payload,
        ) {
            return observe_embedded_text(request).await;
        }
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

#[cfg(feature = "backend-pytorch")]
async fn observe_embedded_text(
    request: DependencyInventoryRequest,
) -> DependencyInventoryObservation {
    use crate::dependency_environment_probe_selector::ProbeShapeError;
    use crate::dependency_environment_probe_snapshot::observations_from_probe_failures;
    use crate::dependency_readiness::PythonPackageReadinessSnapshot;
    use crate::package_readiness_provider::{
        PackageReadinessProbeFailure, PackageReadinessProbeOutcome,
        PackageReadinessProviderDiagnosticCode,
    };
    use inference::{CapabilityAvailabilityId, CapabilityAvailabilityReason};

    if !crate::inference_text_dependency_requirements::is_text_payload(
        request.item.request.as_request(),
        &request.payload,
    ) {
        let (rows, diagnostics) = invalid_probe_shape_observations(
            &request.item,
            &request.payload,
            ProbeShapeError {
                field_path: "dependency_environment.bindings",
                message:
                    "Embedded text inventory requires the exact runtime-owned CPU text profile.",
            },
        );
        return DependencyInventoryObservation::new(rows, diagnostics);
    }
    // The producer retains and joins this metadata-only operation before its
    // next work item or shutdown; executable overrides cannot change its source.
    let observed =
        tokio::task::spawn_blocking(inference::backend::pytorch::observe_text_package_presence)
            .await;
    let (rows, diagnostics) = match observed {
        Ok(Ok(installed)) => {
            let installed = installed
                .iter()
                .map(CapabilityAvailabilityId::parse)
                .collect::<Result<_, _>>()
                .expect("fixed inference-owned distribution ids");
            dependency_inventory_observations_from_probe_outcome(
                &request.item,
                &request.payload,
                PackageReadinessProbeOutcome::Snapshot(PythonPackageReadinessSnapshot::available(
                    installed,
                )),
            )
        }
        Ok(Err(_)) | Err(_) => observations_from_probe_failures(
            &request.item,
            &request.payload,
            vec![PackageReadinessProbeFailure::new(
                PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
                None,
                CapabilityAvailabilityReason::parse(
                    "Embedded Python text package observation failed.",
                )
                .expect("bounded observation reason"),
            )],
        ),
    };
    DependencyInventoryObservation::new(rows, diagnostics)
}

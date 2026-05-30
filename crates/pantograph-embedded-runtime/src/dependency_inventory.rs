//! Dependency inventory service boundary for readiness snapshot production.
//!
//! The snapshot producer owns queue polling and snapshot publication. This
//! module owns provider dispatch for host dependency observations so concrete
//! probes stay behind source-owned infrastructure boundaries.

use std::sync::Arc;

use async_trait::async_trait;
use pantograph_dependency_environment_service::{
    DependencyEnvironmentReadinessSnapshot, DependencyEnvironmentReadinessSnapshotStatus,
    DependencyEnvironmentSnapshotStoreError, DependencyReadinessWorkItem,
    DependencyRequirementsPayload,
};
use pantograph_dependency_planning::{
    dependency_environment_result_from_inventory_observations,
    DependencyInventoryObservationProjection, DependencyInventoryObservationRow,
    DependencyPlanningDiagnostic, ValidatedDependencyInventoryObservationProjection,
};

use crate::dependency_environment_probe_snapshot::environment_ref_for_request;
#[cfg(any(test, feature = "standalone"))]
use crate::dependency_inventory_device_toolchain::DeviceToolchainDependencyInventoryProvider;
#[cfg(any(test, feature = "standalone"))]
use crate::dependency_inventory_device_toolchain_source::DeviceToolchainProviderSource;
#[cfg(feature = "standalone")]
use crate::dependency_inventory_device_toolchain_source::GatewayDeviceToolchainProviderSource;
use crate::dependency_inventory_dispatch::DependencyInventoryDispatchProvider;
#[cfg(feature = "standalone")]
use crate::dependency_inventory_managed_runtime::BlockingManagedRuntimeSnapshotSource;
#[cfg(any(test, feature = "standalone"))]
use crate::dependency_inventory_managed_runtime::{
    ManagedRuntimeDependencyInventoryProvider, ManagedRuntimeSnapshotSource,
};
use crate::dependency_inventory_python::PythonPackageDependencyInventoryProvider;
#[cfg(any(test, feature = "standalone"))]
use crate::dependency_inventory_runtime_feature::RuntimeFeatureDependencyInventoryProvider;
#[cfg(feature = "standalone")]
use crate::dependency_inventory_runtime_feature_source::GatewayRuntimeFeatureProviderSource;
#[cfg(any(test, feature = "standalone"))]
use crate::dependency_inventory_runtime_feature_source::RuntimeFeatureProviderSource;
use crate::package_readiness_provider::PackageReadinessProbeRunner;
use crate::python_package_readiness_probe::ProcessPythonPackageReadinessProbeRunner;

/// Request context passed from the snapshot producer to dependency inventory.
#[derive(Debug, Clone)]
pub(crate) struct DependencyInventoryRequest {
    pub item: DependencyReadinessWorkItem,
    pub payload: DependencyRequirementsPayload,
}

impl DependencyInventoryRequest {
    #[must_use]
    pub fn new(item: &DependencyReadinessWorkItem, payload: DependencyRequirementsPayload) -> Self {
        Self {
            item: item.clone(),
            payload,
        }
    }

    #[must_use]
    pub(crate) fn with_payload(&self, payload: DependencyRequirementsPayload) -> Self {
        Self {
            item: self.item.clone(),
            payload,
        }
    }
}

/// Provider-owned dependency observations for one requirements payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DependencyInventoryObservation {
    pub rows: Vec<DependencyInventoryObservationRow>,
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl DependencyInventoryObservation {
    #[must_use]
    pub fn new(
        rows: Vec<DependencyInventoryObservationRow>,
        diagnostics: Vec<DependencyPlanningDiagnostic>,
    ) -> Self {
        Self { rows, diagnostics }
    }
}

#[async_trait]
pub(crate) trait DependencyInventoryProvider: Send + Sync {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation;
}

/// Inventory service used by the readiness snapshot producer.
#[derive(Clone)]
pub(crate) struct DependencyInventoryService {
    provider: Arc<dyn DependencyInventoryProvider>,
}

impl Default for DependencyInventoryService {
    fn default() -> Self {
        Self::from_package_probe_runner(Arc::new(
            ProcessPythonPackageReadinessProbeRunner::default(),
        ))
    }
}

impl DependencyInventoryService {
    #[must_use]
    pub fn new(provider: Arc<dyn DependencyInventoryProvider>) -> Self {
        Self { provider }
    }

    #[must_use]
    pub fn from_package_probe_runner(
        package_probe_runner: Arc<dyn PackageReadinessProbeRunner>,
    ) -> Self {
        let python_provider = Arc::new(PythonPackageDependencyInventoryProvider::new(
            package_probe_runner,
        ));
        Self::new(Arc::new(DependencyInventoryDispatchProvider::new(
            python_provider,
        )))
    }

    #[must_use]
    #[cfg(feature = "standalone")]
    pub fn from_app_data_dir(
        app_data_dir: std::path::PathBuf,
        gateway: Arc<inference::InferenceGateway>,
    ) -> Self {
        Self::from_package_probe_runner_and_managed_runtime_and_runtime_feature_and_device_toolchain_sources(
            Arc::new(ProcessPythonPackageReadinessProbeRunner::default()),
            Arc::new(BlockingManagedRuntimeSnapshotSource::new(app_data_dir)),
            Arc::new(GatewayRuntimeFeatureProviderSource::new(gateway.clone())),
            Arc::new(GatewayDeviceToolchainProviderSource::new(gateway)),
        )
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn from_package_probe_runner_and_managed_runtime_source(
        package_probe_runner: Arc<dyn PackageReadinessProbeRunner>,
        managed_runtime_source: Arc<dyn ManagedRuntimeSnapshotSource>,
    ) -> Self {
        let python_provider = Arc::new(PythonPackageDependencyInventoryProvider::new(
            package_probe_runner,
        ));
        let managed_runtime_provider = Arc::new(ManagedRuntimeDependencyInventoryProvider::new(
            managed_runtime_source,
        ));
        Self::new(Arc::new(
            DependencyInventoryDispatchProvider::new_with_managed_runtime(
                python_provider,
                managed_runtime_provider,
            ),
        ))
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn from_package_probe_runner_and_managed_runtime_and_runtime_feature_sources(
        package_probe_runner: Arc<dyn PackageReadinessProbeRunner>,
        managed_runtime_source: Arc<dyn ManagedRuntimeSnapshotSource>,
        runtime_feature_source: Arc<dyn RuntimeFeatureProviderSource>,
    ) -> Self {
        let python_provider = Arc::new(PythonPackageDependencyInventoryProvider::new(
            package_probe_runner,
        ));
        let managed_runtime_provider = Arc::new(ManagedRuntimeDependencyInventoryProvider::new(
            managed_runtime_source,
        ));
        let runtime_feature_provider = Arc::new(RuntimeFeatureDependencyInventoryProvider::new(
            runtime_feature_source,
        ));
        Self::new(Arc::new(
            DependencyInventoryDispatchProvider::new_with_managed_runtime_and_runtime_feature(
                python_provider,
                managed_runtime_provider,
                runtime_feature_provider,
            ),
        ))
    }

    #[must_use]
    #[cfg(any(test, feature = "standalone"))]
    pub(crate) fn from_package_probe_runner_and_managed_runtime_and_runtime_feature_and_device_toolchain_sources(
        package_probe_runner: Arc<dyn PackageReadinessProbeRunner>,
        managed_runtime_source: Arc<dyn ManagedRuntimeSnapshotSource>,
        runtime_feature_source: Arc<dyn RuntimeFeatureProviderSource>,
        device_toolchain_source: Arc<dyn DeviceToolchainProviderSource>,
    ) -> Self {
        let python_provider = Arc::new(PythonPackageDependencyInventoryProvider::new(
            package_probe_runner,
        ));
        let managed_runtime_provider = Arc::new(ManagedRuntimeDependencyInventoryProvider::new(
            managed_runtime_source,
        ));
        let runtime_feature_provider = Arc::new(RuntimeFeatureDependencyInventoryProvider::new(
            runtime_feature_source,
        ));
        let device_toolchain_provider = Arc::new(DeviceToolchainDependencyInventoryProvider::new(
            device_toolchain_source,
        ));
        Self::new(Arc::new(
            DependencyInventoryDispatchProvider::new_with_managed_runtime_and_runtime_feature_and_device_toolchain(
                python_provider,
                managed_runtime_provider,
                runtime_feature_provider,
                device_toolchain_provider,
            ),
        ))
    }

    pub async fn snapshot_for_work_item(
        &self,
        item: &DependencyReadinessWorkItem,
        payload: DependencyRequirementsPayload,
    ) -> Result<
        DependencyEnvironmentReadinessSnapshot,
        pantograph_dependency_environment_service::DependencyEnvironmentSnapshotStoreError,
    > {
        let payload_for_projection = payload.clone();
        let request = DependencyInventoryRequest::new(item, payload);
        let observation = self.provider.observe(request).await;
        let result = dependency_environment_result_from_inventory_observation(
            item,
            payload_for_projection,
            observation,
        )?
        .into_inner();
        DependencyEnvironmentReadinessSnapshot::for_request(
            &item.request,
            result,
            DependencyEnvironmentReadinessSnapshotStatus::Fresh,
        )
    }
}

fn dependency_environment_result_from_inventory_observation(
    item: &DependencyReadinessWorkItem,
    payload: DependencyRequirementsPayload,
    observation: DependencyInventoryObservation,
) -> Result<
    pantograph_dependency_planning::ValidatedDependencyEnvironmentResult,
    DependencyEnvironmentSnapshotStoreError,
> {
    let request = item.request.as_request();
    let projection = DependencyInventoryObservationProjection {
        contract_version: 1,
        action: request.action,
        identity_key: request.identity_key.clone(),
        dependency_requirements_id: Some(payload.dependency_requirements_id.clone()),
        environment_ref: Some(environment_ref_for_request(item)),
        requirements: payload.requirements,
        bindings: payload.bindings,
        selected_binding_ids: payload.selected_binding_ids,
        observations: observation.rows,
        diagnostics: observation.diagnostics,
    };
    let projection = ValidatedDependencyInventoryObservationProjection::try_from(projection)
        .map_err(DependencyEnvironmentSnapshotStoreError::InvalidSnapshotResult)?;
    dependency_environment_result_from_inventory_observations(&projection)
        .map_err(DependencyEnvironmentSnapshotStoreError::InvalidSnapshotResult)
}

//! Dependency inventory service boundary for readiness snapshot production.
//!
//! The snapshot producer owns queue polling and snapshot publication. This
//! module owns provider dispatch for host dependency observations so concrete
//! probes stay behind source-owned infrastructure boundaries.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use inference::{CapabilityAvailabilityId, CapabilityAvailabilityReason};
use pantograph_dependency_environment_service::{
    DependencyEnvironmentReadinessSnapshot, DependencyEnvironmentReadinessSnapshotStatus,
    DependencyEnvironmentSnapshotStoreError, DependencyReadinessWorkItem,
    DependencyRequirementsPayload,
};
use pantograph_dependency_planning::{
    dependency_environment_result_from_inventory_observations, DependencyEnvironmentKind,
    DependencyInventoryObservationProjection, DependencyInventoryObservationRow,
    DependencyPlanningDiagnostic, DependencyRequirementKind,
    ValidatedDependencyInventoryObservationProjection,
};

use crate::dependency_environment_probe_selector::python_probe_request_for_payload;
use crate::dependency_environment_probe_snapshot::{
    dependency_inventory_observations_from_probe_outcome, environment_ref_for_request,
    invalid_probe_shape_observations, observations_from_probe_failures,
};
use crate::package_readiness_provider::{
    PackageReadinessProbeFailure, PackageReadinessProbeRunner,
    PackageReadinessProviderDiagnosticCode,
};
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

struct DependencyInventoryDispatchProvider {
    python_provider: Arc<dyn DependencyInventoryProvider>,
    not_implemented_provider: NotImplementedDependencyInventoryProvider,
}

impl DependencyInventoryDispatchProvider {
    fn new(python_provider: Arc<dyn DependencyInventoryProvider>) -> Self {
        Self {
            python_provider,
            not_implemented_provider: NotImplementedDependencyInventoryProvider,
        }
    }
}

#[async_trait]
impl DependencyInventoryProvider for DependencyInventoryDispatchProvider {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation {
        if selected_payload_is_python_only(&request.payload) {
            self.python_provider.observe(request).await
        } else {
            self.not_implemented_provider.observe(request).await
        }
    }
}

/// Python package inventory provider backed by the existing no-shell probe.
pub(crate) struct PythonPackageDependencyInventoryProvider {
    package_probe_runner: Arc<dyn PackageReadinessProbeRunner>,
}

impl PythonPackageDependencyInventoryProvider {
    #[must_use]
    pub fn new(package_probe_runner: Arc<dyn PackageReadinessProbeRunner>) -> Self {
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

struct NotImplementedDependencyInventoryProvider;

#[async_trait]
impl DependencyInventoryProvider for NotImplementedDependencyInventoryProvider {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation {
        let failure = PackageReadinessProbeFailure::new(
            PackageReadinessProviderDiagnosticCode::ProbeNotImplemented,
            non_python_dependency_id(&request.payload),
            CapabilityAvailabilityReason::parse(&not_implemented_reason(&request.payload))
                .expect("inventory provider not implemented reason is valid"),
        );
        let (rows, diagnostics) =
            observations_from_probe_failures(&request.item, &request.payload, vec![failure]);
        DependencyInventoryObservation::new(rows, diagnostics)
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

fn selected_payload_is_python_only(payload: &DependencyRequirementsPayload) -> bool {
    let requirement_by_name = payload
        .requirements
        .iter()
        .map(|requirement| (requirement.name.clone(), requirement))
        .collect::<BTreeMap<_, _>>();
    let selected_ids = payload.selected_binding_ids.iter().collect::<BTreeSet<_>>();

    payload.bindings.iter().all(|binding| {
        if !selected_ids.contains(&binding.binding_id) {
            return true;
        }
        binding.environment_kind == DependencyEnvironmentKind::Python
            && requirement_by_name
                .get(&binding.requirement_name)
                .is_some_and(|requirement| {
                    requirement.kind == DependencyRequirementKind::PythonPackage
                })
    })
}

fn non_python_dependency_id(
    payload: &DependencyRequirementsPayload,
) -> Option<CapabilityAvailabilityId> {
    payload
        .requirements
        .iter()
        .find(|requirement| requirement.kind != DependencyRequirementKind::PythonPackage)
        .and_then(|requirement| CapabilityAvailabilityId::parse(requirement.name.as_str()).ok())
}

fn not_implemented_reason(payload: &DependencyRequirementsPayload) -> String {
    let kind = payload
        .requirements
        .iter()
        .find(|requirement| requirement.kind != DependencyRequirementKind::PythonPackage)
        .map(|requirement| requirement_kind_label(requirement.kind))
        .or_else(|| {
            payload
                .bindings
                .iter()
                .find(|binding| binding.environment_kind != DependencyEnvironmentKind::Python)
                .map(|binding| environment_kind_label(binding.environment_kind))
        })
        .unwrap_or("unknown");
    format!("Dependency inventory provider for {kind} requirements is not implemented.")
}

fn requirement_kind_label(kind: DependencyRequirementKind) -> &'static str {
    match kind {
        DependencyRequirementKind::PythonPackage => "python_package",
        DependencyRequirementKind::RuntimeManagedBinary => "runtime_managed_binary",
        DependencyRequirementKind::SystemPackage => "system_package",
        DependencyRequirementKind::RuntimeFeature => "runtime_feature",
        DependencyRequirementKind::DeviceToolchain => "device_toolchain",
        _ => "unknown",
    }
}

fn environment_kind_label(kind: DependencyEnvironmentKind) -> &'static str {
    match kind {
        DependencyEnvironmentKind::Python => "python",
        DependencyEnvironmentKind::ManagedBinary => "managed_binary",
        DependencyEnvironmentKind::SystemPackage => "system_package",
        DependencyEnvironmentKind::RuntimeFeature => "runtime_feature",
        DependencyEnvironmentKind::DeviceToolchain => "device_toolchain",
        _ => "unknown",
    }
}

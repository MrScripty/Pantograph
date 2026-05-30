//! Dependency inventory service boundary for readiness snapshot production.
//!
//! The snapshot producer owns queue polling and snapshot publication. This
//! module owns provider dispatch for host dependency observations so concrete
//! probes stay behind source-owned infrastructure boundaries.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use inference::CapabilityAvailabilityReason;
use pantograph_dependency_environment_service::{
    DependencyEnvironmentReadinessSnapshot, DependencyEnvironmentReadinessSnapshotStatus,
    DependencyEnvironmentSnapshotStoreError, DependencyReadinessWorkItem,
    DependencyRequirementsPayload,
};
use pantograph_dependency_planning::{
    dependency_environment_result_from_inventory_observations, DependencyBindingId,
    DependencyEnvironmentKind, DependencyInventoryObservationProjection,
    DependencyInventoryObservationRow, DependencyPlanningDiagnostic, DependencyRequirementBinding,
    DependencyRequirementKind, DependencyRequirementName,
    ValidatedDependencyInventoryObservationProjection,
};

use crate::dependency_environment_probe_selector::{
    python_probe_request_for_payload, ProbeShapeError,
};
use crate::dependency_environment_probe_snapshot::{
    dependency_inventory_observations_from_probe_outcome, environment_ref_for_request,
    invalid_probe_shape_observations, observations_from_probe_failures,
};
#[cfg(feature = "standalone")]
use crate::dependency_inventory_managed_runtime::BlockingManagedRuntimeSnapshotSource;
#[cfg(any(test, feature = "standalone"))]
use crate::dependency_inventory_managed_runtime::{
    ManagedRuntimeDependencyInventoryProvider, ManagedRuntimeSnapshotSource,
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

    #[must_use]
    fn with_payload(&self, payload: DependencyRequirementsPayload) -> Self {
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
    pub fn from_app_data_dir(app_data_dir: std::path::PathBuf) -> Self {
        Self::from_package_probe_runner_and_managed_runtime_source(
            Arc::new(ProcessPythonPackageReadinessProbeRunner::default()),
            Arc::new(BlockingManagedRuntimeSnapshotSource::new(app_data_dir)),
        )
    }

    #[must_use]
    #[cfg(any(test, feature = "standalone"))]
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
    #[cfg(any(test, feature = "standalone"))]
    managed_runtime_provider: Arc<dyn DependencyInventoryProvider>,
    not_implemented_provider: NotImplementedDependencyInventoryProvider,
}

impl DependencyInventoryDispatchProvider {
    fn new(python_provider: Arc<dyn DependencyInventoryProvider>) -> Self {
        Self {
            python_provider,
            #[cfg(any(test, feature = "standalone"))]
            managed_runtime_provider: Arc::new(NotImplementedDependencyInventoryProvider),
            not_implemented_provider: NotImplementedDependencyInventoryProvider,
        }
    }

    #[cfg(any(test, feature = "standalone"))]
    fn new_with_managed_runtime(
        python_provider: Arc<dyn DependencyInventoryProvider>,
        managed_runtime_provider: Arc<dyn DependencyInventoryProvider>,
    ) -> Self {
        Self {
            python_provider,
            managed_runtime_provider,
            not_implemented_provider: NotImplementedDependencyInventoryProvider,
        }
    }
}

#[async_trait]
impl DependencyInventoryProvider for DependencyInventoryDispatchProvider {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation {
        let dispatch_plan = DependencyInventoryDispatchPlan::for_payload(&request.payload);
        let mut rows = Vec::new();
        let mut diagnostics = Vec::new();

        if !dispatch_plan.invalid_binding_ids.is_empty() {
            let payload = scoped_payload(&request.payload, &dispatch_plan.invalid_binding_ids);
            let (invalid_rows, invalid_diagnostics) = invalid_probe_shape_observations(
                &request.item,
                &payload,
                ProbeShapeError {
                    field_path: "dependency_environment.bindings",
                    message: "Selected dependency binding does not match the referenced requirement kind.",
                },
            );
            rows.extend(invalid_rows);
            diagnostics.extend(invalid_diagnostics);
        }

        if !dispatch_plan.python_binding_ids.is_empty() {
            let payload = scoped_payload(&request.payload, &dispatch_plan.python_binding_ids);
            let observation = self
                .python_provider
                .observe(request.with_payload(payload))
                .await;
            rows.extend(observation.rows);
            diagnostics.extend(observation.diagnostics);
        }

        #[cfg(any(test, feature = "standalone"))]
        if !dispatch_plan.managed_runtime_binding_ids.is_empty() {
            let payload =
                scoped_payload(&request.payload, &dispatch_plan.managed_runtime_binding_ids);
            let observation = self
                .managed_runtime_provider
                .observe(request.with_payload(payload))
                .await;
            rows.extend(observation.rows);
            diagnostics.extend(observation.diagnostics);
        }

        #[cfg(not(any(test, feature = "standalone")))]
        let not_implemented_binding_ids = dispatch_plan
            .not_implemented_binding_ids
            .iter()
            .chain(dispatch_plan.managed_runtime_binding_ids.iter())
            .collect::<Vec<_>>();
        #[cfg(any(test, feature = "standalone"))]
        let not_implemented_binding_ids = dispatch_plan
            .not_implemented_binding_ids
            .iter()
            .collect::<Vec<_>>();

        for binding_id in not_implemented_binding_ids {
            let payload = scoped_payload(&request.payload, std::slice::from_ref(binding_id));
            let observation = self
                .not_implemented_provider
                .observe(request.with_payload(payload))
                .await;
            rows.extend(observation.rows);
            diagnostics.extend(observation.diagnostics);
        }

        DependencyInventoryObservation::new(rows, diagnostics)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DependencyInventoryDispatchPlan {
    python_binding_ids: Vec<DependencyBindingId>,
    managed_runtime_binding_ids: Vec<DependencyBindingId>,
    not_implemented_binding_ids: Vec<DependencyBindingId>,
    invalid_binding_ids: Vec<DependencyBindingId>,
}

impl DependencyInventoryDispatchPlan {
    fn for_payload(payload: &DependencyRequirementsPayload) -> Self {
        let requirement_by_name = payload
            .requirements
            .iter()
            .map(|requirement| (requirement.name.clone(), requirement.kind))
            .collect::<BTreeMap<_, _>>();

        let mut plan = Self::default();
        for binding in selected_bindings(payload) {
            let Some(requirement_kind) = requirement_by_name.get(&binding.requirement_name) else {
                plan.invalid_binding_ids.push(binding.binding_id);
                continue;
            };
            match dispatch_target(binding.environment_kind, *requirement_kind) {
                Some(DependencyInventoryDispatchTarget::PythonPackage) => {
                    plan.python_binding_ids.push(binding.binding_id);
                }
                #[cfg(any(test, feature = "standalone"))]
                Some(DependencyInventoryDispatchTarget::ManagedRuntime) => {
                    plan.managed_runtime_binding_ids.push(binding.binding_id);
                }
                Some(DependencyInventoryDispatchTarget::NotImplemented) => {
                    plan.not_implemented_binding_ids.push(binding.binding_id);
                }
                None => plan.invalid_binding_ids.push(binding.binding_id),
            }
        }
        plan
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DependencyInventoryDispatchTarget {
    PythonPackage,
    #[cfg(any(test, feature = "standalone"))]
    ManagedRuntime,
    NotImplemented,
}

fn dispatch_target(
    environment_kind: DependencyEnvironmentKind,
    requirement_kind: DependencyRequirementKind,
) -> Option<DependencyInventoryDispatchTarget> {
    match (environment_kind, requirement_kind) {
        (DependencyEnvironmentKind::Python, DependencyRequirementKind::PythonPackage) => {
            Some(DependencyInventoryDispatchTarget::PythonPackage)
        }
        (
            DependencyEnvironmentKind::ManagedBinary,
            DependencyRequirementKind::RuntimeManagedBinary,
        ) => Some(managed_runtime_dispatch_target()),
        (DependencyEnvironmentKind::RuntimeFeature, DependencyRequirementKind::RuntimeFeature)
        | (
            DependencyEnvironmentKind::DeviceToolchain,
            DependencyRequirementKind::DeviceToolchain,
        )
        | (DependencyEnvironmentKind::SystemPackage, DependencyRequirementKind::SystemPackage) => {
            Some(DependencyInventoryDispatchTarget::NotImplemented)
        }
        _ => None,
    }
}

#[cfg(any(test, feature = "standalone"))]
fn managed_runtime_dispatch_target() -> DependencyInventoryDispatchTarget {
    DependencyInventoryDispatchTarget::ManagedRuntime
}

#[cfg(not(any(test, feature = "standalone")))]
fn managed_runtime_dispatch_target() -> DependencyInventoryDispatchTarget {
    DependencyInventoryDispatchTarget::NotImplemented
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
            None,
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

fn selected_bindings(payload: &DependencyRequirementsPayload) -> Vec<DependencyRequirementBinding> {
    let selected_ids = payload.selected_binding_ids.iter().collect::<BTreeSet<_>>();
    payload
        .bindings
        .iter()
        .filter(|binding| selected_ids.contains(&binding.binding_id))
        .cloned()
        .collect()
}

fn scoped_payload(
    payload: &DependencyRequirementsPayload,
    selected_binding_ids: &[DependencyBindingId],
) -> DependencyRequirementsPayload {
    let selected_ids = selected_binding_ids.iter().collect::<BTreeSet<_>>();
    let bindings = payload
        .bindings
        .iter()
        .filter(|binding| selected_ids.contains(&binding.binding_id))
        .cloned()
        .collect::<Vec<_>>();
    let requirement_names = bindings
        .iter()
        .map(|binding| binding.requirement_name.clone())
        .collect::<BTreeSet<DependencyRequirementName>>();
    let requirements = payload
        .requirements
        .iter()
        .filter(|requirement| requirement_names.contains(&requirement.name))
        .cloned()
        .collect::<Vec<_>>();
    DependencyRequirementsPayload {
        dependency_requirements_id: payload.dependency_requirements_id.clone(),
        identity_key: payload.identity_key.clone(),
        requirements,
        bindings,
        selected_binding_ids: selected_binding_ids.to_vec(),
    }
}

fn not_implemented_reason(payload: &DependencyRequirementsPayload) -> String {
    let selected_count = payload.selected_binding_ids.len();
    format!(
        "Dependency inventory provider is not implemented for {selected_count} selected binding(s)."
    )
}

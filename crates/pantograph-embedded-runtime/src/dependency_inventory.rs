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
    DependencyReadinessWorkItem, DependencyRequirementsPayload,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentKind, DependencyEnvironmentResult, DependencyRequirementKind,
};

use crate::dependency_environment_probe_selector::python_probe_request_for_payload;
use crate::dependency_environment_probe_snapshot::{
    dependency_environment_result_from_probe_outcome, invalid_probe_shape_result,
    result_from_probe_failures,
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
    pub result: DependencyEnvironmentResult,
}

impl DependencyInventoryObservation {
    #[must_use]
    pub fn new(result: DependencyEnvironmentResult) -> Self {
        Self { result }
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
        let request = DependencyInventoryRequest::new(item, payload);
        let observation = self.provider.observe(request).await;
        DependencyEnvironmentReadinessSnapshot::for_request(
            &item.request,
            observation.result,
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
                dependency_environment_result_from_probe_outcome(
                    &request.item,
                    request.payload,
                    outcome,
                )
            }
            Err(error) => invalid_probe_shape_result(&request.item, &request.payload, error),
        };
        DependencyInventoryObservation::new(result)
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
        DependencyInventoryObservation::new(result_from_probe_failures(
            &request.item,
            request.payload,
            vec![failure],
        ))
    }
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use async_trait::async_trait;
    use inference::{CapabilityAvailabilityId, CapabilityAvailabilityReason};
    use pantograph_dependency_environment_service::{
        DependencyReadinessTaskId, DependencyReadinessWorkItem,
        DependencyReadinessWorkItemProvenance, DependencyReadinessWorkflowRunId,
        DependencyReadinessWorkflowSessionId, DependencyRequirementsPayload,
    };
    use pantograph_dependency_planning::{
        DependencyEnvironmentKind, DependencyEnvironmentReadinessState,
        DependencyEnvironmentRequest, DependencyPlanningDiagnosticCode, DependencyRequirementKind,
        DependencyRequirementsId, ValidatedDependencyEnvironmentRequest,
    };

    use super::DependencyInventoryService;
    use crate::dependency_readiness::PythonPackageReadinessSnapshot;
    use crate::package_readiness_provider::{
        PackageReadinessEnvironmentSelector, PackageReadinessProbeFailure,
        PackageReadinessProbeOutcome, PackageReadinessProbeRequest, PackageReadinessProbeRunner,
        PackageReadinessProviderDiagnosticCode,
    };

    #[tokio::test]
    async fn inventory_service_routes_python_payloads_through_package_probe() {
        let request = validated_request();
        let item = work_item(request.clone());
        let payload = default_host_requirements_payload(&request);
        let probe_runner = Arc::new(FakePackageProbeRunner::new(
            PackageReadinessProbeOutcome::Snapshot(PythonPackageReadinessSnapshot::available(
                installed_package_ids(&["diffusers"]),
            )),
        ));
        let inventory = DependencyInventoryService::from_package_probe_runner(probe_runner.clone());

        let snapshot = inventory
            .snapshot_for_work_item(&item, payload)
            .await
            .expect("snapshot");

        assert_eq!(
            snapshot.result.readiness_state,
            DependencyEnvironmentReadinessState::Ready
        );
        let probe_requests = probe_runner.requests();
        assert_eq!(probe_requests.len(), 1);
        assert_eq!(
            probe_requests[0].environment,
            PackageReadinessEnvironmentSelector::DefaultHostPython
        );
    }

    #[tokio::test]
    async fn inventory_service_reports_not_implemented_for_non_python_payloads_without_probe() {
        let request = validated_request();
        let item = work_item(request.clone());
        let mut payload = default_host_requirements_payload(&request);
        payload.requirements[0].kind = DependencyRequirementKind::RuntimeManagedBinary;
        payload.requirements[0].python = None;
        payload.requirements[0].managed_runtime = Some(
            serde_json::from_value(serde_json::json!({
                "managed_binary_id": "llama_cpp"
            }))
            .expect("managed runtime requirement details"),
        );
        payload.bindings[0].environment_kind = DependencyEnvironmentKind::ManagedBinary;
        payload.bindings[0].python = None;
        payload.bindings[0].managed_runtime = Some(
            serde_json::from_value(serde_json::json!({
                "managed_binary_id": "llama_cpp"
            }))
            .expect("managed runtime binding details"),
        );
        let probe_runner = Arc::new(FakePackageProbeRunner::new(
            PackageReadinessProbeOutcome::Failed(vec![PackageReadinessProbeFailure::new(
                PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
                None,
                CapabilityAvailabilityReason::parse("probe should not be called").expect("reason"),
            )]),
        ));
        let inventory = DependencyInventoryService::from_package_probe_runner(probe_runner.clone());

        let snapshot = inventory
            .snapshot_for_work_item(&item, payload)
            .await
            .expect("snapshot");

        assert_eq!(
            snapshot.result.readiness_state,
            DependencyEnvironmentReadinessState::Unavailable
        );
        assert_eq!(
            snapshot
                .result
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.code.clone()),
            Some(DependencyPlanningDiagnosticCode::NotImplemented)
        );
        assert_eq!(
            snapshot
                .result
                .binding_statuses
                .first()
                .map(|status| status.state),
            Some(pantograph_dependency_planning::DependencyBindingStatusState::NotImplemented)
        );
        assert!(probe_runner.requests().is_empty());
    }

    fn work_item(request: ValidatedDependencyEnvironmentRequest) -> DependencyReadinessWorkItem {
        DependencyReadinessWorkItem::new(
            DependencyReadinessWorkItemProvenance::new(
                DependencyReadinessWorkflowSessionId::parse("session.001").expect("session id"),
                DependencyReadinessWorkflowRunId::parse("run.001").expect("run id"),
                DependencyReadinessTaskId::parse("infer").expect("task id"),
            ),
            request,
        )
    }

    fn validated_request() -> ValidatedDependencyEnvironmentRequest {
        let mut request: DependencyEnvironmentRequest = serde_json::from_str(include_str!(
            "../../pantograph-dependency-planning/tests/fixtures/dependency_environment_resolve_request.json"
        ))
        .expect("request fixture should parse");
        request.dependency_requirements_id = Some(
            DependencyRequirementsId::parse("tiny-sd:pytorch:linux-x86_64:torch-diffusers")
                .expect("requirements id"),
        );
        ValidatedDependencyEnvironmentRequest::try_from(request)
            .expect("request fixture should validate")
    }

    fn default_host_requirements_payload(
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyRequirementsPayload {
        let mut result: pantograph_dependency_planning::DependencyEnvironmentResult =
            serde_json::from_str(include_str!(
                "../../pantograph-dependency-planning/tests/fixtures/dependency_environment_ready_result.json"
            ))
            .expect("ready fixture should decode");
        result.action = request.as_request().action;
        result.identity_key = request.as_request().identity_key.clone();
        result.dependency_requirements_id = request.as_request().dependency_requirements_id.clone();
        result.selected_binding_ids = request
            .as_request()
            .identity_key
            .selected_binding_ids
            .clone();
        for binding in &mut result.bindings {
            binding.profile_id = None;
        }
        let result =
            pantograph_dependency_planning::ValidatedDependencyEnvironmentResult::try_from(result)
                .expect("ready result should validate");
        DependencyRequirementsPayload::from_result(&result).expect("requirements payload")
    }

    fn installed_package_ids(values: &[&str]) -> BTreeSet<CapabilityAvailabilityId> {
        values
            .iter()
            .map(|value| CapabilityAvailabilityId::parse(value).expect("valid package id"))
            .collect()
    }

    #[derive(Debug)]
    struct FakePackageProbeRunner {
        outcome: PackageReadinessProbeOutcome,
        requests: std::sync::Mutex<Vec<PackageReadinessProbeRequest>>,
    }

    impl FakePackageProbeRunner {
        fn new(outcome: PackageReadinessProbeOutcome) -> Self {
            Self {
                outcome,
                requests: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn requests(&self) -> Vec<PackageReadinessProbeRequest> {
            self.requests
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone()
        }
    }

    #[async_trait]
    impl PackageReadinessProbeRunner for FakePackageProbeRunner {
        async fn probe(
            &self,
            request: PackageReadinessProbeRequest,
        ) -> PackageReadinessProbeOutcome {
            self.requests
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(request);
            self.outcome.clone()
        }
    }
}

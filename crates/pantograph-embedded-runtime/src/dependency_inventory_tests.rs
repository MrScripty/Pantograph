use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use inference::{CapabilityAvailabilityId, CapabilityAvailabilityReason};
use pantograph_dependency_environment_service::{
    DependencyReadinessTaskId, DependencyReadinessWorkItem, DependencyReadinessWorkItemProvenance,
    DependencyReadinessWorkflowRunId, DependencyReadinessWorkflowSessionId,
    DependencyRequirementsPayload,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentKind, DependencyEnvironmentReadinessState, DependencyEnvironmentRequest,
    DependencyPlanningDiagnosticCode, DependencyRequirementKind, DependencyRequirementsId,
    ValidatedDependencyEnvironmentRequest,
};

use crate::dependency_inventory::DependencyInventoryService;
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
        DependencyEnvironmentReadinessState::NotImplemented
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
    async fn probe(&self, request: PackageReadinessProbeRequest) -> PackageReadinessProbeOutcome {
        self.requests
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(request);
        self.outcome.clone()
    }
}

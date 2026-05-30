use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use inference::{CapabilityAvailabilityId, CapabilityAvailabilityReason};
use pantograph_dependency_environment_service::{
    DependencyEnvironmentProvider, DependencyEnvironmentReadinessSnapshotProvider,
    DependencyReadinessTaskId, DependencyReadinessWorkItem, DependencyReadinessWorkItemProvenance,
    DependencyReadinessWorkQueue, DependencyReadinessWorkflowRunId,
    DependencyReadinessWorkflowSessionId, DependencyRequirementsPayload,
    InMemoryDependencyRequirementsRegistry,
};
use pantograph_dependency_planning::{
    DependencyBindingStatusState, DependencyEnvironmentReadinessState,
    DependencyEnvironmentRequest, DependencyEnvironmentValidationState,
    DependencyPlanningDiagnosticCode, DependencyRequirementsId,
    ValidatedDependencyEnvironmentRequest,
};

use crate::dependency_readiness::PythonPackageReadinessSnapshot;
use crate::dependency_readiness_lifecycle::{
    EmbeddedDependencyReadinessSnapshotProducer, EmbeddedDependencyReadinessSnapshotProducerConfig,
};
use crate::package_readiness_provider::{
    PackageReadinessEnvironmentSelector, PackageReadinessProbeFailure,
    PackageReadinessProbeOutcome, PackageReadinessProbeRequest, PackageReadinessProbeRunner,
    PackageReadinessProviderDiagnosticCode,
};

#[tokio::test]
async fn producer_lifecycle_shutdown_is_idempotent_and_does_not_publish_snapshots() {
    let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
    let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
    let requirements_registry = Arc::new(InMemoryDependencyRequirementsRegistry::new());
    let producer = EmbeddedDependencyReadinessSnapshotProducer::new(
        snapshot_provider.clone(),
        work_queue.clone(),
        requirements_registry,
    )
    .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
        poll_interval: Duration::from_millis(5),
    });
    let handle = producer
        .spawn(tokio::runtime::Handle::current())
        .expect("producer should spawn");

    tokio::time::sleep(Duration::from_millis(15)).await;
    assert_eq!(snapshot_provider.snapshot_count(), 0);

    handle.shutdown().await;
    handle.shutdown().await;
    assert_eq!(snapshot_provider.snapshot_count(), 0);
    assert!(work_queue.is_empty());
}

#[tokio::test]
async fn producer_drains_work_queue_into_ready_snapshots_from_package_probe() {
    let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
    let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
    let request = validated_request();
    let requirements_registry = Arc::new(InMemoryDependencyRequirementsRegistry::new());
    requirements_registry.insert_payload(default_host_requirements_payload(&request));
    work_queue.enqueue(work_item(request.clone()));
    let package_probe_runner = Arc::new(FakePackageProbeRunner::new(
        PackageReadinessProbeOutcome::Snapshot(PythonPackageReadinessSnapshot::available(
            installed_package_ids(&["diffusers"]),
        )),
    ));
    let producer = EmbeddedDependencyReadinessSnapshotProducer::new(
        snapshot_provider.clone(),
        work_queue.clone(),
        requirements_registry,
    )
    .with_package_probe_runner(package_probe_runner.clone())
    .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
        poll_interval: Duration::from_millis(5),
    });
    let handle = producer
        .spawn(tokio::runtime::Handle::current())
        .expect("producer should spawn");

    tokio::time::sleep(Duration::from_millis(20)).await;

    assert!(work_queue.is_empty());
    assert_eq!(snapshot_provider.snapshot_count(), 1);
    assert_eq!(
        snapshot_provider.resolve(&request).readiness_state,
        DependencyEnvironmentReadinessState::Ready
    );
    assert_eq!(package_probe_runner.request_count(), 1);
    let probe_requests = package_probe_runner.requests();
    assert_eq!(probe_requests[0].executable_backend_key.as_str(), "pytorch");
    assert_eq!(
        probe_requests[0]
            .dependency_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["diffusers"]
    );
    assert_eq!(
        probe_requests[0].environment,
        PackageReadinessEnvironmentSelector::DefaultHostPython
    );
    handle.shutdown().await;
}

#[tokio::test]
async fn producer_reports_missing_snapshot_when_selected_package_is_absent() {
    let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
    let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
    let request = validated_request();
    let requirements_registry = Arc::new(InMemoryDependencyRequirementsRegistry::new());
    requirements_registry.insert_payload(default_host_requirements_payload(&request));
    work_queue.enqueue(work_item(request.clone()));
    let package_probe_runner = Arc::new(FakePackageProbeRunner::new(
        PackageReadinessProbeOutcome::Snapshot(PythonPackageReadinessSnapshot::available(
            BTreeSet::new(),
        )),
    ));
    let producer = EmbeddedDependencyReadinessSnapshotProducer::new(
        snapshot_provider.clone(),
        work_queue.clone(),
        requirements_registry,
    )
    .with_package_probe_runner(package_probe_runner)
    .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
        poll_interval: Duration::from_millis(5),
    });
    let handle = producer
        .spawn(tokio::runtime::Handle::current())
        .expect("producer should spawn");

    tokio::time::sleep(Duration::from_millis(20)).await;

    let result = snapshot_provider.resolve(&request);
    assert!(work_queue.is_empty());
    assert_eq!(
        result.readiness_state,
        DependencyEnvironmentReadinessState::Missing
    );
    assert_eq!(
        result.binding_statuses.first().map(|status| status.state),
        Some(DependencyBindingStatusState::Missing)
    );
    handle.shutdown().await;
}

#[tokio::test]
async fn producer_preserves_explicit_python_environment_and_fails_closed_on_probe_failure() {
    let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
    let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
    let request = validated_request();
    let requirements_registry = Arc::new(InMemoryDependencyRequirementsRegistry::new());
    requirements_registry.insert_payload(requirements_payload(&request));
    work_queue.enqueue(work_item(request.clone()));
    let package_probe_runner = Arc::new(FakePackageProbeRunner::new(
        PackageReadinessProbeOutcome::Failed(vec![PackageReadinessProbeFailure::new(
            PackageReadinessProviderDiagnosticCode::ProbeNotImplemented,
            None,
            CapabilityAvailabilityReason::parse(
                "Explicit Python package-readiness environments are not implemented.",
            )
            .expect("reason"),
        )]),
    ));
    let producer = EmbeddedDependencyReadinessSnapshotProducer::new(
        snapshot_provider.clone(),
        work_queue.clone(),
        requirements_registry,
    )
    .with_package_probe_runner(package_probe_runner.clone())
    .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
        poll_interval: Duration::from_millis(5),
    });
    let handle = producer
        .spawn(tokio::runtime::Handle::current())
        .expect("producer should spawn");

    tokio::time::sleep(Duration::from_millis(20)).await;

    let result = snapshot_provider.resolve(&request);
    assert!(work_queue.is_empty());
    assert_eq!(
        result.readiness_state,
        DependencyEnvironmentReadinessState::Unavailable
    );
    assert_eq!(
        result
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.code.clone()),
        Some(DependencyPlanningDiagnosticCode::NotImplemented)
    );
    let probe_requests = package_probe_runner.requests();
    assert_eq!(
        probe_requests[0].environment,
        PackageReadinessEnvironmentSelector::PythonEnvironment {
            environment_id: CapabilityAvailabilityId::parse("python:pytorch:cu124")
                .expect("environment id")
        }
    );
    handle.shutdown().await;
}

#[tokio::test]
async fn producer_reports_unavailable_snapshot_when_probe_fails() {
    let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
    let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
    let request = validated_request();
    let requirements_registry = Arc::new(InMemoryDependencyRequirementsRegistry::new());
    requirements_registry.insert_payload(requirements_payload(&request));
    work_queue.enqueue(work_item(request.clone()));
    let package_probe_runner = Arc::new(FakePackageProbeRunner::new(
        PackageReadinessProbeOutcome::Failed(vec![PackageReadinessProbeFailure::new(
            PackageReadinessProviderDiagnosticCode::PythonUnavailable,
            None,
            CapabilityAvailabilityReason::parse("Python runtime is unavailable.").expect("reason"),
        )]),
    ));
    let producer = EmbeddedDependencyReadinessSnapshotProducer::new(
        snapshot_provider.clone(),
        work_queue.clone(),
        requirements_registry,
    )
    .with_package_probe_runner(package_probe_runner)
    .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
        poll_interval: Duration::from_millis(5),
    });
    let handle = producer
        .spawn(tokio::runtime::Handle::current())
        .expect("producer should spawn");

    tokio::time::sleep(Duration::from_millis(20)).await;

    let result = snapshot_provider.resolve(&request);
    assert!(work_queue.is_empty());
    assert_eq!(
        result.readiness_state,
        DependencyEnvironmentReadinessState::Unavailable
    );
    assert_eq!(
        result
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.code.clone()),
        Some(DependencyPlanningDiagnosticCode::RuntimeUnavailable)
    );
    handle.shutdown().await;
}

#[tokio::test]
async fn producer_publishes_typed_unavailable_snapshot_when_registry_payload_is_missing() {
    let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
    let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
    let request = validated_request();
    let requirements_registry = Arc::new(InMemoryDependencyRequirementsRegistry::new());
    work_queue.enqueue(work_item(request.clone()));
    let producer = EmbeddedDependencyReadinessSnapshotProducer::new(
        snapshot_provider.clone(),
        work_queue.clone(),
        requirements_registry,
    )
    .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
        poll_interval: Duration::from_millis(5),
    });
    let handle = producer
        .spawn(tokio::runtime::Handle::current())
        .expect("producer should spawn");

    tokio::time::sleep(Duration::from_millis(20)).await;

    let result = snapshot_provider.resolve(&request);
    assert!(work_queue.is_empty());
    assert_eq!(
        result.readiness_state,
        DependencyEnvironmentReadinessState::Unavailable
    );
    assert_eq!(
        result.validation_state,
        DependencyEnvironmentValidationState::Unavailable
    );
    assert_eq!(
        result
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.code.clone()),
        Some(DependencyPlanningDiagnosticCode::InternalError)
    );
    assert_eq!(
        result
            .diagnostics
            .first()
            .and_then(|diagnostic| diagnostic.field_path.as_deref()),
        Some("dependency_environment.requirements_registry")
    );
    handle.shutdown().await;
}

#[test]
fn producer_rejects_zero_poll_interval() {
    let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
    let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
    let requirements_registry = Arc::new(InMemoryDependencyRequirementsRegistry::new());
    let producer = EmbeddedDependencyReadinessSnapshotProducer::new(
        snapshot_provider,
        work_queue,
        requirements_registry,
    )
    .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
        poll_interval: Duration::ZERO,
    });
    let runtime = tokio::runtime::Runtime::new().expect("runtime");

    let error = producer
        .spawn(runtime.handle().clone())
        .expect_err("zero interval should be rejected");

    assert!(error.to_string().contains("poll interval"));
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

fn requirements_payload(
    request: &ValidatedDependencyEnvironmentRequest,
) -> DependencyRequirementsPayload {
    let result = snapshot_provider_ready_result(request);
    let result =
        pantograph_dependency_planning::ValidatedDependencyEnvironmentResult::try_from(result)
            .expect("ready result should validate");
    DependencyRequirementsPayload::from_result(&result).expect("requirements payload")
}

fn default_host_requirements_payload(
    request: &ValidatedDependencyEnvironmentRequest,
) -> DependencyRequirementsPayload {
    let mut result = snapshot_provider_ready_result(request);
    for binding in &mut result.bindings {
        binding.profile_id = None;
    }
    let result =
        pantograph_dependency_planning::ValidatedDependencyEnvironmentResult::try_from(result)
            .expect("default-host ready result should validate");
    DependencyRequirementsPayload::from_result(&result).expect("default-host requirements payload")
}

fn snapshot_provider_ready_result(
    request: &ValidatedDependencyEnvironmentRequest,
) -> pantograph_dependency_planning::DependencyEnvironmentResult {
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
    result
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

    fn request_count(&self) -> usize {
        self.requests
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len()
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

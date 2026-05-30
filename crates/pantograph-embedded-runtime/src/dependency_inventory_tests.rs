use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use inference::{
    CapabilityAvailabilityId, CapabilityAvailabilityReason, ManagedBinaryId,
    ManagedBinaryInstallState, ManagedRuntimeReadinessState, ManagedRuntimeSelectionState,
    ManagedRuntimeSnapshot, RuntimeVariantId,
};
use pantograph_dependency_environment_service::{
    DependencyReadinessTaskId, DependencyReadinessWorkItem, DependencyReadinessWorkItemProvenance,
    DependencyReadinessWorkflowRunId, DependencyReadinessWorkflowSessionId,
    DependencyRequirementsPayload,
};
use pantograph_dependency_planning::{
    DependencyBindingId, DependencyBindingStatusState, DependencyEnvironmentKind,
    DependencyEnvironmentReadinessState, DependencyEnvironmentRequest,
    DependencyPlanningDiagnosticCode, DependencyProviderSourceState, DependencyRequirement,
    DependencyRequirementBinding, DependencyRequirementKind, DependencyRequirementsId,
    DeviceToolchainProviderSourceRow, DeviceToolchainProviderSourceSnapshot,
    RuntimeFeatureProviderSourceRow, RuntimeFeatureProviderSourceSnapshot,
    ValidatedDependencyEnvironmentRequest,
};

use crate::dependency_inventory::DependencyInventoryService;
use crate::dependency_inventory_device_toolchain_source::DeviceToolchainProviderSource;
use crate::dependency_inventory_managed_runtime::ManagedRuntimeSnapshotSource;
use crate::dependency_inventory_runtime_feature_source::RuntimeFeatureProviderSource;
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

#[tokio::test]
async fn inventory_service_routes_mixed_payloads_per_selected_binding() {
    let managed_binding_id =
        DependencyBindingId::parse("llama_cpp.binary").expect("managed binding id");
    let request = validated_request_with_selected_binding_id(managed_binding_id.clone());
    let item = work_item(request.clone());
    let mut payload = default_host_requirements_payload(&validated_request());
    payload.identity_key = request.as_request().identity_key.clone();
    payload
        .selected_binding_ids
        .push(managed_binding_id.clone());
    payload.requirements.push(managed_runtime_requirement_row());
    payload.bindings.push(managed_runtime_binding_row());
    let probe_runner = Arc::new(FakePackageProbeRunner::new(
        PackageReadinessProbeOutcome::Snapshot(PythonPackageReadinessSnapshot::available(
            installed_package_ids(&["diffusers"]),
        )),
    ));
    let managed_runtime_source = Arc::new(FakeManagedRuntimeSnapshotSource::ready());
    let inventory =
        DependencyInventoryService::from_package_probe_runner_and_managed_runtime_source(
            probe_runner.clone(),
            managed_runtime_source,
        );

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
        probe_requests[0]
            .dependency_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        installed_package_ids(&["diffusers"])
    );
    let python_status = snapshot
        .result
        .binding_statuses
        .iter()
        .find(|status| status.binding_id.as_str() == "diffusers.scheduler")
        .expect("python binding status");
    assert_eq!(python_status.state, DependencyBindingStatusState::Ready);
    let managed_status = snapshot
        .result
        .binding_statuses
        .iter()
        .find(|status| status.binding_id == managed_binding_id)
        .expect("managed binding status");
    assert_eq!(managed_status.state, DependencyBindingStatusState::Ready);
}

#[tokio::test]
async fn inventory_service_reports_missing_for_unmatched_managed_runtime_version() {
    let managed_binding_id =
        DependencyBindingId::parse("llama_cpp.binary").expect("managed binding id");
    let request = validated_request_with_selected_binding_id(managed_binding_id.clone());
    let item = work_item(request.clone());
    let mut payload = default_host_requirements_payload(&validated_request());
    payload.identity_key = request.as_request().identity_key.clone();
    payload
        .selected_binding_ids
        .push(managed_binding_id.clone());
    payload
        .requirements
        .push(managed_runtime_requirement_row_with_version(
            "missing-version",
        ));
    payload.bindings.push(managed_runtime_binding_row());
    let probe_runner = Arc::new(FakePackageProbeRunner::new(
        PackageReadinessProbeOutcome::Snapshot(PythonPackageReadinessSnapshot::available(
            installed_package_ids(&["diffusers"]),
        )),
    ));
    let inventory =
        DependencyInventoryService::from_package_probe_runner_and_managed_runtime_source(
            probe_runner,
            Arc::new(FakeManagedRuntimeSnapshotSource::ready()),
        );

    let snapshot = inventory
        .snapshot_for_work_item(&item, payload)
        .await
        .expect("snapshot");

    let managed_status = snapshot
        .result
        .binding_statuses
        .iter()
        .find(|status| status.binding_id == managed_binding_id)
        .expect("managed binding status");
    assert_eq!(managed_status.state, DependencyBindingStatusState::Missing);
    assert_eq!(
        managed_status
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.code.clone()),
        Some(DependencyPlanningDiagnosticCode::ArtifactMissing)
    );
}

#[tokio::test]
async fn inventory_service_routes_runtime_feature_payloads_through_source_snapshot() {
    let runtime_feature_binding_id =
        DependencyBindingId::parse("pytorch.streaming").expect("runtime feature binding id");
    let request = validated_request_with_selected_binding_id(runtime_feature_binding_id.clone());
    let item = work_item(request.clone());
    let mut payload = default_host_requirements_payload(&validated_request());
    payload.identity_key = request.as_request().identity_key.clone();
    payload
        .selected_binding_ids
        .push(runtime_feature_binding_id.clone());
    payload.requirements.push(runtime_feature_requirement_row());
    payload.bindings.push(runtime_feature_binding_row());
    let probe_runner = Arc::new(FakePackageProbeRunner::new(
        PackageReadinessProbeOutcome::Snapshot(PythonPackageReadinessSnapshot::available(
            installed_package_ids(&["diffusers"]),
        )),
    ));
    let inventory = DependencyInventoryService::from_package_probe_runner_and_managed_runtime_and_runtime_feature_sources(
        probe_runner.clone(),
        Arc::new(FakeManagedRuntimeSnapshotSource::ready()),
        Arc::new(FakeRuntimeFeatureProviderSource::ready()),
    );

    let snapshot = inventory
        .snapshot_for_work_item(&item, payload)
        .await
        .expect("snapshot");

    assert_eq!(
        snapshot.result.readiness_state,
        DependencyEnvironmentReadinessState::Ready
    );
    assert_eq!(probe_runner.requests().len(), 1);
    let runtime_feature_status = snapshot
        .result
        .binding_statuses
        .iter()
        .find(|status| status.binding_id == runtime_feature_binding_id)
        .expect("runtime feature binding status");
    assert_eq!(
        runtime_feature_status.state,
        DependencyBindingStatusState::Ready
    );
}

#[tokio::test]
async fn inventory_service_routes_device_toolchain_payloads_with_alternatives() {
    let device_toolchain_binding_id =
        DependencyBindingId::parse("pytorch.mps_runtime").expect("device toolchain binding id");
    let request = validated_request_with_selected_binding_id(device_toolchain_binding_id.clone());
    let item = work_item(request.clone());
    let mut payload = default_host_requirements_payload(&validated_request());
    payload.identity_key = request.as_request().identity_key.clone();
    payload
        .selected_binding_ids
        .push(device_toolchain_binding_id.clone());
    payload
        .requirements
        .push(device_toolchain_requirement_row());
    payload.bindings.push(device_toolchain_binding_row());
    let probe_runner = Arc::new(FakePackageProbeRunner::new(
        PackageReadinessProbeOutcome::Snapshot(PythonPackageReadinessSnapshot::available(
            installed_package_ids(&["diffusers"]),
        )),
    ));
    let inventory = DependencyInventoryService::from_package_probe_runner_and_managed_runtime_and_runtime_feature_and_device_toolchain_sources(
        probe_runner.clone(),
        Arc::new(FakeManagedRuntimeSnapshotSource::ready()),
        Arc::new(FakeRuntimeFeatureProviderSource::ready()),
        Arc::new(FakeDeviceToolchainProviderSource::with_unavailable_mps_alternative()),
    );

    let snapshot = inventory
        .snapshot_for_work_item(&item, payload)
        .await
        .expect("snapshot");

    assert_eq!(
        snapshot.result.readiness_state,
        DependencyEnvironmentReadinessState::Unavailable
    );
    assert_eq!(probe_runner.requests().len(), 1);
    let device_toolchain_status = snapshot
        .result
        .binding_statuses
        .iter()
        .find(|status| status.binding_id == device_toolchain_binding_id)
        .expect("device toolchain binding status");
    assert_eq!(
        device_toolchain_status.state,
        DependencyBindingStatusState::Unavailable
    );
    assert_eq!(device_toolchain_status.alternatives.len(), 1);
    assert_eq!(
        device_toolchain_status.alternatives[0]
            .toolchain_id
            .as_ref()
            .map(|toolchain_id| toolchain_id.as_str()),
        Some("cuda_runtime")
    );
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

fn validated_request_with_selected_binding_id(
    binding_id: DependencyBindingId,
) -> ValidatedDependencyEnvironmentRequest {
    let mut request: DependencyEnvironmentRequest = serde_json::from_str(include_str!(
        "../../pantograph-dependency-planning/tests/fixtures/dependency_environment_resolve_request.json"
    ))
    .expect("request fixture should parse");
    request.dependency_requirements_id = Some(
        DependencyRequirementsId::parse("tiny-sd:pytorch:linux-x86_64:torch-diffusers")
            .expect("requirements id"),
    );
    request
        .identity_key
        .selected_binding_ids
        .push(binding_id.clone());
    request
        .planning_request
        .selected_binding_ids
        .push(binding_id);
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

fn managed_runtime_requirement_row() -> DependencyRequirement {
    managed_runtime_requirement_row_with_version_value(None)
}

fn managed_runtime_requirement_row_with_version(version: &str) -> DependencyRequirement {
    managed_runtime_requirement_row_with_version_value(Some(version))
}

fn managed_runtime_requirement_row_with_version_value(
    version: Option<&str>,
) -> DependencyRequirement {
    let mut value = serde_json::json!({
        "name": "llama_cpp",
        "kind": "runtime_managed_binary",
        "managed_runtime": {
            "managed_binary_id": "llama_cpp"
        }
    });
    if let Some(version) = version {
        value["managed_runtime"]["version"] = serde_json::Value::String(version.to_string());
    }
    serde_json::from_value(value).expect("managed runtime requirement row")
}

fn managed_runtime_binding_row() -> DependencyRequirementBinding {
    serde_json::from_value(serde_json::json!({
        "binding_id": "llama_cpp.binary",
        "requirement_name": "llama_cpp",
        "environment_kind": "managed_binary",
        "managed_runtime": {
            "managed_binary_id": "llama_cpp"
        }
    }))
    .expect("managed runtime binding row")
}

fn runtime_feature_requirement_row() -> DependencyRequirement {
    serde_json::from_value(serde_json::json!({
        "name": "pytorch_streaming",
        "kind": "runtime_feature",
        "runtime_feature": {
            "runtime_id": "pytorch",
            "feature_id": "streaming"
        }
    }))
    .expect("runtime feature requirement row")
}

fn runtime_feature_binding_row() -> DependencyRequirementBinding {
    serde_json::from_value(serde_json::json!({
        "binding_id": "pytorch.streaming",
        "requirement_name": "pytorch_streaming",
        "environment_kind": "runtime_feature",
        "runtime_feature": {
            "runtime_id": "pytorch",
            "feature_id": "streaming"
        }
    }))
    .expect("runtime feature binding row")
}

fn device_toolchain_requirement_row() -> DependencyRequirement {
    serde_json::from_value(serde_json::json!({
        "name": "pytorch_mps_runtime",
        "kind": "device_toolchain",
        "device_toolchain": {
            "runtime_id": "pytorch",
            "toolchain_id": "mps_runtime"
        }
    }))
    .expect("device toolchain requirement row")
}

fn device_toolchain_binding_row() -> DependencyRequirementBinding {
    serde_json::from_value(serde_json::json!({
        "binding_id": "pytorch.mps_runtime",
        "requirement_name": "pytorch_mps_runtime",
        "environment_kind": "device_toolchain",
        "device_toolchain": {
            "runtime_id": "pytorch",
            "toolchain_id": "mps_runtime"
        }
    }))
    .expect("device toolchain binding row")
}

#[derive(Debug)]
struct FakePackageProbeRunner {
    outcome: PackageReadinessProbeOutcome,
    requests: std::sync::Mutex<Vec<PackageReadinessProbeRequest>>,
}

#[derive(Debug)]
struct FakeManagedRuntimeSnapshotSource {
    snapshots: Vec<ManagedRuntimeSnapshot>,
}

#[derive(Debug)]
struct FakeRuntimeFeatureProviderSource {
    snapshot: RuntimeFeatureProviderSourceSnapshot,
}

#[derive(Debug)]
struct FakeDeviceToolchainProviderSource {
    snapshot: DeviceToolchainProviderSourceSnapshot,
}

impl FakeRuntimeFeatureProviderSource {
    fn ready() -> Self {
        Self {
            snapshot: RuntimeFeatureProviderSourceSnapshot {
                contract_version: 1,
                rows: vec![RuntimeFeatureProviderSourceRow {
                    runtime_id: pantograph_dependency_planning::RuntimeSourceId::parse("pytorch")
                        .expect("runtime id"),
                    feature_id: pantograph_dependency_planning::RuntimeFeatureSourceId::parse(
                        "streaming",
                    )
                    .expect("runtime feature id"),
                    runtime_variant_id: None,
                    state: DependencyProviderSourceState::Ready,
                    freshness: pantograph_dependency_planning::DependencyInventoryObservationFreshness::Fresh,
                    checked_at_ms: None,
                    diagnostics: Vec::new(),
                    alternatives: Vec::new(),
                }],
                diagnostics: Vec::new(),
            },
        }
    }
}

impl FakeDeviceToolchainProviderSource {
    fn with_unavailable_mps_alternative() -> Self {
        Self {
            snapshot: DeviceToolchainProviderSourceSnapshot {
                contract_version: 1,
                rows: vec![
                    DeviceToolchainProviderSourceRow {
                        toolchain_id:
                            pantograph_dependency_planning::DeviceToolchainSourceId::parse(
                                "cuda_runtime",
                            )
                            .expect("toolchain id"),
                        runtime_id: Some(
                            pantograph_dependency_planning::RuntimeSourceId::parse("pytorch")
                                .expect("runtime id"),
                        ),
                        device_class: Some(
                            pantograph_dependency_planning::DeviceClassSourceId::parse("cuda")
                                .expect("device class"),
                        ),
                        device_id: None,
                        state: DependencyProviderSourceState::Ready,
                        freshness: pantograph_dependency_planning::DependencyInventoryObservationFreshness::Fresh,
                        checked_at_ms: None,
                        diagnostics: Vec::new(),
                        alternatives: Vec::new(),
                    },
                    DeviceToolchainProviderSourceRow {
                        toolchain_id:
                            pantograph_dependency_planning::DeviceToolchainSourceId::parse(
                                "mps_runtime",
                            )
                            .expect("toolchain id"),
                        runtime_id: Some(
                            pantograph_dependency_planning::RuntimeSourceId::parse("pytorch")
                                .expect("runtime id"),
                        ),
                        device_class: Some(
                            pantograph_dependency_planning::DeviceClassSourceId::parse("mps")
                                .expect("device class"),
                        ),
                        device_id: None,
                        state: DependencyProviderSourceState::Unavailable,
                        freshness: pantograph_dependency_planning::DependencyInventoryObservationFreshness::Fresh,
                        checked_at_ms: None,
                        diagnostics: Vec::new(),
                        alternatives: vec![
                            pantograph_dependency_planning::DependencyProviderSourceAlternative {
                                runtime_id: Some(
                                    pantograph_dependency_planning::RuntimeSourceId::parse(
                                        "pytorch",
                                    )
                                    .expect("runtime id"),
                                ),
                                runtime_variant_id: None,
                                feature_id: None,
                                toolchain_id: Some(
                                    pantograph_dependency_planning::DeviceToolchainSourceId::parse(
                                        "cuda_runtime",
                                    )
                                    .expect("toolchain id"),
                                ),
                                device_class: Some(
                                    pantograph_dependency_planning::DeviceClassSourceId::parse(
                                        "cuda",
                                    )
                                    .expect("device class"),
                                ),
                                device_id: None,
                                system_package_id: None,
                                package_manager_id: None,
                                platform_id: None,
                                reason: Some(
                                    "CUDA runtime is available on this host.".to_string(),
                                ),
                            },
                        ],
                    },
                ],
                diagnostics: Vec::new(),
            },
        }
    }
}

impl FakeManagedRuntimeSnapshotSource {
    fn ready() -> Self {
        Self {
            snapshots: vec![ManagedRuntimeSnapshot {
                id: ManagedBinaryId::LlamaCpp,
                display_name: "llama.cpp".to_string(),
                install_state: ManagedBinaryInstallState::Installed,
                readiness_state: ManagedRuntimeReadinessState::Ready,
                available: true,
                can_install: true,
                can_remove: true,
                missing_files: Vec::new(),
                unavailable_reason: None,
                versions: vec![inference::ManagedRuntimeVersionStatus {
                    version: Some("b8248".to_string()),
                    display_label: "b8248".to_string(),
                    runtime_key: ManagedBinaryId::LlamaCpp.key().to_string(),
                    runtime_variant_id: RuntimeVariantId::parse("llama_cpp.cpu")
                        .expect("runtime variant id"),
                    platform_key: "linux-x86_64".to_string(),
                    install_root: Some("/tmp/pantograph-test-runtime".to_string()),
                    executable_name: "llama-server".to_string(),
                    executable_ready: true,
                    install_state: ManagedBinaryInstallState::Installed,
                    readiness_state: ManagedRuntimeReadinessState::Ready,
                    catalog_available: true,
                    installable: true,
                    selected: true,
                    active: false,
                }],
                selection: ManagedRuntimeSelectionState::default(),
                active_job: None,
                job_artifact: None,
            }],
        }
    }
}

#[async_trait]
impl ManagedRuntimeSnapshotSource for FakeManagedRuntimeSnapshotSource {
    async fn list_snapshots(&self) -> Result<Vec<ManagedRuntimeSnapshot>, String> {
        Ok(self.snapshots.clone())
    }
}

#[async_trait]
impl RuntimeFeatureProviderSource for FakeRuntimeFeatureProviderSource {
    async fn snapshot(&self) -> Result<RuntimeFeatureProviderSourceSnapshot, String> {
        Ok(self.snapshot.clone())
    }
}

#[async_trait]
impl DeviceToolchainProviderSource for FakeDeviceToolchainProviderSource {
    async fn snapshot(&self) -> Result<DeviceToolchainProviderSourceSnapshot, String> {
        Ok(self.snapshot.clone())
    }
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

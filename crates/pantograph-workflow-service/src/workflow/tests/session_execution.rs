use pantograph_dependency_environment_service::{
    DependencyEnvironmentReadinessSnapshot, DependencyEnvironmentReadinessSnapshotProvider,
    DependencyEnvironmentReadinessSnapshotStatus, DependencyReadinessWorkQueue,
};
use pantograph_dependency_planning::{
    produce_dependency_requirements_proof, DependencyEnvironmentAction, DependencyEnvironmentId,
    DependencyEnvironmentInstallState, DependencyEnvironmentKind,
    DependencyEnvironmentReadinessState, DependencyEnvironmentRef, DependencyEnvironmentRequest,
    DependencyEnvironmentResult, DependencyEnvironmentValidationState, DependencyNodeTypeId,
    DependencyPlanningCallerContext, DependencyPlanningIdentityKey, DependencyPlanningRequest,
    DependencyReadinessProofEnvelope, DependencyRequirement, DependencyRequirementBinding,
    DependencyRequirementKind, DependencyRequirementName, DeviceIntentId, PumasModelRef,
    PythonPackageManagerKind, PythonRequirementDetails, RuntimeIntentId, SchedulerIntent,
    ValidatedDependencyEnvironmentRequest, ValidatedDependencyPlanningRequest,
};
use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, DraftGraphValidationStatus, DraftGraphValidationSummary,
    InferenceAvailabilityStatus, InferenceInterfaceFingerprint, InferenceTaskKind,
    WorkflowGraphRevision, WorkflowNodeId, INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use pantograph_runtime_host_contracts::{
    ReservationLifecycleApplication, ReservationLifecycleApplicationState,
    ReservationLifecycleEvent, ReservationLifecycleOutcome, ReservationLifecyclePort,
    ReservationLifecyclePortError, RuntimeHostExecutionMediaArtifactRef,
    RuntimeHostExecutionOutput, RuntimeHostExecutionOutputValue, RuntimeHostExecutionPort,
    RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    RuntimeHostExecutionState, RESERVATION_LIFECYCLE_CONTRACT_VERSION,
    RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};
use pantograph_scheduler::{
    SchedulerDispatchCandidate, SchedulerDispatchCandidateId, SchedulerReservationLeaseId,
    SchedulerResourceFitAssessment, SchedulerResourceFitState, SchedulerResourceKind,
    SchedulerResourceReservation, SchedulerTaskStateRecord,
};

use super::*;
use crate::workflow::runtime_dispatch_selection::{
    WorkflowRuntimeDispatchCandidateProviderError, WorkflowRuntimeDispatchCandidateSet,
};
use crate::{
    GraphNode, Position, WorkflowTechnicalFitCandidateSetSummary, WorkflowTechnicalFitDecisionCode,
    WorkflowTechnicalFitDeviceClass, WorkflowTechnicalFitHistoryThresholdState,
    WorkflowTechnicalFitPolicyPhase, WorkflowTechnicalFitSelectionPolicyTrace,
};

#[tokio::test]
async fn workflow_execution_session_lifecycle_create_run_close() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("generic-run".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    assert_eq!(created.runtime_capabilities.len(), 1);

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello session"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run session");
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(
        response.outputs[0].value,
        serde_json::json!("hello session")
    );
    assert!(
        host.recorded_run_options
            .lock()
            .expect("run options lock")
            .is_empty(),
        "non-runtime-only session runs must not call the legacy whole-run host path"
    );

    let closed = service
        .close_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCloseRequest {
                session_id: created.session_id.clone(),
            },
        )
        .await
        .expect("close session");
    assert!(closed.ok);

    let err = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("closed session should not run");
    assert!(matches!(err, WorkflowServiceError::SessionNotFound(_)));
}

#[tokio::test]
async fn workflow_execution_session_timeout_applies_to_scheduler_task_runner() {
    let host = SlowWorkflowIoHost::new(std::time::Duration::from_millis(50));
    let service = WorkflowService::with_max_sessions(2);
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-timeout".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("timeout"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: Some(1),
                priority: None,
            },
        )
        .await
        .expect_err("scheduler task runner should honor timeout_ms");

    assert_eq!(error.code(), WorkflowErrorCode::RuntimeTimeout);
    assert!(error.message().contains("timeout_ms 1"));
    assert!(
        host.inner
            .recorded_run_options
            .lock()
            .expect("run options lock")
            .is_empty(),
        "timeout must not route through the legacy whole-run host path"
    );
}

#[tokio::test]
async fn workflow_execution_session_runtime_run_fails_closed_before_legacy_launch() {
    let host = RuntimeInferenceSessionHost::new();
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-runtime-fail-closed".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "prompt".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("paint a red cube"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("runtime-containing scheduler run should fail closed");

    assert_eq!(error.code(), WorkflowErrorCode::InvalidRequest);
    assert!(
        error
            .message()
            .contains("saved executable validation snapshot"),
        "unexpected error: {error}"
    );
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("list queue after rejected runtime inference run");
    assert!(queue.items.is_empty());
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn workflow_execution_session_runtime_run_requires_dependency_readiness_before_dispatch() {
    let host = RuntimeInferenceSessionHost::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_dependency_readiness_work_queue(dependency_readiness_work_queue.clone());
    let workflow_id = "wf-runtime-dispatch-boundary";
    let workflow_semantic_version = "1.2.3";
    let graph = runtime_inference_session_graph();
    let version = service
        .resolve_workflow_graph_version(workflow_id, workflow_semantic_version, &graph)
        .expect("resolve workflow version");
    service
        .store_workflow_executable_validation_snapshot(runtime_executable_validation_snapshot(
            &version, &graph,
        ))
        .expect("store executable validation snapshot");

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: workflow_semantic_version.to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "prompt".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("paint a red cube"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("runtime-containing scheduler run should fail closed at readiness admission");

    assert_eq!(error.code(), WorkflowErrorCode::InvalidRequest);
    assert!(
        error
            .message()
            .contains("dependency requirements registry seed failed"),
        "unexpected error: {error}"
    );
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("list queue after fail-closed runtime inference run");
    assert!(queue.items.is_empty());
    assert_eq!(dependency_readiness_work_queue.len(), 0);
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn workflow_execution_session_fresh_dependency_readiness_snapshot_stops_at_dispatch_boundary()
{
    let host = RuntimeInferenceSessionHost::new();
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_dependency_environment_provider(std::sync::Arc::new(
            dependency_readiness_provider.clone(),
        ))
        .with_dependency_readiness_work_queue(dependency_readiness_work_queue.clone());
    let workflow_id = "wf-runtime-ready-dispatch-boundary";
    let workflow_semantic_version = "1.2.3";
    let graph = runtime_inference_session_graph();
    let version = service
        .resolve_workflow_graph_version(workflow_id, workflow_semantic_version, &graph)
        .expect("resolve workflow version");
    service
        .store_workflow_executable_validation_snapshot(runtime_executable_validation_snapshot(
            &version, &graph,
        ))
        .expect("store executable validation snapshot");
    let dependency_request = runtime_dependency_environment_request(&version);
    dependency_readiness_provider
        .insert_snapshot(
            DependencyEnvironmentReadinessSnapshot::for_request(
                &dependency_request,
                ready_dependency_environment_result(&dependency_request),
                DependencyEnvironmentReadinessSnapshotStatus::Fresh,
            )
            .expect("dependency readiness snapshot should validate"),
        )
        .expect("store dependency readiness snapshot");

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: workflow_semantic_version.to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "prompt".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("paint a red cube"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("ready dependency proof should still stop before dispatch wiring");

    assert_eq!(error.code(), WorkflowErrorCode::CapabilityViolation);
    assert!(
        error
            .message()
            .contains("runtime scheduler dispatch selection failed closed"),
        "unexpected error: {error}"
    );
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("list queue after dispatch fail-closed runtime inference run");
    assert!(queue.items.is_empty());
    assert_eq!(dependency_readiness_work_queue.len(), 1);
    let work_item = dependency_readiness_work_queue
        .pop_next()
        .expect("dependency-readiness work item should be queued after seed");
    assert_eq!(work_item.provenance.session_id.as_str(), session_id);
    assert_eq!(work_item.provenance.task_id.as_str(), "infer");
    assert_eq!(
        work_item.request.as_request().action,
        DependencyEnvironmentAction::Check
    );
    assert_eq!(
        work_item
            .diagnostic_context
            .as_ref()
            .map(|context| context.as_str()),
        Some("runtime task entered WaitingDependencyReadiness")
    );
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection() {
    let host = RuntimeInferenceSessionHost::new();
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let runtime_host_port = Arc::new(CompletingRuntimeHostPort::default());
    let reservation_lifecycle_port = Arc::new(RecordingReservationLifecyclePort::default());
    let service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_dependency_environment_provider(std::sync::Arc::new(
            dependency_readiness_provider.clone(),
        ))
        .with_dependency_readiness_work_queue(dependency_readiness_work_queue.clone())
        .with_runtime_dispatch_candidate_provider(Arc::new(
            SingleCanonicalRuntimeDispatchCandidateProvider,
        ))
        .with_runtime_host_execution_port(runtime_host_port.clone())
        .with_reservation_lifecycle_port(reservation_lifecycle_port.clone());
    let workflow_id = "wf-runtime-selected-dispatch";
    let workflow_semantic_version = "1.2.3";
    let graph = runtime_inference_session_graph();
    let version = service
        .resolve_workflow_graph_version(workflow_id, workflow_semantic_version, &graph)
        .expect("resolve workflow version");
    service
        .store_workflow_executable_validation_snapshot(runtime_executable_validation_snapshot(
            &version, &graph,
        ))
        .expect("store executable validation snapshot");
    let dependency_request = runtime_dependency_environment_request(&version);
    dependency_readiness_provider
        .insert_snapshot(
            DependencyEnvironmentReadinessSnapshot::for_request(
                &dependency_request,
                ready_dependency_environment_result(&dependency_request),
                DependencyEnvironmentReadinessSnapshotStatus::Fresh,
            )
            .expect("dependency readiness snapshot should validate"),
        )
        .expect("store dependency readiness snapshot");

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: workflow_semantic_version.to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "prompt".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("paint a red cube"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "infer".to_string(),
                    port_id: "image".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("ready runtime task should dispatch through scheduler selection");

    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, "infer");
    assert_eq!(response.outputs[0].port_id, "image");
    assert_eq!(
        response.outputs[0].value,
        serde_json::json!({
            "artifact_id": "runtime-output-image",
            "media_type": "image_png"
        })
    );
    let recorded = runtime_host_port.requests();
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        recorded[0]
            .handoff
            .dispatch_decision
            .as_ref()
            .expect("dispatch-selected handoff")
            .selected_runtime_id
            .as_str(),
        "pytorch"
    );
    assert_eq!(dependency_readiness_work_queue.len(), 1);
    let work_item = dependency_readiness_work_queue
        .pop_next()
        .expect("dependency-readiness work item should be queued after seed");
    assert_eq!(work_item.provenance.session_id.as_str(), session_id);
    assert_eq!(work_item.provenance.task_id.as_str(), "infer");
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
    let lifecycle_events = reservation_lifecycle_port.events();
    assert_eq!(
        lifecycle_events
            .iter()
            .map(|event| &event.outcome)
            .collect::<Vec<_>>(),
        vec![
            &ReservationLifecycleOutcome::DispatchStarted,
            &ReservationLifecycleOutcome::RuntimeHostCompleted,
        ]
    );
    assert!(lifecycle_events
        .iter()
        .all(|event| event.reservation_lease_id.as_str() == "reservation.runtime_session_test"));
}

#[tokio::test]
async fn workflow_execution_session_fails_closed_when_reservation_lifecycle_port_is_missing() {
    let host = RuntimeInferenceSessionHost::new();
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let runtime_host_port = Arc::new(CompletingRuntimeHostPort::default());
    let service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_dependency_environment_provider(std::sync::Arc::new(
            dependency_readiness_provider.clone(),
        ))
        .with_dependency_readiness_work_queue(dependency_readiness_work_queue)
        .with_runtime_dispatch_candidate_provider(Arc::new(
            SingleCanonicalRuntimeDispatchCandidateProvider,
        ))
        .with_runtime_host_execution_port(runtime_host_port.clone());
    let workflow_id = "wf-runtime-lifecycle-missing";
    let workflow_semantic_version = "1.2.3";
    let graph = runtime_inference_session_graph();
    let version = service
        .resolve_workflow_graph_version(workflow_id, workflow_semantic_version, &graph)
        .expect("resolve workflow version");
    service
        .store_workflow_executable_validation_snapshot(runtime_executable_validation_snapshot(
            &version, &graph,
        ))
        .expect("store executable validation snapshot");
    let dependency_request = runtime_dependency_environment_request(&version);
    dependency_readiness_provider
        .insert_snapshot(
            DependencyEnvironmentReadinessSnapshot::for_request(
                &dependency_request,
                ready_dependency_environment_result(&dependency_request),
                DependencyEnvironmentReadinessSnapshotStatus::Fresh,
            )
            .expect("dependency readiness snapshot should validate"),
        )
        .expect("store dependency readiness snapshot");

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: workflow_semantic_version.to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "prompt".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("paint a red cube"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "infer".to_string(),
                    port_id: "image".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("missing reservation lifecycle port must fail before runtime dispatch");

    assert_eq!(error.code(), WorkflowErrorCode::CapabilityViolation);
    assert!(
        error
            .message()
            .contains("reservation lifecycle port is not configured"),
        "unexpected error: {error}"
    );
    assert!(runtime_host_port.requests().is_empty());
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

fn ready_dependency_environment_result(
    request: &ValidatedDependencyEnvironmentRequest,
) -> DependencyEnvironmentResult {
    let request = request.as_request();
    DependencyEnvironmentResult {
        contract_version: 1,
        action: request.action,
        identity_key: request.identity_key.clone(),
        readiness_state: DependencyEnvironmentReadinessState::Ready,
        install_state: DependencyEnvironmentInstallState::Installed,
        validation_state: DependencyEnvironmentValidationState::Valid,
        failure_state: None,
        dependency_requirements_id: request.dependency_requirements_id.clone(),
        environment_ref: Some(DependencyEnvironmentRef {
            environment_id: DependencyEnvironmentId::parse(format!(
                "test-env-{}",
                request.identity_key.task_id.as_str()
            ))
            .expect("valid environment id"),
            manifest_id: None,
        }),
        requirements: dependency_requirements(),
        bindings: dependency_bindings(&request.identity_key.selected_binding_ids),
        selected_binding_ids: request.identity_key.selected_binding_ids.clone(),
        binding_statuses: Vec::new(),
        operation: None,
        validation_errors: Vec::new(),
        diagnostics: Vec::new(),
    }
}

fn dependency_requirements() -> Vec<DependencyRequirement> {
    vec![DependencyRequirement {
        name: DependencyRequirementName::parse("diffusers").expect("valid requirement name"),
        kind: DependencyRequirementKind::PythonPackage,
        version_constraint: Some(">=0.29".to_string()),
        python: Some(PythonRequirementDetails {
            import_name: Some("diffusers".to_string()),
            python_requires: Some(">=3.10".to_string()),
            package_manager: Some(PythonPackageManagerKind::Pip),
        }),
        managed_runtime: None,
        runtime_feature: None,
        device_toolchain: None,
        system_package: None,
    }]
}

fn dependency_bindings(
    selected_binding_ids: &[pantograph_dependency_planning::DependencyBindingId],
) -> Vec<DependencyRequirementBinding> {
    selected_binding_ids
        .iter()
        .map(|binding_id| DependencyRequirementBinding {
            binding_id: binding_id.clone(),
            requirement_name: DependencyRequirementName::parse("diffusers")
                .expect("valid requirement name"),
            environment_kind: DependencyEnvironmentKind::Python,
            profile_id: None,
            python: None,
            managed_runtime: None,
            runtime_feature: None,
            device_toolchain: None,
            system_package: None,
        })
        .collect()
}

struct SlowWorkflowIoHost {
    inner: MockWorkflowHost,
    workflow_io_delay: std::time::Duration,
}

impl SlowWorkflowIoHost {
    fn new(workflow_io_delay: std::time::Duration) -> Self {
        Self {
            inner: MockWorkflowHost::new(8, 1024),
            workflow_io_delay,
        }
    }
}

#[async_trait::async_trait]
impl WorkflowHost for SlowWorkflowIoHost {
    fn max_input_bindings(&self) -> usize {
        self.inner.max_input_bindings()
    }

    fn max_output_targets(&self) -> usize {
        self.inner.max_output_targets()
    }

    fn max_value_bytes(&self) -> usize {
        self.inner.max_value_bytes()
    }

    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        self.inner.validate_workflow(workflow_id).await
    }

    async fn workflow_graph_fingerprint(
        &self,
        workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        self.inner.workflow_graph_fingerprint(workflow_id).await
    }

    async fn workflow_graph(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        self.inner.workflow_graph(workflow_id).await
    }

    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        self.inner.workflow_capabilities(workflow_id).await
    }

    async fn workflow_io(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        tokio::time::sleep(self.workflow_io_delay).await;
        self.inner.workflow_io(workflow_id).await
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        self.inner.runtime_capabilities().await
    }

    async fn workflow_technical_fit_decision(
        &self,
        request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        self.inner.workflow_technical_fit_decision(request).await
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        self.inner
            .run_workflow(workflow_id, inputs, output_targets, run_options, run_handle)
            .await
    }
}

#[tokio::test]
async fn workflow_execution_session_initializes_scheduler_task_state_before_run_execution() {
    let host = BlockingRunHost::new();
    let service = WorkflowService::with_max_sessions(2);
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-task-state-init".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let service_for_run = service.clone();
    let host_for_run = host.clone();
    let session_id = created.session_id.clone();
    let run = tokio::spawn(async move {
        service_for_run
            .run_workflow_execution_session(
                &host_for_run,
                WorkflowExecutionSessionRunRequest {
                    session_id,
                    workflow_semantic_version: "1.2.3".to_string(),
                    inputs: vec![WorkflowPortBinding {
                        node_id: "text-input-1".to_string(),
                        port_id: "text".to_string(),
                        value: serde_json::json!("task state initialization"),
                    }],
                    output_targets: Some(vec![WorkflowOutputTarget {
                        node_id: "text-output-1".to_string(),
                        port_id: "text".to_string(),
                    }]),
                    override_selection: None,
                    timeout_ms: None,
                    priority: None,
                },
            )
            .await
    });

    if tokio::time::timeout(Duration::from_secs(5), host.wait_for_first_run_started())
        .await
        .is_err()
    {
        let run_result = tokio::time::timeout(Duration::from_secs(1), run)
            .await
            .expect("workflow run should return after host execution timeout")
            .expect("workflow run task should join");
        panic!("workflow run did not reach host execution: {run_result:?}");
    }
    let workflow_run_id = {
        let store = service.session_store_guard().expect("session store");
        store
            .active_workflow_run_ids()
            .into_iter()
            .next()
            .expect("active workflow run id")
    };

    let read_models = service
        .workflow_get_scheduler_task_state_read_models(
            WorkflowSchedulerTaskStateReadModelQueryRequest {
                session_id: created.session_id,
                workflow_run_id,
            },
        )
        .await
        .expect("scheduler task-state read models");
    assert_eq!(read_models.tasks.len(), 2);
    assert!(read_models
        .tasks
        .iter()
        .any(|task| task.node_id == "text-input-1"));
    assert!(read_models
        .tasks
        .iter()
        .any(|task| task.node_id == "text-output-1"));
    assert!(read_models.tasks.iter().all(|task| task.model_id.is_none()));

    host.release_first_run();
    let response = run
        .await
        .expect("run task should join")
        .expect("workflow run should finish");
    assert_eq!(response.outputs.len(), 1);
}

#[tokio::test]
async fn workflow_execution_session_records_retained_node_io_artifact_bodies() {
    let host = MockWorkflowHost::new(8, 1024);
    let temp = tempfile::tempdir().expect("temp artifact store");
    let artifact_store =
        ArtifactStore::open(temp.path(), retained_io_test_artifact_policy()).expect("store");
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"))
        .with_artifact_store(artifact_store);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-retained-io".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("retained text"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run session");

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("diagnostic events")
    };
    let node_output_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::IoArtifactObserved
                && event
                    .payload_json
                    .contains("\"artifact_role\":\"node_output\"")
        })
        .expect("node output artifact event");
    assert!(!diagnostic_events.iter().any(|event| {
        event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::IoArtifactObserved
            && event
                .payload_json
                .contains("\"artifact_role\":\"node_input\"")
    }));
    assert_eq!(
        node_output_event
            .workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    let payload: serde_json::Value =
        serde_json::from_str(&node_output_event.payload_json).expect("payload json");
    assert_eq!(payload["retention_state"], "retained");
    assert_eq!(payload["payload_kind"], "text");
    assert!(payload["artifact_fact_id"]
        .as_str()
        .is_some_and(|artifact_fact_id| artifact_fact_id.starts_with("workflow-io-fact-")));
    assert!(payload["payload_artifact_id"]
        .as_str()
        .is_some_and(|payload_artifact_id| payload_artifact_id.starts_with("workflow-io-")));
    assert!(payload["logical_payload_lineage_id"]
        .as_str()
        .is_some_and(|lineage_id| lineage_id.starts_with("workflow-io-lineage-")));
    assert_eq!(payload["producer_node_id"], "text-output-1");
    assert_eq!(payload["producer_port_id"], "text");
    assert!(payload["consumer_node_id"].is_null());
    assert!(payload["consumer_port_id"].is_null());
    let workflow_output_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::IoArtifactObserved
                && event
                    .payload_json
                    .contains("\"artifact_role\":\"workflow_output\"")
        })
        .expect("workflow output artifact event");
    let workflow_output_payload: serde_json::Value =
        serde_json::from_str(&workflow_output_event.payload_json).expect("workflow output payload");
    assert_eq!(
        workflow_output_payload["payload_artifact_id"],
        payload["payload_artifact_id"]
    );
    assert_eq!(
        workflow_output_payload["logical_payload_lineage_id"],
        payload["logical_payload_lineage_id"]
    );
    assert_ne!(
        workflow_output_payload["artifact_fact_id"],
        payload["artifact_fact_id"]
    );
    let artifact_id = payload["artifact_id"]
        .as_str()
        .expect("artifact id")
        .to_string();
    assert!(payload["read_handle"].as_str().is_some());

    let retained = service
        .read_artifact_body(ArtifactReadRequest {
            artifact_id,
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect("read retained node output artifact");
    assert_eq!(retained.body, b"retained text");
    let stats = service
        .artifact_store_stats()
        .expect("artifact store stats");
    assert_eq!(stats.retained_body_count, 2);
    assert_eq!(stats.retained_body_bytes, 26);
}

#[tokio::test]
async fn workflow_execution_session_run_passes_logical_session_id_in_run_options() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("create keep-alive session");

    service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello session"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run keep-alive session");

    let recorded = host
        .recorded_run_options
        .lock()
        .expect("run options lock poisoned");
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        recorded[0].workflow_execution_session_id.as_deref(),
        Some(created.session_id.as_str())
    );
    assert_eq!(recorded[0].timeout_ms, None);
}

#[tokio::test]
async fn workflow_execution_session_repeated_runs_create_distinct_backend_run_ids() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("create session");

    let first = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("first run");

    let second = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("second run");

    assert_ne!(first.workflow_run_id, created.session_id);
    assert_ne!(second.workflow_run_id, created.session_id);
    assert_ne!(first.workflow_run_id, second.workflow_run_id);
    assert!(first.workflow_run_id.starts_with("run_"));
    assert!(second.workflow_run_id.starts_with("run_"));

    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("session status");
    assert_eq!(status.session.run_count, 2);
}

#[tokio::test]
async fn workflow_execution_session_run_rejects_stale_graph_before_queue_admission() {
    let host = StaleWorkflowGraphHost::new();
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-stale".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.0.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("stale graph should be rejected before queue admission");

    assert_eq!(error.code(), WorkflowErrorCode::InvalidRequest);
    assert!(error.message().contains("retired_node_type"));
    let Some(WorkflowErrorDetails::Graph(details)) = error.details() else {
        panic!("stale graph rejection should expose typed graph details");
    };
    assert!(details.graph_diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::WorkflowGraphDiagnosticCode::RetiredNodeType
            && diagnostic.node_id.as_deref() == Some("diffusion")
    }));
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: created.session_id,
        })
        .await
        .expect("list queue after rejected run");
    assert!(queue.items.is_empty());
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn workflow_execution_session_run_records_snapshot_before_execution() {
    let host = MockWorkflowHost::with_technical_fit_decision(
        8,
        1024,
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::Automatic,
            selected_candidate_id: Some("candidate-managed-llama".to_string()),
            selected_runtime_id: Some("managed-llama-slot".to_string()),
            selected_runtime_variant_id: Some("llama_cpp.cuda".to_string()),
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: Some("model-a".to_string()),
            selected_device_class: Some(WorkflowTechnicalFitDeviceClass::Cuda),
            selected_device_id: Some("cuda:0".to_string()),
            resource_estimates: Vec::new(),
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            dependency_readiness: Vec::new(),
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::RuntimeRequirements,
                Some("candidate-managed-llama"),
            )],
            selection_policy_trace: None,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        },
    );
    let service = WorkflowService::with_max_sessions(2)
        .with_attribution_store(SqliteAttributionStore::open_in_memory().expect("store"))
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-snapshot".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("snapshotted"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: Some(5000),
                priority: Some(7),
            },
        )
        .await
        .expect("run session");

    let snapshot = service
        .workflow_run_snapshot(&response.workflow_run_id)
        .expect("query snapshot")
        .expect("snapshot");
    assert_eq!(snapshot.workflow_run_id.as_str(), response.workflow_run_id);
    assert_eq!(snapshot.workflow_id.as_str(), "wf-snapshot");
    assert_eq!(snapshot.workflow_execution_session_id, created.session_id);
    assert_eq!(snapshot.workflow_execution_session_kind, "workflow");
    assert_eq!(snapshot.usage_profile, None);
    assert!(!snapshot.keep_alive);
    assert_eq!(snapshot.retention_policy, "ephemeral");
    assert_eq!(snapshot.scheduler_policy, "priority_then_fifo");
    assert_eq!(snapshot.workflow_semantic_version, "1.2.3");
    assert!(snapshot
        .workflow_presentation_revision_id
        .as_str()
        .starts_with("wfpres_"));
    assert_eq!(snapshot.priority, 7);
    assert_eq!(snapshot.timeout_ms, Some(5000));
    assert!(snapshot
        .workflow_execution_fingerprint
        .starts_with("workflow-exec-blake3:"));
    assert!(snapshot.inputs_json.contains("snapshotted"));
    assert!(snapshot.graph_settings_json.contains("text-input-1"));
    assert!(snapshot.runtime_requirements_json.contains("model-a"));
    assert!(snapshot
        .capability_models_json
        .contains("sha256:hash-model-a"));
    assert!(snapshot.runtime_capabilities_json.contains("llama_cpp"));

    let version_projection = service
        .workflow_run_version_projection(&response.workflow_run_id)
        .expect("query run version projection")
        .expect("projection");
    assert_eq!(
        version_projection.snapshot.workflow_run_id.as_str(),
        response.workflow_run_id
    );
    assert_eq!(
        version_projection.workflow_version.workflow_version_id,
        snapshot.workflow_version_id
    );
    assert_eq!(
        version_projection
            .presentation_revision
            .workflow_presentation_revision_id,
        snapshot.workflow_presentation_revision_id
    );
    assert_eq!(
        version_projection.workflow_version.semantic_version,
        "1.2.3"
    );
    assert!(version_projection
        .presentation_revision
        .presentation_metadata_json
        .contains("text-input-1"));
    assert!(version_projection
        .workflow_version
        .executable_topology_json
        .contains("text-input-1"));

    let run_graph = service
        .workflow_run_graph_query(WorkflowRunGraphQueryRequest {
            workflow_run_id: response.workflow_run_id.clone(),
        })
        .expect("query run graph")
        .run_graph
        .expect("run graph");
    assert_eq!(run_graph.workflow_run_id, response.workflow_run_id);
    assert_eq!(run_graph.workflow_id, "wf-snapshot");
    assert_eq!(run_graph.workflow_semantic_version, "1.2.3");
    assert_eq!(
        run_graph.workflow_version_id,
        snapshot.workflow_version_id.as_str()
    );
    assert_eq!(
        run_graph.workflow_presentation_revision_id,
        snapshot.workflow_presentation_revision_id.as_str()
    );
    assert_eq!(run_graph.graph.nodes.len(), 2);
    assert_eq!(run_graph.graph.edges.len(), 1);
    assert_eq!(run_graph.graph.nodes[0].id, "text-input-1");
    assert_eq!(run_graph.graph.nodes[0].node_type, "text-input");
    assert_eq!(run_graph.graph.nodes[0].position.x, 0.0);
    assert_eq!(run_graph.graph.edges[0].id, "edge");
    assert!(!run_graph.executable_topology.nodes[0]
        .contract_version
        .is_empty());

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("diagnostic events")
    };
    assert_eq!(diagnostic_events.len(), 17);
    let event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::RunSnapshotAccepted
        })
        .expect("run snapshot accepted event");
    assert_eq!(
        event.event_kind,
        pantograph_diagnostics_ledger::DiagnosticEventKind::RunSnapshotAccepted
    );
    assert_eq!(
        event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::WorkflowService
    );
    assert_eq!(
        event.workflow_run_id.as_ref().map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert_eq!(
        event.workflow_version_id.as_ref(),
        Some(&snapshot.workflow_version_id)
    );
    assert_eq!(event.workflow_semantic_version.as_deref(), Some("1.2.3"));
    assert_eq!(
        event.scheduler_policy_id.as_deref(),
        Some("priority_then_fifo")
    );
    assert_eq!(event.retention_policy_id.as_deref(), Some("ephemeral"));
    assert!(event
        .payload_json
        .contains(snapshot.workflow_run_snapshot_id.as_str()));
    let snapshot_payload: serde_json::Value =
        serde_json::from_str(&event.payload_json).expect("snapshot payload json");
    assert_eq!(
        snapshot_payload["node_versions"].as_array().unwrap().len(),
        2
    );
    assert!(snapshot_payload["node_versions"][0]["contract_version"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(snapshot_payload["node_versions"][0]["behavior_digest"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));

    let estimate_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerEstimateProduced
        })
        .expect("scheduler estimate event");
    assert_eq!(
        estimate_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
    );
    assert!(estimate_event.event_seq > event.event_seq);
    assert_eq!(
        estimate_event
            .workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert_eq!(
        estimate_event.scheduler_policy_id.as_deref(),
        Some("priority_then_fifo")
    );
    assert!(estimate_event
        .payload_json
        .contains("\"estimate_version\":\"session-scheduler-v1\""));
    assert!(estimate_event
        .payload_json
        .contains("\"confidence\":\"estimated\""));
    assert!(estimate_event
        .payload_json
        .contains("\"model_cache_state\":\"unknown\""));
    assert!(estimate_event.payload_json.contains(
        "\"blocking_conditions\":[\"runtime_admission_pending\",\"model_cache_unknown\"]"
    ));
    assert!(estimate_event
        .payload_json
        .contains("\"missing_asset_ids\":[]"));
    assert!(estimate_event
        .payload_json
        .contains("\"candidate_runtime_ids\":[\"llama_cpp\"]"));
    assert!(estimate_event
        .payload_json
        .contains("requires backend(s): llama_cpp"));
    assert!(estimate_event
        .payload_json
        .contains("requires model(s): model-a"));
    assert!(estimate_event
        .payload_json
        .contains("requires extension(s): inference_gateway"));
    assert!(estimate_event
        .payload_json
        .contains("estimated peak memory: 1073741824 bytes peak VRAM, 2147483648 bytes peak RAM"));
    assert!(estimate_event
        .payload_json
        .contains("candidate runtime(s): llama_cpp"));

    let queue_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerQueuePlacement
        })
        .expect("scheduler queue placement event");
    assert_eq!(
        queue_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
    );
    assert_eq!(
        queue_event.workflow_run_id.as_ref().map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert_eq!(
        queue_event.workflow_version_id.as_ref(),
        Some(&snapshot.workflow_version_id)
    );
    assert!(queue_event.event_seq > estimate_event.event_seq);
    assert_eq!(
        queue_event.scheduler_policy_id.as_deref(),
        Some("priority_then_fifo")
    );
    assert_eq!(
        queue_event.retention_policy_id.as_deref(),
        Some("ephemeral")
    );
    assert!(queue_event.payload_json.contains("\"queue_position\":0"));
    assert!(queue_event.payload_json.contains("\"priority\":7"));

    let admitted_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerRunAdmitted
        })
        .expect("scheduler run admitted event");
    assert_eq!(
        admitted_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
    );
    assert_eq!(
        admitted_event
            .workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert_eq!(
        admitted_event.runtime_id.as_deref(),
        Some("managed-llama-slot")
    );
    assert!(admitted_event.event_seq > queue_event.event_seq);
    assert!(admitted_event.payload_json.contains("\"decision_reason\":"));
    assert!(admitted_event.payload_json.contains("\"queue_wait_ms\":"));
    assert!(admitted_event
        .payload_json
        .contains("\"selected_runtime_id\":\"managed-llama-slot\""));
    assert!(admitted_event
        .payload_json
        .contains("\"selected_backend_key\":\"llama_cpp\""));
    assert!(admitted_event
        .payload_json
        .contains("\"reserved_model_ids\":[\"model-a\"]"));

    let reservation_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerReservationChanged
        })
        .collect::<Vec<_>>();
    assert_eq!(reservation_events.len(), 2);
    assert!(reservation_events.iter().all(|event| event.source_component
        == pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler));
    assert!(reservation_events.iter().all(|event| event
        .workflow_run_id
        .as_ref()
        .map(|id| id.as_str())
        == Some(response.workflow_run_id.as_str())));
    assert_eq!(
        reservation_events[0].runtime_id.as_deref(),
        Some("managed-llama-slot")
    );
    assert_eq!(
        reservation_events[1].runtime_id.as_deref(),
        Some("managed-llama-slot")
    );
    assert!(reservation_events.iter().all(|event| event
        .payload_json
        .contains("\"resource_kind\":\"runtime_slot\"")));
    assert!(reservation_events.iter().all(|event| event
        .payload_json
        .contains("\"reserved_model_ids\":[\"model-a\"]")));
    assert!(reservation_events[0].event_seq > admitted_event.event_seq);
    assert!(reservation_events[0]
        .payload_json
        .contains("\"transition\":\"created\""));
    assert!(reservation_events[0]
        .payload_json
        .contains("\"reason\":\"local runtime slot admitted\""));

    let started_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::RunStarted
        })
        .expect("run started event");
    assert_eq!(
        started_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
    );
    assert_eq!(
        started_event.workflow_run_id.as_ref().map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert!(started_event.event_seq > reservation_events[0].event_seq);
    assert!(started_event
        .payload_json
        .contains("\"scheduler_decision_reason\":"));

    let model_lifecycle_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerModelLifecycleChanged
        })
        .collect::<Vec<_>>();
    assert_eq!(model_lifecycle_events.len(), 5);
    assert!(model_lifecycle_events
        .iter()
        .all(|event| event.source_component
            == pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler));
    assert!(model_lifecycle_events.iter().all(|event| event
        .workflow_run_id
        .as_ref()
        .map(|id| id.as_str())
        == Some(response.workflow_run_id.as_str())));
    assert!(model_lifecycle_events
        .iter()
        .all(|event| event.workflow_version_id.as_ref() == Some(&snapshot.workflow_version_id)));
    assert!(model_lifecycle_events
        .iter()
        .all(|event| event.model_id.as_deref() == Some("model-a")));
    assert!(model_lifecycle_events
        .iter()
        .all(|event| event.runtime_id.as_deref() == Some("managed-llama-slot")));
    assert!(model_lifecycle_events[0].event_seq > started_event.event_seq);
    assert!(model_lifecycle_events[0]
        .payload_json
        .contains("\"transition\":\"load_requested\""));
    assert!(model_lifecycle_events[0]
        .payload_json
        .contains("\"cache_state\":\"load_requested\""));
    assert!(model_lifecycle_events[0]
        .payload_json
        .contains("\"reason\":\"runtime admission requested required models\""));
    let load_requested_payload: serde_json::Value =
        serde_json::from_str(&model_lifecycle_events[0].payload_json)
            .expect("load requested payload json");
    let load_dependency_payload: serde_json::Value =
        serde_json::from_str(&model_lifecycle_events[1].payload_json)
            .expect("load dependency payload json");
    let timing_attempt_id = load_requested_payload["timing_attempt_id"]
        .as_str()
        .expect("load requested timing attempt id");
    assert!(timing_attempt_id.starts_with("timing_attempt_"));
    assert!(model_lifecycle_events[1].event_seq > model_lifecycle_events[0].event_seq);
    assert!(model_lifecycle_events[1]
        .payload_json
        .contains("\"transition\":\"load_dependency_resolved\""));
    assert!(model_lifecycle_events[1]
        .payload_json
        .contains("\"cache_state\":\"load_requested\""));
    assert!(model_lifecycle_events[1]
        .payload_json
        .contains("\"reason\":\"runtime admission resolved required model dependencies\""));
    assert_eq!(
        load_dependency_payload["timing_attempt_id"].as_str(),
        Some(timing_attempt_id)
    );
    assert!(model_lifecycle_events.iter().all(|event| !event
        .payload_json
        .contains("\"transition\":\"load_completed\"")));
    assert!(model_lifecycle_events[1]
        .payload_json
        .contains("\"duration_ms\":"));

    let terminal_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::RunTerminal
        })
        .expect("run terminal event");
    assert_eq!(
        terminal_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::WorkflowService
    );
    assert_eq!(
        terminal_event
            .workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert!(terminal_event.event_seq > model_lifecycle_events[1].event_seq);
    assert!(terminal_event
        .payload_json
        .contains("\"status\":\"completed\""));
    assert!(terminal_event.payload_json.contains("\"duration_ms\":"));
    assert!(reservation_events[1].event_seq > terminal_event.event_seq);
    assert!(reservation_events[1]
        .payload_json
        .contains("\"transition\":\"released\""));
    assert!(reservation_events[1]
        .payload_json
        .contains("\"selected_runtime_id\":\"managed-llama-slot\""));
    assert!(reservation_events[1]
        .payload_json
        .contains("\"reason\":\"workflow run finished\""));

    let io_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::IoArtifactObserved
        })
        .collect::<Vec<_>>();
    assert_eq!(io_events.len(), 3);
    assert!(io_events[0].event_seq > reservation_events[1].event_seq);
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"workflow_input\"")));
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"workflow_output\"")));
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"node_output\"")));
    assert!(io_events
        .iter()
        .all(|event| event.node_type.as_deref() == Some("text-output")));
    assert!(io_events.iter().all(|event| event
        .payload_json
        .contains("\"retention_state\":\"metadata_only\"")));
    let last_io_event_seq = io_events
        .iter()
        .map(|event| event.event_seq)
        .max()
        .expect("last io event");
    assert!(model_lifecycle_events[2].event_seq > last_io_event_seq);
    assert!(model_lifecycle_events[2]
        .payload_json
        .contains("\"transition\":\"unload_scheduled\""));
    let unload_scheduled_payload: serde_json::Value =
        serde_json::from_str(&model_lifecycle_events[2].payload_json)
            .expect("unload scheduled payload json");
    let unload_started_payload: serde_json::Value =
        serde_json::from_str(&model_lifecycle_events[3].payload_json)
            .expect("unload started payload json");
    let unload_completed_payload: serde_json::Value =
        serde_json::from_str(&model_lifecycle_events[4].payload_json)
            .expect("unload completed payload json");
    let unload_timing_attempt_id = unload_scheduled_payload["timing_attempt_id"]
        .as_str()
        .expect("unload scheduled timing attempt id");
    assert!(unload_timing_attempt_id.starts_with("timing_attempt_"));
    assert!(model_lifecycle_events[2]
        .payload_json
        .contains("\"cache_state\":\"unload_requested\""));
    assert!(model_lifecycle_events[2]
        .payload_json
        .contains("\"reason\":\"keep-alive disabled after run completion\""));
    assert!(model_lifecycle_events[3].event_seq > model_lifecycle_events[2].event_seq);
    assert!(model_lifecycle_events[3]
        .payload_json
        .contains("\"transition\":\"unload_started\""));
    assert_eq!(
        unload_started_payload["timing_attempt_id"].as_str(),
        Some(unload_timing_attempt_id)
    );
    assert!(model_lifecycle_events[3]
        .payload_json
        .contains("\"cache_state\":\"unload_requested\""));
    assert!(model_lifecycle_events[4].event_seq > model_lifecycle_events[3].event_seq);
    assert!(model_lifecycle_events[4]
        .payload_json
        .contains("\"transition\":\"unload_completed\""));
    assert_eq!(
        unload_completed_payload["timing_attempt_id"].as_str(),
        Some(unload_timing_attempt_id)
    );
    assert!(model_lifecycle_events[4]
        .payload_json
        .contains("\"cache_state\":\"unloaded\""));
    assert!(model_lifecycle_events[4]
        .payload_json
        .contains("\"duration_ms\":"));

    let library_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::LibraryAssetAccessed
        })
        .expect("library asset access event");
    assert_eq!(
        library_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Library
    );
    assert_eq!(
        library_event.workflow_run_id.as_ref().map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert_eq!(library_event.model_id.as_deref(), Some("model-a"));
    assert!(library_event
        .payload_json
        .contains("\"asset_id\":\"pumas://models/model-a\""));
    assert!(library_event
        .payload_json
        .contains("\"operation\":\"run_usage\""));
    service
        .workflow_diagnostics_projection_refresh(WorkflowDiagnosticsProjectionRefreshRequest {
            projections: vec![WorkflowDiagnosticsProjectionKind::LibraryUsage],
            workflow_run_id: Some(response.workflow_run_id.clone()),
            workflow_id: Some("workflow-a".to_string()),
            reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
            batch_size: 100,
        })
        .expect("library usage projection refresh");

    let library_usage = service
        .workflow_library_usage_query(WorkflowLibraryUsageQueryRequest {
            asset_id: Some("pumas://models/model-a".to_string()),
            workflow_run_id: Some(response.workflow_run_id.clone()),
            workflow_id: None,
            workflow_version_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(100),
        })
        .expect("library usage query");
    assert_eq!(library_usage.assets.len(), 1);
    assert_eq!(library_usage.assets[0].asset_id, "pumas://models/model-a");
    assert_eq!(library_usage.assets[0].run_access_count, 1);
}

#[tokio::test]
async fn workflow_execution_session_records_load_completed_only_with_runtime_proof() {
    let mut host = MockWorkflowHost::with_runtime_load_proof(
        8,
        1024,
        WorkflowSessionRuntimeLoadProof {
            backend_key: "llama_cpp".to_string(),
            runtime_id: Some("managed-llama-slot".to_string()),
            model_id: Some("model-a".to_string()),
            active_model_path: Some("/models/model-a.gguf".to_string()),
            requested_model_active: true,
        },
    );
    host.technical_fit_decision = Some(WorkflowTechnicalFitDecision {
        selection_mode: WorkflowTechnicalFitSelectionMode::Automatic,
        selected_candidate_id: Some("candidate-managed-llama".to_string()),
        selected_runtime_id: Some("managed-llama-slot".to_string()),
        selected_runtime_variant_id: Some("llama_cpp.cuda".to_string()),
        selected_backend_key: Some("llama_cpp".to_string()),
        selected_model_id: Some("model-a".to_string()),
        selected_device_class: Some(WorkflowTechnicalFitDeviceClass::Cuda),
        selected_device_id: Some("cuda:0".to_string()),
        resource_estimates: Vec::new(),
        observed_throughput_hint: None,
        device_diagnostics: Vec::new(),
        dependency_readiness: Vec::new(),
        reasons: vec![WorkflowTechnicalFitReason::new(
            WorkflowTechnicalFitReasonCode::RuntimeRequirements,
            Some("candidate-managed-llama"),
        )],
        selection_policy_trace: Some(WorkflowTechnicalFitSelectionPolicyTrace {
            policy_version: 1,
            policy_phase: Some(WorkflowTechnicalFitPolicyPhase::CandidateRanking),
            decision_code: Some(WorkflowTechnicalFitDecisionCode::SelectedCandidate),
            history_threshold_state: Some(WorkflowTechnicalFitHistoryThresholdState::NotEvaluated),
            candidate_set_summary: Some(WorkflowTechnicalFitCandidateSetSummary {
                total_candidate_count: 2,
                eligible_candidate_count: 2,
                rejected_candidate_count: 0,
                eligible_candidate_ids: vec![
                    "candidate-managed-llama".to_string(),
                    "candidate-pytorch".to_string(),
                ],
            }),
            ranking_reason: Some("candidate_priority".to_string()),
            exploration_reason: Some("equal_priority_seeded_choice".to_string()),
            seed_basis: Some(
                "workflow:wf-runtime-proof|snapshot:123|candidates:candidate-managed-llama,candidate-pytorch"
                    .to_string(),
            ),
        }),
        compatibility_report: None,
        compatibility_issue_count: 0,
        compatibility_issues: Vec::new(),
    });
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-runtime-proof".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run session");

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 30,
        )
        .expect("diagnostic events")
    };
    let admission_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerRunAdmitted
                && event.workflow_run_id.as_ref().map(|id| id.as_str())
                    == Some(response.workflow_run_id.as_str())
        })
        .expect("scheduler admission event");
    assert!(admission_event
        .payload_json
        .contains("\"selected_runtime_variant_id\":\"llama_cpp.cuda\""));
    assert!(admission_event
        .payload_json
        .contains("\"selected_backend_key\":\"llama_cpp\""));
    assert!(admission_event
        .payload_json
        .contains("\"selected_device_class\":\"cuda\""));
    assert!(admission_event
        .payload_json
        .contains("\"selected_device_id\":\"cuda:0\""));
    assert!(admission_event
        .payload_json
        .contains("\"technical_fit_selection_policy_trace\""));
    assert!(admission_event
        .payload_json
        .contains("\"execution_plan_summary\""));
    assert!(admission_event
        .payload_json
        .contains("\"schema_version\":1"));
    assert!(admission_event
        .payload_json
        .contains("\"node_decision_count\":1"));
    assert!(admission_event
        .payload_json
        .contains("\"policy_trace_ids\":[\"technical_fit_policy_v1\"]"));
    assert!(admission_event
        .payload_json
        .contains("\"policy_phase\":\"candidate_ranking\""));
    assert!(admission_event
        .payload_json
        .contains("\"decision_code\":\"selected_candidate\""));
    assert!(admission_event
        .payload_json
        .contains("\"history_threshold_state\":\"not_evaluated\""));
    assert!(admission_event
        .payload_json
        .contains("\"ranking_reason\":\"candidate_priority\""));
    assert!(admission_event
        .payload_json
        .contains("\"exploration_reason\":\"equal_priority_seeded_choice\""));

    let lifecycle_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerModelLifecycleChanged
                && event
                    .workflow_run_id
                    .as_ref()
                    .map(|id| id.as_str())
                    == Some(response.workflow_run_id.as_str())
        })
        .collect::<Vec<_>>();

    let load_requested = lifecycle_events
        .iter()
        .find(|event| {
            event
                .payload_json
                .contains("\"transition\":\"load_requested\"")
        })
        .expect("load requested event");
    let dependency_resolved = lifecycle_events
        .iter()
        .find(|event| {
            event
                .payload_json
                .contains("\"transition\":\"load_dependency_resolved\"")
        })
        .expect("dependency resolved event");
    let load_completed = lifecycle_events
        .iter()
        .find(|event| {
            event
                .payload_json
                .contains("\"transition\":\"load_completed\"")
        })
        .expect("load completed event");

    assert!(dependency_resolved.event_seq > load_requested.event_seq);
    assert!(load_completed.event_seq > dependency_resolved.event_seq);
    assert!(load_completed
        .payload_json
        .contains("\"cache_state\":\"loaded\""));
    assert!(load_completed
        .payload_json
        .contains("\"reason\":\"runtime admission proved requested model active\""));
    assert!(lifecycle_events.iter().all(|event| event
        .payload_json
        .contains("\"selected_runtime_variant_id\":\"llama_cpp.cuda\"")));
    assert!(lifecycle_events
        .iter()
        .all(|event| event.payload_json.contains("\"execution_plan_summary\"")));
    assert!(lifecycle_events.iter().all(|event| event
        .payload_json
        .contains("\"policy_trace_ids\":[\"technical_fit_policy_v1\"]")));
    let reservation_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerReservationChanged
                && event.workflow_run_id.as_ref().map(|id| id.as_str())
                    == Some(response.workflow_run_id.as_str())
        })
        .collect::<Vec<_>>();
    assert!(reservation_events[0]
        .payload_json
        .contains("\"transition\":\"created\""));
    assert!(reservation_events[0]
        .payload_json
        .contains("\"selected_runtime_variant_id\":\"llama_cpp.cuda\""));
    assert!(reservation_events[0]
        .payload_json
        .contains("\"selected_device_class\":\"cuda\""));
    assert!(reservation_events[0]
        .payload_json
        .contains("\"selected_device_id\":\"cuda:0\""));
    assert!(reservation_events[1]
        .payload_json
        .contains("\"transition\":\"released\""));
    assert!(reservation_events[1]
        .payload_json
        .contains("\"selected_runtime_variant_id\":\"llama_cpp.cuda\""));
    assert!(reservation_events[1]
        .payload_json
        .contains("\"selected_device_class\":\"cuda\""));
    assert!(reservation_events[1]
        .payload_json
        .contains("\"selected_device_id\":\"cuda:0\""));
}

#[tokio::test]
async fn attributed_workflow_execution_session_carries_client_bucket_into_run_events() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2)
        .with_attribution_store(SqliteAttributionStore::open_in_memory().expect("store"))
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));
    let registered = service
        .register_attribution_client(ClientRegistrationRequest {
            display_name: Some("local gui".to_string()),
            metadata_json: None,
        })
        .expect("register client");
    let opened = service
        .open_client_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: Some("launch".to_string()),
        })
        .expect("open client session");

    let created = service
        .create_attributed_workflow_execution_session(
            &host,
            WorkflowExecutionSessionAttributedCreateRequest {
                workflow_id: "wf-attributed".to_string(),
                usage_profile: Some("developer".to_string()),
                keep_alive: false,
                attribution: WorkflowExecutionSessionAttributionRequest {
                    credential: registered.credential_proof_request(),
                    client_session_id: opened.session.client_session_id.as_str().to_string(),
                    bucket_selection: BucketSelection::Default,
                },
            },
        )
        .await
        .expect("create attributed session");

    assert_eq!(
        created
            .attribution
            .as_ref()
            .map(|context| context.client_id.as_str()),
        Some(registered.client.client_id.as_str())
    );
    assert_eq!(
        created
            .attribution
            .as_ref()
            .map(|context| context.bucket_id.as_str()),
        Some(opened.default_bucket.bucket_id.as_str())
    );

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("attributed"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run attributed session");

    let snapshot = service
        .workflow_run_snapshot(&response.workflow_run_id)
        .expect("query snapshot")
        .expect("snapshot");
    assert_eq!(
        snapshot.client_id,
        Some(registered.client.client_id.clone())
    );
    assert_eq!(
        snapshot.client_session_id,
        Some(opened.session.client_session_id.clone())
    );
    assert_eq!(snapshot.bucket_id, Some(opened.default_bucket.bucket_id));

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };
    assert!(diagnostic_events
        .iter()
        .all(|event| event.client_id.as_ref() == Some(&registered.client.client_id)));
    assert!(diagnostic_events
        .iter()
        .all(|event| event.client_session_id.as_ref() == Some(&opened.session.client_session_id)));
}

#[tokio::test]
async fn keep_alive_session_loads_runtime_with_keep_alive_retention_hint() {
    let retention_hints = Arc::new(Mutex::new(Vec::new()));
    let host = RecordingRuntimeHost::new(retention_hints.clone());
    let service = WorkflowService::with_max_sessions(2);

    service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create keep-alive session");

    assert_eq!(
        *retention_hints
            .lock()
            .expect("retention hints lock poisoned"),
        vec![WorkflowExecutionSessionRetentionHint::KeepAlive]
    );
}

#[tokio::test]
async fn one_shot_session_run_loads_runtime_with_ephemeral_retention_hint() {
    let retention_hints = Arc::new(Mutex::new(Vec::new()));
    let host = RecordingRuntimeHost::new(retention_hints.clone());
    let service = WorkflowService::with_max_sessions(2);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create one-shot session");

    service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run one-shot session");

    assert_eq!(
        *retention_hints
            .lock()
            .expect("retention hints lock poisoned"),
        vec![WorkflowExecutionSessionRetentionHint::Ephemeral]
    );
}

#[tokio::test]
async fn workflow_execution_session_run_records_failed_terminal_event_with_sanitized_error() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2)
        .with_attribution_store(SqliteAttributionStore::open_in_memory().expect("store"))
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-control-error".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("runtime-error-control"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("runtime error should fail the run");
    assert_eq!(error.code(), WorkflowErrorCode::RuntimeNotReady);

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("diagnostic events")
    };
    let terminal_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::RunTerminal
        })
        .expect("failed terminal event");
    assert!(terminal_event
        .payload_json
        .contains("\"status\":\"failed\""));
    assert!(terminal_event
        .payload_json
        .contains("llama.cpp stderr line"));
    assert!(!terminal_event.payload_json.chars().any(char::is_control));
    let error_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
        })
        .expect("canonical node execution error event");
    assert!(error_event.payload_json.contains("node_execution"));
    assert!(error_event.payload_json.contains("backend not ready"));
    assert!(!error_event.payload_json.contains("\\n"));

    let terminal_workflow_run_id = terminal_event
        .workflow_run_id
        .as_ref()
        .expect("terminal event workflow run id")
        .as_str()
        .to_string();
    service
        .workflow_diagnostics_projection_refresh(WorkflowDiagnosticsProjectionRefreshRequest {
            projections: vec![
                WorkflowDiagnosticsProjectionKind::RunDetail,
                WorkflowDiagnosticsProjectionKind::NodeStatus,
            ],
            workflow_run_id: Some(terminal_workflow_run_id.clone()),
            workflow_id: terminal_event
                .workflow_id
                .as_ref()
                .map(|workflow_id| workflow_id.as_str().to_string()),
            reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
            batch_size: 20,
        })
        .expect("projection refresh");
    let detail = service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: terminal_workflow_run_id,
            projection_batch_size: Some(20),
        })
        .expect("run detail query")
        .run
        .expect("run detail");
    assert_eq!(detail.status, RunListProjectionStatus::Failed);
    assert!(!detail
        .terminal_error
        .as_deref()
        .unwrap_or_default()
        .chars()
        .any(char::is_control));
}

#[tokio::test]
async fn workflow_execution_session_run_snapshot_failure_records_canonical_error() {
    let host = FailingRunSnapshotHost::new();
    let service = WorkflowService::with_max_sessions(2)
        .with_attribution_store(SqliteAttributionStore::open_in_memory().expect("store"))
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-snapshot-error".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("snapshot failure should fail the run");

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("diagnostic events")
    };
    let error_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
        })
        .expect("canonical run snapshot error event");

    assert!(error_event.payload_json.contains("run_snapshot"));
    assert!(error_event.payload_json.contains("run_snapshot_failed"));
    assert_eq!(
        error
            .diagnostics()
            .and_then(|diagnostics| diagnostics.diagnostic_event_id.as_deref()),
        Some(error_event.event_id.as_str())
    );
}

#[tokio::test]
async fn workflow_execution_session_runtime_load_failure_records_canonical_error() {
    let host = FailingRuntimeLoadHost::new();
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-runtime-load-error".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("runtime load should fail the run");
    assert_eq!(error.code(), WorkflowErrorCode::RuntimeNotReady);
    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest { session_id })
        .await
        .expect("session status after runtime-load failure");
    assert_eq!(
        status.session.state,
        WorkflowExecutionSessionState::IdleUnloaded
    );
    assert_eq!(status.session.run_count, 1);

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 30,
        )
        .expect("diagnostic events")
    };
    let error_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
        })
        .expect("canonical runtime load error event");
    assert!(error_event.payload_json.contains("runtime_model_load"));
    assert!(error_event
        .payload_json
        .contains("runtime_model_load_failed"));
    assert!(error_event.payload_json.contains("llama.cpp spawn failed"));
    assert!(!error_event.payload_json.contains("\\n"));
    assert_eq!(
        error
            .diagnostics()
            .and_then(|diagnostics| diagnostics.diagnostic_event_id.as_deref()),
        Some(error_event.event_id.as_str())
    );

    let lifecycle_failed_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerModelLifecycleChanged
                && event.payload_json.contains("load_failed")
        })
        .expect("failed scheduler model lifecycle event");
    assert!(lifecycle_failed_event.payload_json.contains(&format!(
        "\"canonical_error_event_id\":\"{}\"",
        error_event.event_id
    )));

    let terminal_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::RunTerminal
        })
        .expect("failed terminal event");
    assert!(terminal_event.payload_json.contains(&format!(
        "\"canonical_error_event_id\":\"{}\"",
        error_event.event_id
    )));
    let terminal_workflow_run_id = terminal_event
        .workflow_run_id
        .as_ref()
        .expect("terminal event workflow run id")
        .as_str()
        .to_string();
    service
        .workflow_diagnostics_projection_refresh(WorkflowDiagnosticsProjectionRefreshRequest {
            projections: vec![
                WorkflowDiagnosticsProjectionKind::RunDetail,
                WorkflowDiagnosticsProjectionKind::NodeStatus,
            ],
            workflow_run_id: Some(terminal_workflow_run_id.clone()),
            workflow_id: terminal_event
                .workflow_id
                .as_ref()
                .map(|workflow_id| workflow_id.as_str().to_string()),
            reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
            batch_size: 30,
        })
        .expect("projection refresh");
    let detail = service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: terminal_workflow_run_id,
            projection_batch_size: Some(30),
        })
        .expect("run detail query")
        .run
        .expect("run detail");
    assert_eq!(detail.status, RunListProjectionStatus::Failed);
}

#[tokio::test]
async fn workflow_execution_session_preserves_run_error_when_execution_diagnostics_unavailable() {
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));
    let diagnostics_ledger = service
        .diagnostics_ledger
        .as_ref()
        .expect("diagnostics ledger configured")
        .clone();
    let host = FailingRunWithPoisonedDiagnosticsHost::new(diagnostics_ledger);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-execution-diagnostics-unavailable".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("workflow run should preserve execution failure");

    assert_eq!(error.code(), WorkflowErrorCode::InvalidRequest);
    assert!(error.message().contains("workflow execution failed"));
    let diagnostics = error
        .diagnostics()
        .expect("diagnostics unavailable link should be attached");
    assert!(diagnostics.diagnostic_event_id.is_none());
    assert!(diagnostics
        .diagnostics_unavailable
        .as_deref()
        .unwrap_or_default()
        .contains("diagnostics ledger lock poisoned"));
}

#[tokio::test]
async fn workflow_execution_session_preserves_unload_error_when_unload_diagnostics_unavailable() {
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));
    let diagnostics_ledger = service
        .diagnostics_ledger
        .as_ref()
        .expect("diagnostics ledger configured")
        .clone();
    let host = FailingUnloadWithPoisonedDiagnosticsHost::new(diagnostics_ledger);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-unload-diagnostics-unavailable".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("workflow run should preserve unload failure");

    assert_eq!(error.code(), WorkflowErrorCode::RuntimeNotReady);
    assert!(error.message().contains("runtime unload failed"));
    assert!(!error.message().contains("diagnostics ledger lock poisoned"));
}

#[tokio::test]
async fn workflow_execution_session_runtime_load_failure_uses_phase_hint() {
    let host =
        FailingRuntimeLoadHost::with_phase_hint(WorkflowRuntimeDiagnosticPhaseHint::ManagedBinary);
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-runtime-load-error".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("runtime load should fail the run");
    assert_eq!(
        error.runtime_diagnostic_phase_hint(),
        Some(WorkflowRuntimeDiagnosticPhaseHint::ManagedBinary)
    );

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 30,
        )
        .expect("diagnostic events")
    };
    let error_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
        })
        .expect("canonical runtime load error event");

    assert!(error_event.payload_json.contains("managed_binary"));
    assert!(error_event.payload_json.contains("managed_binary_failed"));
}

struct RuntimeInferenceSessionHost {
    inner: MockWorkflowHost,
    runtime_load_attempts: Arc<AtomicUsize>,
    run_attempts: Arc<AtomicUsize>,
}

impl RuntimeInferenceSessionHost {
    fn new() -> Self {
        Self {
            inner: MockWorkflowHost::new(8, 1024),
            runtime_load_attempts: Arc::new(AtomicUsize::new(0)),
            run_attempts: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[derive(Default)]
struct RecordingReservationLifecyclePort {
    events: Mutex<Vec<ReservationLifecycleEvent>>,
}

impl RecordingReservationLifecyclePort {
    fn events(&self) -> Vec<ReservationLifecycleEvent> {
        self.events
            .lock()
            .expect("reservation lifecycle event lock")
            .clone()
    }
}

#[async_trait::async_trait]
impl ReservationLifecyclePort for RecordingReservationLifecyclePort {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.events
            .lock()
            .expect("reservation lifecycle event lock")
            .push(event.clone());
        Ok(ReservationLifecycleApplication {
            contract_version: RESERVATION_LIFECYCLE_CONTRACT_VERSION,
            lifecycle_event_id: event.lifecycle_event_id,
            reservation_lease_id: event.reservation_lease_id,
            state: ReservationLifecycleApplicationState::Applied,
            diagnostics: Vec::new(),
        })
    }
}

#[derive(Default)]
struct CompletingRuntimeHostPort {
    requests: Mutex<Vec<RuntimeHostExecutionRequest>>,
}

impl CompletingRuntimeHostPort {
    fn requests(&self) -> Vec<RuntimeHostExecutionRequest> {
        self.requests
            .lock()
            .expect("runtime host request lock")
            .clone()
    }
}

#[async_trait::async_trait]
impl RuntimeHostExecutionPort for CompletingRuntimeHostPort {
    async fn execute_runtime_host_request(
        &self,
        request: RuntimeHostExecutionRequest,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        self.requests
            .lock()
            .expect("runtime host request lock")
            .push(request.clone());
        Ok(RuntimeHostExecutionResponse {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            execution_request_id: request.execution_request_id,
            workflow_id: request.handoff.task_intent.workflow_id,
            workflow_run_id: request.handoff.task_intent.workflow_run_id,
            node_id: request.handoff.task_intent.node_id,
            task_id: request.handoff.task_intent.task_id,
            state: RuntimeHostExecutionState::Completed,
            outputs: vec![RuntimeHostExecutionOutput {
                port_id: "image".to_string(),
                value: RuntimeHostExecutionOutputValue::MediaArtifactRef(
                    RuntimeHostExecutionMediaArtifactRef {
                        artifact_id: "runtime-output-image".to_string(),
                        media_type: Some("image_png".to_string()),
                    },
                ),
            }],
            diagnostics: Vec::new(),
            terminal_metadata: None,
        })
    }
}

struct SingleCanonicalRuntimeDispatchCandidateProvider;

impl WorkflowRuntimeDispatchCandidateProvider for SingleCanonicalRuntimeDispatchCandidateProvider {
    fn runtime_dispatch_candidates(
        &self,
        task: &WorkflowSchedulerTask,
        _ready_record: &SchedulerTaskStateRecord,
        _readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>
    {
        let intent = task.schedulable_intent.as_ref().ok_or_else(|| {
            WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: format!(
                    "runtime scheduler task '{}' is missing schedulable intent",
                    task.task_id.as_str()
                ),
            }
        })?;
        let selected_runtime_id =
            intent
                .constraints
                .requested_runtime_id
                .clone()
                .ok_or_else(|| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                    message: format!(
                    "runtime scheduler task '{}' has no requested runtime id for test candidate",
                    task.task_id.as_str()
                ),
                })?;
        let selected_device_id =
            intent
                .constraints
                .requested_device_id
                .clone()
                .ok_or_else(|| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                    message: format!(
                        "runtime scheduler task '{}' has no requested device id for test candidate",
                        task.task_id.as_str()
                    ),
                })?;
        Ok(WorkflowRuntimeDispatchCandidateSet {
            candidates: vec![SchedulerDispatchCandidate {
                candidate_id: SchedulerDispatchCandidateId::parse("candidate.runtime_session_test")
                    .map_err(
                        |error| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                            message: error.to_string(),
                        },
                    )?,
                selected_runtime_id,
                selected_runtime_variant_id: None,
                selected_device_ids: vec![selected_device_id.clone()],
                selected_model_ref: intent.model_ref.clone(),
                runtime_trait_settings: Vec::new(),
                reservation: Some(SchedulerResourceReservation {
                    reservation_lease_id: SchedulerReservationLeaseId::parse(
                        "reservation.runtime_session_test",
                    )
                    .map_err(|error| {
                        WorkflowRuntimeDispatchCandidateProviderError::Failed {
                            message: error.to_string(),
                        }
                    })?,
                    workflow_run_id: intent.workflow_run_id.clone(),
                    task_id: intent.task_id.clone(),
                    device_id: selected_device_id,
                    resource_kind: SchedulerResourceKind::DeviceVram,
                    reserved_bytes: 1,
                }),
                resource_fit_assessment: Some(SchedulerResourceFitAssessment {
                    workflow_run_id: intent.workflow_run_id.clone(),
                    task_id: intent.task_id.clone(),
                    state: SchedulerResourceFitState::Fits,
                    diagnostics: Vec::new(),
                }),
                batching_group_id: None,
                candidate_source_diagnostics: Vec::new(),
            }],
            diagnostics: Vec::new(),
        })
    }
}

#[async_trait::async_trait]
impl WorkflowHost for RuntimeInferenceSessionHost {
    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        self.inner.validate_workflow(workflow_id).await
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("runtime-inference-session-graph".to_string())
    }

    async fn workflow_graph(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        Ok(runtime_inference_session_graph())
    }

    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        self.inner.workflow_capabilities(workflow_id).await
    }

    async fn workflow_io(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        Ok(WorkflowIoResponse {
            inputs: vec![WorkflowIoNode {
                node_id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                name: None,
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "text".to_string(),
                    name: None,
                    description: None,
                    data_type: Some("string".to_string()),
                    required: Some(true),
                    multiple: Some(false),
                }],
            }],
            outputs: vec![WorkflowIoNode {
                node_id: "infer".to_string(),
                node_type: "llm-inference".to_string(),
                name: None,
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "image".to_string(),
                    name: None,
                    description: None,
                    data_type: Some("media_artifact_ref".to_string()),
                    required: Some(false),
                    multiple: Some(false),
                }],
            }],
        })
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        self.inner.runtime_capabilities().await
    }

    async fn load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        self.runtime_load_attempts.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        self.run_attempts.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }
}

fn runtime_inference_session_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({}),
            },
            GraphNode {
                id: "infer".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: serde_json::json!({
                    "task_kind": "image_generation",
                    "runtime": "pytorch",
                    "device": "cuda:0",
                    "pumas_model_ref": {
                        "model_id": "image/example/tiny-diffusion",
                        "revision": "main",
                        "selected_artifact_id": "diffusers-bundle"
                    }
                }),
            },
        ],
        edges: Vec::new(),
        derived_graph: None,
    }
}

fn runtime_executable_validation_snapshot(
    version: &pantograph_runtime_attribution::WorkflowVersionRecord,
    graph: &WorkflowGraph,
) -> WorkflowExecutableValidationSnapshotRecord {
    let model_ref = PumasModelRef {
        model_id: "image/example/tiny-diffusion".to_string(),
        revision: Some("main".to_string()),
        selected_artifact_id: Some("diffusers-bundle".to_string()),
        selected_artifact_path: None,
        migration_diagnostics: Vec::new(),
    };
    let selected_binding_ids =
        vec![
            pantograph_dependency_planning::DependencyBindingId::parse("torch-diffusers")
                .expect("valid binding id"),
        ];
    let dependency_proof =
        runtime_dependency_requirements_proof(version, &model_ref, selected_binding_ids);
    WorkflowExecutableValidationSnapshotRecord {
        schema_version: WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION,
        validation_snapshot_id: WorkflowExecutableValidationSnapshotId::parse(
            "wfvalsnap_00000000-0000-4000-8000-000000000020",
        )
        .expect("valid snapshot id"),
        workflow_id: version.workflow_id.clone(),
        workflow_version_id: version.workflow_version_id.clone(),
        workflow_semantic_version: version.semantic_version.clone(),
        workflow_execution_fingerprint: version.execution_fingerprint.clone(),
        descriptor_contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
        graph_revision: WorkflowGraphRevision::parse(&graph.compute_fingerprint())
            .expect("valid graph revision"),
        validation_session_id: DraftGraphValidationSessionId::parse("runtime_validation_session_1")
            .expect("valid validation session id"),
        validation_summary: DraftGraphValidationSummary {
            status: DraftGraphValidationStatus::Executable,
            executable: true,
            enqueue_disabled_reasons: Vec::new(),
            diagnostics_count: 0,
            blocking_diagnostics_count: 0,
        },
        nodes: vec![WorkflowExecutableValidationSnapshotNode {
            node_id: WorkflowNodeId::parse("infer").expect("valid node id"),
            descriptor_fingerprint: InferenceInterfaceFingerprint::parse(
                "runtime_descriptor_fingerprint_1",
            )
            .expect("valid descriptor fingerprint"),
            task_kind: InferenceTaskKind::parse("image_generation").expect("valid task kind"),
            model_ref,
            constraints: pantograph_scheduler::SchedulerRuntimeDeviceConstraints {
                requested_runtime_id: Some(
                    RuntimeIntentId::parse("pytorch").expect("valid runtime id"),
                ),
                requested_device_id: Some(
                    DeviceIntentId::parse("cuda:0").expect("valid device id"),
                ),
            },
            availability_status: InferenceAvailabilityStatus::Available,
            validation_status: DraftGraphValidationStatus::Executable,
            trait_settings: Vec::new(),
            estimate_hints: Vec::new(),
            dependency_requirements_id: dependency_proof.dependency_requirements_id,
            selected_binding_ids: dependency_proof.identity_key.selected_binding_ids,
            dependency_override_fingerprint: dependency_proof.dependency_override_fingerprint,
            blocking_diagnostics: Vec::new(),
        }],
    }
}

fn runtime_dependency_requirements_proof(
    version: &pantograph_runtime_attribution::WorkflowVersionRecord,
    model_ref: &PumasModelRef,
    selected_binding_ids: Vec<pantograph_dependency_planning::DependencyBindingId>,
) -> pantograph_dependency_planning::DependencyRequirementsProof {
    let request = runtime_dependency_planning_request(version, model_ref, selected_binding_ids);
    let validated_request =
        ValidatedDependencyPlanningRequest::try_from(request).expect("valid planning request");
    produce_dependency_requirements_proof(&validated_request, None)
        .expect("dependency requirements proof")
}

fn runtime_dependency_environment_request(
    version: &pantograph_runtime_attribution::WorkflowVersionRecord,
) -> ValidatedDependencyEnvironmentRequest {
    let model_ref = PumasModelRef {
        model_id: "image/example/tiny-diffusion".to_string(),
        revision: Some("main".to_string()),
        selected_artifact_id: Some("diffusers-bundle".to_string()),
        selected_artifact_path: None,
        migration_diagnostics: Vec::new(),
    };
    let selected_binding_ids =
        vec![
            pantograph_dependency_planning::DependencyBindingId::parse("torch-diffusers")
                .expect("valid binding id"),
        ];
    let planning_request =
        runtime_dependency_planning_request(version, &model_ref, selected_binding_ids);
    let identity_key = DependencyPlanningIdentityKey::from_planning_request(&planning_request)
        .expect("dependency identity key");
    let validated_request = ValidatedDependencyPlanningRequest::try_from(planning_request.clone())
        .expect("valid planning request");
    let dependency_proof = produce_dependency_requirements_proof(&validated_request, None)
        .expect("dependency requirements proof");
    ValidatedDependencyEnvironmentRequest::try_from(DependencyEnvironmentRequest {
        contract_version: 1,
        action: DependencyEnvironmentAction::Resolve,
        identity_key,
        planning_request,
        dependency_requirements_id: Some(dependency_proof.dependency_requirements_id),
        environment_ref: None,
    })
    .expect("valid dependency environment request")
}

fn runtime_dependency_planning_request(
    version: &pantograph_runtime_attribution::WorkflowVersionRecord,
    model_ref: &PumasModelRef,
    selected_binding_ids: Vec<pantograph_dependency_planning::DependencyBindingId>,
) -> DependencyPlanningRequest {
    DependencyPlanningRequest {
        model_ref: model_ref.clone(),
        task_id: pantograph_dependency_planning::DependencyTaskId::parse("image_generation")
            .expect("valid task id"),
        task_type: Some(
            pantograph_dependency_planning::DependencyTaskId::parse("image_generation")
                .expect("valid task type"),
        ),
        expected_artifact_kind: None,
        scheduler_intent: SchedulerIntent {
            requested_runtime_id: Some(
                RuntimeIntentId::parse("pytorch").expect("valid runtime id"),
            ),
            requested_device_id: Some(DeviceIntentId::parse("cuda:0").expect("valid device id")),
        },
        platform_context: None,
        selected_binding_ids,
        dependency_override_patches: Vec::new(),
        trait_intents: Vec::new(),
        caller_context: DependencyPlanningCallerContext {
            source_node_type: Some(
                DependencyNodeTypeId::parse("llm-inference").expect("valid node type"),
            ),
            workflow_id: Some(version.workflow_id.as_str().to_string()),
            node_id: Some("infer".to_string()),
            port_id: None,
            run_id: None,
        },
    }
}

fn retained_io_test_artifact_policy() -> ArtifactPolicy {
    ArtifactPolicy {
        policy_id: "retained-io-test-policy".to_string(),
        policy_version: 1,
        ttl_seconds: None,
        max_disk_bytes: Some(1024 * 1024),
        max_memory_bytes: Some(1024 * 1024),
        max_single_artifact_bytes: Some(1024 * 1024),
        spill_threshold_bytes: Some(1024),
        delete_on_consume: false,
    }
}

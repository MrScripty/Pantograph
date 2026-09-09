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
    DependencyReadinessPolicy, DependencyReadinessProofEnvelope, DependencyRequirement,
    DependencyRequirementBinding, DependencyRequirementKind, DependencyRequirementName,
    DeviceIntentId, PumasModelRef, PythonPackageManagerKind, PythonRequirementDetails,
    RuntimeIntentId, SchedulerIntent, ValidatedDependencyEnvironmentRequest,
    ValidatedDependencyPlanningRequest,
};
use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, DraftGraphValidationStatus, DraftGraphValidationSummary,
    InferenceAvailabilityStatus, InferenceInterfaceFingerprint, InferenceTaskKind,
    WorkflowGraphRevision, WorkflowNodeId, INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use pantograph_runtime_host_contracts::{
    ReservationLifecycleApplication, ReservationLifecycleApplicationState,
    ReservationLifecycleEvent, ReservationLifecycleOutcome, ReservationLifecyclePort,
    ReservationLifecyclePortError, RuntimeHostBatchExecutionMemberResponse,
    RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionPort,
    RuntimeHostBatchExecutionRequest, RuntimeHostBatchExecutionResponse,
    RuntimeHostBatchExecutionState, RuntimeHostBatchMemberReservationDisposition,
    RuntimeHostBatchMemberRetryDisposition, RuntimeHostExecutionCancellationHandle,
    RuntimeHostExecutionCancellationSnapshot, RuntimeHostExecutionDiagnostic,
    RuntimeHostExecutionDiagnosticCode, RuntimeHostExecutionDiagnosticSeverity,
    RuntimeHostExecutionInputValue, RuntimeHostExecutionMediaArtifactRef,
    RuntimeHostExecutionOutput, RuntimeHostExecutionOutputValue, RuntimeHostExecutionPort,
    RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    RuntimeHostExecutionState, RESERVATION_LIFECYCLE_CONTRACT_VERSION,
    RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};
use pantograph_scheduler::{
    SchedulerDispatchCandidateId, SchedulerEstimateHint, SchedulerEstimateHintKind,
    SchedulerReservationLeaseId, SchedulerResourceFitAssessment, SchedulerResourceFitState,
    SchedulerResourceKind, SchedulerResourceReservation, SchedulerTaskStateKind,
    SchedulerTaskStateRecord,
};

use super::*;
use crate::scheduler::WorkflowDependencyReadinessLifecycle;
use crate::scheduler::{
    WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistryHandle,
    WorkflowSchedulerLifecycleComponentState,
};
use crate::workflow::runtime_dispatch_selection::{
    ValidatedWorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateFact,
    WorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateProviderError,
    WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchLoadState,
    WorkflowRuntimeDispatchSourceRefreshError, WorkflowRuntimeDispatchSourceRefresher,
    WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
};
use crate::{GraphNode, Position, WorkflowTechnicalFitDeviceClass};

fn assert_immediate_runtime_members<'a>(
    service: &WorkflowService,
    requests: &'a [RuntimeHostBatchExecutionRequest],
) -> Vec<&'a pantograph_runtime_host_contracts::RuntimeHostBatchExecutionMemberRequest> {
    assert!(requests
        .iter()
        .all(|request| (1..=8).contains(&request.members.len())));
    let members = requests
        .iter()
        .flat_map(|request| &request.members)
        .collect::<Vec<_>>();
    assert_eq!(members.len(), 2);
    assert_ne!(
        members[0].execution_request_id,
        members[1].execution_request_id
    );
    assert_ne!(members[0].assignment_id, members[1].assignment_id);
    assert_ne!(
        members[0].handoff.workflow_run_id,
        members[1].handoff.workflow_run_id
    );
    let mut attempts = std::collections::BTreeSet::new();
    for member in &members {
        let assignment_id =
            super::super::runtime_dispatch_assignment::WorkflowRuntimeDispatchAssignmentId::parse(
                member.assignment_id.clone(),
            )
            .expect("assignment id");
        let assignment = service
            .runtime_dispatch_assignment_for_test(&assignment_id)
            .expect("persisted assignment");
        assert_eq!(
            assignment.workflow_run_id,
            member.handoff.workflow_run_id.as_str()
        );
        assert!(attempts.insert(assignment.scheduler_task_attempt_id));
    }
    members
}

fn assert_runtime_member_sessions(
    service: &WorkflowService,
    members: &[&pantograph_runtime_host_contracts::RuntimeHostBatchExecutionMemberRequest],
    expected: &[String],
) {
    let mut sessions = members.iter().map(|member| {
        let assignment_id = super::super::runtime_dispatch_assignment::WorkflowRuntimeDispatchAssignmentId::parse(member.assignment_id.clone()).expect("assignment id");
        let assignment = service.runtime_dispatch_assignment_for_test(&assignment_id).expect("persisted assignment");
        let event = service.runtime_branch_task_event_for_test(&assignment.runtime_branch_event_id).expect("persisted event");
        assert_eq!(event.workflow_run_id, member.handoff.workflow_run_id.as_str());
        assert_eq!(event.session_id, assignment.session_id);
        event.session_id
    }).collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    sessions.sort();
    expected.sort();
    assert_eq!(sessions, expected);
}

fn assert_runtime_member_run_ids(
    members: &[&pantograph_runtime_host_contracts::RuntimeHostBatchExecutionMemberRequest],
    expected: &[String],
) {
    let mut actual = members
        .iter()
        .map(|member| member.handoff.workflow_run_id.as_str().to_string())
        .collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
}

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
async fn workflow_execution_session_records_non_runtime_scheduler_attempt_lifecycle_events() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-non-runtime-attempt-diagnostics".to_string(),
                usage_profile: Some("generic-run".to_string()),
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
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("attempt diagnostics"),
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
    let attempt_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerTaskAttemptLifecycleChanged
                && event.node_id.as_deref() == Some("text-output-1")
        })
        .expect("non-runtime scheduler attempt started event");
    assert_eq!(
        attempt_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
    );
    assert_eq!(
        attempt_event.workflow_run_id.as_ref().map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert!(attempt_event
        .payload_json
        .contains("\"transition\":\"started\""));
    assert!(attempt_event
        .payload_json
        .contains("\"execution_class\":\"non_runtime_node_engine\""));
    assert!(attempt_event
        .payload_json
        .contains("\"scheduler_task_id\":\"text-output-1\""));
    assert!(attempt_event
        .payload_json
        .contains("\"scheduler_attempt_id\":\"scheduler-task-attempt."));
    assert!(attempt_event.payload_json.contains("\"started_at_ms\":"));
    let completed_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerTaskAttemptLifecycleChanged
                && event.node_id.as_deref() == Some("text-output-1")
                && event.payload_json.contains("\"transition\":\"completed\"")
        })
        .expect("non-runtime scheduler attempt completed event");
    assert_eq!(
        completed_event
            .workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert!(completed_event
        .payload_json
        .contains("\"execution_class\":\"non_runtime_node_engine\""));
    assert!(completed_event.payload_json.contains("\"ended_at_ms\":"));
    assert!(completed_event.payload_json.contains("\"duration_ms\":"));
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
    service
        .workflow_shutdown_scheduler_task_lifecycle(
            std::time::Duration::ZERO,
            std::time::Duration::ZERO,
        )
        .await
        .expect("timeout cleanup should release task lifecycle handles");
}

#[tokio::test]
async fn workflow_execution_session_rejects_new_run_when_task_lifecycle_shutdown() {
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
    service
        .workflow_shutdown_scheduler_task_lifecycle(
            std::time::Duration::ZERO,
            std::time::Duration::ZERO,
        )
        .await
        .expect("shutdown scheduler task lifecycle");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.0.0".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "input".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "output".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("task lifecycle shutdown should reject new execution");

    assert_eq!(error.code(), WorkflowErrorCode::CapabilityViolation);
    assert!(
        error
            .message()
            .contains("task execution owner is unavailable"),
        "unexpected error: {error}"
    );
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: created.session_id,
        })
        .await
        .expect("list queue after lifecycle rejection");
    assert!(queue.items.is_empty());
    assert!(
        host.recorded_run_options
            .lock()
            .expect("run options lock")
            .is_empty(),
        "task lifecycle rejection must not route through the legacy whole-run host path"
    );
}

#[tokio::test]
async fn workflow_execution_session_runtime_run_fails_closed_before_legacy_launch() {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let created = service
        .create_workflow_execution_session(
            host.as_ref(),
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
            host.as_ref(),
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
async fn workflow_execution_session_unhandled_scheduler_classes_finalize_failed_run() {
    let host = PumasMaterializationSessionHost::new();
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-pumas-materialization-unhandled".to_string(),
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
                session_id: session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("unhandled scheduler classes should fail closed");

    assert_eq!(error.code(), WorkflowErrorCode::CapabilityViolation);
    assert!(error.message().contains("pumas_materialization=1"));
    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("session status after unhandled scheduler class finalization");
    assert_eq!(status.session.run_count, 1);
    assert_eq!(status.session.queued_runs, 0);
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id,
        })
        .await
        .expect("list queue after unhandled scheduler class failure");
    assert!(queue.items.is_empty());

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
        .expect("run terminal event");
    assert!(terminal_event
        .payload_json
        .contains("\"status\":\"failed\""));
    assert!(terminal_event
        .payload_json
        .contains("pumas_materialization=1"));
}

#[tokio::test]
async fn workflow_execution_session_runtime_run_defers_pending_dependency_readiness_before_dispatch(
) {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let runtime = WorkflowSessionExecutionRuntime::new(
        WorkflowService::with_ephemeral_attribution_store()
            .expect("service")
            .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"))
            .with_dependency_readiness_work_queue(dependency_readiness_work_queue.clone()),
        Arc::clone(&host),
    );
    let service = runtime.service();
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
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
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
        })
        .await
        .expect_err("runtime-containing scheduler run should defer at readiness admission");

    assert_eq!(error.code(), WorkflowErrorCode::RuntimeNotReady);
    assert!(
        error
            .message()
            .contains("runtime dependency readiness is pending for scheduler task(s): infer"),
        "unexpected error: {error}"
    );
    let workflow_run_id = {
        let store = service.session_store_guard().expect("session store");
        let active_run_ids = store.active_workflow_run_ids();
        assert_eq!(
            active_run_ids.len(),
            1,
            "readiness-pending runtime run must stay active"
        );
        active_run_ids[0].clone()
    };
    let task_states = service
        .workflow_get_scheduler_task_state_read_models(
            WorkflowSchedulerTaskStateReadModelQueryRequest {
                session_id: session_id.clone(),
                workflow_run_id: workflow_run_id.clone(),
            },
        )
        .await
        .expect("read readiness-pending task states");
    let infer_task = task_states
        .tasks
        .iter()
        .find(|task| task.task_id == "infer")
        .expect("runtime inference task state");
    assert_eq!(infer_task.state, SchedulerTaskStateKind::PausedDeferred);
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("list queue after deferred runtime inference run");
    assert_eq!(queue.items.len(), 1);
    assert_eq!(queue.items[0].workflow_run_id, workflow_run_id);
    assert_eq!(
        queue.items[0].status,
        WorkflowExecutionSessionQueueItemStatus::Running
    );
    assert_eq!(dependency_readiness_work_queue.len(), 1);
    let work_item = dependency_readiness_work_queue
        .pop_next()
        .expect("dependency-readiness work item should be queued before deferred admission");
    assert_eq!(work_item.provenance.session_id.as_str(), session_id);
    assert_eq!(work_item.provenance.task_id.as_str(), "infer");
    assert_eq!(
        work_item.request.as_request().action,
        DependencyEnvironmentAction::Check
    );
    let resumed_error = service
        .resume_workflow_execution_session_runtime_dependency_readiness(
            host.as_ref(),
            WorkflowExecutionSessionResumeRequest {
                session_id: session_id.clone(),
                workflow_run_id: workflow_run_id.clone(),
            },
        )
        .await
        .expect_err("resume should remain pending while readiness facts are missing");
    assert_eq!(resumed_error.code(), WorkflowErrorCode::RuntimeNotReady);
    assert!(
        resumed_error
            .message()
            .contains("runtime dependency readiness is pending for scheduler task(s): infer"),
        "unexpected resume error: {resumed_error}"
    );
    let active_run_ids = {
        let store = service.session_store_guard().expect("session store");
        store.active_workflow_run_ids()
    };
    assert_eq!(active_run_ids, vec![workflow_run_id.clone()]);
    assert_eq!(dependency_readiness_work_queue.len(), 1);
    let resumed_work_item = dependency_readiness_work_queue
        .pop_next()
        .expect("dependency-readiness work item should be requeued on pending resume");
    assert_eq!(resumed_work_item.provenance.session_id.as_str(), session_id);
    assert_eq!(resumed_work_item.provenance.task_id.as_str(), "infer");
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("diagnostic events")
    };
    assert!(
        diagnostic_events.iter().all(|event| {
            event.event_kind != pantograph_diagnostics_ledger::DiagnosticEventKind::RunStarted
                || event.workflow_run_id.as_ref().map(|id| id.as_str())
                    != Some(workflow_run_id.as_str())
        }),
        "readiness-pending worker-owned runtime run must not record a run-started event before readiness proof exists"
    );
    assert!(
        diagnostic_events.iter().all(|event| {
            event.event_kind != pantograph_diagnostics_ledger::DiagnosticEventKind::RunTerminal
                || event.workflow_run_id.as_ref().map(|id| id.as_str())
                    != Some(workflow_run_id.as_str())
        }),
        "readiness-pending runtime run must not record a terminal event"
    );
    service
        .workflow_diagnostics_projection_refresh(WorkflowDiagnosticsProjectionRefreshRequest {
            projections: vec![
                WorkflowDiagnosticsProjectionKind::RunList,
                WorkflowDiagnosticsProjectionKind::RunDetail,
            ],
            workflow_run_id: Some(workflow_run_id.clone()),
            workflow_id: Some(workflow_id.to_string()),
            reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
            batch_size: 20,
        })
        .expect("projection refresh");
    let run_list = service
        .workflow_run_list_query(WorkflowRunListQueryRequest {
            workflow_id: Some(workflow_id.to_string()),
            limit: Some(10),
            projection_batch_size: Some(20),
            ..WorkflowRunListQueryRequest::default()
        })
        .expect("run list query");
    let run = run_list
        .runs
        .iter()
        .find(|run| run.workflow_run_id.as_str() == workflow_run_id)
        .expect("readiness-pending run list record");
    assert_eq!(
        run.workflow_execution_session_resume_state,
        Some(pantograph_diagnostics_ledger::WorkflowExecutionSessionResumeState::DependencyReadinessPending)
    );
    let detail = service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: workflow_run_id.clone(),
            projection_batch_size: Some(20),
        })
        .expect("run detail query")
        .run
        .expect("run detail");
    assert_eq!(
        detail.workflow_execution_session_resume_state,
        Some(pantograph_diagnostics_ledger::WorkflowExecutionSessionResumeState::DependencyReadinessPending)
    );
    assert_eq!(
        service
            .workflow_execution_session_runtime_dependency_readiness_resume_candidates()
            .expect("resume candidates"),
        vec![WorkflowExecutionSessionResumeRequest {
            session_id: session_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
        }]
    );
    let recovery_report = service
        .workflow_execution_session_bootstrap_recovery_report()
        .expect("bootstrap recovery report");
    assert_eq!(recovery_report.active_runs.len(), 1);
    assert_eq!(recovery_report.active_runs[0].session_id, session_id);
    assert_eq!(
        recovery_report.active_runs[0].workflow_run_id,
        workflow_run_id
    );
    assert_eq!(recovery_report.active_runs[0].runtime_tasks.len(), 1);
    assert_eq!(
        recovery_report.active_runs[0].runtime_tasks[0].task_id,
        "infer"
    );
    assert_eq!(
        recovery_report.active_runs[0].runtime_tasks[0].state_kind,
        Some(SchedulerTaskStateKind::PausedDeferred)
    );
    assert_eq!(
        recovery_report.active_runs[0].runtime_tasks[0].action,
        WorkflowExecutionSessionBootstrapRecoveryAction::RetryDependencyReadiness
    );
    let recovery_plan = service
        .workflow_execution_session_bootstrap_recovery_plan()
        .expect("bootstrap recovery plan");
    assert_eq!(recovery_plan.blocking_decision_count, 0);
    assert_eq!(
        recovery_plan.resume_requests,
        vec![WorkflowExecutionSessionResumeRequest {
            session_id,
            workflow_run_id,
        }]
    );
    assert_eq!(recovery_plan.decisions.len(), 1);
    assert_eq!(
        recovery_plan.decisions[0].decision_kind,
        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeRuntimeDependencyReadiness
    );
}

#[tokio::test]
async fn workflow_execution_session_ready_runtime_task_fails_closed_without_dispatch_candidate() {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let runtime = WorkflowSessionExecutionRuntime::new(
        WorkflowService::with_ephemeral_attribution_store()
            .expect("service")
            .with_dependency_environment_provider(std::sync::Arc::new(
                dependency_readiness_provider.clone(),
            ))
            .with_dependency_readiness_work_queue(dependency_readiness_work_queue.clone()),
        Arc::clone(&host),
    );
    let service = runtime.service();
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
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
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
        })
        .await
        .expect_err("ready runtime task should fail closed without dispatch candidate wiring");

    assert_eq!(error.code(), WorkflowErrorCode::InternalError);
    assert!(
        error
            .message()
            .contains("scheduler dispatch selection did not select a runtime task"),
        "unexpected error: {error}"
    );
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("list queue after dispatch fail-closed runtime inference run");
    assert_eq!(queue.items.len(), 1);
    assert_eq!(
        queue.items[0].status,
        WorkflowExecutionSessionQueueItemStatus::Running
    );
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
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(RecordingRuntimeDispatchSourceRefresher::default());
    let runtime_host_batch_port = Arc::new(CompletingRuntimeHostBatchPort::default());
    let reservation_lifecycle_port = Arc::new(RecordingReservationLifecyclePort::default());
    let runtime = WorkflowSessionExecutionRuntime::new(
        WorkflowService::with_ephemeral_attribution_store()
            .expect("service")
            .with_dependency_environment_provider(std::sync::Arc::new(
                dependency_readiness_provider.clone(),
            ))
            .with_dependency_readiness_work_queue(dependency_readiness_work_queue.clone())
            .with_runtime_dispatch_source_refresher(source_refresher.clone())
            .with_runtime_dispatch_candidate_provider(Arc::new(
                SingleCanonicalRuntimeDispatchCandidateProvider,
            ))
            .with_runtime_host_batch_execution_port(runtime_host_batch_port.clone())
            .with_reservation_lifecycle_port(reservation_lifecycle_port.clone()),
        Arc::clone(&host),
    );
    let service = runtime.service();
    reservation_lifecycle_port.observe_reservation_cleanup_lifecycle(
        service
            .scheduler_task_orchestrator
            .scheduler_lifecycle_handle(),
    );
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

    let first_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first session");
    let second_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create second session");
    let first_session_id = first_created.session_id.clone();
    let second_session_id = second_created.session_id.clone();

    let first_run_request = WorkflowExecutionSessionRunRequest {
        session_id: first_created.session_id,
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
    };
    let second_run_request = WorkflowExecutionSessionRunRequest {
        session_id: second_created.session_id,
        workflow_semantic_version: workflow_semantic_version.to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("paint a blue cube"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "infer".to_string(),
            port_id: "image".to_string(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    let first_run = runtime.run_workflow_execution_session(first_run_request);
    let second_run = runtime.run_workflow_execution_session(second_run_request);
    let (first_response, second_response) = tokio::join!(first_run, second_run);
    let first_response =
        first_response.expect("first compatible runtime task should complete through batch");
    let second_response =
        second_response.expect("second compatible runtime task should complete through batch");

    for response in [&first_response, &second_response] {
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
    }
    for session_id in [&first_session_id, &second_session_id] {
        let status = service
            .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
                session_id: session_id.clone(),
            })
            .await
            .expect("session status after worker-owned runtime run finalization");
        assert_eq!(status.session.run_count, 1);
        assert_eq!(status.session.queued_runs, 0);
    }
    let recorded = runtime_host_batch_port.requests();
    let members = assert_immediate_runtime_members(&service, &recorded);
    assert_runtime_member_run_ids(
        &members,
        &[
            first_response.workflow_run_id.clone(),
            second_response.workflow_run_id.clone(),
        ],
    );
    let mut recorded_prompts = members
        .iter()
        .map(|member| {
            assert_eq!(member.materialized_inputs.len(), 1);
            assert_eq!(member.materialized_inputs[0].port_id, "prompt");
            assert_eq!(
                member
                    .handoff
                    .dispatch_decision
                    .as_ref()
                    .expect("dispatch-selected handoff")
                    .selected_runtime_id
                    .as_str(),
                "pytorch"
            );
            let RuntimeHostExecutionInputValue::String(value) =
                &member.materialized_inputs[0].value
            else {
                panic!(
                    "unexpected runtime host input value: {:?}",
                    member.materialized_inputs[0].value
                );
            };
            value.clone()
        })
        .collect::<Vec<_>>();
    recorded_prompts.sort();
    assert_eq!(
        recorded_prompts,
        vec![
            "paint a blue cube".to_string(),
            "paint a red cube".to_string()
        ]
    );
    for (response, session_id) in [
        (&first_response, &first_session_id),
        (&second_response, &second_session_id),
    ] {
        let event_id =
            super::super::runtime_branch_task_event::WorkflowRuntimeBranchTaskEventId::parse(
                format!(
                    "runtime-branch-task-event.{}.infer",
                    response.workflow_run_id
                ),
            )
            .expect("runtime branch event id");
        let event = service
            .runtime_branch_task_event_for_test(&event_id)
            .expect("runtime branch event should persist worker dispatch facts");
        assert!(event.selected_candidate_fact.is_some());
        let assignment_link = event
            .dispatch_assignment_link
            .as_ref()
            .expect("runtime branch event should link durable dispatch assignment");
        assert_eq!(
            event.scheduler_task_attempt_id.as_deref(),
            Some(assignment_link.scheduler_task_attempt_id.as_str())
        );
        let assignment = service
            .runtime_dispatch_assignment_for_test(&assignment_link.assignment_id)
            .expect("runtime dispatch assignment should be persisted");
        assert_eq!(
            assignment.runtime_branch_event_id.as_str(),
            event.event_id.as_str()
        );
        assert_eq!(assignment.session_id, **session_id);
        assert_eq!(assignment.workflow_id, workflow_id);
        assert_eq!(assignment.workflow_run_id, response.workflow_run_id);
        assert_eq!(assignment.scheduler_task_id, "infer");
        assert_eq!(assignment.timeout_ms, None);
        assert_eq!(
            assignment.runtime_source_context.operation_type,
            "image-generation.txt2img"
        );
        assert_eq!(
            assignment.runtime_source_context.context_shape_key,
            "txt2img.1024x1024.steps30"
        );
        assert_eq!(
            assignment.runtime_source_context.cancellation_mode,
            "per-run-fanout"
        );
        assert_eq!(
            assignment.scheduler_task_attempt_id,
            assignment_link.scheduler_task_attempt_id.as_str()
        );
        assert_eq!(
            assignment.selected_candidate_fact.candidate_id,
            event
                .selected_candidate_fact
                .as_ref()
                .expect("selected candidate fact")
                .candidate_id
        );
        assert_eq!(
            assignment.reservation_lease_id.as_str(),
            "reservation.runtime_session_test"
        );
        let task_attempt_fact = assignment
            .task_attempt_fact
            .as_ref()
            .expect("runtime dispatch assignment should persist task-attempt fact when running");
        assert_eq!(task_attempt_fact.workflow_id, workflow_id);
        assert_eq!(task_attempt_fact.workflow_run_id, response.workflow_run_id);
        assert_eq!(task_attempt_fact.scheduler_task_id, "infer");
        assert_eq!(
            task_attempt_fact.scheduler_task_attempt_id,
            assignment.scheduler_task_attempt_id
        );
        assert_eq!(
            task_attempt_fact.runtime_residency_key,
            "test-runtime:image/example/tiny-diffusion"
        );
        assert_eq!(task_attempt_fact.reservations.len(), 1);
        assert_eq!(
            task_attempt_fact.reservations[0].reservation_lease_id,
            "reservation.runtime_session_test"
        );
        assert_eq!(task_attempt_fact.reservations[0].device_id, "cuda:0");
    }
    assert_eq!(dependency_readiness_work_queue.len(), 2);
    let first_work_item = dependency_readiness_work_queue
        .pop_next()
        .expect("first dependency-readiness work item should be queued after seed");
    let second_work_item = dependency_readiness_work_queue
        .pop_next()
        .expect("second dependency-readiness work item should be queued after seed");
    let mut work_item_sessions = vec![
        first_work_item.provenance.session_id.to_string(),
        second_work_item.provenance.session_id.to_string(),
    ];
    work_item_sessions.sort();
    let mut expected_sessions = vec![first_session_id.clone(), second_session_id.clone()];
    expected_sessions.sort();
    assert_eq!(work_item_sessions, expected_sessions);
    assert_eq!(first_work_item.provenance.task_id.as_str(), "infer");
    assert_eq!(second_work_item.provenance.task_id.as_str(), "infer");
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(
        service
            .scheduler_task_orchestrator
            .scheduler_lifecycle_handle()
            .component(WorkflowSchedulerLifecycleComponentKind::ReservationCleanup)
            .expect("reservation cleanup lifecycle component")
            .state,
        WorkflowSchedulerLifecycleComponentState::NotStarted
    );
    assert_eq!(
        source_refresher.model_refs(),
        vec![
            "image/example/tiny-diffusion",
            "image/example/tiny-diffusion"
        ]
    );
}

#[tokio::test]
async fn workflow_execution_session_resume_consumes_fresh_dependency_readiness_snapshot_and_dispatches_active_run(
) {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(RecordingRuntimeDispatchSourceRefresher::default());
    let runtime_host_port = Arc::new(CompletingRuntimeHostPort::default());
    let reservation_lifecycle_port = Arc::new(RecordingReservationLifecyclePort::default());
    let runtime = WorkflowSessionExecutionRuntime::new(
        WorkflowService::with_ephemeral_attribution_store()
            .expect("service")
            .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"))
            .with_dependency_environment_provider(std::sync::Arc::new(
                dependency_readiness_provider.clone(),
            ))
            .with_dependency_readiness_work_queue(dependency_readiness_work_queue.clone())
            .with_runtime_dispatch_source_refresher(source_refresher.clone())
            .with_runtime_dispatch_candidate_provider(Arc::new(
                SingleCanonicalRuntimeDispatchCandidateProvider,
            ))
            .with_runtime_host_execution_port(runtime_host_port.clone())
            .with_reservation_lifecycle_port(reservation_lifecycle_port.clone()),
        Arc::clone(&host),
    );
    let service = runtime.service();
    let workflow_id = "wf-runtime-resume-dispatch";
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
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let pending_error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
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
        })
        .await
        .expect_err("runtime run should pause before readiness facts exist");
    assert_eq!(pending_error.code(), WorkflowErrorCode::RuntimeNotReady);
    let workflow_run_id = {
        let store = service.session_store_guard().expect("session store");
        let active_run_ids = store.active_workflow_run_ids();
        assert_eq!(active_run_ids.len(), 1);
        active_run_ids[0].clone()
    };
    dependency_readiness_work_queue
        .pop_next()
        .expect("initial dependency-readiness work item");

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

    let response = service
        .resume_workflow_execution_session_runtime_dependency_readiness(
            host.as_ref(),
            WorkflowExecutionSessionResumeRequest {
                session_id: session_id.clone(),
                workflow_run_id: workflow_run_id.clone(),
            },
        )
        .await
        .expect("resume should dispatch once dependency readiness facts are fresh");

    assert_eq!(response.workflow_run_id, workflow_run_id);
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, "infer");
    assert_eq!(response.outputs[0].port_id, "image");
    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("session status after direct dependency-readiness resume finalization");
    assert_eq!(status.session.run_count, 1);
    assert_eq!(status.session.queued_runs, 0);
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("list queue after resumed dispatch");
    assert!(queue.items.is_empty());
    assert_eq!(runtime_host_port.requests().len(), 1);
    assert_eq!(
        source_refresher.model_refs(),
        vec!["image/example/tiny-diffusion"]
    );
    assert_eq!(
        reservation_lifecycle_port
            .events()
            .iter()
            .map(|event| &event.outcome)
            .collect::<Vec<_>>(),
        vec![
            &ReservationLifecycleOutcome::DispatchStarted,
            &ReservationLifecycleOutcome::RuntimeHostCompleted,
        ]
    );
    assert_eq!(dependency_readiness_work_queue.len(), 1);
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 40,
        )
        .expect("diagnostic events")
    };
    let runtime_attempt_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerTaskAttemptLifecycleChanged
                && event.workflow_run_id.as_ref().map(|id| id.as_str())
                    == Some(workflow_run_id.as_str())
                && event.node_id.as_deref() == Some("infer")
        })
        .expect("runtime scheduler attempt started event");
    assert_eq!(
        runtime_attempt_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
    );
    assert!(runtime_attempt_event
        .payload_json
        .contains("\"transition\":\"started\""));
    assert!(runtime_attempt_event
        .payload_json
        .contains("\"execution_class\":\"runtime\""));
    assert!(runtime_attempt_event
        .payload_json
        .contains("\"scheduler_task_id\":\"infer\""));
    let runtime_completed_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerTaskAttemptLifecycleChanged
                && event.workflow_run_id.as_ref().map(|id| id.as_str())
                    == Some(workflow_run_id.as_str())
                && event.node_id.as_deref() == Some("infer")
                && event.payload_json.contains("\"transition\":\"completed\"")
        })
        .expect("runtime scheduler attempt completed event");
    assert_eq!(
        runtime_completed_event.runtime_id.as_deref(),
        Some("pytorch")
    );
    assert!(runtime_completed_event
        .payload_json
        .contains("\"execution_class\":\"runtime\""));
    assert!(runtime_completed_event
        .payload_json
        .contains("\"selected_runtime_id\":\"pytorch\""));
    assert!(runtime_completed_event
        .payload_json
        .contains("\"reservation_id\":\"reservation.runtime_session_test\""));
    assert!(runtime_completed_event
        .payload_json
        .contains("\"ended_at_ms\":"));
    assert!(runtime_completed_event
        .payload_json
        .contains("\"duration_ms\":"));
    assert!(diagnostic_events.iter().any(|event| {
        event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::RunTerminal
            && event.workflow_run_id.as_ref().map(|id| id.as_str())
                == Some(workflow_run_id.as_str())
            && event.payload_json.contains("\"status\":\"completed\"")
    }));
    service
        .workflow_diagnostics_projection_refresh(WorkflowDiagnosticsProjectionRefreshRequest {
            projections: vec![
                WorkflowDiagnosticsProjectionKind::RunList,
                WorkflowDiagnosticsProjectionKind::RunDetail,
            ],
            workflow_run_id: Some(workflow_run_id.clone()),
            workflow_id: Some(workflow_id.to_string()),
            reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
            batch_size: 20,
        })
        .expect("projection refresh after resumed dispatch");
    let run_list = service
        .workflow_run_list_query(WorkflowRunListQueryRequest {
            workflow_id: Some(workflow_id.to_string()),
            limit: Some(10),
            projection_batch_size: Some(20),
            ..WorkflowRunListQueryRequest::default()
        })
        .expect("run list query after resumed dispatch");
    let run = run_list
        .runs
        .iter()
        .find(|run| run.workflow_run_id.as_str() == workflow_run_id)
        .expect("resumed run list record");
    assert_eq!(run.workflow_execution_session_resume_state, None);
    let detail = service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: workflow_run_id.clone(),
            projection_batch_size: Some(20),
        })
        .expect("run detail query after resumed dispatch")
        .run
        .expect("run detail after resumed dispatch");
    assert_eq!(detail.workflow_execution_session_resume_state, None);
    assert!(service
        .workflow_execution_session_runtime_dependency_readiness_resume_candidates()
        .expect("resume candidates after resumed dispatch")
        .is_empty());
}

#[tokio::test]
async fn workflow_execution_session_bootstrap_recovery_applies_dependency_readiness_resume_plan() {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(RecordingRuntimeDispatchSourceRefresher::default());
    let runtime_host_batch_port = Arc::new(CompletingRuntimeHostBatchPort::default());
    let reservation_lifecycle_port = Arc::new(RecordingReservationLifecyclePort::default());
    let service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"))
        .with_dependency_environment_provider(std::sync::Arc::new(
            dependency_readiness_provider.clone(),
        ))
        .with_dependency_readiness_work_queue(dependency_readiness_work_queue.clone())
        .with_runtime_dispatch_source_refresher(source_refresher.clone())
        .with_runtime_dispatch_candidate_provider(Arc::new(
            SingleCanonicalRuntimeDispatchCandidateProvider,
        ))
        .with_runtime_host_batch_execution_port(runtime_host_batch_port.clone())
        .with_reservation_lifecycle_port(reservation_lifecycle_port.clone());
    let runtime = WorkflowSessionExecutionRuntime::new(service, Arc::clone(&host));
    let service = runtime.service();
    let workflow_id = "wf-bootstrap-recovery-resume";
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

    let first_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first session");
    let second_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create second session");
    let first_session_id = first_created.session_id.clone();
    let second_session_id = second_created.session_id.clone();
    let first_run_request = WorkflowExecutionSessionRunRequest {
        session_id: first_created.session_id,
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
    };
    runtime
        .run_workflow_execution_session(first_run_request)
        .await
        .expect_err("first runtime run should pause before readiness facts exist");
    let first_workflow_run_id = {
        let store = service.session_store_guard().expect("session store");
        let active_run_ids = store.active_workflow_run_ids();
        assert_eq!(active_run_ids.len(), 1);
        active_run_ids[0].clone()
    };
    let second_run_request = WorkflowExecutionSessionRunRequest {
        session_id: second_created.session_id,
        workflow_semantic_version: workflow_semantic_version.to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("paint a blue cube"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "infer".to_string(),
            port_id: "image".to_string(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    runtime
        .run_workflow_execution_session(second_run_request)
        .await
        .expect_err("second runtime run should pause before readiness facts exist");
    let workflow_run_ids = {
        let store = service.session_store_guard().expect("session store");
        let mut active_run_ids = store.active_workflow_run_ids();
        active_run_ids.sort();
        assert_eq!(active_run_ids.len(), 2);
        active_run_ids
    };
    let second_workflow_run_id = workflow_run_ids
        .iter()
        .find(|workflow_run_id| *workflow_run_id != &first_workflow_run_id)
        .expect("second workflow run id")
        .clone();
    dependency_readiness_work_queue
        .pop_next()
        .expect("first dependency-readiness work item");
    dependency_readiness_work_queue
        .pop_next()
        .expect("second dependency-readiness work item");

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

    let recovery_result = runtime
        .recover_workflow_execution_session_bootstrap()
        .await
        .expect("bootstrap recovery should resume dependency readiness");

    assert_eq!(recovery_result.plan.blocking_decision_count, 0);
    let mut actual_resume_requests = recovery_result.plan.resume_requests.clone();
    actual_resume_requests.sort_by(|left, right| left.session_id.cmp(&right.session_id));
    let mut expected_resume_requests = vec![
        WorkflowExecutionSessionResumeRequest {
            session_id: first_session_id.clone(),
            workflow_run_id: first_workflow_run_id,
        },
        WorkflowExecutionSessionResumeRequest {
            session_id: second_session_id.clone(),
            workflow_run_id: second_workflow_run_id,
        },
    ];
    expected_resume_requests.sort_by(|left, right| left.session_id.cmp(&right.session_id));
    assert_eq!(actual_resume_requests, expected_resume_requests);
    let mut resumed_run_ids = recovery_result
        .resumed_runs
        .iter()
        .map(|run| run.workflow_run_id.clone())
        .collect::<Vec<_>>();
    resumed_run_ids.sort();
    assert_eq!(resumed_run_ids, workflow_run_ids);
    let recorded_batches = runtime_host_batch_port.requests();
    let members = assert_immediate_runtime_members(&service, &recorded_batches);
    assert_runtime_member_run_ids(&members, &workflow_run_ids);
    assert!(service
        .workflow_execution_session_runtime_dependency_readiness_resume_candidates()
        .expect("resume candidates after bootstrap recovery")
        .is_empty());
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn workflow_execution_session_bootstrap_recovery_applies_progress_loop_before_readiness_resume(
) {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(RecordingRuntimeDispatchSourceRefresher::default());
    let runtime_host_batch_port = Arc::new(CompletingRuntimeHostBatchPort::default());
    let reservation_lifecycle_port = Arc::new(RecordingReservationLifecyclePort::default());
    let service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_dependency_environment_provider(std::sync::Arc::new(
            dependency_readiness_provider.clone(),
        ))
        .with_dependency_readiness_work_queue(dependency_readiness_work_queue.clone())
        .with_runtime_dispatch_source_refresher(source_refresher)
        .with_runtime_dispatch_candidate_provider(Arc::new(
            SingleCanonicalRuntimeDispatchCandidateProvider,
        ))
        .with_runtime_host_batch_execution_port(runtime_host_batch_port.clone())
        .with_reservation_lifecycle_port(reservation_lifecycle_port);
    let runtime = WorkflowSessionExecutionRuntime::new(service, Arc::clone(&host));
    let service = runtime.service();
    let workflow_id = "wf-bootstrap-progress-loop";
    let workflow_semantic_version = "1.2.3";
    let graph = runtime_inference_session_graph();
    let version = service
        .resolve_workflow_graph_version(workflow_id, workflow_semantic_version, &graph)
        .expect("resolve workflow version");
    let snapshot = ValidatedWorkflowExecutableValidationSnapshotRecord::try_from(
        runtime_executable_validation_snapshot(&version, &graph),
    )
    .expect("validated executable snapshot");
    let projections = snapshot
        .scheduler_inference_task_projections()
        .expect("scheduler inference task projections");

    let first_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first session");
    let second_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create second session");
    let first_session_id = first_created.session_id.clone();
    let second_session_id = second_created.session_id.clone();
    let run_request = |session_id: String, prompt: &str| WorkflowExecutionSessionRunRequest {
        session_id,
        workflow_semantic_version: workflow_semantic_version.to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!(prompt),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "infer".to_string(),
            port_id: "image".to_string(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    let first_request = run_request(first_session_id.clone(), "paint a red cube");
    let second_request = run_request(second_session_id.clone(), "paint a blue cube");
    let prepare_progress_recovery_run =
        |session_id: &str, request: &WorkflowExecutionSessionRunRequest| {
            let workflow_run_id = {
                let mut store = service.session_store_guard().expect("session store");
                let workflow_run_id = store.enqueue_run(session_id, request).expect("enqueue run");
                store
                    .begin_queued_run(session_id, &workflow_run_id)
                    .expect("begin run")
                    .expect("dequeued run");
                workflow_run_id
            };
            let task_graph = workflow_scheduler_task_graph_with_inference_projections(
                &pantograph_runtime_attribution::WorkflowId::try_from(workflow_id.to_string())
                    .expect("workflow id"),
                &pantograph_runtime_attribution::WorkflowRunId::try_from(workflow_run_id.clone())
                    .expect("workflow run id"),
                &graph,
                &projections,
            )
            .expect("scheduler task graph");
            let initial_records = service
                .scheduler_task_orchestrator
                .initial_task_state_records(&task_graph)
                .expect("initial task state");

            let mut store = service.session_store_guard().expect("session store");
            store
                .set_active_run_scheduler_task_state(
                    session_id,
                    &workflow_run_id,
                    task_graph,
                    initial_records,
                )
                .expect("store active task state");
            service
                .scheduler_task_orchestrator
                .materialize_external_inputs_for_active_run(
                    &mut store,
                    session_id,
                    &workflow_run_id,
                    &request.inputs,
                )
                .expect("materialize source input");
            workflow_run_id
        };
    let mut workflow_run_ids = vec![
        prepare_progress_recovery_run(&first_session_id, &first_request),
        prepare_progress_recovery_run(&second_session_id, &second_request),
    ];
    workflow_run_ids.sort();
    let recovery_plan = service
        .workflow_execution_session_bootstrap_recovery_plan()
        .expect("recovery plan before apply");
    assert_eq!(recovery_plan.blocking_decision_count, 0);
    assert_eq!(recovery_plan.decisions.len(), 2);
    assert!(recovery_plan.decisions.iter().all(|decision| {
        decision.decision_kind
            == WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeProgressLoop
    }));

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

    let recovery_result = runtime
        .recover_workflow_execution_session_bootstrap()
        .await
        .expect("bootstrap recovery should run progress loop then resume readiness");

    assert!(recovery_result.plan.decisions.iter().all(|decision| {
        decision.decision_kind
            == WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeProgressLoop
    }));
    let mut resumed_run_ids = recovery_result
        .resumed_runs
        .iter()
        .map(|run| run.workflow_run_id.clone())
        .collect::<Vec<_>>();
    resumed_run_ids.sort();
    assert_eq!(resumed_run_ids, workflow_run_ids);
    assert!(recovery_result.final_plan.decisions.is_empty());
    let recorded_batches = runtime_host_batch_port.requests();
    let members = assert_immediate_runtime_members(&service, &recorded_batches);
    assert_runtime_member_run_ids(&members, &workflow_run_ids);
    assert!(service
        .workflow_execution_session_runtime_dependency_readiness_resume_candidates()
        .expect("resume candidates after bootstrap progress recovery")
        .is_empty());
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn workflow_execution_session_bootstrap_recovery_redispatches_ready_runtime_task() {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(RecordingRuntimeDispatchSourceRefresher::default());
    let runtime_host_batch_port = Arc::new(CompletingRuntimeHostBatchPort::default());
    let reservation_lifecycle_port = Arc::new(RecordingReservationLifecyclePort::default());
    let service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"))
        .with_dependency_environment_provider(std::sync::Arc::new(
            dependency_readiness_provider.clone(),
        ))
        .with_dependency_readiness_work_queue(dependency_readiness_work_queue)
        .with_runtime_dispatch_source_refresher(source_refresher)
        .with_runtime_dispatch_candidate_provider(Arc::new(
            SingleCanonicalRuntimeDispatchCandidateProvider,
        ))
        .with_runtime_host_batch_execution_port(runtime_host_batch_port.clone())
        .with_reservation_lifecycle_port(reservation_lifecycle_port);
    let runtime = WorkflowSessionExecutionRuntime::new(service, Arc::clone(&host));
    let service = runtime.service();
    let workflow_id = "wf-bootstrap-ready-redispatch";
    let workflow_semantic_version = "1.2.3";
    let graph = runtime_inference_session_graph();
    let version = service
        .resolve_workflow_graph_version(workflow_id, workflow_semantic_version, &graph)
        .expect("resolve workflow version");
    let snapshot = ValidatedWorkflowExecutableValidationSnapshotRecord::try_from(
        runtime_executable_validation_snapshot(&version, &graph),
    )
    .expect("validated executable snapshot");
    let projections = snapshot
        .scheduler_inference_task_projections()
        .expect("scheduler inference task projections");

    let first_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first session");
    let second_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create second session");
    let first_session_id = first_created.session_id.clone();
    let second_session_id = second_created.session_id.clone();
    let run_request = |session_id: String, prompt: &str| WorkflowExecutionSessionRunRequest {
        session_id,
        workflow_semantic_version: workflow_semantic_version.to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!(prompt),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "infer".to_string(),
            port_id: "image".to_string(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    let first_request = run_request(first_session_id.clone(), "paint a red cube");
    let second_request = run_request(second_session_id.clone(), "paint a blue cube");
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

    let prepare_ready_recovery_run =
        |session_id: &str, request: &WorkflowExecutionSessionRunRequest| {
            let workflow_run_id = {
                let mut store = service.session_store_guard().expect("session store");
                let workflow_run_id = store.enqueue_run(session_id, request).expect("enqueue run");
                store
                    .begin_queued_run(session_id, &workflow_run_id)
                    .expect("begin run")
                    .expect("dequeued run");
                workflow_run_id
            };
            let task_graph = workflow_scheduler_task_graph_with_inference_projections(
                &pantograph_runtime_attribution::WorkflowId::try_from(workflow_id.to_string())
                    .expect("workflow id"),
                &pantograph_runtime_attribution::WorkflowRunId::try_from(workflow_run_id.clone())
                    .expect("workflow run id"),
                &graph,
                &projections,
            )
            .expect("scheduler task graph");
            let initial_records = service
                .scheduler_task_orchestrator
                .initial_task_state_records(&task_graph)
                .expect("initial task state");

            let mut store = service.session_store_guard().expect("session store");
            store
                .set_active_run_scheduler_task_state(
                    session_id,
                    &workflow_run_id,
                    task_graph,
                    initial_records,
                )
                .expect("store active task state");
            service
                .scheduler_task_orchestrator
                .materialize_external_inputs_for_active_run(
                    &mut store,
                    session_id,
                    &workflow_run_id,
                    &request.inputs,
                )
                .expect("materialize source input");
            service
                .scheduler_task_orchestrator
                .advance_awaiting_runtime_task_inputs(
                    &mut store,
                    session_id,
                    &workflow_run_id,
                    "infer",
                )
                .expect("advance runtime task inputs")
                .expect("runtime task should wait for readiness");
            let lifecycle = WorkflowDependencyReadinessLifecycle::new(
                service.scheduler_task_orchestrator.clone(),
            );
            let ready_record = lifecycle
                .resolve_and_admit_active_runtime_task(
                    &mut store,
                    service.dependency_readiness_provider.as_ref(),
                    session_id,
                    &workflow_run_id,
                    "infer",
                    DependencyReadinessPolicy::CheckOnly,
                )
                .expect("admit ready runtime task");
            assert_eq!(ready_record.state.kind(), SchedulerTaskStateKind::Ready);
            workflow_run_id
        };
    let first_workflow_run_id = prepare_ready_recovery_run(&first_session_id, &first_request);
    let second_workflow_run_id = prepare_ready_recovery_run(&second_session_id, &second_request);

    let recovery_plan = service
        .workflow_execution_session_bootstrap_recovery_plan()
        .expect("recovery plan before ready redispatch");
    assert_eq!(recovery_plan.blocking_decision_count, 0);
    assert_eq!(recovery_plan.decisions.len(), 2);
    for decision in &recovery_plan.decisions {
        assert_eq!(
            decision.decision_kind,
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::RedispatchReadyRuntime
        );
        assert!(decision.runtime_dispatch_recovery_state_available);
    }

    let direct_recovery_error = service
        .recover_workflow_execution_session_bootstrap(host.as_ref())
        .await
        .expect_err("direct service recovery should reject runtime redispatch");
    assert!(
        direct_recovery_error
            .message()
            .contains("WorkflowSessionExecutionRuntime"),
        "unexpected direct recovery error: {}",
        direct_recovery_error.message()
    );
    assert_eq!(runtime_host_batch_port.requests().len(), 0);

    let recovery_result = runtime
        .recover_workflow_execution_session_bootstrap()
        .await
        .expect("bootstrap recovery should redispatch ready runtime task");

    assert!(recovery_result.plan.decisions.iter().all(|decision| {
        decision.decision_kind
            == WorkflowExecutionSessionBootstrapRecoveryDecisionKind::RedispatchReadyRuntime
    }));
    let mut resumed_run_ids = recovery_result
        .resumed_runs
        .iter()
        .map(|run| run.workflow_run_id.clone())
        .collect::<Vec<_>>();
    resumed_run_ids.sort();
    let mut expected_run_ids = vec![
        first_workflow_run_id.clone(),
        second_workflow_run_id.clone(),
    ];
    expected_run_ids.sort();
    assert_eq!(resumed_run_ids, expected_run_ids);
    assert!(recovery_result.final_plan.decisions.is_empty());
    let recorded_batches = runtime_host_batch_port.requests();
    let members = assert_immediate_runtime_members(&service, &recorded_batches);
    assert_runtime_member_run_ids(&members, &expected_run_ids);
    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 40,
        )
        .expect("diagnostic events")
    };
    let redispatched_attempt_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerTaskAttemptLifecycleChanged
                && event.node_id.as_deref() == Some("infer")
                && event.payload_json.contains("\"transition\":\"redispatched\"")
        })
        .collect::<Vec<_>>();
    assert_eq!(redispatched_attempt_events.len(), 2);
    for redispatched_attempt_event in redispatched_attempt_events {
        assert_eq!(
            redispatched_attempt_event.source_component,
            pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
        );
        assert!(redispatched_attempt_event
            .payload_json
            .contains("\"execution_class\":\"runtime\""));
        assert!(redispatched_attempt_event
            .payload_json
            .contains("\"scheduler_attempt_id\":\"scheduler-task-attempt."));
        assert!(redispatched_attempt_event
            .payload_json
            .contains("\"started_at_ms\":"));
    }
    assert!(
        !diagnostic_events.iter().any(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerTaskAttemptLifecycleChanged
                && event.node_id.as_deref() == Some("infer")
                && event.payload_json.contains("\"transition\":\"started\"")
        }),
        "bootstrap redispatch should not also emit a started event for the same runtime attempt"
    );
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn workflow_execution_session_resume_rejects_inactive_and_non_runtime_runs() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2);
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-resume-reject".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let inactive_error = service
        .resume_workflow_execution_session_runtime_dependency_readiness(
            &host,
            WorkflowExecutionSessionResumeRequest {
                session_id: created.session_id.clone(),
                workflow_run_id: "run_00000000-0000-4000-8000-000000000001".to_string(),
            },
        )
        .await
        .expect_err("resume should reject missing active run");
    assert_eq!(inactive_error.code(), WorkflowErrorCode::InvalidRequest);
    assert!(inactive_error.message().contains("is not active"));

    let request = WorkflowExecutionSessionRunRequest {
        session_id: created.session_id.clone(),
        workflow_semantic_version: "1.2.3".to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "text-input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("not runtime"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    let workflow_run_id = {
        let mut store = service.session_store_guard().expect("session store");
        let workflow_run_id = store
            .enqueue_run(&created.session_id, &request)
            .expect("enqueue non-runtime run");
        store
            .begin_queued_run(&created.session_id, &workflow_run_id)
            .expect("begin non-runtime run")
            .expect("dequeued non-runtime run");
        workflow_run_id
    };
    let workflow_id =
        pantograph_scheduler::SchedulerWorkflowId::parse("wf-resume-reject").expect("workflow id");
    let scheduler_workflow_run_id =
        pantograph_scheduler::SchedulerWorkflowRunId::parse(&workflow_run_id)
            .expect("scheduler workflow run id");
    let scheduler_task_graph = WorkflowSchedulerTaskGraph {
        schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
        workflow_id: workflow_id.clone(),
        workflow_run_id: scheduler_workflow_run_id.clone(),
        tasks: vec![WorkflowSchedulerTask {
            workflow_id,
            workflow_run_id: scheduler_workflow_run_id,
            node_id: pantograph_scheduler::SchedulerNodeId::parse("text-input-1").expect("node id"),
            task_id: pantograph_scheduler::SchedulerTaskId::parse("text-input-1").expect("task id"),
            node_type: "text-input".to_string(),
            execution_class: WorkflowSchedulerTaskExecutionClass::SourceInput,
            dependency_task_ids: Vec::new(),
            input_bindings: Vec::new(),
            schedulable_intent: None,
            schedulable_intent_template: None,
            non_runtime_task_template: None,
            source_input_task_template: None,
            inference_descriptor_fingerprint: None,
            runtime_source_context: None,
            diagnostics: Vec::new(),
        }],
    };
    let initial_scheduler_task_records = service
        .scheduler_task_orchestrator
        .initial_task_state_records(&scheduler_task_graph)
        .expect("initial non-runtime task state");
    {
        let mut store = service.session_store_guard().expect("session store");
        store
            .set_active_run_scheduler_task_state(
                &created.session_id,
                &workflow_run_id,
                scheduler_task_graph,
                initial_scheduler_task_records,
            )
            .expect("store active non-runtime task state");
    }

    let non_runtime_error = service
        .resume_workflow_execution_session_runtime_dependency_readiness(
            &host,
            WorkflowExecutionSessionResumeRequest {
                session_id: created.session_id,
                workflow_run_id,
            },
        )
        .await
        .expect_err("resume should reject non-runtime active run");
    assert_eq!(non_runtime_error.code(), WorkflowErrorCode::InvalidRequest);
    assert!(non_runtime_error
        .message()
        .contains("is not a runtime inference run"));
}

#[tokio::test]
async fn workflow_execution_session_records_failed_runtime_host_result_as_terminal_task_failure() {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(RecordingRuntimeDispatchSourceRefresher::default());
    let runtime_host_batch_port = Arc::new(FailingRuntimeHostBatchPort::default());
    let reservation_lifecycle_port = Arc::new(RecordingReservationLifecyclePort::default());
    let runtime = WorkflowSessionExecutionRuntime::new(
        WorkflowService::with_ephemeral_attribution_store()
            .expect("service")
            .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"))
            .with_dependency_environment_provider(std::sync::Arc::new(
                dependency_readiness_provider.clone(),
            ))
            .with_dependency_readiness_work_queue(dependency_readiness_work_queue)
            .with_runtime_dispatch_source_refresher(source_refresher)
            .with_runtime_dispatch_candidate_provider(Arc::new(
                SingleCanonicalRuntimeDispatchCandidateProvider,
            ))
            .with_runtime_host_batch_execution_port(runtime_host_batch_port.clone())
            .with_reservation_lifecycle_port(reservation_lifecycle_port.clone()),
        Arc::clone(&host),
    );
    let service = runtime.service();
    let workflow_id = "wf-runtime-host-failed-result";
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

    let first_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first session");
    let second_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create second session");
    let first_session_id = first_created.session_id.clone();
    let second_session_id = second_created.session_id.clone();
    let run_request = |session_id: String, prompt: &str| WorkflowExecutionSessionRunRequest {
        session_id,
        workflow_semantic_version: workflow_semantic_version.to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!(prompt),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "infer".to_string(),
            port_id: "image".to_string(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    let first_run = runtime
        .run_workflow_execution_session(run_request(first_session_id.clone(), "paint a red cube"));
    let second_run = runtime.run_workflow_execution_session(run_request(
        second_session_id.clone(),
        "paint a blue cube",
    ));
    let (first_error, second_error) = tokio::join!(first_run, second_run);
    let first_error =
        first_error.expect_err("first failed runtime-host batch member should fail workflow run");
    let second_error =
        second_error.expect_err("second failed runtime-host batch member should fail workflow run");

    for error in [&first_error, &second_error] {
        assert_eq!(error.code(), WorkflowErrorCode::InternalError);
        assert!(
            error
                .message()
                .contains("runtime branch batch member failed"),
            "unexpected error: {error}"
        );
    }
    let recorded = runtime_host_batch_port.requests();
    let members = assert_immediate_runtime_members(&service, &recorded);
    assert_runtime_member_sessions(
        &service,
        &members,
        &[first_session_id.clone(), second_session_id.clone()],
    );
    for session_id in [&first_session_id, &second_session_id] {
        let status = service
            .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
                session_id: session_id.clone(),
            })
            .await
            .expect("session status after failed grouped runtime run finalization");
        assert_eq!(status.session.run_count, 1);
        assert_eq!(status.session.queued_runs, 0);
    }
}

#[tokio::test]
async fn workflow_execution_session_records_runtime_batch_dispatch_rejection_as_typed_error() {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(RecordingRuntimeDispatchSourceRefresher::default());
    let runtime_host_batch_port = Arc::new(RejectingRuntimeHostBatchPort::default());
    let reservation_lifecycle_port = Arc::new(RecordingReservationLifecyclePort::default());
    let runtime = WorkflowSessionExecutionRuntime::new(
        WorkflowService::with_ephemeral_attribution_store()
            .expect("service")
            .with_dependency_environment_provider(std::sync::Arc::new(
                dependency_readiness_provider.clone(),
            ))
            .with_dependency_readiness_work_queue(dependency_readiness_work_queue)
            .with_runtime_dispatch_source_refresher(source_refresher)
            .with_runtime_dispatch_candidate_provider(Arc::new(
                SingleCanonicalRuntimeDispatchCandidateProvider,
            ))
            .with_runtime_host_batch_execution_port(runtime_host_batch_port.clone())
            .with_reservation_lifecycle_port(reservation_lifecycle_port.clone()),
        Arc::clone(&host),
    );
    let service = runtime.service();
    let workflow_id = "wf-runtime-host-batch-rejection";
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

    let first_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first session");
    let second_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create second session");
    let first_session_id = first_created.session_id.clone();
    let second_session_id = second_created.session_id.clone();
    let run_request = |session_id: String, prompt: &str| WorkflowExecutionSessionRunRequest {
        session_id,
        workflow_semantic_version: workflow_semantic_version.to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!(prompt),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "infer".to_string(),
            port_id: "image".to_string(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    let first_run = runtime
        .run_workflow_execution_session(run_request(first_session_id.clone(), "paint a red cube"));
    let second_run = runtime.run_workflow_execution_session(run_request(
        second_session_id.clone(),
        "paint a blue cube",
    ));
    let (first_error, second_error) = tokio::join!(first_run, second_run);
    let first_error =
        first_error.expect_err("first rejected runtime-host batch dispatch should fail run");
    let second_error =
        second_error.expect_err("second rejected runtime-host batch dispatch should fail run");

    for error in [&first_error, &second_error] {
        assert_eq!(error.code(), WorkflowErrorCode::InternalError);
        assert!(
            error.message().contains("runtime branch batch request"),
            "unexpected error: {error}"
        );
        assert!(
            error.message().contains("runtime-host dispatch failed"),
            "unexpected error: {error}"
        );
    }
    let recorded = runtime_host_batch_port.requests();
    let members = assert_immediate_runtime_members(&service, &recorded);
    assert_runtime_member_sessions(
        &service,
        &members,
        &[first_session_id.clone(), second_session_id.clone()],
    );
}

#[tokio::test]
async fn workflow_shutdown_cancels_blocked_runtime_batch_dispatch() {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(RecordingRuntimeDispatchSourceRefresher::default());
    let runtime_host_batch_port = Arc::new(BlockingRuntimeHostBatchPort::default());
    let reservation_lifecycle_port = Arc::new(RecordingReservationLifecyclePort::default());
    let service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"))
        .with_dependency_environment_provider(std::sync::Arc::new(
            dependency_readiness_provider.clone(),
        ))
        .with_dependency_readiness_work_queue(dependency_readiness_work_queue)
        .with_runtime_dispatch_source_refresher(source_refresher)
        .with_runtime_dispatch_candidate_provider(Arc::new(
            SingleCanonicalRuntimeDispatchCandidateProvider,
        ))
        .with_runtime_host_batch_execution_port(runtime_host_batch_port.clone())
        .with_reservation_lifecycle_port(reservation_lifecycle_port.clone());
    let runtime = Arc::new(WorkflowSessionExecutionRuntime::new(
        service,
        Arc::clone(&host),
    ));
    let service = runtime.service();
    let workflow_id = "wf-runtime-host-shutdown-abort";
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

    let first_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first session");
    let second_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create second session");
    let run_request = |session_id: String, prompt: &str| WorkflowExecutionSessionRunRequest {
        session_id,
        workflow_semantic_version: workflow_semantic_version.to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!(prompt),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "infer".to_string(),
            port_id: "image".to_string(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    let first_run_request = run_request(first_created.session_id, "paint a red cube");
    let second_run_request = WorkflowExecutionSessionRunRequest {
        session_id: second_created.session_id,
        workflow_semantic_version: workflow_semantic_version.to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("paint a blue cube"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "infer".to_string(),
            port_id: "image".to_string(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    let runtime_request_started = runtime_host_batch_port.request_started.notified();
    let first_run_runtime = Arc::clone(&runtime);
    let first_run_handle = tokio::spawn(async move {
        first_run_runtime
            .run_workflow_execution_session(first_run_request)
            .await
    });
    let second_run_runtime = Arc::clone(&runtime);
    let second_run_handle = tokio::spawn(async move {
        second_run_runtime
            .run_workflow_execution_session(second_run_request)
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(1), runtime_request_started)
        .await
        .expect("runtime batch dispatch should start");

    runtime
        .shutdown_workflow_execution_runtime(
            std::time::Duration::from_millis(1),
            std::time::Duration::from_secs(1),
        )
        .await
        .expect("shutdown should cancel blocked runtime batch dispatch");
    let first_error = first_run_handle
        .await
        .expect("first run task should not panic")
        .expect_err("first cancelled runtime dispatch should cancel the workflow run");
    let second_error = second_run_handle
        .await
        .expect("second run task should not panic")
        .expect_err("second cancelled runtime dispatch should cancel the workflow run");

    for error in [&first_error, &second_error] {
        assert_eq!(error.code(), WorkflowErrorCode::Cancelled);
        assert!(
            error
                .message()
                .contains("runtime host reported cancellation"),
            "unexpected error: {error}"
        );
    }
    let cancellation_snapshot = runtime_host_batch_port
        .cancellation_snapshot()
        .expect("runtime host should retain cancellation handle");
    assert_eq!(
        cancellation_snapshot.state,
        pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationState::ShutdownRequested
    );
    let lifecycle_events = reservation_lifecycle_port.events();
    assert_eq!(
        lifecycle_events
            .iter()
            .map(|event| &event.outcome)
            .collect::<Vec<_>>(),
        vec![
            &ReservationLifecycleOutcome::DispatchStarted,
            &ReservationLifecycleOutcome::DispatchStarted,
            &ReservationLifecycleOutcome::WorkflowCancelled,
            &ReservationLifecycleOutcome::WorkflowCancelled,
        ]
    );
}

#[tokio::test]
async fn workflow_execution_session_fails_closed_when_reservation_lifecycle_port_is_missing() {
    let host = Arc::new(RuntimeInferenceSessionHost::new());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = std::sync::Arc::new(DependencyReadinessWorkQueue::new());
    let runtime_host_batch_port = Arc::new(CompletingRuntimeHostBatchPort::default());
    let runtime = WorkflowSessionExecutionRuntime::new(
        WorkflowService::with_ephemeral_attribution_store()
            .expect("service")
            .with_dependency_environment_provider(std::sync::Arc::new(
                dependency_readiness_provider.clone(),
            ))
            .with_dependency_readiness_work_queue(dependency_readiness_work_queue)
            .with_runtime_dispatch_candidate_provider(Arc::new(
                SingleCanonicalRuntimeDispatchCandidateProvider,
            ))
            .with_runtime_host_batch_execution_port(runtime_host_batch_port.clone()),
        Arc::clone(&host),
    );
    let service = runtime.service();
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

    let first_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first session");
    let second_created = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create second session");
    let run_request = |session_id: String, prompt: &str| WorkflowExecutionSessionRunRequest {
        session_id,
        workflow_semantic_version: workflow_semantic_version.to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!(prompt),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "infer".to_string(),
            port_id: "image".to_string(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    let first_run = runtime
        .run_workflow_execution_session(run_request(first_created.session_id, "paint a red cube"));
    let second_run = runtime.run_workflow_execution_session(run_request(
        second_created.session_id,
        "paint a blue cube",
    ));
    let (first_error, second_error) = tokio::join!(first_run, second_run);
    let first_error =
        first_error.expect_err("first missing lifecycle port run should fail before dispatch");
    let second_error =
        second_error.expect_err("second missing lifecycle port run should fail before dispatch");

    for error in [&first_error, &second_error] {
        assert_eq!(error.code(), WorkflowErrorCode::InternalError);
        assert!(
            error
                .message()
                .contains("reservation lifecycle port is not configured"),
            "unexpected error: {error}"
        );
    }
    assert!(runtime_host_batch_port.requests().is_empty());
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

#[derive(Clone)]
struct SlowWorkflowIoHost {
    inner: Arc<MockWorkflowHost>,
    workflow_io_delay: std::time::Duration,
}

impl SlowWorkflowIoHost {
    fn new(workflow_io_delay: std::time::Duration) -> Self {
        Self {
            inner: Arc::new(MockWorkflowHost::new(8, 1024)),
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
    let host = SlowWorkflowIoHost::new(Duration::from_secs(30));
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

    let workflow_run_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(workflow_run_id) = {
                let store = service.session_store_guard().expect("session store");
                store.active_workflow_run_ids().into_iter().next()
            } {
                break workflow_run_id;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("scheduler run should become active before workflow I/O completes");

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

    run.abort();
    let _ = run.await;
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
    let session_id = created.session_id.clone();
    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
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
    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest { session_id })
        .await
        .expect("session status after finalized run");
    assert_eq!(status.session.run_count, 1);
    assert_eq!(status.session.queued_runs, 0);

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
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("first run"),
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
        .expect("first run");

    let second = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("second run"),
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
    let workflow_id = "wf-snapshot";
    let workflow_semantic_version = "1.2.3";
    let graph = mock_workflow_graph();
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

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: workflow_semantic_version.to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
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
    assert!(started_event.event_seq > queue_event.event_seq);
    assert!(started_event
        .payload_json
        .contains("\"scheduler_decision_reason\":"));

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
    assert!(terminal_event.event_seq > started_event.event_seq);
    assert!(terminal_event
        .payload_json
        .contains("\"status\":\"completed\""));
    assert!(terminal_event.payload_json.contains("\"duration_ms\":"));

    let io_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::IoArtifactObserved
        })
        .collect::<Vec<_>>();
    assert_eq!(io_events.len(), 3);
    assert!(io_events[0].event_seq > terminal_event.event_seq);
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"workflow_input\"")));
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"workflow_output\"")));
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"node_output\"")));
    assert!(io_events.iter().any(|event| {
        event
            .payload_json
            .contains("\"artifact_role\":\"workflow_input\"")
            && event.node_type.as_deref() == Some("text-input")
    }));
    assert!(io_events.iter().any(|event| {
        event
            .payload_json
            .contains("\"artifact_role\":\"workflow_output\"")
            && event.node_type.as_deref() == Some("text-output")
    }));
    assert!(io_events.iter().any(|event| {
        event
            .payload_json
            .contains("\"artifact_role\":\"node_output\"")
            && event.node_type.as_deref() == Some("text-output")
    }));
    assert!(io_events.iter().all(|event| event
        .payload_json
        .contains("\"retention_state\":\"metadata_only\"")));

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
async fn attributed_workflow_execution_session_carries_client_bucket_into_run_events() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2)
        .with_attribution_store(SqliteAttributionStore::open_in_memory().expect("store"))
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));
    let workflow_id = "wf-attributed";
    let workflow_semantic_version = "1.2.3";
    let graph = mock_workflow_graph();
    let version = service
        .resolve_workflow_graph_version(workflow_id, workflow_semantic_version, &graph)
        .expect("resolve workflow version");
    service
        .store_workflow_executable_validation_snapshot(runtime_executable_validation_snapshot(
            &version, &graph,
        ))
        .expect("store executable validation snapshot");
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
                workflow_id: workflow_id.to_string(),
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
                workflow_semantic_version: workflow_semantic_version.to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
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
    let run_diagnostic_events = diagnostic_events
        .iter()
        .filter(|event| {
            event
                .workflow_run_id
                .as_ref()
                .is_some_and(|workflow_run_id| workflow_run_id.as_str() == response.workflow_run_id)
        })
        .collect::<Vec<_>>();
    assert!(
        !run_diagnostic_events.is_empty(),
        "expected run-scoped diagnostic events"
    );
    let unattributed_run_event_kinds = run_diagnostic_events
        .iter()
        .filter(|event| event.client_id.as_ref() != Some(&registered.client.client_id))
        .map(|event| format!("{:?}", event.event_kind))
        .collect::<Vec<_>>();
    assert!(
        unattributed_run_event_kinds.is_empty(),
        "unattributed run event kinds: {unattributed_run_event_kinds:?}"
    );
    assert!(run_diagnostic_events
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
async fn one_shot_non_runtime_session_run_does_not_load_session_runtime() {
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
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("one-shot runtime"),
                }],
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
        Vec::<WorkflowExecutionSessionRetentionHint>::new()
    );
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

struct PumasMaterializationSessionHost {
    inner: MockWorkflowHost,
}

impl PumasMaterializationSessionHost {
    fn new() -> Self {
        Self {
            inner: MockWorkflowHost::new(8, 1024),
        }
    }
}

#[async_trait::async_trait]
impl WorkflowHost for PumasMaterializationSessionHost {
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
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("pumas-materialization-session-graph".to_string())
    }

    async fn workflow_graph(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        Ok(pumas_materialization_session_graph())
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
            inputs: Vec::new(),
            outputs: Vec::new(),
        })
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        self.inner.runtime_capabilities().await
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        unreachable!("pumas materialization fail-closed test must not execute workflow runs")
    }
}

#[derive(Default)]
struct RecordingReservationLifecyclePort {
    events: Mutex<Vec<ReservationLifecycleEvent>>,
    reservation_cleanup_lifecycle: Mutex<Option<WorkflowSchedulerLifecycleComponentRegistryHandle>>,
}

impl RecordingReservationLifecyclePort {
    fn events(&self) -> Vec<ReservationLifecycleEvent> {
        self.events
            .lock()
            .expect("reservation lifecycle event lock")
            .clone()
    }

    fn observe_reservation_cleanup_lifecycle(
        &self,
        scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    ) {
        *self
            .reservation_cleanup_lifecycle
            .lock()
            .expect("reservation cleanup lifecycle lock") = Some(scheduler_lifecycle);
    }
}

#[async_trait::async_trait]
impl ReservationLifecyclePort for RecordingReservationLifecyclePort {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.assert_reservation_cleanup_lifecycle(&event);
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

impl RecordingReservationLifecyclePort {
    fn assert_reservation_cleanup_lifecycle(&self, event: &ReservationLifecycleEvent) {
        let Some(scheduler_lifecycle) = self
            .reservation_cleanup_lifecycle
            .lock()
            .expect("reservation cleanup lifecycle lock")
            .clone()
        else {
            return;
        };
        let expected_state = match event.outcome {
            ReservationLifecycleOutcome::RuntimeHostCompleted
            | ReservationLifecycleOutcome::RuntimeHostFailed
            | ReservationLifecycleOutcome::RuntimeHostDispatchRejected
            | ReservationLifecycleOutcome::WorkflowCancelled => {
                WorkflowSchedulerLifecycleComponentState::Running
            }
            _ => WorkflowSchedulerLifecycleComponentState::NotStarted,
        };
        assert_eq!(
            scheduler_lifecycle
                .component(WorkflowSchedulerLifecycleComponentKind::ReservationCleanup)
                .expect("reservation cleanup lifecycle component")
                .state,
            expected_state
        );
    }
}

#[derive(Default)]
struct RecordingRuntimeDispatchSourceRefresher {
    model_refs: Mutex<Vec<String>>,
}

impl RecordingRuntimeDispatchSourceRefresher {
    fn model_refs(&self) -> Vec<String> {
        self.model_refs
            .lock()
            .expect("runtime dispatch source refresh lock")
            .clone()
    }
}

#[async_trait::async_trait]
impl WorkflowRuntimeDispatchSourceRefresher for RecordingRuntimeDispatchSourceRefresher {
    async fn refresh_runtime_dispatch_sources(
        &self,
        _task: &WorkflowSchedulerTask,
        _ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<(), WorkflowRuntimeDispatchSourceRefreshError> {
        self.model_refs
            .lock()
            .expect("runtime dispatch source refresh lock")
            .push(
                readiness_proof
                    .preflight_result
                    .identity_key
                    .model_ref
                    .model_id
                    .clone(),
            );
        Ok(())
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
        _cancellation: RuntimeHostExecutionCancellationHandle,
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

#[derive(Default)]
struct CompletingRuntimeHostBatchPort {
    requests: Mutex<Vec<RuntimeHostBatchExecutionRequest>>,
}

impl CompletingRuntimeHostBatchPort {
    fn requests(&self) -> Vec<RuntimeHostBatchExecutionRequest> {
        self.requests
            .lock()
            .expect("runtime host batch request lock")
            .clone()
    }
}

#[async_trait::async_trait]
impl RuntimeHostBatchExecutionPort for CompletingRuntimeHostBatchPort {
    async fn execute_runtime_host_batch_request(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        _cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        self.requests
            .lock()
            .expect("runtime host batch request lock")
            .push(request.clone());
        Ok(RuntimeHostBatchExecutionResponse {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            batch_execution_request_id: request.batch_execution_request_id,
            state: RuntimeHostBatchExecutionState::Completed,
            members: request
                .members
                .into_iter()
                .map(|member| RuntimeHostBatchExecutionMemberResponse {
                    execution_request_id: member.execution_request_id,
                    assignment_id: member.assignment_id,
                    workflow_id: member.handoff.workflow_id,
                    workflow_run_id: member.handoff.workflow_run_id,
                    node_id: member.handoff.node_id,
                    task_id: member.handoff.task_id,
                    state: RuntimeHostBatchExecutionMemberState::Completed,
                    retry_disposition: RuntimeHostBatchMemberRetryDisposition::NotRetryable,
                    reservation_disposition: RuntimeHostBatchMemberReservationDisposition::Released,
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
                .collect(),
            diagnostics: Vec::new(),
        })
    }
}

#[derive(Default)]
struct FailingRuntimeHostBatchPort {
    requests: Mutex<Vec<RuntimeHostBatchExecutionRequest>>,
}

impl FailingRuntimeHostBatchPort {
    fn requests(&self) -> Vec<RuntimeHostBatchExecutionRequest> {
        self.requests
            .lock()
            .expect("runtime host batch request lock")
            .clone()
    }
}

#[async_trait::async_trait]
impl RuntimeHostBatchExecutionPort for FailingRuntimeHostBatchPort {
    async fn execute_runtime_host_batch_request(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        _cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        self.requests
            .lock()
            .expect("runtime host batch request lock")
            .push(request.clone());
        Ok(RuntimeHostBatchExecutionResponse {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            batch_execution_request_id: request.batch_execution_request_id,
            state: RuntimeHostBatchExecutionState::PartiallyCompleted,
            members: request
                .members
                .into_iter()
                .map(|member| RuntimeHostBatchExecutionMemberResponse {
                    execution_request_id: member.execution_request_id,
                    assignment_id: member.assignment_id,
                    workflow_id: member.handoff.workflow_id,
                    workflow_run_id: member.handoff.workflow_run_id,
                    node_id: member.handoff.node_id,
                    task_id: member.handoff.task_id,
                    state: RuntimeHostBatchExecutionMemberState::Failed,
                    retry_disposition: RuntimeHostBatchMemberRetryDisposition::NotRetryable,
                    reservation_disposition: RuntimeHostBatchMemberReservationDisposition::Released,
                    outputs: Vec::new(),
                    diagnostics: vec![RuntimeHostExecutionDiagnostic {
                        severity: RuntimeHostExecutionDiagnosticSeverity::Error,
                        code: RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                        message: "runtime host failed image batch execution".to_string(),
                        hint: Some("test.runtime_host_batch_failed".to_string()),
                    }],
                    terminal_metadata: None,
                })
                .collect(),
            diagnostics: Vec::new(),
        })
    }
}

#[derive(Default)]
struct RejectingRuntimeHostBatchPort {
    requests: Mutex<Vec<RuntimeHostBatchExecutionRequest>>,
}

impl RejectingRuntimeHostBatchPort {
    fn requests(&self) -> Vec<RuntimeHostBatchExecutionRequest> {
        self.requests
            .lock()
            .expect("runtime host batch request lock")
            .clone()
    }
}

#[async_trait::async_trait]
impl RuntimeHostBatchExecutionPort for RejectingRuntimeHostBatchPort {
    async fn execute_runtime_host_batch_request(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        _cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        self.requests
            .lock()
            .expect("runtime host batch request lock")
            .push(request);
        Err(RuntimeHostExecutionPortError::ExecutionFailed {
            message: "runtime host rejected image batch dispatch".to_string(),
        })
    }
}

#[derive(Default)]
struct BlockingRuntimeHostBatchPort {
    request_started: tokio::sync::Notify,
    cancellation: Mutex<Option<RuntimeHostExecutionCancellationHandle>>,
}

impl BlockingRuntimeHostBatchPort {
    fn cancellation_snapshot(&self) -> Option<RuntimeHostExecutionCancellationSnapshot> {
        self.cancellation
            .lock()
            .expect("runtime host cancellation lock")
            .as_ref()
            .map(RuntimeHostExecutionCancellationHandle::snapshot)
    }
}

#[async_trait::async_trait]
impl RuntimeHostBatchExecutionPort for BlockingRuntimeHostBatchPort {
    async fn execute_runtime_host_batch_request(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        *self
            .cancellation
            .lock()
            .expect("runtime host cancellation lock") = Some(cancellation.clone());
        self.request_started.notify_waiters();

        loop {
            if cancellation.snapshot().state
                == pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationState::ShutdownRequested
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }

        Ok(RuntimeHostBatchExecutionResponse {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            batch_execution_request_id: request.batch_execution_request_id,
            state: RuntimeHostBatchExecutionState::Cancelled,
            members: request
                .members
                .into_iter()
                .map(|member| RuntimeHostBatchExecutionMemberResponse {
                    execution_request_id: member.execution_request_id,
                    assignment_id: member.assignment_id,
                    workflow_id: member.handoff.workflow_id,
                    workflow_run_id: member.handoff.workflow_run_id,
                    node_id: member.handoff.node_id,
                    task_id: member.handoff.task_id,
                    state: RuntimeHostBatchExecutionMemberState::Cancelled,
                    retry_disposition: RuntimeHostBatchMemberRetryDisposition::NotRetryable,
                    reservation_disposition: RuntimeHostBatchMemberReservationDisposition::Released,
                    outputs: Vec::new(),
                    diagnostics: vec![RuntimeHostExecutionDiagnostic {
                        severity: RuntimeHostExecutionDiagnosticSeverity::Error,
                        code: RuntimeHostExecutionDiagnosticCode::ShutdownRequested,
                        message:
                            "runtime host reported cancellation after workflow-service shutdown"
                                .to_string(),
                        hint: Some("test.runtime_host_batch_cancelled".to_string()),
                    }],
                    terminal_metadata: None,
                })
                .collect(),
            diagnostics: vec![RuntimeHostExecutionDiagnostic {
                severity: RuntimeHostExecutionDiagnosticSeverity::Error,
                code: RuntimeHostExecutionDiagnosticCode::ShutdownRequested,
                message: "runtime host batch cancelled after workflow-service shutdown".to_string(),
                hint: Some("test.runtime_host_batch_cancelled".to_string()),
            }],
        })
    }
}

struct SingleCanonicalRuntimeDispatchCandidateProvider;

impl WorkflowRuntimeDispatchCandidateProvider for SingleCanonicalRuntimeDispatchCandidateProvider {
    fn runtime_dispatch_candidates(
        &self,
        task: &WorkflowSchedulerTask,
        _ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
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
        let environment_ref = readiness_proof
            .preflight_result
            .environment_ref
            .clone()
            .ok_or_else(|| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: format!(
                    "runtime scheduler task '{}' has no environment ref for test candidate",
                    task.task_id.as_str()
                ),
            })?;
        let reservation = SchedulerResourceReservation {
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
            device_id: selected_device_id.clone(),
            resource_kind: SchedulerResourceKind::DeviceVram,
            reserved_bytes: 1,
        };
        let fact = WorkflowRuntimeDispatchCandidateFact {
            candidate_id: SchedulerDispatchCandidateId::parse("candidate.runtime_session_test")
                .map_err(
                    |error| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                        message: error.to_string(),
                    },
                )?,
            selected_runtime_id,
            selected_runtime_variant_id: None,
            selected_backend_key: "test-runtime".to_string(),
            runtime_family: "test-runtime".to_string(),
            resolved_load_target: format!("test:{}", intent.model_ref.model_id),
            runtime_residency_key: format!("test-runtime:{}", intent.model_ref.model_id),
            loaded_runtime_memory_estimate_bytes: 1,
            runtime_load_state: WorkflowRuntimeDispatchLoadState::Loaded,
            runtime_instance_id: Some("runtime.session-test.001".to_string()),
            selected_device_ids: vec![selected_device_id],
            selected_model_ref: intent.model_ref.clone(),
            runtime_trait_settings: Vec::new(),
            environment_ref,
            reservations: vec![reservation],
            resource_fit_assessment: SchedulerResourceFitAssessment {
                workflow_run_id: intent.workflow_run_id.clone(),
                task_id: intent.task_id.clone(),
                state: SchedulerResourceFitState::Fits,
                diagnostics: Vec::new(),
            },
            batching_group_id: None,
        };
        let bundle = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(
            WorkflowRuntimeDispatchCandidateFactBundle {
                contract_version: WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
                facts: vec![fact],
                diagnostics: Vec::new(),
            },
        )
        .map_err(
            |error| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: error.to_string(),
            },
        )?;
        Ok(WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(bundle))
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
                    "runtime_source_context": {
                        "operation_type": "image-generation.txt2img",
                        "context_shape_key": "txt2img.1024x1024.steps30",
                        "cancellation_mode": "per-run-fanout"
                    },
                    "inference_interface_snapshot": runtime_inference_interface_snapshot_json(),
                    "pumas_model_ref": {
                        "model_id": "image/example/tiny-diffusion",
                        "revision": "main",
                        "selected_artifact_id": "diffusers-bundle"
                    }
                }),
            },
        ],
        edges: vec![crate::graph::GraphEdge {
            id: "prompt-to-infer".to_string(),
            source: "prompt".to_string(),
            source_handle: "text".to_string(),
            target: "infer".to_string(),
            target_handle: "prompt".to_string(),
        }],
        derived_graph: None,
    }
}

fn pumas_materialization_session_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![GraphNode {
            id: "model".to_string(),
            node_type: "puma-lib".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            data: serde_json::json!({}),
        }],
        edges: Vec::new(),
        derived_graph: None,
    }
}

fn runtime_inference_interface_snapshot_json() -> serde_json::Value {
    serde_json::json!({
        "contract_version": INFERENCE_INTERFACE_CONTRACT_VERSION,
        "descriptor_fingerprint": "runtime_descriptor_fingerprint_1",
        "task_kind": "image_generation",
        "inputs": [
            {
                "port_id": "prompt",
                "label": "Prompt",
                "direction": "input",
                "requirement": "required",
                "value_type": {
                    "category": "scalar",
                    "kind": "string"
                },
                "availability": {
                    "status": "available"
                }
            }
        ],
        "outputs": [
            {
                "port_id": "image",
                "label": "Image",
                "direction": "output",
                "requirement": "required",
                "value_type": {
                    "category": "artifact",
                    "kind": "image"
                },
                "availability": {
                    "status": "available"
                }
            }
        ]
    })
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
            runtime_source_context: runtime_source_context(),
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
            estimate_hints: runtime_resource_estimate_hints(),
            dependency_requirements_id: dependency_proof.dependency_requirements_id,
            selected_binding_ids: dependency_proof.identity_key.selected_binding_ids,
            dependency_override_fingerprint: dependency_proof.dependency_override_fingerprint,
            blocking_diagnostics: Vec::new(),
        }],
    }
}

fn runtime_resource_estimate_hints() -> Vec<SchedulerEstimateHint> {
    vec![
        SchedulerEstimateHint {
            kind: SchedulerEstimateHintKind::PeakRamBytes,
            value: 2_147_483_648,
        },
        SchedulerEstimateHint {
            kind: SchedulerEstimateHintKind::PeakVramBytes,
            value: 4_294_967_296,
        },
    ]
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

fn runtime_source_context() -> crate::graph::WorkflowRuntimeSourceContext {
    crate::graph::WorkflowRuntimeSourceContext {
        operation_type: "image-generation.txt2img".to_string(),
        context_shape_key: "txt2img.1024x1024.steps30".to_string(),
        cancellation_mode: "per-run-fanout".to_string(),
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

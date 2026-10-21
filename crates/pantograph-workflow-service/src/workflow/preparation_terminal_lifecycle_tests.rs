// Controlled worker integration; no native model or M4 cohort is executed.
struct RefusedPreparationProvider;

impl WorkflowRuntimeDispatchCandidateProvider for RefusedPreparationProvider {
    fn runtime_dispatch_candidates(
        &self,
        _task: &WorkflowSchedulerTask,
        _record: &SchedulerTaskStateRecord,
        _proof: &DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>
    {
        Err(worker_candidate_provider_error(
            "controlled preparation pressure refusal",
        ))
    }
}

fn preparation_run_terminals(
    service: &WorkflowService,
    run: &str,
) -> Vec<pantograph_diagnostics_ledger::RunTerminalPayload> {
    use pantograph_diagnostics_ledger::{DiagnosticEventPayload, DiagnosticsLedgerRepository};
    service
        .diagnostics_ledger_guard()
        .unwrap()
        .diagnostic_events_after(0, 500)
        .unwrap()
        .into_iter()
        .filter(|event| {
            event
                .workflow_run_id
                .as_ref()
                .is_some_and(|id| id.as_str() == run)
        })
        .filter_map(
            |event| match serde_json::from_str(&event.payload_json).unwrap() {
                DiagnosticEventPayload::RunTerminal(terminal) => Some(terminal),
                _ => None,
            },
        )
        .collect()
}

#[tokio::test]
async fn preparation_refusal_finishes_run_once_and_releases_session_without_native_work() {
    use pantograph_diagnostics_ledger::{
        DiagnosticsLedgerRepository, RunTerminalStatus, SqliteDiagnosticsLedger,
    };
    let port = Arc::new(RecordingWorkerBatchExecutionPort::default());
    let lifecycle = Arc::new(RecordingReservationLifecyclePort::default());
    let service = Arc::new(
        WorkflowService::new()
            .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().unwrap())
            .with_runtime_dispatch_candidate_provider(Arc::new(RefusedPreparationProvider))
            .with_runtime_host_batch_execution_port(port.clone())
            .with_reservation_lifecycle_port(lifecycle.clone()),
    );
    let run = "run.preparation.refused";
    let session = prepare_ready_runtime_branch_run(&service, run);
    let event = enqueue_ready_runtime_branch_event(&service, &session, run);
    let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
    let command = runtime_branch_command_for_session_run(&session, run);
    let (responder, receiver) =
        WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
    let registration = registry.register_workflow_run(&command, responder).unwrap();
    // Rejecting a second responder must not finalize the first owner's run.
    let (duplicate, duplicate_rx) =
        WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
    let rejected = registry
        .register_workflow_run(&command, duplicate)
        .unwrap_err();
    rejected
        .completion_responder
        .complete(rejected.outcome)
        .unwrap();
    assert!(matches!(
        duplicate_rx.await.unwrap(),
        WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(_)
    ));
    assert!(preparation_run_terminals(&service, run).is_empty());
    assert!(service
        .session_store_guard()
        .unwrap()
        .active_run_context(&session, run)
        .is_ok());
    assert_eq!(registry.active_responder_count(), 1);
    let environment =
        WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(service.clone(), test_host());
    drive_runtime_branch_continuations(
        &environment,
        &registry,
        vec![WorkflowRuntimeBranchContinuation {
            command,
            registration,
        }],
    )
    .await;
    let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(outcome) = receiver.await.unwrap()
    else {
        panic!("expected preparation refusal")
    };
    assert!(outcome
        .error_message
        .contains("controlled preparation pressure refusal"));
    assert!(port.requests().is_empty(), "no native host execution");
    assert!(
        lifecycle.events().is_empty(),
        "no reservation lifecycle side effect"
    );
    assert_eq!(
        runtime_branch_event_state(&service, &event),
        WorkflowRuntimeBranchTaskEventState::Failed
    );
    assert_eq!(registry.active_responder_count(), 0);
    let events = service
        .diagnostics_ledger_guard()
        .unwrap()
        .diagnostic_events_after(0, 500)
        .unwrap();
    assert!(
        !events
            .iter()
            .any(|event| event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerTaskAttemptLifecycleChanged),
        "no Start attempt"
    );
    let terminals = preparation_run_terminals(&service, run);
    assert_eq!(
        terminals.len(),
        1,
        "refusal must publish exactly one workflow terminal"
    );
    assert_eq!(terminals[0].status, RunTerminalStatus::Failed);
    assert!(terminals[0]
        .error
        .as_ref()
        .unwrap()
        .contains("controlled preparation pressure refusal"));
    {
        let store = service.session_store_guard().unwrap();
        assert!(store.active_run_context(&session, run).is_err());
        assert!(store.list_queue(&session).unwrap().is_empty());
        assert_eq!(store.session_summary(&session).unwrap().run_count, 1);
    }
    // A replayed command must not finish twice or create a replacement Start.
    let command = runtime_branch_command_for_session_run(&session, run);
    let (responder, replay) =
        WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
    let registration = registry.register_workflow_run(&command, responder).unwrap();
    drive_runtime_branch_continuations(
        &environment,
        &registry,
        vec![WorkflowRuntimeBranchContinuation {
            command,
            registration,
        }],
    )
    .await;
    assert!(matches!(
        replay.await.unwrap(),
        WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(_)
    ));
    assert_eq!(preparation_run_terminals(&service, run).len(), 1);
    assert!(service
        .scheduler_task_orchestrator
        .start_ready_runtime_task(
            &mut service.session_store_guard().unwrap(),
            &session,
            run,
            WORKER_BATCH_TASK_ID
        )
        .is_err());
    assert!(
        service
            .close_workflow_execution_session(
                &WorkerHost,
                super::super::WorkflowExecutionSessionCloseRequest {
                    session_id: session
                }
            )
            .await
            .unwrap()
            .ok
    );
}

fn preparation_service() -> Arc<WorkflowService> {
    Arc::new(WorkflowService::new().with_diagnostics_ledger(
        pantograph_diagnostics_ledger::SqliteDiagnosticsLedger::open_in_memory().unwrap(),
    ))
}

fn finalize_controlled_preparation(
    service: &WorkflowService,
    session: &str,
    run: &str,
    workflow: &str,
) -> Result<(), WorkflowServiceError> {
    super::super::workflow_run_finalization::finalize_unstarted_runtime_branch_failure(
        service,
        session,
        run,
        workflow,
        WorkflowServiceError::Internal("controlled refusal".into()),
    )
}

#[test]
fn preparation_finalization_protects_foreign_and_incoherent_active_runs() {
    let service = preparation_service();
    let run = "run.preparation.identity";
    let session = prepare_ready_runtime_branch_run(&service, run);
    assert!(finalize_controlled_preparation(
        &service,
        &session,
        "run.foreign",
        WORKER_BATCH_WORKFLOW_ID
    )
    .is_err());
    assert!(finalize_controlled_preparation(&service, &session, run, "workflow.foreign").is_err());
    service
        .session_store_guard()
        .unwrap()
        .set_active_run_scheduler_task_state(&session, run, ready_runtime_task_graph(run), vec![])
        .unwrap();
    assert!(
        finalize_controlled_preparation(&service, &session, run, WORKER_BATCH_WORKFLOW_ID).is_err()
    );
    assert!(preparation_run_terminals(&service, run).is_empty());
    let store = service.session_store_guard().unwrap();
    assert!(store.active_run_context(&session, run).is_ok());
    assert_eq!(store.session_summary(&session).unwrap().run_count, 0);
}

#[tokio::test]
async fn preparation_readiness_deferral_stays_active_then_typed_cancel_finishes_once() {
    let service = preparation_service();
    let run = "run.preparation.deferred";
    let session = prepare_ready_runtime_branch_run(&service, run);
    // Ready-only fixtures omit validation source; Waiting requires a real
    // canonical requirements proof so the observation reaches readiness deferral.
    use pantograph_dependency_planning::{
        produce_dependency_requirements_proof, DependencyNodeTypeId,
        DependencyPlanningCallerContext, SchedulerIntent, ValidatedDependencyPlanningRequest,
    };
    let mut intent = task_intent_for_run(run);
    intent.trait_settings.clear();
    let request = crate::inference_dependency_planning::inference_dependency_planning_request(
        intent.model_ref.clone(),
        intent.task_type.clone(),
        SchedulerIntent {
            requested_runtime_id: intent.constraints.requested_runtime_id.clone(),
            requested_device_id: intent.constraints.requested_device_id.clone(),
        },
        vec![],
        intent.dependency_override_patches.clone(),
        vec![],
        DependencyPlanningCallerContext {
            source_node_type: Some(DependencyNodeTypeId::parse("llm-inference").unwrap()),
            workflow_id: Some(WORKER_BATCH_WORKFLOW_ID.into()),
            node_id: Some(WORKER_BATCH_NODE_ID.into()),
            port_id: None,
            run_id: Some(run.into()),
        },
    )
    .unwrap();
    let proof = produce_dependency_requirements_proof(
        &ValidatedDependencyPlanningRequest::try_from(request).unwrap(),
        None,
    )
    .unwrap();
    let context = readiness_proof_for_run(run).execution_context;
    let mut graph = ready_runtime_task_graph(run);
    graph.tasks[0].schedulable_intent = Some(intent.clone());
    graph.tasks[0].schedulable_intent_template =
        Some(super::super::WorkflowSchedulerTaskIntentTemplate {
            task_type: intent.task_type.clone(),
            constraints: intent.constraints.clone(),
            trait_settings: vec![],
            dependency_override_patches: intent.dependency_override_patches.clone(),
            estimate_hints: intent.estimate_hints.clone(),
            dependency_readiness_source: super::super::WorkflowSchedulerDependencyReadinessSource {
                graph_revision: context.graph_revision,
                validation_session_id: context.validation_session_id,
                validation_snapshot_id: context.validation_snapshot_id,
                descriptor_fingerprint: context.descriptor_fingerprint,
                dependency_requirements_id: proof.dependency_requirements_id,
                selected_binding_ids: proof.identity_key.selected_binding_ids,
                dependency_override_fingerprint: proof.dependency_override_fingerprint,
            },
        });
    let mut record = ready_runtime_task_record(run);
    record.state = SchedulerTaskState::WaitingDependencyReadiness {
        execution_intent: SchedulerTaskExecutionIntent::runtime(intent),
    };
    service
        .session_store_guard()
        .unwrap()
        .set_active_run_scheduler_task_state(&session, run, graph, vec![record])
        .unwrap();
    let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
    let command = runtime_branch_command_for_session_run(&session, run);
    let (responder, receiver) =
        WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
    let registration = registry.register_workflow_run(&command, responder).unwrap();
    let environment =
        WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(service.clone(), test_host());
    drive_runtime_branch_continuations(
        &environment,
        &registry,
        vec![WorkflowRuntimeBranchContinuation {
            command: command.clone(),
            registration,
        }],
    )
    .await;
    let deferred = receiver.await.unwrap();
    assert!(
        matches!(
            &deferred,
            WorkflowTaskExecutionWorkerOutcome::RuntimeBranchDeferred(_)
        ),
        "{deferred:?}"
    );
    assert!(service
        .session_store_guard()
        .unwrap()
        .active_run_context(&session, run)
        .is_ok());
    assert!(preparation_run_terminals(&service, run).is_empty());
    assert!(!service
        .workflow_execution_session_runtime_dependency_readiness_resume_candidates()
        .unwrap()
        .is_empty());
    let (responder, receiver) =
        WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
    let registration = registry.register_workflow_run(&command, responder).unwrap();
    let mut outcome = WorkflowTaskExecutionWorkerOutcome::runtime_branch_cancelled(
        &command,
        "controlled early cancellation",
        vec![],
    );
    settle_runtime_branch_terminal_response(
        &service,
        &registry,
        &registration,
        &command,
        &mut outcome,
    );
    registry.complete(registration, outcome).unwrap();
    assert!(matches!(
        receiver.await.unwrap(),
        WorkflowTaskExecutionWorkerOutcome::RuntimeBranchCancelled(_)
    ));
    let terminals = preparation_run_terminals(&service, run);
    assert_eq!(terminals.len(), 1);
    assert_eq!(
        terminals[0].status,
        pantograph_diagnostics_ledger::RunTerminalStatus::Cancelled
    );
    assert!(terminals[0]
        .error
        .as_ref()
        .unwrap()
        .contains("controlled early cancellation"));
    assert!(
        service
            .close_workflow_execution_session(
                &WorkerHost,
                super::super::WorkflowExecutionSessionCloseRequest {
                    session_id: session
                }
            )
            .await
            .unwrap()
            .ok
    );
}

#[tokio::test]
async fn preparation_finalization_retains_started_and_unacknowledged_assignment_custody() {
    let lifecycle = Arc::new(RecordingReservationLifecyclePort::default());
    let service = Arc::new(
        WorkflowService::new()
            .with_diagnostics_ledger(
                pantograph_diagnostics_ledger::SqliteDiagnosticsLedger::open_in_memory().unwrap(),
            )
            .with_runtime_dispatch_candidate_provider(Arc::new(
                SingleCanonicalRuntimeDispatchCandidateProvider,
            ))
            .with_reservation_lifecycle_port(lifecycle.clone()),
    );
    let run = "run.preparation.cleanup";
    let session = prepare_ready_runtime_branch_run(&service, run);
    let event_id = enqueue_ready_runtime_branch_event(&service, &session, run);
    let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
    let command = runtime_branch_command_for_session_run(&session, run);
    let (responder, mut receiver) =
        WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
    let registration = registry.register_workflow_run(&command, responder).unwrap();
    let (claimed, proof) = claim_runtime_branch_task_event_for_worker(
        &service,
        &command,
        WORKER_BATCH_TASK_ID,
        unix_timestamp_ms(),
    )
    .unwrap()
    .unwrap();
    let owned = WorkflowRuntimeBranchOwnedEventClaim {
        event_id: event_id.clone(),
        claim: claimed.claim.clone(),
        proof,
    };
    registry.record_claim_identity(&registration, &owned);
    let dispatching = mark_claimed_runtime_branch_task_event_dispatching(
        &service,
        &event_id,
        &claimed.claim,
        unix_timestamp_ms(),
        Some(&owned.proof),
    )
    .unwrap();
    let started = WorkflowPreDispatchPreparationBoundary::new(&service)
        .start_runtime_branch_dispatch_attempt(
            &session,
            run,
            WORKER_BATCH_TASK_ID,
            &[],
            pantograph_diagnostics_ledger::SchedulerTaskAttemptLifecycleTransition::Started,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(
        finalize_controlled_preparation(&service, &session, run, WORKER_BATCH_WORKFLOW_ID).is_err()
    );
    let evidenced = record_runtime_branch_selected_candidate_fact(
        &service,
        &dispatching.event_id,
        &claimed.claim,
        started.selected_candidate_fact.clone(),
        Some(&owned.proof),
    )
    .unwrap();
    let assignment = create_runtime_branch_dispatch_assignment(
        &service,
        &evidenced,
        &claimed.claim,
        &started,
        unix_timestamp_ms(),
    )
    .unwrap();
    let registration = registry
        .attach_runtime_dispatch_assignment(&registration, &assignment.assignment_id, Some(owned))
        .unwrap();
    let mutation = service
        .scheduler_task_orchestrator
        .cancel_started_runtime_task_terminal_mutation(
            &mut service.session_store_guard().unwrap(),
            &session,
            run,
            &started.started_runtime_task,
            "controlled stopped task, cleanup not acknowledged",
        )
        .unwrap();
    assert!(mutation.reservation_release_intent.is_some());
    assert!(service
        .session_store_guard()
        .unwrap()
        .active_run_scheduler_task_attempt_read_facts(&session, run)
        .unwrap()
        .is_empty());
    assert!(
        lifecycle.events().is_empty(),
        "release intent is not a native cleanup ACK"
    );
    let mut outcome = WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
        &command,
        "controlled cleanup failure",
        vec![],
    );
    settle_runtime_branch_terminal_response(
        &service,
        &registry,
        &registration,
        &command,
        &mut outcome,
    );
    assert!(service
        .session_store_guard()
        .unwrap()
        .active_run_context(&session, run)
        .is_ok());
    assert!(preparation_run_terminals(&service, run).is_empty());
    assert_eq!(registry.active_responder_count(), 1);
    assert!(matches!(
        receiver.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));
    assert!(matches!(
        service
            .close_workflow_execution_session(
                &WorkerHost,
                super::super::WorkflowExecutionSessionCloseRequest {
                    session_id: session
                }
            )
            .await,
        Err(WorkflowServiceError::SchedulerBusy { .. })
    ));
}

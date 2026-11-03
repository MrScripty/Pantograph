use super::super::WorkflowReservationCleanupAcknowledgement;
use super::*;
use crate::workflow::{
    WorkflowCompletionSuccessorSnapshot, WorkflowSchedulerDependencyReadinessSource,
};
use pantograph_runtime_host_contracts::ValidatedReservationLifecycleApplication;
use pantograph_scheduler::{SchedulerReservationLeaseId, SchedulerTaskExecutionIntent};

struct Pair {
    store: WorkflowExecutionSessionStore,
    orchestrator: WorkflowSchedulerTaskOrchestrator,
    session: String,
    run: String,
    first: WorkflowSchedulerTask,
    ready: SchedulerTaskStateRecord,
    proof: DependencyReadinessProofEnvelope,
    snapshot: WorkflowCompletionSuccessorSnapshot,
}
fn pair() -> Pair {
    let fixture = dispatch_selection_request_fixture();
    let mut proof = fixture.readiness_proof;
    proof.execution_context.dependency_override_fingerprint =
        Some(DependencyOverrideFingerprint::parse("override.fixture").unwrap());
    let mut intent = fixture.task_intent;
    intent.constraints = SchedulerRuntimeDeviceConstraints::default();
    let ctx = &proof.execution_context;
    let source = WorkflowSchedulerDependencyReadinessSource {
        graph_revision: ctx.graph_revision.clone(),
        validation_session_id: ctx.validation_session_id.clone(),
        validation_snapshot_id: ctx.validation_snapshot_id.clone(),
        descriptor_fingerprint: ctx.descriptor_fingerprint.clone(),
        dependency_requirements_id: ctx.dependency_requirements_id.clone(),
        selected_binding_ids: ctx.selected_binding_ids.clone(),
        dependency_override_fingerprint: ctx.dependency_override_fingerprint.clone().unwrap(),
    };
    let template = WorkflowSchedulerTaskIntentTemplate {
        task_type: intent.task_type.clone(),
        constraints: intent.constraints.clone(),
        trait_settings: vec![],
        dependency_override_patches: vec![],
        estimate_hints: vec![],
        dependency_readiness_source: source,
    };
    let mut first = task_from_intent(intent.clone());
    first.schedulable_intent_template = Some(template.clone());
    intent.task_id = SchedulerTaskId::parse("task.successor").unwrap();
    intent.node_id = SchedulerNodeId::parse("node.successor").unwrap();
    let mut next = task_from_intent(intent);
    next.schedulable_intent_template = Some(template);
    next.dependency_task_ids = vec![first.task_id.clone()];
    next.input_bindings = vec![WorkflowSchedulerTaskInputBinding {
        source_node_id: first.node_id.clone(),
        source_task_id: first.task_id.clone(),
        source_port_id: "text".into(),
        target_port_id: "prompt".into(),
    }];
    let graph = task_graph(vec![first.clone(), next]);
    let run = graph.workflow_run_id.to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session = begin_active_run_for_task_graph(&mut store, &graph);
    let orchestrator = orchestrator_without_runtime_host_response()
        .with_reservation_lifecycle_port(Arc::new(AcceptingReservationLifecyclePort));
    orchestrator
        .initialize_active_run_task_state(&mut store, &session, &run, graph)
        .unwrap();
    orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session,
            &run,
            first.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
            Some(proof.clone()),
        )
        .unwrap();
    let (_, records) = store
        .active_run_scheduler_task_state(&session, &run)
        .unwrap()
        .unwrap();
    let ready = records
        .into_iter()
        .find(|r| r.task_id == first.task_id)
        .unwrap();
    let snapshot = store
        .completion_successor_snapshot(&session, &run, &first, &ready, &proof)
        .expect("supported owned advisory pair");
    Pair {
        store,
        orchestrator,
        session,
        run,
        first,
        ready,
        proof,
        snapshot,
    }
}
fn start(p: &mut Pair, bind: bool) -> super::super::StartedRuntimeTaskExecution {
    let started = p
        .orchestrator
        .start_ready_runtime_task(&mut p.store, &p.session, &p.run, p.first.task_id.as_str())
        .unwrap();
    p.store
        .install_completion_cleanup_gate(&p.session, &p.run, &p.snapshot, started.attempt_id());
    if bind {
        p.store
            .bind_active_run_scheduler_task_reservation(
                &p.session,
                &p.run,
                &p.first.task_id,
                started.attempt_id(),
                SchedulerReservationLeaseId::parse("lease.first").unwrap(),
                None,
            )
            .unwrap();
    }
    started
}
fn acknowledgement(
    p: &Pair,
    lease: &str,
    state: ReservationLifecycleApplicationState,
    terminal: bool,
) -> WorkflowReservationCleanupAcknowledgement {
    WorkflowReservationCleanupAcknowledgement {
        application: ValidatedReservationLifecycleApplication::try_from(
            ReservationLifecycleApplication {
                contract_version: 1,
                lifecycle_event_id: "terminal.fixture".into(),
                reservation_lease_id: SchedulerReservationLeaseId::parse(lease).unwrap(),
                state,
                diagnostics: vec![],
            },
        )
        .unwrap(),
        workflow_run_id: p.run.clone(),
        task_id: p.first.task_id.to_string(),
        cleanup_release: terminal,
    }
}
#[test]
fn dependency_owned_snapshot_has_symbolic_inputs_and_refuses_changed_or_oversized_records_and_actual_intent(
) {
    let mut p = pair();
    assert!(p.snapshot.known_inputs.is_empty());
    assert_eq!(p.snapshot.predecessor_inputs.len(), 1);
    let mut changed = p.snapshot.clone();
    changed.predecessor_state_version += 1;
    assert!(p
        .store
        .validate_completion_successor_snapshot(
            &p.session, &p.run, &p.first, &p.ready, &p.proof, &changed
        )
        .is_err());
    let mut other_ready = p.ready.clone();
    let SchedulerTaskState::Ready { execution_intent } = &mut other_ready.state else {
        unreachable!()
    };
    let SchedulerTaskExecutionIntent::Runtime { task_intent } = execution_intent else {
        unreachable!()
    };
    task_intent
        .trait_settings
        .push(pantograph_scheduler::SchedulerTraitSetting {
            trait_id: "temperature".parse().unwrap(),
            value: pantograph_scheduler::SchedulerTraitValue::U64(2),
        });
    assert!(p
        .store
        .completion_successor_snapshot(&p.session, &p.run, &p.first, &other_ready, &p.proof)
        .is_none());
    let (graph, mut records) = p
        .store
        .active_run_scheduler_task_state(&p.session, &p.run)
        .unwrap()
        .unwrap();
    let next = records
        .iter_mut()
        .find(|r| r.task_id == p.snapshot.task.task_id)
        .unwrap();
    next.state = SchedulerTaskState::AwaitingInputs {
        diagnostics: vec![pantograph_scheduler::SchedulerTaskStateDiagnostic {
            severity: SchedulerTaskStateDiagnosticSeverity::Info,
            code: SchedulerTaskStateDiagnosticCode::AwaitingInputs,
            message: format!("x{}", " ".repeat(1024)),
            hint: None,
        }],
    };
    p.store
        .set_active_run_scheduler_task_state(&p.session, &p.run, graph, records)
        .unwrap();
    assert!(p
        .store
        .completion_successor_snapshot(&p.session, &p.run, &p.first, &p.ready, &p.proof)
        .is_none());
}
#[tokio::test]
async fn dependency_completed_state_cannot_advance_inputs_before_actual_terminal_cleanup_ack() {
    let mut p = pair();
    let started = start(&mut p, true);
    let mut result = runtime_task_result_fixture(&p.first);
    result.outputs = vec![WorkflowSchedulerTaskResultOutput {
        port_id: "text".into(),
        value: WorkflowSchedulerTaskResultValue::String("actual output".into()),
    }];
    let mutation = p
        .orchestrator
        .complete_started_runtime_task_terminal_mutation(
            &mut p.store,
            &p.session,
            &p.run,
            &started,
            result.clone(),
        )
        .unwrap();
    assert!(p
        .orchestrator
        .advance_awaiting_runtime_task_inputs(
            &mut p.store,
            &p.session,
            &p.run,
            p.snapshot.task.task_id.as_str()
        )
        .unwrap()
        .is_none());
    let ack = p
        .orchestrator
        .apply_runtime_task_result_reservation_lifecycle(&p.first, &mutation, &result)
        .await
        .unwrap()
        .unwrap();
    p.store.acknowledge_completion_cleanup(
        &p.session,
        &p.run,
        p.first.task_id.as_str(),
        started.attempt_id(),
        &ack,
    );
    assert!(!p.store.completion_cleanup_pending(
        &p.session,
        &p.run,
        p.snapshot.task.task_id.as_str()
    ));
    let advanced = p
        .orchestrator
        .advance_awaiting_runtime_task_inputs(
            &mut p.store,
            &p.session,
            &p.run,
            p.snapshot.task.task_id.as_str(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        advanced.state.kind(),
        SchedulerTaskStateKind::WaitingDependencyReadiness
    );
}
#[test]
fn dependency_gate_rejects_wrong_lease_attempt_task_nonterminal_or_already_applied_and_survives_state_replacement(
) {
    let mut p = pair();
    let started = start(&mut p, true);
    for (lease, state, terminal) in [
        (
            "lease.foreign",
            ReservationLifecycleApplicationState::Applied,
            true,
        ),
        (
            "lease.first",
            ReservationLifecycleApplicationState::Applied,
            false,
        ),
        (
            "lease.first",
            ReservationLifecycleApplicationState::AlreadyApplied,
            true,
        ),
    ] {
        let ack = acknowledgement(&p, lease, state, terminal);
        p.store.acknowledge_completion_cleanup(
            &p.session,
            &p.run,
            p.first.task_id.as_str(),
            started.attempt_id(),
            &ack,
        );
        assert!(p.store.completion_cleanup_pending(
            &p.session,
            &p.run,
            p.snapshot.task.task_id.as_str()
        ));
    }
    let ack = acknowledgement(
        &p,
        "lease.first",
        ReservationLifecycleApplicationState::Applied,
        true,
    );
    p.store.acknowledge_completion_cleanup(
        &p.session,
        &p.run,
        "another-task",
        started.attempt_id(),
        &ack,
    );
    p.store.acknowledge_completion_cleanup(
        &p.session,
        &p.run,
        p.first.task_id.as_str(),
        &super::super::super::WorkflowSchedulerTaskAttemptId::new(),
        &ack,
    );
    assert!(p.store.completion_cleanup_pending(
        &p.session,
        &p.run,
        p.snapshot.task.task_id.as_str()
    ));
    let (graph, records) = p
        .store
        .active_run_scheduler_task_state(&p.session, &p.run)
        .unwrap()
        .unwrap();
    p.store
        .set_active_run_scheduler_task_state(&p.session, &p.run, graph, records)
        .unwrap();
    assert!(p.store.completion_cleanup_pending(
        &p.session,
        &p.run,
        p.snapshot.task.task_id.as_str()
    ));
    // Even a forced Ready successor cannot bypass the authoritative start gate.
    let transition = p.ready_transition_for_successor();
    assert!(p
        .store
        .start_active_run_scheduler_task_attempt(
            &p.session,
            &p.run,
            super::super::super::WorkflowSchedulerTaskAttemptId::new(),
            transition
        )
        .unwrap_err()
        .message()
        .contains("acknowledged predecessor cleanup"));
    // New retry owner replaces attempt; old acknowledgement cannot clear it.
    let retry = super::super::super::WorkflowSchedulerTaskAttemptId::new();
    p.store
        .install_completion_cleanup_gate(&p.session, &p.run, &p.snapshot, &retry);
    p.store.acknowledge_completion_cleanup(
        &p.session,
        &p.run,
        p.first.task_id.as_str(),
        started.attempt_id(),
        &ack,
    );
    assert!(p.store.completion_cleanup_pending(
        &p.session,
        &p.run,
        p.snapshot.task.task_id.as_str()
    ));
}
impl Pair {
    fn ready_transition_for_successor(&self) -> pantograph_scheduler::SchedulerTaskStateTransition {
        pantograph_scheduler::SchedulerTaskStateTransition {
            contract_version: 1,
            transition_id: "successor.start".parse().unwrap(),
            workflow_id: self.snapshot.task.workflow_id.clone(),
            workflow_run_id: self.snapshot.task.workflow_run_id.clone(),
            node_id: self.snapshot.task.node_id.clone(),
            task_id: self.snapshot.task.task_id.clone(),
            expected_previous_state: Some(SchedulerTaskStateKind::Ready),
            next_state: SchedulerTaskState::Running {
                execution_intent: SchedulerTaskExecutionIntent::runtime(
                    self.snapshot.task.schedulable_intent.clone().unwrap(),
                ),
            },
        }
    }
}
#[tokio::test]
async fn dependency_no_release_intent_returns_no_ack_and_keeps_gate_closed() {
    let mut p = pair();
    let started = start(&mut p, false);
    let result = runtime_task_result_fixture(&p.first);
    let mutation = p
        .orchestrator
        .complete_started_runtime_task_terminal_mutation(
            &mut p.store,
            &p.session,
            &p.run,
            &started,
            result.clone(),
        )
        .unwrap();
    assert!(p
        .orchestrator
        .apply_runtime_task_result_reservation_lifecycle(&p.first, &mutation, &result)
        .await
        .unwrap()
        .is_none());
    assert!(p.store.completion_cleanup_pending(
        &p.session,
        &p.run,
        p.snapshot.task.task_id.as_str()
    ));
}

#[tokio::test]
async fn dependency_provider_mutation_of_successor_binding_rejects_start_and_rolls_back_prepared_custody(
) {
    use crate::workflow::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Debug)]
    struct Custody(Arc<AtomicUsize>);
    impl Drop for Custody {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    impl WorkflowRuntimeDispatchReservationCustody for Custody {
        fn transfer(self: Box<Self>) -> Result<(), WorkflowRuntimeDispatchCandidateProviderError> {
            panic!("changed pair must never transfer custody");
        }
    }
    struct Refresher;
    #[async_trait]
    impl WorkflowRuntimeDispatchSourceRefresher for Refresher {
        async fn refresh_runtime_dispatch_sources(
            &self,
            _: &WorkflowSchedulerTask,
            _: &SchedulerTaskStateRecord,
            _: &DependencyReadinessProofEnvelope,
        ) -> Result<(), WorkflowRuntimeDispatchSourceRefreshError> {
            Ok(())
        }
    }
    struct MutatingProvider {
        store: Arc<Mutex<WorkflowExecutionSessionStore>>,
        session: String,
        run: String,
        rollbacks: Arc<AtomicUsize>,
    }
    impl WorkflowRuntimeDispatchCandidateProvider for MutatingProvider {
        fn runtime_dispatch_candidates(
            &self,
            _: &WorkflowSchedulerTask,
            _: &SchedulerTaskStateRecord,
            _: &DependencyReadinessProofEnvelope,
        ) -> Result<
            WorkflowRuntimeDispatchCandidateSet,
            WorkflowRuntimeDispatchCandidateProviderError,
        > {
            unreachable!()
        }
        fn requires_dependency_lookahead(&self) -> bool {
            true
        }
        fn runtime_dispatch_candidates_with_successor(
            &self,
            _: &WorkflowSchedulerTask,
            _: &SchedulerTaskStateRecord,
            _: &DependencyReadinessProofEnvelope,
            _: Option<&[pantograph_runtime_host_contracts::RuntimeHostExecutionInput]>,
            next: Option<&WorkflowCompletionSuccessorSnapshot>,
        ) -> Result<
            WorkflowRuntimeDispatchCandidateSet,
            WorkflowRuntimeDispatchCandidateProviderError,
        > {
            let next = next.expect("real captured owned pair");
            let mut store = self.store.lock().unwrap();
            let (mut graph, records) = store
                .active_run_scheduler_task_state(&self.session, &self.run)
                .unwrap()
                .unwrap();
            graph
                .tasks
                .iter_mut()
                .find(|t| t.task_id == next.task.task_id)
                .unwrap()
                .input_bindings[0]
                .source_port_id = "replacement-output".into();
            store
                .set_active_run_scheduler_task_state(&self.session, &self.run, graph, records)
                .unwrap();
            let mut set = WorkflowRuntimeDispatchCandidateSet::default();
            set.candidates = dispatch_selection_request_fixture().candidates;
            Ok(set.with_reservation_custody(Box::new(Custody(self.rollbacks.clone()))))
        }
    }
    let mut p = pair();
    let store = Arc::new(Mutex::new(std::mem::replace(
        &mut p.store,
        WorkflowExecutionSessionStore::new(1, 1),
    )));
    let rollbacks = Arc::new(AtomicUsize::new(0));
    let provider = MutatingProvider {
        store: store.clone(),
        session: p.session.clone(),
        run: p.run.clone(),
        rollbacks: rollbacks.clone(),
    };
    let boundary =
        WorkflowRuntimeDispatchSelectionBoundary::new(&Refresher, &provider, &p.orchestrator);
    let prepared = boundary
        .prepare_ready_runtime_task_dispatch_with_successor(
            &p.first,
            &p.ready,
            p.proof.clone(),
            None,
            Some(&p.snapshot),
        )
        .await
        .unwrap();
    {
        let store = store.lock().unwrap();
        assert!(store
            .validate_ready_dispatch_snapshot(
                &p.session, &p.run, &p.first, &p.ready, &p.proof, None, false
            )
            .is_ok());
        assert!(store
            .validate_completion_successor_snapshot(
                &p.session,
                &p.run,
                &p.first,
                &p.ready,
                &p.proof,
                &p.snapshot
            )
            .is_err());
        assert!(store
            .active_run_scheduler_task_attempt_read_facts(&p.session, &p.run)
            .unwrap()
            .is_empty());
    }
    drop(prepared);
    assert_eq!(rollbacks.load(Ordering::SeqCst), 1);
}

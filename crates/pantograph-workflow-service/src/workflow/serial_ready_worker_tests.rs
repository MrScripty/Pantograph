mod serial_ready_tests {
    use super::*;
    use crate::workflow::*;
    use pantograph_runtime_host_contracts::{
        RuntimeHostExecutionDiagnostic, RuntimeHostExecutionDiagnosticCode,
        RuntimeHostExecutionDiagnosticSeverity, SerialRuntimeHostBatchExecutionPort,
        SerialRuntimeHostCpuOwnerEvidence,
    };
    use pantograph_scheduler::{
        SchedulerSerialBoundDispatch, SchedulerSerialDrainState, SchedulerSerialDrainedDispatch,
        SchedulerSerialOwnerSnapshot, SchedulerSerialRuntimeOwnerLease,
        SchedulerSourceInputTaskIntent, SchedulerSourceInputTaskKind, SchedulerTaskStateKind,
    };
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    struct CpuPort {
        owner: Arc<tokio::sync::Mutex<SchedulerSerialOwnerSnapshot>>,
        generation: Mutex<Arc<AtomicU64>>,
        requests: Mutex<Vec<RuntimeHostBatchExecutionRequest>>,
        cleanups: Arc<Mutex<Vec<ReservationLifecycleEvent>>>,
        verified_reuse: AtomicBool,
        evidence: AtomicBool,
        change_before_forward: AtomicBool,
        change_after_drain: AtomicBool,
        cleanup_started: tokio::sync::Notify,
        cleanup_release: tokio::sync::Notify,
        block_cleanup: AtomicBool,
        fail_cleanup: AtomicBool,
        fail_execution: AtomicBool,
        started: tokio::sync::Notify,
        release: tokio::sync::Notify,
        block: AtomicBool,
    }
    impl CpuPort {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                owner: Arc::new(tokio::sync::Mutex::new(SchedulerSerialOwnerSnapshot {
                    loaded_instance: [1; 16],
                    loaded_profile: [2; 32],
                    effective_settings: [3; 32],
                    generation: 2,
                    cpu_threads: 1,
                })),
                generation: Mutex::new(Arc::new(AtomicU64::new(2))),
                requests: Mutex::new(vec![]),
                cleanups: Arc::new(Mutex::new(vec![])),
                verified_reuse: AtomicBool::new(false),
                evidence: AtomicBool::new(true),
                change_before_forward: AtomicBool::new(false),
                change_after_drain: AtomicBool::new(false),
                cleanup_started: tokio::sync::Notify::new(),
                cleanup_release: tokio::sync::Notify::new(),
                block_cleanup: AtomicBool::new(false),
                fail_cleanup: AtomicBool::new(false),
                fail_execution: AtomicBool::new(false),
                started: tokio::sync::Notify::new(),
                release: tokio::sync::Notify::new(),
                block: AtomicBool::new(false),
            })
        }
        fn tasks(&self) -> Vec<String> {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .map(|r| r.members[0].handoff.task_id.to_string())
                .collect()
        }
    }
    struct HeldOwner(tokio::sync::OwnedMutexGuard<SchedulerSerialOwnerSnapshot>);
    impl SchedulerSerialRuntimeOwnerLease for HeldOwner {
        fn snapshot(&self) -> SchedulerSerialOwnerSnapshot {
            *self.0
        }
    }
    #[async_trait::async_trait]
    impl RuntimeHostBatchExecutionPort for CpuPort {
        async fn execute_runtime_host_batch_request(
            &self,
            _: RuntimeHostBatchExecutionRequest,
            _: RuntimeHostExecutionCancellationHandle,
        ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
            panic!("opt-in worker must call the constructor-owned serial capability")
        }
    }
    fn application(event: ReservationLifecycleEvent) -> ReservationLifecycleApplication {
        ReservationLifecycleApplication {
            contract_version: 1,
            lifecycle_event_id: event.lifecycle_event_id,
            reservation_lease_id: event.reservation_lease_id,
            state: ReservationLifecycleApplicationState::Applied,
            diagnostics: vec![],
        }
    }
    #[async_trait::async_trait]
    impl ReservationLifecyclePort for CpuPort {
        async fn apply_reservation_lifecycle(
            &self,
            event: ReservationLifecycleEvent,
        ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
            Ok(application(event))
        }
    }
    #[async_trait::async_trait]
    impl SerialRuntimeHostBatchExecutionPort for CpuPort {
        fn resident_serial_cpu_owner(
            &self,
            intent: &SchedulableTaskIntent,
        ) -> Option<SerialRuntimeHostCpuOwnerEvidence> {
            if !self.evidence.load(Ordering::Acquire)
                || intent.model_ref.model_id != "synthetic.cpu.model"
            {
                return None;
            }
            Some(SerialRuntimeHostCpuOwnerEvidence {
                snapshot: *self.owner.try_lock().ok()?,
                generation: self.generation.lock().unwrap().clone(),
            })
        }
        async fn execute_serial_singleton(
            &self,
            request: RuntimeHostBatchExecutionRequest,
            _: RuntimeHostExecutionCancellationHandle,
            bound: SchedulerSerialBoundDispatch,
        ) -> Result<
            (
                RuntimeHostBatchExecutionResponse,
                SchedulerSerialDrainedDispatch,
                Option<SerialRuntimeHostCpuOwnerEvidence>,
            ),
            RuntimeHostExecutionPortError,
        > {
            assert_eq!(request.members.len(), 1);
            let ids = bound.identity();
            let attempt = ids.attempt_id.to_owned();
            let candidate = ids.candidate_id.to_owned();
            let member = &request.members[0];
            let identity = pantograph_scheduler::SchedulerSerialAttemptIdentity {
                workflow_id: member.handoff.workflow_id.as_str(),
                workflow_run_id: member.handoff.workflow_run_id.as_str(),
                node_id: member.handoff.node_id.as_str(),
                task_id: member.handoff.task_id.as_str(),
                attempt_id: &attempt,
                candidate_id: &candidate,
                execution_request_id: &member.execution_request_id,
                reservation_lease_id: member
                    .handoff
                    .dispatch_decision
                    .as_ref()
                    .unwrap()
                    .reservation_lease_id
                    .as_str(),
            };
            let mut guard = self.owner.clone().lock_owned().await;
            if self.change_before_forward.swap(false, Ordering::AcqRel) {
                guard.generation += 2;
                guard.loaded_instance = [9; 16];
                self.generation
                    .lock()
                    .unwrap()
                    .store(guard.generation, Ordering::Release);
            }
            let held = HeldOwner(guard);
            let ranked = bound.expected_owner().is_some();
            let (executing, unranked) = if ranked {
                (
                    Some(bound.acquire_owner(&held).map_err(|e| {
                        RuntimeHostExecutionPortError::ExecutionFailed {
                            message: format!("owner refused: {e:?}"),
                        }
                    })?),
                    None,
                )
            } else {
                (None, Some(bound))
            };
            self.requests.lock().unwrap().push(request.clone());
            self.started.notify_one();
            if self.block.load(Ordering::Acquire) {
                self.release.notified().await;
            }
            if matches!(member.materialized_inputs.first().map(|i| &i.value),Some(pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue::String(s)) if s == "slow")
            {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            let failed = self.fail_execution.swap(false, Ordering::AcqRel);
            let drain_state = if failed {
                SchedulerSerialDrainState::Failed
            } else {
                SchedulerSerialDrainState::Completed
            };
            let drained = match executing {
                Some(executing) => executing
                    .record_drained_response(identity, drain_state)
                    .unwrap(),
                None => unranked
                    .unwrap()
                    .record_unranked_drained_response(identity, drain_state)
                    .unwrap(),
            };
            let evidence =
                self.evidence
                    .load(Ordering::Acquire)
                    .then(|| SerialRuntimeHostCpuOwnerEvidence {
                        snapshot: held.snapshot(),
                        generation: self.generation.lock().unwrap().clone(),
                    });
            drop(held);
            if self.change_after_drain.swap(false, Ordering::AcqRel) {
                let mut guard = self.owner.lock().await;
                guard.generation += 2;
                guard.loaded_instance = [8; 16];
                self.generation
                    .lock()
                    .unwrap()
                    .store(guard.generation, Ordering::Release);
            }
            let mut response = runtime_host_batch_response_from_request(&request);
            if failed {
                response.state = RuntimeHostBatchExecutionState::Failed;
                response.members[0].state = RuntimeHostBatchExecutionMemberState::Failed;
                response.members[0].outputs.clear();
                response.diagnostics = vec![RuntimeHostExecutionDiagnostic {
                    severity: RuntimeHostExecutionDiagnosticSeverity::Info,
                    code: RuntimeHostExecutionDiagnosticCode::HandoffRejected,
                    message: "synthetic terminal execution failure".into(),
                    hint: None,
                }];
                response.members[0].diagnostics = response.diagnostics.clone();
                response.members[0].retry_disposition =
                    RuntimeHostBatchMemberRetryDisposition::NotRetryable;
                response
                    .validate()
                    .expect("valid synthetic failed host response");
            }
            Ok((response, drained, evidence))
        }
        async fn execute_serial_singleton_with_cleanup(
            &self,
            request: RuntimeHostBatchExecutionRequest,
            cancellation: RuntimeHostExecutionCancellationHandle,
            bound: SchedulerSerialBoundDispatch,
        ) -> Result<
            pantograph_runtime_host_contracts::SerialRuntimeHostDrainedExecution,
            RuntimeHostExecutionPortError,
        > {
            let candidate = bound.identity().candidate_id.to_owned();
            let (response, drained, old) = self
                .execute_serial_singleton(request.clone(), cancellation, bound)
                .await?;
            if !self.verified_reuse.load(Ordering::Acquire) {
                return Ok(
                    pantograph_runtime_host_contracts::SerialRuntimeHostDrainedExecution {
                        response,
                        drained,
                        owner: old,
                        cleanup: None,
                    },
                );
            }
            // Synthetic protocol fixture only; actual native publication/drain
            // authority is exercised by the embedded real CPU worker tests.
            let old = old.unwrap();
            let mut owner = self.owner.lock().await;
            owner.generation += 2;
            let new = SerialRuntimeHostCpuOwnerEvidence {
                snapshot: *owner,
                generation: old.generation.clone(),
            };
            new.generation
                .store(new.snapshot.generation, Ordering::Release);
            drop(owner);
            let member = &request.members[0];
            let receipt = SyntheticWarmCleanup {
                old,
                new: new.clone(),
                owner: self.owner.clone(),
                cleanups: self.cleanups.clone(),
                fail: self.fail_cleanup.load(Ordering::Acquire),
                workflow: member.handoff.workflow_id.to_string(),
                run: member.handoff.workflow_run_id.to_string(),
                node: member.handoff.node_id.to_string(),
                task: member.handoff.task_id.to_string(),
                candidate,
                lease: member
                    .handoff
                    .dispatch_decision
                    .as_ref()
                    .unwrap()
                    .reservation_lease_id
                    .to_string(),
            };
            Ok(
                pantograph_runtime_host_contracts::SerialRuntimeHostDrainedExecution {
                    response,
                    drained,
                    owner: Some(new),
                    cleanup: Some(Box::new(receipt)),
                },
            )
        }
        async fn apply_serial_cleanup(
            &self,
            event: ReservationLifecycleEvent,
            expected: Option<SchedulerSerialOwnerSnapshot>,
        ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
            let guard = self.owner.lock().await;
            self.cleanup_started.notify_one();
            if self.block_cleanup.load(Ordering::Acquire) {
                self.cleanup_release.notified().await;
            }
            if expected.is_some_and(|s| s != *guard) || self.fail_cleanup.load(Ordering::Acquire) {
                return Err(ReservationLifecyclePortError::Failed {
                    message: "synthetic fenced cleanup refused".into(),
                });
            }
            self.cleanups.lock().unwrap().push(event.clone());
            Ok(application(event))
        }
    }

    struct SyntheticWarmCleanup {
        old: SerialRuntimeHostCpuOwnerEvidence,
        new: SerialRuntimeHostCpuOwnerEvidence,
        owner: Arc<tokio::sync::Mutex<SchedulerSerialOwnerSnapshot>>,
        cleanups: Arc<Mutex<Vec<ReservationLifecycleEvent>>>,
        fail: bool,
        workflow: String,
        run: String,
        node: String,
        task: String,
        candidate: String,
        lease: String,
    }
    #[async_trait::async_trait]
    impl pantograph_runtime_host_contracts::SerialRuntimeHostCleanupReceipt for SyntheticWarmCleanup {
        fn previous_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence {
            self.old.clone()
        }
        fn current_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence {
            self.new.clone()
        }
        async fn apply(
            self: Box<Self>,
            event: ReservationLifecycleEvent,
        ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
            let owner = self.owner.lock().await;
            if self.fail
                || *owner != self.new.snapshot
                || event.workflow_id.as_str() != self.workflow
                || event.workflow_run_id.as_str() != self.run
                || event.node_id.as_str() != self.node
                || event.task_id.as_str() != self.task
                || event.reservation_lease_id.as_str() != self.lease
                || event.candidate_id.as_ref().map(|id| id.as_str())
                    != Some(self.candidate.as_str())
            {
                return Err(ReservationLifecyclePortError::Failed {
                    message: "synthetic linear receipt refused".into(),
                });
            }
            self.cleanups.lock().unwrap().push(event.clone());
            Ok(application(event))
        }
    }
    fn cpu_run(service: &WorkflowService, run: &str) -> String {
        cpu_run_with_texts(service, run, ["slow", "fast"])
    }
    fn cpu_run_with_texts(service: &WorkflowService, run: &str, texts: [&str; 2]) -> String {
        cpu_run_with_inputs(service, run, &texts)
    }
    fn cpu_run_with_inputs(service: &WorkflowService, run: &str, texts: &[&str]) -> String {
        cpu_run_with_attribution_inputs(service, run, texts, None)
    }
    fn cpu_run_with_attribution_inputs(
        service: &WorkflowService,
        run: &str,
        texts: &[&str],
        attribution: Option<WorkflowExecutionSessionAttributionContext>,
    ) -> String {
        let session = prepare_ready_runtime_branch_run_with_attribution(service, run, attribution);
        let mut graph = ready_runtime_task_graph(run);
        let mut records = vec![];
        let mut proofs = vec![];
        let mut results = vec![];
        graph.tasks.clear();
        for (i, text) in texts.iter().enumerate() {
            let mut task = ready_runtime_task_graph(run).tasks.remove(0);
            let mut intent = task_intent_for_run(run);
            intent.task_id = SchedulerTaskId::parse(format!("task.cpu.{i}")).unwrap();
            intent.task_type =
                pantograph_dependency_planning::DependencyTaskId::parse("embedding").unwrap();
            intent.model_ref.model_id = "synthetic.cpu.model".into();
            intent.model_ref.selected_artifact_id = Some("synthetic.bert.fixture".into());
            intent.constraints.requested_runtime_id =
                Some(pantograph_dependency_planning::RuntimeIntentId::parse("candle").unwrap());
            intent.constraints.requested_device_id =
                Some(pantograph_dependency_planning::DeviceIntentId::parse("cpu").unwrap());
            intent.trait_settings.clear();
            intent.estimate_hints.clear();
            let mut proof = readiness_proof_for_run(run);
            proof.execution_context.scheduler_task_id =
                pantograph_dependency_planning::DependencyReadinessSchedulerTaskId::parse(
                    intent.task_id.as_str(),
                )
                .unwrap();
            proof.execution_context.dependency_override_fingerprint = Some(
                pantograph_dependency_planning::DependencyOverrideFingerprint::parse(
                    "override.fixture",
                )
                .unwrap(),
            );
            proof.preflight_result.identity_key.model_ref = intent.model_ref.clone();
            proof.preflight_result.identity_key.task_id = intent.task_type.clone();
            proof
                .preflight_result
                .identity_key
                .scheduler_intent
                .requested_runtime_id = intent.constraints.requested_runtime_id.clone();
            proof
                .preflight_result
                .identity_key
                .scheduler_intent
                .requested_device_id = intent.constraints.requested_device_id.clone();
            proof.readiness_proof_id =
                DependencyReadinessProofId::parse(format!("proof.{run}.{i}")).unwrap();
            let ctx = &proof.execution_context;
            task.task_id = intent.task_id.clone();
            task.schedulable_intent = Some(intent.clone());
            task.schedulable_intent_template = Some(WorkflowSchedulerTaskIntentTemplate {
                task_type: intent.task_type.clone(),
                constraints: intent.constraints.clone(),
                trait_settings: vec![],
                dependency_override_patches: vec![],
                estimate_hints: vec![],
                dependency_readiness_source: WorkflowSchedulerDependencyReadinessSource {
                    graph_revision: ctx.graph_revision.clone(),
                    validation_session_id: ctx.validation_session_id.clone(),
                    validation_snapshot_id: ctx.validation_snapshot_id.clone(),
                    descriptor_fingerprint: ctx.descriptor_fingerprint.clone(),
                    dependency_requirements_id: ctx.dependency_requirements_id.clone(),
                    selected_binding_ids: ctx.selected_binding_ids.clone(),
                    dependency_override_fingerprint: ctx
                        .dependency_override_fingerprint
                        .clone()
                        .unwrap(),
                },
            });
            let source_id = SchedulerTaskId::parse(format!("task.source.{i}")).unwrap();
            let source_node = SchedulerNodeId::parse(format!("node.source.{i}")).unwrap();
            task.dependency_task_ids = vec![source_id.clone()];
            task.input_bindings = vec![WorkflowSchedulerTaskInputBinding {
                source_node_id: source_node.clone(),
                source_task_id: source_id.clone(),
                source_port_id: "text".into(),
                target_port_id: "text".into(),
            }];
            let mut record = ready_runtime_task_record(run);
            record.task_id = task.task_id.clone();
            record.state = SchedulerTaskState::Ready {
                execution_intent: SchedulerTaskExecutionIntent::runtime(intent),
            };
            records.push(record);
            proofs.push((task.task_id.to_string(), proof));
            graph.tasks.push(task.clone());
            task.task_id = source_id.clone();
            task.node_id = source_node.clone();
            task.execution_class = WorkflowSchedulerTaskExecutionClass::SourceInput;
            task.schedulable_intent = None;
            task.schedulable_intent_template = None;
            task.dependency_task_ids.clear();
            task.input_bindings.clear();
            task.source_input_task_template = Some(WorkflowSchedulerSourceInputTemplate::Text {
                port_id: "text".into(),
            });
            let mut record = ready_runtime_task_record(run);
            record.task_id = source_id.clone();
            record.node_id = source_node.clone();
            record.state = SchedulerTaskState::Completed {
                execution_intent: SchedulerTaskExecutionIntent::SourceInput {
                    task_intent: SchedulerSourceInputTaskIntent {
                        contract_version: 1,
                        workflow_id: task.workflow_id.clone(),
                        workflow_run_id: task.workflow_run_id.clone(),
                        node_id: source_node.clone(),
                        task_id: source_id.clone(),
                        task_kind: SchedulerSourceInputTaskKind::parse("text-input").unwrap(),
                    },
                },
            };
            records.push(record);
            graph.tasks.push(task);
            results.push(WorkflowSchedulerTaskResult {
                schema_version: 1,
                workflow_id: WORKER_BATCH_WORKFLOW_ID.into(),
                workflow_run_id: run.into(),
                node_id: source_node.to_string(),
                task_id: source_id.to_string(),
                status: WorkflowSchedulerTaskResultStatus::Completed,
                outputs: vec![WorkflowSchedulerTaskResultOutput {
                    port_id: "text".into(),
                    value: WorkflowSchedulerTaskResultValue::String((*text).into()),
                }],
                diagnostics: vec![],
                terminal_metadata: None,
            });
        }
        let mut store = service.session_store_guard().unwrap();
        store
            .set_active_run_scheduler_task_state(&session, run, graph, records)
            .unwrap();
        store
            .set_active_run_scheduler_task_results(&session, run, results)
            .unwrap();
        for (id, proof) in proofs {
            store
                .record_active_run_runtime_dispatch_readiness_proof(&session, run, &id, proof)
                .unwrap();
        }
        drop(store);
        for i in 0..2 {
            let mut event = WorkflowRuntimeBranchTaskEventRecord::ready(
                WorkflowRuntimeBranchTaskEventRequest {
                    event_id: WorkflowRuntimeBranchTaskEventId::parse(format!("event.{run}.{i}"))
                        .unwrap(),
                    session_id: session.clone(),
                    workflow_id: WORKER_BATCH_WORKFLOW_ID.into(),
                    workflow_run_id: run.into(),
                    scheduler_task_id: format!("task.cpu.{i}"),
                    scheduler_task_attempt_id: None,
                    attempt_generation: 1,
                    queued_input_keys: vec![],
                    output_targets: Some(vec![WorkflowOutputTarget {
                        node_id: WORKER_BATCH_NODE_ID.into(),
                        port_id: "image".into(),
                    }]),
                    timeout_ms: Some(500),
                    batching_key: None,
                    runtime_source_context: runtime_source_context(),
                    batch_eligibility: None,
                    ready_at_ms: unix_timestamp_ms().saturating_sub(1),
                },
            )
            .unwrap();
            // Synthetic contracts exercise the caller boundary; this is not a native embedding adapter.
            event.batching_key = None;
            service
                .runtime_branch_task_event_repository
                .lock()
                .unwrap()
                .enqueue(event)
                .unwrap();
        }
        session
    }
    struct SyntheticCpuCandidateProvider;

    impl WorkflowRuntimeDispatchCandidateProvider for SyntheticCpuCandidateProvider {
        fn runtime_dispatch_candidates(
            &self,
            task: &WorkflowSchedulerTask,
            _ready_record: &SchedulerTaskStateRecord,
            readiness_proof: &DependencyReadinessProofEnvelope,
        ) -> Result<
            WorkflowRuntimeDispatchCandidateSet,
            WorkflowRuntimeDispatchCandidateProviderError,
        > {
            let intent = task.schedulable_intent.as_ref().ok_or_else(|| {
                worker_candidate_provider_error(
                    "worker batch test runtime task is missing a schedulable intent",
                )
            })?;
            let selected_runtime_id =
                intent
                    .constraints
                    .requested_runtime_id
                    .clone()
                    .ok_or_else(|| {
                        worker_candidate_provider_error(
                            "worker batch test runtime task is missing a requested runtime id",
                        )
                    })?;
            let selected_device_id =
                intent
                    .constraints
                    .requested_device_id
                    .clone()
                    .ok_or_else(|| {
                        worker_candidate_provider_error(
                            "worker batch test runtime task is missing a requested device id",
                        )
                    })?;
            let environment_ref = readiness_proof
                .preflight_result
                .environment_ref
                .clone()
                .ok_or_else(|| {
                    worker_candidate_provider_error(
                        "worker batch test readiness proof is missing an environment reference",
                    )
                })?;
            let reservation = SchedulerResourceReservation {
                reservation_lease_id: SchedulerReservationLeaseId::parse(format!(
                    "reservation.{}",
                    intent.workflow_run_id.as_str()
                ))
                .map_err(|error| worker_candidate_provider_error(error.to_string()))?,
                workflow_run_id: intent.workflow_run_id.clone(),
                task_id: intent.task_id.clone(),
                device_id: selected_device_id.clone(),
                resource_kind: SchedulerResourceKind::SystemRam,
                reserved_bytes: 128,
            };
            let fact = WorkflowRuntimeDispatchCandidateFact {
                candidate_id: SchedulerDispatchCandidateId::parse("candidate.runtime_worker_test")
                    .map_err(|error| worker_candidate_provider_error(error.to_string()))?,
                selected_runtime_id,
                selected_runtime_variant_id: None,
                selected_backend_key: "synthetic-cpu-owner".to_string(),
                runtime_family: "synthetic-cpu-owner".to_string(),
                resolved_load_target: format!("test:{}", intent.model_ref.model_id),
                runtime_residency_key: format!("test-runtime:{}", intent.model_ref.model_id),
                loaded_runtime_memory_estimate_bytes: 128,
                runtime_load_state: WorkflowRuntimeDispatchLoadState::Loaded,
                runtime_instance_id: Some("runtime.worker-batch-test.001".to_string()),
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
                    contract_version:
                        WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
                    facts: vec![fact],
                    diagnostics: Vec::new(),
                },
            )
            .map_err(|error| worker_candidate_provider_error(error.to_string()))?;
            Ok(WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(bundle))
        }
    }

    fn service(port: Arc<CpuPort>, config: WorkflowSerialReadyConfig) -> Arc<WorkflowService> {
        Arc::new(
            WorkflowService::new_serial_ready_cpu(port, config)
                .with_runtime_dispatch_candidate_provider(Arc::new(SyntheticCpuCandidateProvider)),
        )
    }
    async fn run(service: Arc<WorkflowService>, session: &str, id: &str) {
        let environment =
            WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(service, test_host());
        let command = runtime_branch_command_for_session_run(session, id);
        let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
        let (responder, _rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let mut registration = registry.register_workflow_run(&command, responder).unwrap();
        let result = claim_and_execute_runtime_branch_event(
            &environment,
            &command,
            &registry,
            &mut registration,
        )
        .await;
        if let WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::Continue(continuations) =
            result
        {
            drive_runtime_branch_continuations(&environment, &registry, continuations).await;
        }
    }
    fn attributed_cpu_run(service: &WorkflowService, run: &str, client: &str) -> String {
        // Controlled store fixture only; actual credential verification is covered
        // by public attributed-session tests and the native M3 cohort.
        cpu_run_with_attribution_inputs(
            service,
            run,
            &["slow", "fast"],
            Some(WorkflowExecutionSessionAttributionContext {
                client_id: client.into(),
                client_session_id: format!("session.{client}"),
                bucket_id: "bucket.default".into(),
            }),
        )
    }

    async fn wait_for_fair_waiters(service: &WorkflowService, expected: usize) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while service.serial_service_pending_for_test().unwrap() != expected {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[derive(Default)]
    struct FairStartRefreshBarrier {
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
    }
    #[async_trait::async_trait]
    impl WorkflowRuntimeDispatchSourceRefresher for FairStartRefreshBarrier {
        async fn refresh_runtime_dispatch_sources(
            &self,
            _: &WorkflowSchedulerTask,
            _: &SchedulerTaskStateRecord,
            _: &DependencyReadinessProofEnvelope,
        ) -> Result<(), WorkflowRuntimeDispatchSourceRefreshError> {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn fair_serial_final_start_fences_population_changed_during_async_preselection() {
        let port = CpuPort::new();
        let barrier = Arc::new(FairStartRefreshBarrier::default());
        let service = Arc::new(
            WorkflowService::new_serial_ready_cpu(
                port.clone(),
                WorkflowSerialReadyConfig::default().with_fair_service_opportunities(),
            )
            .with_runtime_dispatch_candidate_provider(Arc::new(SyntheticCpuCandidateProvider))
            .with_runtime_dispatch_source_refresher(barrier.clone()),
        );
        let session = cpu_run_with_attribution_inputs(
            &service,
            "run.fair.final.fence",
            &["slow"],
            Some(WorkflowExecutionSessionAttributionContext {
                client_id: "client.a".into(),
                client_session_id: "session.a".into(),
                bucket_id: "bucket.a".into(),
            }),
        );
        assert!(service
            .session_store_guard()
            .unwrap()
            .serial_ready_pair(&session, "run.fair.final.fence")
            .is_none());
        let handle = tokio::spawn({
            let service = service.clone();
            let session = session.clone();
            async move { run(service, &session, "run.fair.final.fence").await }
        });
        tokio::time::timeout(Duration::from_secs(2), barrier.entered.notified())
            .await
            .unwrap();
        {
            let mut store = service.session_store_guard().unwrap();
            let (mut graph, mut records) = store
                .active_run_scheduler_task_state(&session, "run.fair.final.fence")
                .unwrap()
                .unwrap();
            // Add unrelated completed sources: selected task, inputs and proof
            // remain identical across the candidate-preparation await.
            let source = graph
                .tasks
                .iter()
                .find(|task| {
                    task.execution_class == WorkflowSchedulerTaskExecutionClass::SourceInput
                })
                .unwrap()
                .clone();
            let record = records
                .iter()
                .find(|record| record.task_id == source.task_id)
                .unwrap()
                .clone();
            for index in 0..127 {
                let mut task = source.clone();
                let mut record = record.clone();
                task.task_id = SchedulerTaskId::parse(format!("task.extra.{index}")).unwrap();
                record.task_id = task.task_id.clone();
                let SchedulerTaskState::Completed {
                    execution_intent: SchedulerTaskExecutionIntent::SourceInput { task_intent },
                } = &mut record.state
                else {
                    panic!("completed source fixture")
                };
                task_intent.task_id = task.task_id.clone();
                graph.tasks.push(task);
                records.push(record);
            }
            store
                .set_active_run_scheduler_task_state(
                    &session,
                    "run.fair.final.fence",
                    graph,
                    records,
                )
                .unwrap();
        }
        barrier.release.notify_one();
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .unwrap()
            .unwrap();
        assert!(port.requests.lock().unwrap().is_empty());
        assert!(port.cleanups.lock().unwrap().is_empty());
        let store = service.session_store_guard().unwrap();
        assert!(store
            .active_run_scheduler_task_attempt_id(&session, "run.fair.final.fence", "task.cpu.0")
            .is_err());
        let (_, records) = store
            .active_run_scheduler_task_state(&session, "run.fair.final.fence")
            .unwrap()
            .unwrap();
        assert_eq!(
            records
                .iter()
                .find(|record| record.task_id.as_str() == "task.cpu.0")
                .unwrap()
                .state
                .kind(),
            SchedulerTaskStateKind::Ready
        );
    }

    #[tokio::test]
    async fn fair_serial_missing_identity_refuses_without_claim_or_native_forward() {
        let port = CpuPort::new();
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::default().with_fair_service_opportunities(),
        );
        let session = cpu_run(&service, "run.fair.anonymous");
        run(service.clone(), &session, "run.fair.anonymous").await;
        assert!(port.requests.lock().unwrap().is_empty());
        assert!(port.cleanups.lock().unwrap().is_empty());
        assert_eq!(service.serial_service_pending_for_test().unwrap(), 0);
        assert!(matches!(service.session_store_guard().unwrap()
            .serial_service_identity(&session, "run.fair.anonymous"),
            Err(WorkflowServiceError::InvalidRequest(message)) if message == "fair serial service requires authenticated client/bucket session attribution"));
        assert_eq!(
            runtime_branch_event_state(
                &service,
                &WorkflowRuntimeBranchTaskEventId::parse("event.run.fair.anonymous.0").unwrap()
            ),
            WorkflowRuntimeBranchTaskEventState::Ready
        );
    }

    #[tokio::test]
    async fn fair_serial_stale_waiter_refuses_before_claim_and_keeps_other_owner_cleanup() {
        let port = CpuPort::new();
        port.block.store(true, Ordering::Release);
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::default().with_fair_service_opportunities(),
        );
        let first = attributed_cpu_run(&service, "run.fair.active", "client.a");
        let second = attributed_cpu_run(&service, "run.fair.stale", "client.b");
        let first_handle = tokio::spawn({
            let service = service.clone();
            async move { run(service, &first, "run.fair.active").await }
        });
        tokio::time::timeout(Duration::from_secs(2), port.started.notified())
            .await
            .unwrap();
        let second_handle = tokio::spawn({
            let service = service.clone();
            let second = second.clone();
            async move { run(service, &second, "run.fair.stale").await }
        });
        wait_for_fair_waiters(&service, 1).await;
        assert_eq!(port.requests.lock().unwrap().len(), 1);
        {
            let mut store = service.session_store_guard().unwrap();
            let mut results = store
                .active_run_scheduler_task_results(&second, "run.fair.stale")
                .unwrap();
            results[0].outputs[0].value =
                WorkflowSchedulerTaskResultValue::String("stale exact input".into());
            store
                .set_active_run_scheduler_task_results(&second, "run.fair.stale", results)
                .unwrap();
        }
        port.block.store(false, Ordering::Release);
        port.release.notify_one();
        tokio::time::timeout(Duration::from_secs(2), first_handle)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), second_handle)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(port.requests.lock().unwrap().len(), 2);
        assert!(port
            .requests
            .lock()
            .unwrap()
            .iter()
            .all(
                |request| request.members[0].handoff.workflow_run_id.as_str() == "run.fair.active"
            ));
        assert_eq!(port.cleanups.lock().unwrap().len(), 2);
        assert_eq!(
            runtime_branch_event_state(
                &service,
                &WorkflowRuntimeBranchTaskEventId::parse("event.run.fair.stale.0").unwrap()
            ),
            WorkflowRuntimeBranchTaskEventState::Ready
        );
        assert_eq!(service.serial_service_pending_for_test().unwrap(), 0);
    }

    #[tokio::test]
    async fn fair_serial_aborted_owner_poison_wakes_ready_waiter_without_forward() {
        let port = CpuPort::new();
        port.block.store(true, Ordering::Release);
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::default().with_fair_service_opportunities(),
        );
        let first = attributed_cpu_run(&service, "run.fair.abort", "client.a");
        let second = attributed_cpu_run(&service, "run.fair.after.abort", "client.b");
        let first_handle = tokio::spawn({
            let service = service.clone();
            async move { run(service, &first, "run.fair.abort").await }
        });
        tokio::time::timeout(Duration::from_secs(2), port.started.notified())
            .await
            .unwrap();
        let second_handle = tokio::spawn({
            let service = service.clone();
            async move { run(service, &second, "run.fair.after.abort").await }
        });
        wait_for_fair_waiters(&service, 1).await;
        first_handle.abort();
        assert!(first_handle.await.unwrap_err().is_cancelled());
        tokio::time::timeout(Duration::from_secs(2), second_handle)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(port.requests.lock().unwrap().len(), 1);
        assert!(port.cleanups.lock().unwrap().is_empty());
        assert_eq!(service.serial_service_pending_for_test().unwrap(), 0);
    }

    #[test]
    fn fair_ready_eligibility_fences_order_state_run_and_population_bounds() {
        let service = service(
            CpuPort::new(),
            WorkflowSerialReadyConfig::default().with_fair_service_opportunities(),
        );
        let session = attributed_cpu_run(&service, "run.fair.bounds", "client.a");
        let mut store = service.session_store_guard().unwrap();
        assert!(store
            .serial_service_ready_member(&session, "run.fair.bounds", "task.cpu.0")
            .is_some());
        assert!(store
            .serial_service_ready_member(&session, "run.fair.bounds", "task.cpu.1")
            .is_none());
        assert!(store
            .serial_service_ready_member(&session, "run.other", "task.cpu.0")
            .is_none());
        let (mut graph, records) = store
            .active_run_scheduler_task_state(&session, "run.fair.bounds")
            .unwrap()
            .unwrap();
        graph.tasks.swap(0, 2);
        store
            .set_active_run_scheduler_task_state(
                &session,
                "run.fair.bounds",
                graph.clone(),
                records.clone(),
            )
            .unwrap();
        assert!(store
            .serial_service_ready_member(&session, "run.fair.bounds", "task.cpu.0")
            .is_none());
        graph.tasks.resize(129, graph.tasks[0].clone());
        store
            .set_active_run_scheduler_task_state(&session, "run.fair.bounds", graph, records)
            .unwrap();
        assert!(store
            .serial_service_ready_member(&session, "run.fair.bounds", "task.cpu.1")
            .is_none());
    }
    #[tokio::test]
    async fn serial_ready_worker_learns_only_real_drained_cleaned_dispatches_and_ranks_first_pair()
    {
        let port = CpuPort::new();
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        for i in 0..4 {
            let id = format!("run.serial.{i}");
            let session = cpu_run(&service, &id);
            assert!(service
                .session_store_guard()
                .unwrap()
                .serial_ready_pair(&session, &id)
                .is_some());
            run(service.clone(), &session, &id).await;
        }
        assert_eq!(
            port.tasks(),
            vec![
                "task.cpu.0",
                "task.cpu.1",
                "task.cpu.0",
                "task.cpu.1",
                "task.cpu.0",
                "task.cpu.1",
                "task.cpu.1",
                "task.cpu.0"
            ]
        );
        assert_eq!(port.cleanups.lock().unwrap().len(), 8);
        assert!(service.serial_ready_predecessor_for_test().is_none());
        assert!(service
            .serial_ready_conditioned_samples_for_test()
            .iter()
            .all(|s| s.2.is_none()));
    }
    #[tokio::test]
    async fn serial_ready_worker_stale_samples_keep_fifo() {
        let port = CpuPort::new();
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::new(3, Duration::from_nanos(1)).unwrap(),
        );
        for i in 0..4 {
            let id = format!("run.stale.{i}");
            let session = cpu_run(&service, &id);
            run(service.clone(), &session, &id).await;
        }
        assert_eq!(port.tasks(), ["task.cpu.0", "task.cpu.1"].repeat(4));
    }
    #[tokio::test]
    async fn serial_ready_worker_changed_owner_refuses_before_forward_and_poison_refuses_next_run()
    {
        let port = CpuPort::new();
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        for i in 0..3 {
            let id = format!("run.warm.change.{i}");
            let session = cpu_run(&service, &id);
            run(service.clone(), &session, &id).await;
        }
        port.requests.lock().unwrap().clear();
        port.cleanups.lock().unwrap().clear();
        port.change_before_forward.store(true, Ordering::Release);
        let session = cpu_run(&service, "run.changed");
        run(service.clone(), &session, "run.changed").await;
        let session = cpu_run(&service, "run.after.changed");
        run(service.clone(), &session, "run.after.changed").await;
        assert!(port.tasks().is_empty());
        assert!(port.cleanups.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn serial_ready_worker_cleanup_failure_blocks_next_claim() {
        let port = CpuPort::new();
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        port.fail_cleanup.store(true, Ordering::Release);
        let session = cpu_run(&service, "run.cleanup");
        run(service.clone(), &session, "run.cleanup").await;
        let session = cpu_run(&service, "run.after.cleanup");
        run(service.clone(), &session, "run.after.cleanup").await;
        assert_eq!(port.tasks(), vec!["task.cpu.0"]);
    }
    #[tokio::test]
    async fn serial_ready_worker_missing_evidence_keeps_fifo_on_same_serial_port() {
        let port = CpuPort::new();
        port.evidence.store(false, Ordering::Release);
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        for i in 0..4 {
            let id = format!("run.no.evidence.{i}");
            let session = cpu_run(&service, &id);
            run(service.clone(), &session, &id).await;
        }
        assert_eq!(port.tasks(), ["task.cpu.0", "task.cpu.1"].repeat(4));
        assert_eq!(port.cleanups.lock().unwrap().len(), 8);
    }
    #[tokio::test]
    async fn serial_ready_worker_ties_preserve_authoritative_first_position() {
        let port = CpuPort::new();
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        for i in 0..3 {
            let id = format!("run.tie.{i}");
            let session = cpu_run_with_texts(&service, &id, ["same", "same"]);
            run(service.clone(), &session, &id).await;
        }
        assert_eq!(port.tasks(), ["task.cpu.0", "task.cpu.1"].repeat(3));
    }
    #[tokio::test]
    async fn serial_ready_worker_new_generation_authority_cannot_reuse_old_samples() {
        let port = CpuPort::new();
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        for i in 0..3 {
            let id = format!("run.domain.{i}");
            let session = cpu_run(&service, &id);
            run(service.clone(), &session, &id).await;
        }
        *port.generation.lock().unwrap() = Arc::new(AtomicU64::new(2));
        let session = cpu_run(&service, "run.domain.new");
        run(service.clone(), &session, "run.domain.new").await;
        assert_eq!(port.tasks(), ["task.cpu.0", "task.cpu.1"].repeat(4));
    }
    #[tokio::test]
    async fn serial_ready_worker_queued_command_waits_without_claim_and_reacquires_after_drain_cleanup(
    ) {
        let port = CpuPort::new();
        port.block.store(true, Ordering::Release);
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        let first = cpu_run(&service, "run.serial.first");
        let second = cpu_run(&service, "run.serial.second");
        let first_handle = tokio::spawn({
            let service = service.clone();
            async move { run(service, &first, "run.serial.first").await }
        });
        tokio::time::timeout(Duration::from_secs(2), port.started.notified())
            .await
            .unwrap();
        let second_handle = tokio::spawn({
            let service = service.clone();
            let second = second.clone();
            async move { run(service, &second, "run.serial.second").await }
        });
        tokio::task::yield_now().await;
        assert_eq!(port.requests.lock().unwrap().len(), 1);
        let second_event =
            WorkflowRuntimeBranchTaskEventId::parse("event.run.serial.second.0").unwrap();
        assert_eq!(
            runtime_branch_event_state(&service, &second_event),
            WorkflowRuntimeBranchTaskEventState::Ready
        );
        port.block.store(false, Ordering::Release);
        port.release.notify_one();
        tokio::time::timeout(Duration::from_secs(2), first_handle)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), second_handle)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(port.requests.lock().unwrap().len(), 4);
        assert_eq!(port.cleanups.lock().unwrap().len(), 4);
        assert!(port
            .requests
            .lock()
            .unwrap()
            .iter()
            .all(|r| r.members.len() == 1));
    }
    #[tokio::test]
    async fn serial_ready_worker_aborted_owned_attempt_poison_wakes_queued_command_without_forward()
    {
        let port = CpuPort::new();
        port.block.store(true, Ordering::Release);
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        let first = cpu_run(&service, "run.abort.first");
        let second = cpu_run(&service, "run.abort.second");
        let first_handle = tokio::spawn({
            let service = service.clone();
            async move { run(service, &first, "run.abort.first").await }
        });
        tokio::time::timeout(Duration::from_secs(2), port.started.notified())
            .await
            .unwrap();
        let second_handle = tokio::spawn({
            let service = service.clone();
            async move { run(service, &second, "run.abort.second").await }
        });
        tokio::task::yield_now().await;
        first_handle.abort();
        assert!(first_handle.await.unwrap_err().is_cancelled());
        tokio::time::timeout(Duration::from_secs(2), second_handle)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(port.requests.lock().unwrap().len(), 1);
        assert!(port.cleanups.lock().unwrap().is_empty());
    }
    #[test]
    fn serial_ready_pair_fences_unselected_population_order_inputs_and_proof() {
        let port = CpuPort::new();
        let service = service(port, WorkflowSerialReadyConfig::default());
        let session = cpu_run(&service, "run.fence");
        let mut store = service.session_store_guard().unwrap();
        let pair = store.serial_ready_pair(&session, "run.fence").unwrap();
        assert!(store
            .validate_serial_ready_pair(&session, "run.fence", &pair)
            .is_ok());
        let (graph, mut records) = store
            .active_run_scheduler_task_state(&session, "run.fence")
            .unwrap()
            .unwrap();
        records
            .iter_mut()
            .find(|r| r.task_id.as_str() == "task.cpu.1")
            .unwrap()
            .state_version += 1;
        store
            .set_active_run_scheduler_task_state(
                &session,
                "run.fence",
                graph.clone(),
                records.clone(),
            )
            .unwrap();
        assert!(store
            .validate_serial_ready_pair(&session, "run.fence", &pair)
            .is_err());
        let pair = store.serial_ready_pair(&session, "run.fence").unwrap();
        let mut reordered = graph;
        reordered.tasks.swap(0, 2);
        store
            .set_active_run_scheduler_task_state(&session, "run.fence", reordered, records)
            .unwrap();
        assert!(store
            .validate_serial_ready_pair(&session, "run.fence", &pair)
            .is_err());
        let pair = store.serial_ready_pair(&session, "run.fence").unwrap();
        let mut results = store
            .active_run_scheduler_task_results(&session, "run.fence")
            .unwrap();
        results[0].outputs[0].value =
            WorkflowSchedulerTaskResultValue::String("changed exact text".into());
        store
            .set_active_run_scheduler_task_results(&session, "run.fence", results)
            .unwrap();
        assert!(store
            .validate_serial_ready_pair(&session, "run.fence", &pair)
            .is_err());
    }

    #[tokio::test]
    async fn serial_ready_worker_owner_changed_after_drain_keeps_original_cleanup_fence() {
        let port = CpuPort::new();
        port.change_after_drain.store(true, Ordering::Release);
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        let session = cpu_run(&service, "run.after.drain");
        run(service.clone(), &session, "run.after.drain").await;
        let session = cpu_run(&service, "run.after.drain.next");
        run(service.clone(), &session, "run.after.drain.next").await;
        assert_eq!(port.tasks(), vec!["task.cpu.0"]);
        assert!(port.cleanups.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn serial_ready_worker_handle_retains_owned_work_when_terminal_receiver_drops() {
        let port = CpuPort::new();
        port.block.store(true, Ordering::Release);
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        let first = cpu_run(&service, "run.handle.first");
        let second = cpu_run(&service, "run.handle.second");
        let worker = WorkflowTaskExecutionWorker::spawn(
            scheduler_lifecycle(),
            WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(service.clone(), test_host()),
        )
        .unwrap();
        let (responder, completion) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        worker
            .try_enqueue(WorkflowTaskExecutionWorkerCommand::execute_runtime_branch(
                runtime_branch_command_for_session_run(&first, "run.handle.first"),
                responder,
            ))
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), port.started.notified())
            .await
            .unwrap();
        drop(completion);
        let (responder, completion) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        worker
            .try_enqueue(WorkflowTaskExecutionWorkerCommand::execute_runtime_branch(
                runtime_branch_command_for_session_run(&second, "run.handle.second"),
                responder,
            ))
            .unwrap();
        tokio::task::yield_now().await;
        assert_eq!(port.requests.lock().unwrap().len(), 1);
        port.block.store(false, Ordering::Release);
        port.release.notify_one();
        let outcome = tokio::time::timeout(Duration::from_secs(2), completion)
            .await
            .unwrap()
            .unwrap();
        assert_runtime_branch_completed_response(
            outcome,
            "run.handle.second",
            "image for run.handle.second",
        );
        assert_eq!(port.requests.lock().unwrap().len(), 4);
        assert_eq!(port.cleanups.lock().unwrap().len(), 4);
        worker.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn serial_ready_worker_cancelled_before_actual_drain_does_not_train() {
        let port = CpuPort::new();
        port.block.store(true, Ordering::Release);
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        let session = cpu_run(&service, "run.cancel.cleanup");
        let handle = tokio::spawn({
            let service = service.clone();
            let session = session.clone();
            async move { run(service, &session, "run.cancel.cleanup").await }
        });
        tokio::time::timeout(Duration::from_secs(2), port.started.notified())
            .await
            .unwrap();
        let attempt = service
            .session_store_guard()
            .unwrap()
            .active_run_scheduler_task_attempt_id(&session, "run.cancel.cleanup", "task.cpu.0")
            .unwrap();
        service
            .scheduler_task_orchestrator
            .request_started_runtime_task_cancellation(
                &SchedulerTaskId::parse("task.cpu.0").unwrap(),
                &attempt,
                "fixture cancellation before actual drain",
            )
            .unwrap();
        port.block.store(false, Ordering::Release);
        port.release.notify_one();
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .unwrap()
            .unwrap();
        // Two subsequent warm runs provide only two valid slow samples: FIFO remains.
        for i in 0..3 {
            let id = format!("run.after.cancel.{i}");
            let session = cpu_run(&service, &id);
            run(service.clone(), &session, &id).await;
        }
        assert_eq!(
            port.tasks()
                .iter()
                .filter(|id| id.as_str() == "task.cpu.0")
                .count(),
            4
        );
        assert_eq!(
            port.tasks()[port.tasks().len() - 2..],
            ["task.cpu.0", "task.cpu.1"]
        );
    }
    struct TimedSerialRejectHost;
    #[async_trait::async_trait]
    impl WorkflowHost for TimedSerialRejectHost {
        fn workflow_roots(&self) -> Vec<std::path::PathBuf> {
            panic!("timed serial refusal must precede host graph access")
        }
        async fn run_workflow(
            &self,
            _: &str,
            _: &[WorkflowPortBinding],
            _: Option<&[WorkflowOutputTarget]>,
            _: WorkflowRunOptions,
            _: WorkflowRunHandle,
        ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
            panic!("timed serial refusal must precede host execution")
        }
    }

    fn assert_serial_timeout_refused(error: WorkflowServiceError) {
        assert!(
            matches!(error, WorkflowServiceError::InvalidRequest(message)
            if message == "serial Ready CPU mode does not support workflow timeout_ms; use None")
        );
    }

    #[tokio::test]
    async fn serial_ready_timed_submission_refuses_before_host_or_admission() {
        let port = CpuPort::new();
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        let session = service
            .session_store_guard()
            .unwrap()
            .create_session(
                WORKER_BATCH_WORKFLOW_ID.into(),
                None,
                None,
                Vec::new(),
                Vec::new(),
                false,
            )
            .unwrap();
        let before = service
            .session_store_guard()
            .unwrap()
            .session_summary(&session)
            .unwrap();
        let runtime = WorkflowSessionExecutionRuntime::from_shared_service(
            service.clone(),
            Arc::new(TimedSerialRejectHost),
        );
        for supervised in [false, true] {
            let request = WorkflowExecutionSessionRunRequest {
                session_id: session.clone(),
                workflow_semantic_version: "1.0.0".into(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: Some(500),
                priority: None,
            };
            let result = if supervised {
                runtime.run_workflow_execution_session(request).await
            } else {
                service
                    .run_workflow_execution_session(&TimedSerialRejectHost, request)
                    .await
            };
            assert_serial_timeout_refused(
                result.expect_err("unsupported timeout must be explicit"),
            );
            {
                let store = service.session_store_guard().unwrap();
                assert_eq!(
                    serde_json::to_value(store.session_summary(&session).unwrap()).unwrap(),
                    serde_json::to_value(&before).unwrap(),
                );
                assert!(store.list_queue(&session).unwrap().is_empty());
                assert_eq!(store.active_run_count(), 0);
            }
            assert!(port.tasks().is_empty());
            assert!(port.cleanups.lock().unwrap().is_empty());
            assert!(service.serial_ready_observations_for_test().is_empty());
            let scope = tokio::time::timeout(
                Duration::from_secs(1),
                service.serial_ready_mode.as_ref().unwrap().enter(),
            )
            .await
            .unwrap()
            .expect("refused submission cannot claim or poison serial admission");
            drop(scope);
        }
    }

    #[tokio::test]
    async fn serial_ready_timed_stored_resume_refuses_before_events_or_worker_admission() {
        let port = CpuPort::new();
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        let id = "run.timed.resume";
        // This existing fixture represents a persisted active timed run. New
        // serial submission no longer admits this unsupported timeout contract.
        let session = cpu_run(&service, id);
        let before = service
            .session_store_guard()
            .unwrap()
            .active_run_scheduler_task_state(&session, id)
            .unwrap();
        assert_eq!(
            service
                .session_store_guard()
                .unwrap()
                .active_run_context(&session, id)
                .unwrap()
                .timeout_ms,
            Some(500)
        );
        let event_ids: Vec<_> = (0..2)
            .map(|i| WorkflowRuntimeBranchTaskEventId::parse(format!("event.{id}.{i}")).unwrap())
            .collect();
        let events_before: Vec<_> = event_ids
            .iter()
            .map(|event| {
                service
                    .runtime_branch_task_event_repository
                    .lock()
                    .unwrap()
                    .get(event)
                    .unwrap()
            })
            .collect();
        let runtime = WorkflowSessionExecutionRuntime::from_shared_service(
            service.clone(),
            Arc::new(TimedSerialRejectHost),
        );
        for supervised in [false, true] {
            let request = WorkflowExecutionSessionResumeRequest {
                session_id: session.clone(),
                workflow_run_id: id.into(),
            };
            let result = if supervised {
                runtime
                    .resume_workflow_execution_session_runtime_dependency_readiness(request)
                    .await
            } else {
                service
                    .resume_workflow_execution_session_runtime_dependency_readiness(
                        &TimedSerialRejectHost,
                        request,
                    )
                    .await
            };
            assert_serial_timeout_refused(result.expect_err("stored timed run must not enqueue"));
            assert_eq!(
                service
                    .session_store_guard()
                    .unwrap()
                    .active_run_scheduler_task_state(&session, id)
                    .unwrap(),
                before
            );
            let events_after: Vec<_> = event_ids
                .iter()
                .map(|event| {
                    service
                        .runtime_branch_task_event_repository
                        .lock()
                        .unwrap()
                        .get(event)
                        .unwrap()
                })
                .collect();
            assert_eq!(events_after, events_before);
            assert!(port.tasks().is_empty());
            assert!(port.cleanups.lock().unwrap().is_empty());
            assert!(service.serial_ready_observations_for_test().is_empty());
            let scope = tokio::time::timeout(
                Duration::from_secs(1),
                service.serial_ready_mode.as_ref().unwrap().enter(),
            )
            .await
            .unwrap()
            .expect("refused resume cannot claim or poison serial admission");
            drop(scope);
        }
    }

    #[tokio::test]
    async fn serial_ready_invalid_clock_discards_partial_ranking_and_reports_typed_fifo_reason() {
        let service = service(CpuPort::new(), WorkflowSerialReadyConfig::default());
        let id = "run.clock.empty";
        let session = cpu_run(&service, id);
        let pair = service
            .session_store_guard()
            .unwrap()
            .serial_ready_pair(&session, id)
            .unwrap();
        let mode = service.serial_ready_mode.as_ref().unwrap();
        let initial = mode.select(&pair);
        assert_eq!(
            initial.reason,
            crate::workflow::serial_ready::SerialReadyReason::InsufficientFreshSamples
        );
        assert_eq!(initial.position, 0);
        assert!(initial.owner.is_none());
        assert_eq!(initial.counts, [0, 0]);
        for i in 0..3 {
            let id = format!("run.clock.warm.{i}");
            let session = cpu_run(&service, &id);
            run(service.clone(), &session, &id).await;
        }
        crate::workflow::serial_ready::tests::assert_invalid_clock_clears_partial_ranking(
            mode, &pair,
        );
    }
    #[tokio::test]
    async fn serial_ready_worker_selection_cost_reports_actual_bounded_policy_cpu() {
        let port = CpuPort::new();
        let service = service(port, WorkflowSerialReadyConfig::default());
        for i in 0..3 {
            let id = format!("run.cost.warm.{i}");
            let session = cpu_run(&service, &id);
            run(service.clone(), &session, &id).await;
        }
        let session = cpu_run(&service, "run.cost.probe");
        let pair = service
            .session_store_guard()
            .unwrap()
            .serial_ready_pair(&session, "run.cost.probe")
            .unwrap();
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            assert_eq!(
                service
                    .serial_ready_mode
                    .as_ref()
                    .unwrap()
                    .select(&pair)
                    .position,
                1
            );
        }
        eprintln!("serial Ready observed CPU: 1000 selections in {:?}; 2 keys / 6 actual completed cleanup samples, 25 ms synthetic slow execution",start.elapsed());
    }
    #[tokio::test]
    async fn verified_linear_reuse_migrates_fresh_samples_and_ranks_the_first_ready_pair() {
        let port = CpuPort::new();
        port.verified_reuse.store(true, Ordering::Release);
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        for i in 0..4 {
            let id = format!("run.warm.{i}");
            let session = cpu_run(&service, &id);
            run(service.clone(), &session, &id).await;
            let generation = port.owner.lock().await.generation;
            assert_eq!(
                service.serial_ready_observations_for_test(),
                vec![(generation, (i + 1) * 2)]
            );
        }
        assert_eq!(
            port.tasks(),
            vec![
                "task.cpu.0",
                "task.cpu.1",
                "task.cpu.0",
                "task.cpu.1",
                "task.cpu.0",
                "task.cpu.1",
                "task.cpu.1",
                "task.cpu.0"
            ]
        );
        assert_eq!(port.cleanups.lock().unwrap().len(), 8);
    }
    #[tokio::test]
    async fn failed_linear_cleanup_never_migrates_samples_or_reopens_serial_admission() {
        let port = CpuPort::new();
        port.verified_reuse.store(true, Ordering::Release);
        let service = service(port.clone(), WorkflowSerialReadyConfig::default());
        for i in 0..2 {
            let id = format!("run.clean.{i}");
            let session = cpu_run(&service, &id);
            run(service.clone(), &session, &id).await;
        }
        let prior = service.serial_ready_observations_for_test();
        port.fail_cleanup.store(true, Ordering::Release);
        let session = cpu_run(&service, "run.fail");
        run(service.clone(), &session, "run.fail").await;
        assert_eq!(service.serial_ready_observations_for_test(), prior);
        assert_eq!(port.cleanups.lock().unwrap().len(), 4);
        assert_eq!(port.tasks().len(), 5);
        assert!(service
            .serial_ready_mode
            .as_ref()
            .unwrap()
            .enter()
            .await
            .is_err());
    }

    #[tokio::test]
    async fn serial_ready_completion_advisory_hand_oracle_preserves_immediate_policy() {
        let port = CpuPort::new();
        let service = service(
            port,
            WorkflowSerialReadyConfig::default().with_completion_lookahead(),
        );
        let session = cpu_run(&service, "run.lookahead.oracle");
        let pair = service
            .session_store_guard()
            .unwrap()
            .serial_ready_pair(&session, "run.lookahead.oracle")
            .unwrap();
        crate::workflow::serial_ready::tests::assert_conditioned_hand_oracle_and_refusals(
            service.serial_ready_mode.as_ref().unwrap(),
            &pair,
        );
    }

    #[tokio::test]
    async fn serial_ready_conditioned_history_follows_real_cleanup_transitions_and_reuse() {
        let port = CpuPort::new();
        port.verified_reuse.store(true, Ordering::Release);
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::default().with_completion_lookahead(),
        );
        // This fixed protocol needs thirteen completed sessions plus its pair.
        service.session_store_guard().unwrap().max_sessions = 14;
        let sequence = [
            "slow", "slow", "fast", "fast", "slow", "slow", "fast", "fast", "slow", "slow", "fast",
            "fast", "slow",
        ];
        let mut expected_predecessor = None;
        let mut workload_keys = std::collections::BTreeMap::new();
        for (i, text) in sequence.iter().enumerate() {
            let id = format!("run.transition.{i}");
            let session = cpu_run_with_inputs(&service, &id, &[*text]);
            run(service.clone(), &session, &id).await;
            let samples = service.serial_ready_conditioned_samples_for_test();
            assert_eq!(samples.len(), i + 1);
            let sample = samples.last().unwrap();
            assert_eq!(sample.2, expected_predecessor);
            assert!(sample.3 > 0);
            assert_eq!(sample.5, port.owner.lock().await.generation);
            assert!(samples.iter().all(|s| s.5 == sample.5));
            if let Some(key) = workload_keys.insert(*text, sample.1) {
                assert_eq!(key, sample.1);
            }
            expected_predecessor = Some(sample.1);
            assert_eq!(
                service.serial_ready_predecessor_for_test(),
                Some((sample.1, sample.5))
            );
        }
        let session = cpu_run(&service, "run.transition.pair");
        let pair = service
            .session_store_guard()
            .unwrap()
            .serial_ready_pair(&session, "run.transition.pair")
            .unwrap();
        crate::workflow::serial_ready::tests::assert_real_conditioned_advice(
            service.serial_ready_mode.as_ref().unwrap(),
            &pair,
        );
        assert_eq!(port.requests.lock().unwrap().len(), 13);
        assert_eq!(port.cleanups.lock().unwrap().len(), 13);
    }

    #[tokio::test]
    async fn serial_ready_unsupported_execution_invalidates_conditioned_predecessor() {
        let port = CpuPort::new();
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::default().with_completion_lookahead(),
        );
        let session = cpu_run_with_inputs(&service, "run.supported", &["fast"]);
        run(service.clone(), &session, "run.supported").await;
        assert!(service.serial_ready_predecessor_for_test().is_some());
        let session = cpu_run_with_inputs(&service, "run.unsupported", &[""]);
        run(service.clone(), &session, "run.unsupported").await;
        assert_eq!(port.requests.lock().unwrap().len(), 2);
        assert_eq!(port.cleanups.lock().unwrap().len(), 2);
        assert!(service.serial_ready_predecessor_for_test().is_none());
        let session = cpu_run_with_inputs(&service, "run.after.unsupported", &["fast"]);
        run(service.clone(), &session, "run.after.unsupported").await;
        assert_eq!(service.serial_ready_conditioned_samples_for_test().len(), 2);
        assert!(service
            .serial_ready_conditioned_samples_for_test()
            .last()
            .unwrap()
            .2
            .is_none());
    }

    #[tokio::test]
    async fn serial_ready_unrelated_owner_cannot_relabel_consumed_predecessor() {
        let port = CpuPort::new();
        port.verified_reuse.store(true, Ordering::Release);
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::default().with_completion_lookahead(),
        );
        let session = cpu_run_with_inputs(&service, "run.owner.original", &["fast"]);
        run(service.clone(), &session, "run.owner.original").await;
        // An unrelated authority with identical scalar snapshot is a new owner.
        let generation = port.owner.lock().await.generation;
        *port.generation.lock().unwrap() = Arc::new(AtomicU64::new(generation));
        let session = cpu_run_with_inputs(&service, "run.owner.replaced", &["fast"]);
        run(service.clone(), &session, "run.owner.replaced").await;
        let samples = service.serial_ready_conditioned_samples_for_test();
        assert_eq!(samples.len(), 2);
        assert!(samples[1].2.is_none());
        assert_ne!(samples[0].5, samples[1].5);
        assert_eq!(port.cleanups.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn serial_ready_failed_cleanup_clears_conditioned_predecessor_without_training() {
        let port = CpuPort::new();
        port.verified_reuse.store(true, Ordering::Release);
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::default().with_completion_lookahead(),
        );
        let session = cpu_run_with_inputs(&service, "run.cleanup.seed", &["fast"]);
        run(service.clone(), &session, "run.cleanup.seed").await;
        let prior = service.serial_ready_conditioned_samples_for_test();
        assert!(service.serial_ready_predecessor_for_test().is_some());
        port.fail_cleanup.store(true, Ordering::Release);
        let session = cpu_run_with_inputs(&service, "run.cleanup.fail", &["fast"]);
        run(service.clone(), &session, "run.cleanup.fail").await;
        assert!(service.serial_ready_predecessor_for_test().is_none());
        let samples = service.serial_ready_conditioned_samples_for_test();
        assert_eq!(samples.len(), prior.len());
        assert_eq!(samples[0].5, prior[0].5);
        assert_eq!(port.cleanups.lock().unwrap().len(), 1);
        assert!(service
            .serial_ready_mode
            .as_ref()
            .unwrap()
            .enter()
            .await
            .is_err());
    }

    #[tokio::test]
    async fn serial_ready_cancelled_execution_clears_conditioned_predecessor_without_training() {
        let port = CpuPort::new();
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::default().with_completion_lookahead(),
        );
        let session = cpu_run_with_inputs(&service, "run.cancel.seed", &["fast"]);
        run(service.clone(), &session, "run.cancel.seed").await;
        assert!(service.serial_ready_predecessor_for_test().is_some());
        // Consume the seed notification before observing the blocked dispatch.
        port.started.notified().await;
        port.block.store(true, Ordering::Release);
        let session = cpu_run_with_inputs(&service, "run.cancel.conditioned", &["fast"]);
        let handle = tokio::spawn({
            let service = service.clone();
            let session = session.clone();
            async move { run(service, &session, "run.cancel.conditioned").await }
        });
        tokio::time::timeout(Duration::from_secs(2), port.started.notified())
            .await
            .unwrap();
        assert!(service.serial_ready_predecessor_for_test().is_none());
        let attempt = service
            .session_store_guard()
            .unwrap()
            .active_run_scheduler_task_attempt_id(&session, "run.cancel.conditioned", "task.cpu.0")
            .unwrap();
        service
            .scheduler_task_orchestrator
            .request_started_runtime_task_cancellation(
                &SchedulerTaskId::parse("task.cpu.0").unwrap(),
                &attempt,
                "fixture cancellation invalidates predecessor",
            )
            .unwrap();
        port.block.store(false, Ordering::Release);
        port.release.notify_one();
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(service.serial_ready_conditioned_samples_for_test().len(), 1);
        assert!(service.serial_ready_predecessor_for_test().is_none());
        assert_eq!(port.cleanups.lock().unwrap().len(), 2);
    }
    #[tokio::test]
    async fn serial_ready_failed_execution_clears_conditioned_predecessor_without_training() {
        let port = CpuPort::new();
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::default().with_completion_lookahead(),
        );
        let session = cpu_run_with_inputs(&service, "run.failure.seed", &["fast"]);
        run(service.clone(), &session, "run.failure.seed").await;
        assert!(service.serial_ready_predecessor_for_test().is_some());
        port.fail_execution.store(true, Ordering::Release);
        let session = cpu_run_with_inputs(&service, "run.failure.actual", &["fast"]);
        run(service.clone(), &session, "run.failure.actual").await;
        assert_eq!(service.serial_ready_conditioned_samples_for_test().len(), 1);
        assert!(service.serial_ready_predecessor_for_test().is_none());
        assert_eq!(port.requests.lock().unwrap().len(), 2);
        assert_eq!(port.cleanups.lock().unwrap().len(), 2);
        let session = cpu_run_with_inputs(&service, "run.failure.next", &["fast"]);
        run(service.clone(), &session, "run.failure.next").await;
        assert!(service
            .serial_ready_conditioned_samples_for_test()
            .last()
            .unwrap()
            .2
            .is_none());
    }

    #[tokio::test]
    async fn serial_ready_preflight_refusal_preserves_unconsumed_predecessor() {
        use crate::workflow::serial_ready::SerialReadyPortAdapter;
        let port = CpuPort::new();
        let service = service(
            port.clone(),
            WorkflowSerialReadyConfig::default().with_completion_lookahead(),
        );
        let session = cpu_run_with_inputs(&service, "run.preflight.seed", &["fast"]);
        run(service.clone(), &session, "run.preflight.seed").await;
        let previous = service.serial_ready_predecessor_for_test().unwrap();
        let session = cpu_run(&service, "run.preflight.current");
        let pair = service
            .session_store_guard()
            .unwrap()
            .serial_ready_pair(&session, "run.preflight.current")
            .unwrap();
        let mode = service.serial_ready_mode.as_ref().unwrap();
        let scope = mode.enter().await.unwrap();
        scope.capture(&pair.members[0], None).unwrap();
        let request = port.requests.lock().unwrap()[0].clone();
        let cancellation =
            RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone());
        let error = SerialReadyPortAdapter(mode.clone())
            .execute_runtime_host_batch_request(request, cancellation)
            .await
            .unwrap_err();
        assert!(
            matches!(error, RuntimeHostExecutionPortError::ExecutionFailed { message }
            if message == "serial admitted handoff changed before owner execution")
        );
        assert_eq!(service.serial_ready_predecessor_for_test(), Some(previous));
        assert_eq!(port.requests.lock().unwrap().len(), 1);
        drop(scope);
    }
}

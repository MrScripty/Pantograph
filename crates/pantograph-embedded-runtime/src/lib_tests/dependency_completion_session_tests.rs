//! Public session path, real owned input/output mapping and lifecycle adapter.
//! Model resolver/backend and costs are controlled fixtures, no model downloads.
use super::*;
use pantograph_runtime_host_contracts::*;
use pantograph_scheduler::SchedulerTaskStateKind;
use std::sync::atomic::{AtomicBool, Ordering};

struct Owner {
    mode: u8,
    queries: Mutex<Vec<EmbeddedCompletionTimingQuery>>,
    conditional: Mutex<Vec<EmbeddedDependencyCompletionQuery>>,
    registry: Mutex<Option<pantograph_runtime_registry::SharedRuntimeRegistry>>,
}
impl EmbeddedCompletionTimingSource for Owner {
    fn dependency_lookahead_enabled(&self) -> bool {
        self.mode != 10
    }
    fn timing_for(
        &self,
        q: &EmbeddedCompletionTimingQuery,
    ) -> Option<EmbeddedCompletionTimingRecord> {
        assert!(self
            .registry
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .snapshot()
            .reservations
            .is_empty());
        self.queries.lock().unwrap().push(q.clone());
        if q.backend_key != "transformers" || q.task_kind != "text_generation" || q.device_id != "cpu"
            || !q.materialized_inputs.iter().any(|i| i.port_id == "prompt" && matches!(&i.value, RuntimeHostExecutionInputValue::String(s) if s == "paint a red cube" || s == "expanded:paint a red cube")) { return None; }
        let estimate = |us: u64| inference::RuntimeServiceTimingValue::ConfiguredEstimate {
            elapsed_ns: us * 1000,
        };
        Some(EmbeddedCompletionTimingRecord {
            query: q.clone(),
            observed_at_ms: crate::runtime_dispatch_candidate_provider::current_time_ms(),
            successful_sample_count: 1,
            preparation: estimate(0),
            required_transfer: estimate(0),
            execution: estimate(if q.runtime_id == "pytorch" { 2 } else { 5 }),
        })
    }
    fn timing_after_successful_completion(
        &self,
        q: &EmbeddedDependencyCompletionQuery,
    ) -> Option<EmbeddedDependencyCompletionRecord> {
        assert!(
            self.registry
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .snapshot()
                .reservations
                .is_empty(),
            "forecast is read-only, no successor or first lease"
        );
        assert_eq!(
            q.successor.record.state.kind(),
            SchedulerTaskStateKind::AwaitingInputs
        );
        assert_eq!(q.successor.predecessor_inputs.len(), 1);
        assert!(
            q.successor
                .known_inputs
                .iter()
                .all(|i| i.port_id != "prompt"),
            "future prompt remains symbolic"
        );
        self.conditional.lock().unwrap().push(q.clone());
        if self.mode == 1 {
            return None;
        }
        let queries = self.queries.lock().unwrap();
        let next = queries
            .iter()
            .rev()
            .find(|r| r.runtime_id == q.placement.candidate.selected_runtime_id.as_str())
            .unwrap();
        let mut observation = next.resource_observation.clone();
        observation.observed_at_ms = crate::runtime_dispatch_candidate_provider::current_time_ms();
        if next.runtime_id == q.predecessor.runtime_id {
            observation.runtime_status =
                pantograph_runtime_registry::RuntimeRegistryStatus::Stopped;
            observation.runtime_instance_id = None;
        }
        let successor_residency = observation.runtime_instance_id.clone().map_or(
            EmbeddedCompletionProjectedResidency::Unloaded,
            |runtime_instance_id| EmbeddedCompletionProjectedResidency::Retained {
                runtime_instance_id,
            },
        );
        let intent = q.successor.task.schedulable_intent.as_ref().unwrap();
        let estimate = |us: u64| {
            Some(inference::RuntimeServiceTimingValue::ConfiguredEstimate {
                elapsed_ns: us * 1000,
            })
        };
        let mut r = EmbeddedDependencyCompletionRecord {
            query: q.clone(),
            observed_at_ms: observation.observed_at_ms,
            successful_sample_count: 1,
            post_cleanup_state_fingerprint: format!(
                "fixture-after-{}-release-reconcile",
                q.predecessor.runtime_id
            ),
            resource_observation: observation,
            resource_fit: pantograph_scheduler::SchedulerResourceFitAssessment {
                workflow_run_id: intent.workflow_run_id.clone(),
                task_id: intent.task_id.clone(),
                state: pantograph_scheduler::SchedulerResourceFitState::Fits,
                diagnostics: vec![],
            },
            predecessor_residency: EmbeddedCompletionProjectedResidency::Unloaded,
            successor_residency,
            preparation: estimate(0),
            required_transfer: estimate(0),
            execution: estimate(if q.predecessor.runtime_id == "pytorch" {
                20
            } else if next.runtime_id == "pytorch" {
                1
            } else {
                2
            }),
        };
        match self.mode {
            2 => r.resource_fit.state = pantograph_scheduler::SchedulerResourceFitState::Unknown,
            3 => r.observed_at_ms = 0,
            4 => r.required_transfer = None,
            5 if next.runtime_id == "pytorch.transformers" => {
                r.post_cleanup_state_fingerprint = "incompatible-post-cleanup-world".into()
            }
            6 => r.resource_observation.resources[0].requested_bytes = 0,
            8 => r.resource_observation.resources[0].capacity_bytes = Some(1),
            _ => {}
        }
        Some(r)
    }
}
struct Host {
    inner: Arc<EmbeddedRuntimeHostExecutionPort>,
    dispatches: Mutex<Vec<(String, String)>>,
    registry: pantograph_runtime_registry::SharedRuntimeRegistry,
    cancellation: Mutex<Option<(std::sync::Weak<WorkflowService>, String)>>,
}
impl Host {
    async fn cancel_if_requested(&self, handoff: &pantograph_scheduler::SchedulerRuntimeHandoff) {
        let target = self.cancellation.lock().unwrap().clone();
        if let Some((service, session_id)) = target {
            service
                .upgrade()
                .unwrap()
                .workflow_cancel_active_execution_session_task(
                    pantograph_workflow_service::WorkflowExecutionSessionActiveTaskCancelRequest {
                        session_id,
                        workflow_run_id: handoff.task_intent.workflow_run_id.to_string(),
                        task_id: handoff.task_intent.task_id.to_string(),
                        reason: Some("controlled pair cancellation".into()),
                    },
                )
                .await
                .unwrap();
        }
    }
    fn record(&self, handoff: &pantograph_scheduler::SchedulerRuntimeHandoff) {
        assert_eq!(
            self.registry.snapshot().reservations.len(),
            1,
            "only actual selected lease exists"
        );
        self.dispatches.lock().unwrap().push((
            handoff.task_intent.node_id.to_string(),
            handoff
                .dispatch_decision
                .as_ref()
                .unwrap()
                .selected_runtime_id
                .to_string(),
        ));
    }
}
#[async_trait]
impl RuntimeHostExecutionPort for Host {
    async fn execute_runtime_host_request(
        &self,
        request: RuntimeHostExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        self.record(&request.handoff);
        self.cancel_if_requested(&request.handoff).await;
        self.inner
            .execute_runtime_host_request(request, cancellation)
            .await
    }
}
#[async_trait]
impl RuntimeHostBatchExecutionPort for Host {
    async fn execute_runtime_host_batch_request(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        assert_eq!(request.members.len(), 1, "existing singleton envelope");
        self.record(&request.members[0].handoff);
        self.cancel_if_requested(&request.members[0].handoff).await;
        let response = self
            .inner
            .execute_runtime_host_batch_request(request, cancellation)
            .await;
        response
    }
}
struct TimedProvider {
    inner: crate::runtime_dispatch_candidate_provider::EmbeddedRuntimeDispatchCandidateProvider,
    elapsed: Arc<Mutex<Vec<std::time::Duration>>>,
}
impl pantograph_workflow_service::workflow::WorkflowRuntimeDispatchCandidateProvider
    for TimedProvider
{
    fn requires_materialized_inputs(&self) -> bool {
        self.inner.requires_materialized_inputs()
    }
    fn requires_dependency_lookahead(&self) -> bool {
        self.inner.requires_dependency_lookahead()
    }
    fn runtime_dispatch_candidates(
        &self,
        task: &WorkflowSchedulerTask,
        ready: &pantograph_scheduler::SchedulerTaskStateRecord,
        proof: &DependencyReadinessProofEnvelope,
    ) -> Result<
        pantograph_workflow_service::workflow::WorkflowRuntimeDispatchCandidateSet,
        pantograph_workflow_service::workflow::WorkflowRuntimeDispatchCandidateProviderError,
    > {
        self.inner.runtime_dispatch_candidates(task, ready, proof)
    }
    fn runtime_dispatch_candidates_with_successor(
        &self,
        task: &WorkflowSchedulerTask,
        ready: &pantograph_scheduler::SchedulerTaskStateRecord,
        proof: &DependencyReadinessProofEnvelope,
        inputs: Option<&[RuntimeHostExecutionInput]>,
        successor: Option<
            &pantograph_workflow_service::workflow::WorkflowCompletionSuccessorSnapshot,
        >,
    ) -> Result<
        pantograph_workflow_service::workflow::WorkflowRuntimeDispatchCandidateSet,
        pantograph_workflow_service::workflow::WorkflowRuntimeDispatchCandidateProviderError,
    > {
        let start = std::time::Instant::now();
        let result = self
            .inner
            .runtime_dispatch_candidates_with_successor(task, ready, proof, inputs, successor);
        self.elapsed.lock().unwrap().push(start.elapsed());
        result
    }
}
struct Cleanup {
    inner: Arc<dyn ReservationLifecyclePort>,
    first: AtomicBool,
    pause: bool,
    fail: bool,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Semaphore,
    run_id: Mutex<Option<String>>,
}
#[async_trait]
impl ReservationLifecyclePort for Cleanup {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        let first = event.node_id.as_str() == "text-infer"
            && matches!(
                event.outcome,
                ReservationLifecycleOutcome::RuntimeHostCompleted
                    | ReservationLifecycleOutcome::RetryDeferred
            )
            && !self.first.swap(true, Ordering::SeqCst);
        if first {
            *self.run_id.lock().unwrap() = Some(event.workflow_run_id.to_string());
            if self.pause {
                self.entered.notify_one();
                self.resume.acquire().await.unwrap().forget();
            }
        }
        let application = self.inner.apply_reservation_lifecycle(event).await?;
        if first && self.fail {
            return Err(ReservationLifecyclePortError::Failed {
                message: "controlled missing cleanup acknowledgement after release".into(),
            });
        }
        Ok(application)
    }
}

#[tokio::test]
async fn dependency_public_session_changes_first_choice_and_uses_real_output_with_fresh_successor_admission(
) {
    qualification(0, false, false).await;
}
#[tokio::test]
async fn dependency_public_session_missing_unknown_stale_incoherent_or_wrong_demand_forecasts_preserve_one_decision(
) {
    for mode in [1, 2, 3, 4, 5, 6, 8, 10] {
        qualification(mode, false, false).await;
    }
}
#[tokio::test]
async fn dependency_public_session_completed_state_waits_for_cleanup_ack_and_failure_never_dispatches_successor(
) {
    qualification(0, true, false).await;
    qualification(0, true, true).await;
    qualification(11, false, false).await;
}
async fn qualification(mode: u8, pause: bool, fail: bool) -> Vec<std::time::Duration> {
    const MODEL: &str = "llm/example/tiny-transformers";
    const ARTIFACT: &str = "text-bundle";
    let temp = TempDir::new().unwrap();
    let artifact_writer = test_artifact_writer(&temp);
    let readiness = DependencyEnvironmentReadinessSnapshotProvider::new();
    let owner = Arc::new(Owner {
        mode,
        queries: Mutex::default(),
        conditional: Mutex::default(),
        registry: Mutex::default(),
    });
    let (provider, registry) =
        crate::runtime_dispatch_candidate_provider::completion_text_test_provider(
            PumasModelRef {
                model_id: MODEL.into(),
                revision: None,
                selected_artifact_id: Some(ARTIFACT.into()),
                selected_artifact_path: None,
                migration_diagnostics: vec![],
            },
            Some(EmbeddedCompletionTimingOptIn {
                source: owner.clone(),
                owner_epoch: "configured-dependency-fixture".into(),
                max_sample_age_ms: 1000,
                allow_configured_estimates: true,
            }),
        );
    *owner.registry.lock().unwrap() = Some(registry.clone());
    let elapsed = Arc::new(Mutex::new(Vec::new()));
    let provider = TimedProvider {
        inner: provider,
        elapsed: elapsed.clone(),
    };
    let target = temp.path().join("controlled-resolver");
    std::fs::create_dir_all(&target).unwrap();
    let resolver = Arc::new(SelectedTextResolver(target));
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(SelectedWorkflowTextBackend(
            prompts.clone(),
            None,
            None,
            None,
            None,
        )),
        "PyTorch",
    ));
    let host = Arc::new(Host {
        inner: Arc::new(EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            resolver.clone(),
            resolver,
            Arc::new(WorkflowServiceRuntimeHostMediaArtifactSink::new(
                artifact_writer.clone(),
            )),
            gateway.clone(),
        )),
        dispatches: Mutex::default(),
        registry: registry.clone(),
        cancellation: Mutex::default(),
    });
    let cleanup = Arc::new(Cleanup {
        inner: Arc::new(
            crate::reservation_lifecycle::EmbeddedReservationLifecyclePort::new(
                registry.clone(),
                gateway,
            ),
        ),
        first: AtomicBool::new(false),
        pause,
        fail,
        entered: tokio::sync::Notify::new(),
        resume: tokio::sync::Semaphore::new(0),
        run_id: Mutex::default(),
    });
    let service = Arc::new(
        WorkflowService::with_ephemeral_attribution_store()
            .unwrap()
            .with_artifact_writer(artifact_writer)
            .with_dependency_environment_provider(Arc::new(readiness.clone()))
            .with_dependency_readiness_work_queue(Arc::new(DependencyReadinessWorkQueue::new()))
            .with_runtime_dispatch_source_refresher(Arc::new(
                TestRuntimeDispatchSourceRefresher::default(),
            ))
            .with_runtime_dispatch_candidate_provider(Arc::new(provider))
            .with_runtime_host_execution_port(host.clone())
            .with_runtime_host_batch_execution_port(host.clone())
            .with_reservation_lifecycle_port(cleanup.clone()),
    );
    let mut graph = dependent_text_image_session_graph(MODEL, ARTIFACT, MODEL, ARTIFACT);
    for node in graph
        .nodes
        .iter_mut()
        .filter(|n| n.node_type == "llm-inference")
    {
        make_text_runtime_inference_node(node);
        node.data.as_object_mut().unwrap().remove("runtime");
    }
    let version = service
        .resolve_workflow_graph_version("dependency-completion-fixture", "1.2.3", &graph)
        .unwrap();
    let mut snapshot = image_runtime_validation_snapshot(&version, &graph, MODEL, ARTIFACT);
    let template = snapshot.nodes[0].clone();
    snapshot.nodes.clear();
    for node_id in ["text-infer", "image-infer"] {
        let mut node = template.clone();
        node.node_id = WorkflowNodeId::parse(node_id).unwrap();
        node.task_kind = InferenceTaskKind::parse("text_generation").unwrap();
        node.estimate_hints
            .retain(|h| h.kind == SchedulerEstimateHintKind::PeakRamBytes);
        node.constraints.requested_runtime_id = None;
        node.constraints.requested_device_id = Some(DeviceIntentId::parse("cpu").unwrap());
        node.runtime_source_context =
            pantograph_workflow_service::graph::WorkflowRuntimeSourceContext {
                operation_type: "text-generation.chat".into(),
                context_shape_key: "configured-known-output-class".into(),
                cancellation_mode: "per-run-fanout".into(),
            };
        let mut planning = image_runtime_dependency_planning_request(
            &version,
            &node.model_ref,
            vec![DependencyBindingId::parse("torch-transformers").unwrap()],
        );
        planning.task_id = DependencyTaskId::parse("text_generation").unwrap();
        planning.task_type = None;
        planning.scheduler_intent.requested_runtime_id = None;
        planning.scheduler_intent.requested_device_id = Some(DeviceIntentId::parse("cpu").unwrap());
        planning.caller_context.node_id = Some(node_id.into());
        let proof = produce_dependency_requirements_proof(
            &ValidatedDependencyPlanningRequest::try_from(planning.clone()).unwrap(),
            None,
        )
        .unwrap();
        node.dependency_requirements_id = proof.dependency_requirements_id.clone();
        node.selected_binding_ids = proof.identity_key.selected_binding_ids.clone();
        node.dependency_override_fingerprint = proof.dependency_override_fingerprint.clone();
        let req = ValidatedDependencyEnvironmentRequest::try_from(DependencyEnvironmentRequest {
            contract_version: 1,
            action: DependencyEnvironmentAction::Resolve,
            identity_key: proof.identity_key,
            planning_request: planning,
            dependency_requirements_id: Some(proof.dependency_requirements_id),
            environment_ref: None,
        })
        .unwrap();
        let mut result = ready_image_dependency_environment_result(&req);
        result.requirements[0].name = DependencyRequirementName::parse("transformers").unwrap();
        result.requirements[0].python.as_mut().unwrap().import_name = Some("transformers".into());
        for binding in &mut result.bindings {
            binding.requirement_name = DependencyRequirementName::parse("transformers").unwrap();
        }
        readiness
            .insert_snapshot(
                DependencyEnvironmentReadinessSnapshot::for_request(
                    &req,
                    result,
                    DependencyEnvironmentReadinessSnapshotStatus::Fresh,
                )
                .unwrap(),
            )
            .unwrap();
        snapshot.nodes.push(node);
    }
    service
        .store_workflow_executable_validation_snapshot(snapshot)
        .unwrap();
    let graph_host = Arc::new(ImageRuntimeSessionHost::new(graph));
    let session = service
        .create_workflow_execution_session(
            graph_host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "dependency-completion-fixture".into(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .unwrap();
    if mode == 11 {
        *host.cancellation.lock().unwrap() =
            Some((Arc::downgrade(&service), session.session_id.clone()));
    }
    let request = WorkflowExecutionSessionRunRequest {
        session_id: session.session_id.clone(),
        workflow_semantic_version: "1.2.3".into(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".into(),
            port_id: "text".into(),
            value: serde_json::json!("paint a red cube"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "image-infer".into(),
            port_id: "text".into(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };
    let run_service = service.clone();
    let run = tokio::spawn(async move {
        pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(
            run_service,
            graph_host,
        )
        .run_workflow_execution_session(request)
        .await
    });
    if pause {
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            cleanup.entered.notified(),
        )
        .await
        .unwrap();
        let run_id = cleanup.run_id.lock().unwrap().clone().unwrap();
        let state=service.workflow_get_scheduler_task_state_read_models(pantograph_workflow_service::workflow::WorkflowSchedulerTaskStateReadModelQueryRequest { session_id:session.session_id.clone(),workflow_run_id:run_id }).await.unwrap();
        assert_eq!(
            state
                .tasks
                .iter()
                .find(|t| t.node_id == "text-infer")
                .unwrap()
                .state,
            SchedulerTaskStateKind::Completed
        );
        assert_eq!(
            state
                .tasks
                .iter()
                .find(|t| t.node_id == "image-infer")
                .unwrap()
                .state,
            SchedulerTaskStateKind::AwaitingInputs
        );
        assert_eq!(host.dispatches.lock().unwrap().len(), 1);
        assert_eq!(registry.snapshot().reservations.len(), 1);
        cleanup.resume.add_permits(1);
    }
    let result = run.await.unwrap();
    if fail || mode == 11 {
        assert!(result.is_err());
        assert_eq!(host.dispatches.lock().unwrap().len(), 1);
    } else {
        let result = result.unwrap_or_else(|e| {
            panic!(
                "mode {mode}: {e:?}; dispatches {:?}; queries {:?}; conditional {}",
                host.dispatches.lock().unwrap(),
                owner.queries.lock().unwrap(),
                owner.conditional.lock().unwrap().len()
            )
        });
        assert!(serde_json::to_string(&result)
            .unwrap()
            .contains("expanded:expanded:paint a red cube"));
        let dispatches = host.dispatches.lock().unwrap();
        assert_eq!(dispatches.len(), 2);
        assert_eq!(
            dispatches[0].1,
            if mode == 0 {
                "pytorch.transformers"
            } else {
                "pytorch"
            }
        );
        assert_eq!(dispatches[1].1, "pytorch");
        assert_eq!(
            *prompts.lock().unwrap(),
            vec!["paint a red cube", "expanded:paint a red cube"]
        );
        let queries = owner.queries.lock().unwrap();
        assert_eq!(
            queries.len(),
            4,
            "fresh actual successor queries after real output/admission"
        );
        assert_ne!(
            queries[0].admitted_task_fingerprint,
            queries[2].admitted_task_fingerprint
        );
    }
    assert!(registry.snapshot().reservations.is_empty());
    if mode == 0 {
        assert_eq!(owner.conditional.lock().unwrap().len(), 4);
    }
    if mode == 10 {
        assert!(owner.conditional.lock().unwrap().is_empty());
    }
    let result = elapsed.lock().unwrap().clone();
    result
}

#[tokio::test]
#[ignore = "controlled new native forecast dispatch cost; run without concurrent compilation"]
async fn dependency_native_dispatch_cost_probe() {
    let mut first = Vec::new();
    let mut successor = Vec::new();
    for _ in 0..100 {
        let elapsed = qualification(0, false, false).await;
        assert_eq!(elapsed.len(), 2);
        first.push(elapsed[0].as_nanos());
        successor.push(elapsed[1].as_nanos());
    }
    first.sort_unstable();
    successor.sort_unstable();
    for (label, values) in [
        ("owned-2x2-first", first),
        ("fresh-actual-successor", successor),
    ] {
        println!(
            "{label}: n={} median_us={:.3} p95_us={:.3} max_us={:.3}",
            values.len(),
            values[values.len() / 2] as f64 / 1000.,
            values[values.len() * 95 / 100] as f64 / 1000.,
            *values.last().unwrap() as f64 / 1000.
        );
    }
}

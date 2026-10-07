//! Controlled owner evidence, no models/GPU/native batch execution.
use super::*;
use crate::{
    EmbeddedCompletionTimingOptIn, EmbeddedCompletionTimingQuery, EmbeddedCompletionTimingRecord,
    EmbeddedCompletionTimingSource,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostBatchExecutionPort, RuntimeHostExecutionInputValue, RuntimeHostExecutionPort,
    RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
};

struct ControlledOwner {
    cold_load_ns: u64,
    queries: Mutex<Vec<EmbeddedCompletionTimingQuery>>,
    registry: Mutex<Option<pantograph_runtime_registry::SharedRuntimeRegistry>>,
    mode: u8,
}
impl EmbeddedCompletionTimingSource for ControlledOwner {
    fn timing_for(
        &self,
        q: &EmbeddedCompletionTimingQuery,
    ) -> Option<EmbeddedCompletionTimingRecord> {
        let registry = self.registry.lock().unwrap().clone().unwrap();
        assert!(
            registry.snapshot().reservations.is_empty(),
            "ranking must not acquire speculative leases"
        );
        self.queries.lock().unwrap().push(q.clone());
        // Explicitly controlled exact fixture workload/device/backend. Do not reuse
        // these authored values for real devices/models or unknown prompt shapes.
        if q.backend_key != "diffusers"
            || q.device_id != "cpu"
            || q.artifact_fingerprint != "sha256:abc"
            || !q.materialized_inputs.iter().any(|i| {
                i.port_id == "prompt"
                    && i.value == RuntimeHostExecutionInputValue::String("paint a red cube".into())
            })
        {
            return None;
        }
        if self.mode == 1 {
            return None;
        }
        let estimate =
            |elapsed_ns| inference::RuntimeServiceTimingValue::ConfiguredEstimate { elapsed_ns };
        let cold = q.runtime_id == "pytorch-alt";
        let mut record = EmbeddedCompletionTimingRecord {
            query: q.clone(),
            successful_sample_count: 1,
            observed_at_ms: crate::runtime_dispatch_candidate_provider::current_time_ms(),
            preparation: estimate(if cold { self.cold_load_ns } else { 0 }),
            required_transfer: if self.mode == 2 {
                inference::RuntimeServiceTimingValue::Unknown {
                    reason: inference::RuntimeServiceTimingUnavailableReason::PhaseNotReached,
                }
            } else {
                estimate(0)
            },
            execution: estimate(if cold { 3_000_000_000 } else { 12_000_000_000 }),
        };
        match self.mode {
            3 if q.runtime_id == "pytorch-alt" => {
                // Test-only fault injection between advisory evaluation and commit.
                registry.register_runtime(
                    pantograph_runtime_registry::RuntimeRegistration::new(
                        "pytorch-alt",
                        "changed capacity",
                    )
                    .with_backend_keys(vec!["diffusers".into()])
                    .with_admission_budget(
                        pantograph_runtime_registry::RuntimeAdmissionBudget::from_resources(vec![
                            pantograph_runtime_registry::RuntimeAdmissionResourceBudget::ram_bytes(
                                Some(0),
                            ),
                            pantograph_runtime_registry::RuntimeAdmissionResourceBudget::vram_bytes(
                                Some(0),
                            ),
                        ]),
                    ),
                );
            }
            4 => {
                let observed = |elapsed_ns| inference::RuntimeServiceTimingValue::Observed {
                    elapsed_ns,
                    outcome: inference::RuntimeServiceTimingOutcome::Completed,
                };
                record.preparation = observed(if cold { self.cold_load_ns } else { 0 });
                record.required_transfer = observed(0);
                record.execution = observed(if cold { 3_000_000_000 } else { 12_000_000_000 });
            }
            5 => record.observed_at_ms = 0,
            6 => record.query.device_id = "foreign-device".into(),
            7 => {
                record.execution = inference::RuntimeServiceTimingValue::Observed {
                    elapsed_ns: 1,
                    outcome: inference::RuntimeServiceTimingOutcome::Failed,
                }
            }
            _ => {}
        }
        Some(record)
    }
}
struct ObservedHost {
    inner: Arc<EmbeddedRuntimeHostExecutionPort>,
    runtime_ids: Mutex<Vec<String>>,
    registry: pantograph_runtime_registry::SharedRuntimeRegistry,
    cancellation_target: Mutex<Option<(std::sync::Weak<WorkflowService>, String)>>,
}
#[async_trait]
impl RuntimeHostExecutionPort for ObservedHost {
    async fn execute_runtime_host_request(
        &self,
        request: RuntimeHostExecutionRequest,
        cancellation: pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        assert_eq!(self.registry.snapshot().reservations.len(), 1);
        self.runtime_ids.lock().unwrap().push(
            request
                .handoff
                .dispatch_decision
                .as_ref()
                .unwrap()
                .selected_runtime_id
                .as_str()
                .into(),
        );
        let target = self.cancellation_target.lock().unwrap().clone();
        if let Some((service, session_id)) = target {
            let task = &request.handoff.task_intent;
            service
                .upgrade()
                .unwrap()
                .workflow_cancel_active_execution_session_task(
                    pantograph_workflow_service::WorkflowExecutionSessionActiveTaskCancelRequest {
                        session_id,
                        workflow_run_id: task.workflow_run_id.as_str().into(),
                        task_id: task.task_id.as_str().into(),
                        reason: Some("controlled cancellation".into()),
                    },
                )
                .await
                .unwrap();
        }
        self.inner
            .execute_runtime_host_request(request, cancellation)
            .await
    }
}

#[async_trait]
impl RuntimeHostBatchExecutionPort for ObservedHost {
    async fn execute_runtime_host_batch_request(
        &self,
        request: pantograph_runtime_host_contracts::RuntimeHostBatchExecutionRequest,
        cancellation: pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle,
    ) -> Result<
        pantograph_runtime_host_contracts::RuntimeHostBatchExecutionResponse,
        RuntimeHostExecutionPortError,
    > {
        assert_eq!(
            request.members.len(),
            1,
            "existing singleton envelope, no scheduler batch planning"
        );
        assert_eq!(self.registry.snapshot().reservations.len(), 1);
        let handoff = &request.members[0].handoff;
        self.runtime_ids.lock().unwrap().push(
            handoff
                .dispatch_decision
                .as_ref()
                .unwrap()
                .selected_runtime_id
                .as_str()
                .into(),
        );
        let target = self.cancellation_target.lock().unwrap().clone();
        if let Some((service, session_id)) = target {
            let task = &handoff.task_intent;
            service
                .upgrade()
                .unwrap()
                .workflow_cancel_active_execution_session_task(
                    pantograph_workflow_service::WorkflowExecutionSessionActiveTaskCancelRequest {
                        session_id,
                        workflow_run_id: task.workflow_run_id.as_str().into(),
                        task_id: task.task_id.as_str().into(),
                        reason: Some("controlled cancellation".into()),
                    },
                )
                .await
                .unwrap();
        }
        self.inner
            .execute_runtime_host_batch_request(request, cancellation)
            .await
    }
}

#[tokio::test]
async fn public_session_completion_uses_controlled_owner_costs_and_preserves_artifact_custody() {
    qualification(2_000_000_000, 4, true, Some("pytorch-alt")).await;
    for (load, expected) in [(2_000_000_000, "pytorch-alt"), (10_000_000_000, "pytorch")] {
        qualification(load, 0, true, Some(expected)).await;
    }
}
#[tokio::test]
async fn public_session_unknown_transfer_unknown_workload_and_default_preserve_ambiguity_without_leases(
) {
    for (mode, opt_in) in [
        (1, true),
        (2, true),
        (5, true),
        (6, true),
        (7, true),
        (8, true),
        (0, false),
    ] {
        qualification(2_000_000_000, mode, opt_in, None).await;
    }
}

#[tokio::test]
async fn public_session_revalidates_capacity_and_cancellation_releases_selected_custody() {
    qualification(2_000_000_000, 3, true, None).await;
    qualification(2_000_000_000, 9, true, None).await;
}

async fn qualification(load: u64, mode: u8, opt_in: bool, expected: Option<&str>) {
    const MODEL: &str = "image/example/tiny-diffusion";
    const ARTIFACT: &str = "diffusers-bundle";
    let temp = TempDir::new().unwrap();
    let artifact_writer = test_artifact_writer(&temp);
    let readiness = DependencyEnvironmentReadinessSnapshotProvider::new();
    let owner = Arc::new(ControlledOwner {
        cold_load_ns: load,
        mode,
        queries: Mutex::default(),
        registry: Mutex::default(),
    });
    let (provider, registry) = crate::runtime_dispatch_candidate_provider::completion_test_provider(
        PumasModelRef {
            model_id: MODEL.into(),
            revision: None,
            selected_artifact_id: Some(ARTIFACT.into()),
            selected_artifact_path: None,
            migration_diagnostics: vec![],
        },
        opt_in.then(|| EmbeddedCompletionTimingOptIn {
            source: owner.clone(),
            owner_epoch: "controlled-owner".into(),
            max_sample_age_ms: 1000,
            allow_configured_estimates: mode != 8,
        }),
    );
    *owner.registry.lock().unwrap() = Some(registry.clone());
    std::fs::create_dir_all(temp.path().join("pumas")).unwrap();
    let pumas_api = Arc::new(
        crate::pumas_test_support::builder(temp.path().join("pumas"))
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .unwrap(),
    );
    seed_diffusers_model(&pumas_api, MODEL, ARTIFACT).await;
    pumas_api.resolve_model_package_facts(MODEL).await.unwrap();
    let access = Arc::new(workflow_nodes::setup::PumasSelectorAccess::Owner(pumas_api));
    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(TestImageBackend {
            recorded_guidance: Arc::default(),
            recorded_counts: Arc::default(),
            recorded_schedulers: Arc::default(),
        }),
        "PyTorch",
    ));
    let observed_host = Arc::new(ObservedHost {
        inner: Arc::new(EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(RuntimeHostPumasLoadTargetResolver::new(access.clone())),
            Arc::new(RuntimeHostPumasPackageFactsResolver::new(access)),
            Arc::new(WorkflowServiceRuntimeHostMediaArtifactSink::new(
                artifact_writer.clone(),
            )),
            gateway.clone(),
        )),
        runtime_ids: Mutex::default(),
        registry: registry.clone(),
        cancellation_target: Mutex::default(),
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
            .with_runtime_host_execution_port(observed_host.clone())
            .with_runtime_host_batch_execution_port(observed_host.clone())
            .with_reservation_lifecycle_port(Arc::new(
                crate::reservation_lifecycle::EmbeddedReservationLifecyclePort::new(
                    registry.clone(),
                    gateway,
                ),
            )),
    );
    let mut graph = image_runtime_session_graph(MODEL, ARTIFACT);
    graph
        .nodes
        .iter_mut()
        .find(|n| n.id == "infer")
        .unwrap()
        .data
        .as_object_mut()
        .unwrap()
        .remove("runtime");
    graph
        .nodes
        .iter_mut()
        .find(|n| n.id == "infer")
        .unwrap()
        .data["device"] = serde_json::json!("cpu");
    let version = service
        .resolve_workflow_graph_version("completion-test", "1.2.3", &graph)
        .unwrap();
    let mut snapshot = image_runtime_validation_snapshot(&version, &graph, MODEL, ARTIFACT);
    snapshot.nodes[0].constraints.requested_runtime_id = None;
    snapshot.nodes[0].constraints.requested_device_id = Some(DeviceIntentId::parse("cpu").unwrap());
    let mut planning = image_runtime_dependency_planning_request(
        &version,
        &snapshot.nodes[0].model_ref,
        snapshot.nodes[0].selected_binding_ids.clone(),
    );
    planning.scheduler_intent.requested_runtime_id = None;
    planning.scheduler_intent.requested_device_id = Some(DeviceIntentId::parse("cpu").unwrap());
    let proof = produce_dependency_requirements_proof(
        &ValidatedDependencyPlanningRequest::try_from(planning.clone()).unwrap(),
        None,
    )
    .unwrap();
    snapshot.nodes[0].dependency_requirements_id = proof.dependency_requirements_id.clone();
    snapshot.nodes[0].dependency_override_fingerprint = proof.dependency_override_fingerprint;
    service
        .store_workflow_executable_validation_snapshot(snapshot)
        .unwrap();
    let request = ValidatedDependencyEnvironmentRequest::try_from(DependencyEnvironmentRequest {
        contract_version: 1,
        action: DependencyEnvironmentAction::Resolve,
        identity_key: DependencyPlanningIdentityKey::from_planning_request(&planning).unwrap(),
        planning_request: planning,
        dependency_requirements_id: Some(proof.dependency_requirements_id),
        environment_ref: None,
    })
    .unwrap();
    readiness
        .insert_snapshot(
            DependencyEnvironmentReadinessSnapshot::for_request(
                &request,
                ready_image_dependency_environment_result(&request),
                DependencyEnvironmentReadinessSnapshotStatus::Fresh,
            )
            .unwrap(),
        )
        .unwrap();
    let host = Arc::new(ImageRuntimeSessionHost::new(graph));
    let session = service
        .create_workflow_execution_session(
            host.as_ref(),
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "completion-test".into(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .unwrap();
    if mode == 9 {
        *observed_host.cancellation_target.lock().unwrap() =
            Some((Arc::downgrade(&service), session.session_id.clone()));
    }
    let result = pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(service.clone(), host)
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id, workflow_semantic_version: "1.2.3".into(),
            inputs: vec![WorkflowPortBinding { node_id: "prompt".into(), port_id: "text".into(), value: serde_json::json!("paint a red cube") }],
            output_targets: Some(vec![WorkflowOutputTarget { node_id: "infer".into(), port_id: "image".into() }]),
            override_selection: None, timeout_ms: None, priority: None,
        }).await;
    if let Some(expected) = expected {
        let response = result.expect("qualified completion dispatch");
        assert_eq!(*observed_host.runtime_ids.lock().unwrap(), vec![expected]);
        let artifact = response.outputs[0].value["artifact_id"].as_str().unwrap();
        let body = service
            .read_artifact_body(ArtifactReadRequest {
                artifact_id: artifact.into(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .unwrap();
        assert_eq!(body.body, b"hello");
        assert_eq!(owner.queries.lock().unwrap().len(), 2);
    } else {
        assert!(result.is_err(), "unknown evidence must preserve ambiguity");
        if mode == 9 {
            assert_eq!(
                *observed_host.runtime_ids.lock().unwrap(),
                vec!["pytorch-alt"]
            );
        } else {
            assert!(observed_host.runtime_ids.lock().unwrap().is_empty());
        }
    }
    assert!(registry.snapshot().reservations.is_empty());
}

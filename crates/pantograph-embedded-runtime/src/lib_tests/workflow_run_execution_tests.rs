use super::*;
use crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort;
use crate::runtime_host_load_target::RuntimeHostPumasLoadTargetResolver;
use crate::runtime_host_media_artifact_sink::WorkflowServiceRuntimeHostMediaArtifactSink;
use crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsResolver;
use async_trait::async_trait;
use inference::types::{EncodedImage, ImageGenerationResult};
use inference::{BackendExecutionContext, ImageGenerationExecutionPlan};
use pantograph_dependency_environment_service::{
    DependencyEnvironmentReadinessSnapshot, DependencyEnvironmentReadinessSnapshotProvider,
    DependencyEnvironmentReadinessSnapshotStatus, DependencyReadinessWorkQueue,
};
use pantograph_dependency_planning::{
    produce_dependency_requirements_proof, DependencyBindingId, DependencyEnvironmentAction,
    DependencyEnvironmentId, DependencyEnvironmentInstallState, DependencyEnvironmentKind,
    DependencyEnvironmentReadinessState, DependencyEnvironmentRef, DependencyEnvironmentRequest,
    DependencyEnvironmentResult, DependencyEnvironmentValidationState, DependencyNodeTypeId,
    DependencyPlanningCallerContext, DependencyPlanningIdentityKey, DependencyPlanningRequest,
    DependencyReadinessProofEnvelope, DependencyRequirement, DependencyRequirementBinding,
    DependencyRequirementKind, DependencyRequirementName, DependencyTaskId, DeviceIntentId,
    PumasModelRef, PythonPackageManagerKind, PythonRequirementDetails, RuntimeIntentId,
    SchedulerIntent, ValidatedDependencyEnvironmentRequest, ValidatedDependencyPlanningRequest,
};
use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, DraftGraphValidationStatus, DraftGraphValidationSummary,
    InferenceAvailabilityStatus, InferenceInterfaceFingerprint, InferenceTaskKind,
    WorkflowGraphRevision, WorkflowNodeId, INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use pantograph_runtime_attribution::WorkflowVersionRecord;
use pantograph_runtime_host_contracts::{
    ReservationLifecycleApplication, ReservationLifecycleApplicationState,
    ReservationLifecycleEvent, ReservationLifecycleOutcome, ReservationLifecyclePort,
    ReservationLifecyclePortError, RESERVATION_LIFECYCLE_CONTRACT_VERSION,
    RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION,
};
use pantograph_scheduler::{
    SchedulerDispatchCandidate, SchedulerDispatchCandidateId, SchedulerEstimateHint,
    SchedulerEstimateHintKind, SchedulerReservationLeaseId, SchedulerResourceFitAssessment,
    SchedulerResourceFitState, SchedulerResourceKind, SchedulerResourceReservation,
    SchedulerTaskStateRecord,
};
use pantograph_workflow_service::workflow::{
    WorkflowRuntimeDispatchCandidateProvider, WorkflowRuntimeDispatchCandidateProviderError,
    WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchSourceRefreshError,
    WorkflowRuntimeDispatchSourceRefresher,
};
use pantograph_workflow_service::{
    ArtifactReadRequest, WorkflowArtifactWriter, WorkflowExecutableValidationSnapshotId,
    WorkflowExecutableValidationSnapshotNode, WorkflowExecutableValidationSnapshotRecord,
    WorkflowHostCapabilities, WorkflowIoNode, WorkflowIoPort, WorkflowIoResponse,
    WorkflowSchedulerTask, WorkflowSessionRuntimeLoadProofDiagnosticPhase,
    WorkflowSessionRuntimeLoadProofReadinessState,
    WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION,
};
use pumas_library::models::{
    AssetValidationState, BundleFormat, ImportState, ModelMetadata, StorageKind,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

async fn run_workflow_through_scheduler(
    runtime: &EmbeddedRuntime,
    workflow_id: &str,
    inputs: Vec<WorkflowPortBinding>,
    output_targets: Option<Vec<WorkflowOutputTarget>>,
) -> Result<WorkflowRunResponse, WorkflowServiceError> {
    run_workflow_through_scheduler_with_override(runtime, workflow_id, inputs, output_targets, None)
        .await
}

async fn run_workflow_through_scheduler_with_override(
    runtime: &EmbeddedRuntime,
    workflow_id: &str,
    inputs: Vec<WorkflowPortBinding>,
    output_targets: Option<Vec<WorkflowOutputTarget>>,
    override_selection: Option<WorkflowTechnicalFitOverride>,
) -> Result<WorkflowRunResponse, WorkflowServiceError> {
    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: workflow_id.to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await?;

    runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: created.session_id,
            workflow_semantic_version: "0.1.0".to_string(),
            inputs,
            output_targets,
            override_selection,
            timeout_ms: None,
            priority: None,
        })
        .await
}

#[tokio::test]
async fn workflow_execution_session_dispatches_through_production_embedded_image_runtime_host() {
    const MODEL_ID: &str = "image/example/tiny-diffusion";
    const SELECTED_ARTIFACT_ID: &str = "diffusers-bundle";

    let temp = TempDir::new().expect("temp dir");
    let artifact_writer = test_artifact_writer(&temp);
    let workflow_service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_artifact_writer(artifact_writer.clone());
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(TestRuntimeDispatchSourceRefresher::default());
    let reservation_lifecycle_port = Arc::new(TestReservationLifecyclePort::default());
    let pumas_root = temp.path().join("pumas");
    std::fs::create_dir_all(&pumas_root).expect("pumas root");
    let pumas_api = Arc::new(
        pumas_library::PumasApi::builder(pumas_root)
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .expect("pumas api"),
    );
    seed_diffusers_model(&pumas_api, MODEL_ID, SELECTED_ARTIFACT_ID).await;
    pumas_api
        .resolve_model_package_facts(MODEL_ID)
        .await
        .expect("seed package facts");
    let runtime_host_port = Arc::new(EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
        Arc::new(RuntimeHostPumasLoadTargetResolver::new(pumas_api.clone())),
        Arc::new(RuntimeHostPumasPackageFactsResolver::new(pumas_api)),
        Arc::new(WorkflowServiceRuntimeHostMediaArtifactSink::new(
            artifact_writer,
        )),
        Arc::new(inference::InferenceGateway::with_backend(
            Box::new(TestImageBackend),
            "PyTorch",
        )),
    ));
    let service = Arc::new(
        workflow_service
            .with_dependency_environment_provider(Arc::new(dependency_readiness_provider.clone()))
            .with_dependency_readiness_work_queue(dependency_readiness_work_queue.clone())
            .with_runtime_dispatch_source_refresher(source_refresher.clone())
            .with_runtime_dispatch_candidate_provider(Arc::new(
                TestRuntimeDispatchCandidateProvider,
            ))
            .with_runtime_host_execution_port(runtime_host_port)
            .with_reservation_lifecycle_port(reservation_lifecycle_port.clone()),
    );
    let workflow_id = "wf-production-embedded-image-runtime-host";
    let workflow_semantic_version = "1.2.3";
    let graph = image_runtime_session_graph(MODEL_ID, SELECTED_ARTIFACT_ID);
    let version = service
        .resolve_workflow_graph_version(workflow_id, workflow_semantic_version, &graph)
        .expect("resolve workflow version");
    service
        .store_workflow_executable_validation_snapshot(image_runtime_validation_snapshot(
            &version,
            &graph,
            MODEL_ID,
            SELECTED_ARTIFACT_ID,
        ))
        .expect("store validation snapshot");
    let dependency_request =
        image_runtime_dependency_environment_request(&version, MODEL_ID, SELECTED_ARTIFACT_ID);
    dependency_readiness_provider
        .insert_snapshot(
            DependencyEnvironmentReadinessSnapshot::for_request(
                &dependency_request,
                ready_image_dependency_environment_result(&dependency_request),
                DependencyEnvironmentReadinessSnapshotStatus::Fresh,
            )
            .expect("valid dependency readiness snapshot"),
        )
        .expect("insert readiness snapshot");

    let host = Arc::new(ImageRuntimeSessionHost::new(graph));
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
    let response = pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(
        service.clone(),
        host.clone(),
    )
        .run_workflow_execution_session(
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
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
        .expect("production embedded image runtime host should complete");

    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, "infer");
    assert_eq!(response.outputs[0].port_id, "image");
    let artifact_id = response.outputs[0]
        .value
        .get("artifact_id")
        .and_then(serde_json::Value::as_str)
        .expect("path-free artifact id output");
    assert!(!artifact_id.contains('/'));
    let body = service
        .read_artifact_body(ArtifactReadRequest {
            artifact_id: artifact_id.to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect("image artifact retained");
    assert_eq!(body.body, b"hello");
    assert_eq!(body.response.media_type, "image/png");
    assert_eq!(dependency_readiness_work_queue.len(), 1);
    assert_eq!(source_refresher.model_refs(), vec![MODEL_ID.to_string()]);
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
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

fn image_runtime_session_graph(model_id: &str, selected_artifact_id: &str) -> WorkflowGraph {
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
                    "inference_interface_snapshot": image_runtime_inference_interface_snapshot_json(),
                    "pumas_model_ref": {
                        "model_id": model_id,
                        "revision": "main",
                        "selected_artifact_id": selected_artifact_id
                    }
                }),
            },
        ],
        edges: vec![GraphEdge {
            id: "prompt-to-infer".to_string(),
            source: "prompt".to_string(),
            source_handle: "text".to_string(),
            target: "infer".to_string(),
            target_handle: "prompt".to_string(),
        }],
        derived_graph: None,
    }
}

fn image_runtime_inference_interface_snapshot_json() -> serde_json::Value {
    serde_json::json!({
        "contract_version": INFERENCE_INTERFACE_CONTRACT_VERSION,
        "descriptor_fingerprint": "embedded_runtime_descriptor_fingerprint_1",
        "task_kind": "image_generation",
        "inputs": [{
            "port_id": "prompt",
            "label": "Prompt",
            "direction": "input",
            "requirement": "required",
            "value_type": { "category": "scalar", "kind": "string" },
            "availability": { "status": "available" }
        }],
        "outputs": [{
            "port_id": "image",
            "label": "Image",
            "direction": "output",
            "requirement": "required",
            "value_type": { "category": "artifact", "kind": "image" },
            "availability": { "status": "available" }
        }]
    })
}

fn image_runtime_validation_snapshot(
    version: &WorkflowVersionRecord,
    graph: &WorkflowGraph,
    model_id: &str,
    selected_artifact_id: &str,
) -> WorkflowExecutableValidationSnapshotRecord {
    let model_ref = PumasModelRef {
        model_id: model_id.to_string(),
        revision: Some("main".to_string()),
        selected_artifact_id: Some(selected_artifact_id.to_string()),
        selected_artifact_path: None,
        migration_diagnostics: Vec::new(),
    };
    let selected_binding_ids =
        vec![DependencyBindingId::parse("torch-diffusers").expect("valid binding id")];
    let dependency_proof =
        image_runtime_dependency_requirements_proof(version, &model_ref, selected_binding_ids);
    WorkflowExecutableValidationSnapshotRecord {
        schema_version: WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION,
        validation_snapshot_id: WorkflowExecutableValidationSnapshotId::parse(
            "wfvalsnap_00000000-0000-4000-8000-000000000050",
        )
        .expect("valid snapshot id"),
        workflow_id: version.workflow_id.clone(),
        workflow_version_id: version.workflow_version_id.clone(),
        workflow_semantic_version: version.semantic_version.clone(),
        workflow_execution_fingerprint: version.execution_fingerprint.clone(),
        descriptor_contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
        graph_revision: WorkflowGraphRevision::parse(&graph.compute_fingerprint())
            .expect("valid graph revision"),
        validation_session_id: DraftGraphValidationSessionId::parse(
            "embedded_runtime_validation_session_1",
        )
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
                "embedded_runtime_descriptor_fingerprint_1",
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
            estimate_hints: vec![
                SchedulerEstimateHint {
                    kind: SchedulerEstimateHintKind::PeakRamBytes,
                    value: 2_147_483_648,
                },
                SchedulerEstimateHint {
                    kind: SchedulerEstimateHintKind::PeakVramBytes,
                    value: 4_294_967_296,
                },
            ],
            dependency_requirements_id: dependency_proof.dependency_requirements_id,
            selected_binding_ids: dependency_proof.identity_key.selected_binding_ids,
            dependency_override_fingerprint: dependency_proof.dependency_override_fingerprint,
            blocking_diagnostics: Vec::new(),
        }],
    }
}

fn image_runtime_dependency_requirements_proof(
    version: &WorkflowVersionRecord,
    model_ref: &PumasModelRef,
    selected_binding_ids: Vec<DependencyBindingId>,
) -> pantograph_dependency_planning::DependencyRequirementsProof {
    let request =
        image_runtime_dependency_planning_request(version, model_ref, selected_binding_ids);
    let validated_request =
        ValidatedDependencyPlanningRequest::try_from(request).expect("valid planning request");
    produce_dependency_requirements_proof(&validated_request, None)
        .expect("dependency requirements proof")
}

fn image_runtime_dependency_environment_request(
    version: &WorkflowVersionRecord,
    model_id: &str,
    selected_artifact_id: &str,
) -> ValidatedDependencyEnvironmentRequest {
    let model_ref = PumasModelRef {
        model_id: model_id.to_string(),
        revision: Some("main".to_string()),
        selected_artifact_id: Some(selected_artifact_id.to_string()),
        selected_artifact_path: None,
        migration_diagnostics: Vec::new(),
    };
    let selected_binding_ids =
        vec![DependencyBindingId::parse("torch-diffusers").expect("valid binding id")];
    let planning_request =
        image_runtime_dependency_planning_request(version, &model_ref, selected_binding_ids);
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

fn image_runtime_dependency_planning_request(
    version: &WorkflowVersionRecord,
    model_ref: &PumasModelRef,
    selected_binding_ids: Vec<DependencyBindingId>,
) -> DependencyPlanningRequest {
    DependencyPlanningRequest {
        model_ref: model_ref.clone(),
        task_id: DependencyTaskId::parse("image_generation").expect("valid task id"),
        task_type: Some(DependencyTaskId::parse("image_generation").expect("valid task type")),
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

fn ready_image_dependency_environment_result(
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
        requirements: vec![DependencyRequirement {
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
        }],
        bindings: request
            .identity_key
            .selected_binding_ids
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
            .collect(),
        selected_binding_ids: request.identity_key.selected_binding_ids.clone(),
        binding_statuses: Vec::new(),
        operation: None,
        validation_errors: Vec::new(),
        diagnostics: Vec::new(),
    }
}

#[derive(Default)]
struct TestReservationLifecyclePort {
    events: Mutex<Vec<ReservationLifecycleEvent>>,
}

impl TestReservationLifecyclePort {
    fn events(&self) -> Vec<ReservationLifecycleEvent> {
        self.events
            .lock()
            .expect("reservation lifecycle events lock")
            .clone()
    }
}

#[async_trait]
impl ReservationLifecyclePort for TestReservationLifecyclePort {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.events
            .lock()
            .expect("reservation lifecycle events lock")
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
struct TestRuntimeDispatchSourceRefresher {
    model_refs: Mutex<Vec<String>>,
}

impl TestRuntimeDispatchSourceRefresher {
    fn model_refs(&self) -> Vec<String> {
        self.model_refs
            .lock()
            .expect("runtime dispatch source refresh lock")
            .clone()
    }
}

#[async_trait]
impl WorkflowRuntimeDispatchSourceRefresher for TestRuntimeDispatchSourceRefresher {
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

struct TestRuntimeDispatchCandidateProvider;

impl WorkflowRuntimeDispatchCandidateProvider for TestRuntimeDispatchCandidateProvider {
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
                        "runtime scheduler task '{}' has no requested runtime id",
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
                        "runtime scheduler task '{}' has no requested device id",
                        task.task_id.as_str()
                    ),
                })?;
        Ok(WorkflowRuntimeDispatchCandidateSet {
            candidates: vec![SchedulerDispatchCandidate {
                candidate_id: SchedulerDispatchCandidateId::parse(
                    "candidate.embedded_runtime_session_test",
                )
                .map_err(|error| {
                    WorkflowRuntimeDispatchCandidateProviderError::Failed {
                        message: error.to_string(),
                    }
                })?,
                selected_runtime_id,
                selected_runtime_variant_id: None,
                selected_device_ids: vec![selected_device_id.clone()],
                selected_model_ref: intent.model_ref.clone(),
                runtime_trait_settings: Vec::new(),
                reservations: vec![SchedulerResourceReservation {
                    reservation_lease_id: SchedulerReservationLeaseId::parse(
                        "reservation.embedded_runtime_session_test",
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
                }],
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

struct ImageRuntimeSessionHost {
    graph: WorkflowGraph,
    runtime_load_attempts: Arc<AtomicUsize>,
    run_attempts: Arc<AtomicUsize>,
}

impl ImageRuntimeSessionHost {
    fn new(graph: WorkflowGraph) -> Self {
        Self {
            graph,
            runtime_load_attempts: Arc::new(AtomicUsize::new(0)),
            run_attempts: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl WorkflowHost for ImageRuntimeSessionHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok(self.graph.compute_fingerprint())
    }

    async fn workflow_graph(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        Ok(self.graph.clone())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(WorkflowHostCapabilities {
            max_input_bindings: 8,
            max_output_targets: 8,
            max_value_bytes: 1024 * 1024,
            runtime_requirements: WorkflowRuntimeRequirements {
                resource_estimates: Vec::new(),
                required_models: Vec::new(),
                required_backends: Vec::new(),
                required_extensions: Vec::new(),
            },
            models: Vec::new(),
            runtime_capabilities: Vec::new(),
        })
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

    async fn session_runtime_load_proof(
        &self,
        _session_id: &str,
        _workflow_id: &str,
    ) -> Result<Option<WorkflowSessionRuntimeLoadProof>, WorkflowServiceError> {
        Ok(Some(WorkflowSessionRuntimeLoadProof {
            contract_version: RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION,
            workflow_id: "wf-production-embedded-image-runtime-host".to_string(),
            task_id: Some("infer".to_string()),
            backend_key: "pytorch".to_string(),
            runtime_id: Some("pytorch".to_string()),
            model_id: Some("image/example/tiny-diffusion".to_string()),
            artifact_id: Some("diffusers-bundle".to_string()),
            load_target_id: None,
            readiness_state: WorkflowSessionRuntimeLoadProofReadinessState::Ready,
            diagnostic_phase: Some(
                WorkflowSessionRuntimeLoadProofDiagnosticPhase::RuntimeModelLoad,
            ),
            requested_model_active: true,
        }))
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: pantograph_workflow_service::WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        self.run_attempts.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }
}

struct TestImageBackend;

#[async_trait]
impl InferenceBackend for TestImageBackend {
    fn name(&self) -> &'static str {
        "Mock"
    }

    fn description(&self) -> &'static str {
        "Mock image backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            image_generation: true,
            ..BackendCapabilities::default()
        }
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("started_mock_runtime".to_string()),
        })
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<
        Pin<Box<dyn futures_util::Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
        BackendError,
    > {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }

    async fn generate_image_from_plan(
        &self,
        plan: ImageGenerationExecutionPlan,
        _context: BackendExecutionContext,
    ) -> Result<ImageGenerationResult, BackendError> {
        Ok(ImageGenerationResult {
            images: vec![EncodedImage {
                data_base64: "aGVsbG8=".to_string(),
                mime_type: "image/png".to_string(),
                width: plan.width,
                height: plan.height,
            }],
            seed_used: plan.seed,
            metadata: serde_json::Value::Null,
        })
    }
}

async fn seed_diffusers_model(
    pumas_api: &pumas_library::PumasApi,
    model_id: &str,
    selected_artifact_id: &str,
) {
    let library = pumas_api.model_library();
    let model_dir = library.build_model_path("image", "example", "tiny-diffusion");
    create_diffusers_bundle(&model_dir);
    let metadata = ModelMetadata {
        schema_version: Some(2),
        model_id: Some(model_id.to_string()),
        family: Some("stable-diffusion".to_string()),
        model_type: Some("diffusion".to_string()),
        official_name: Some("tiny-diffusion".to_string()),
        cleaned_name: Some("tiny-diffusion".to_string()),
        storage_kind: Some(StorageKind::LibraryOwned),
        bundle_format: Some(BundleFormat::DiffusersDirectory),
        pipeline_class: Some("StableDiffusionPipeline".to_string()),
        import_state: Some(ImportState::Ready),
        validation_state: Some(AssetValidationState::Valid),
        task_type_primary: Some("text-to-image".to_string()),
        input_modalities: Some(vec!["text".to_string()]),
        output_modalities: Some(vec!["image".to_string()]),
        recommended_backend: Some("diffusers".to_string()),
        runtime_engine_hints: Some(vec!["diffusers".to_string(), "pytorch".to_string()]),
        selected_artifact_id: Some(selected_artifact_id.to_string()),
        ..Default::default()
    };
    library
        .save_metadata(&model_dir, &metadata)
        .await
        .expect("save model metadata");
    library
        .index_model_dir(&model_dir)
        .await
        .expect("index model metadata");
}

fn create_diffusers_bundle(model_dir: &Path) {
    std::fs::create_dir_all(model_dir.join("unet")).expect("unet dir");
    std::fs::create_dir_all(model_dir.join("vae")).expect("vae dir");
    std::fs::create_dir_all(model_dir.join("scheduler")).expect("scheduler dir");
    std::fs::create_dir_all(model_dir.join("text_encoder")).expect("text encoder dir");
    std::fs::create_dir_all(model_dir.join("tokenizer")).expect("tokenizer dir");
    write_min_safetensors(&model_dir.join("unet/diffusion_pytorch_model.safetensors"));
    write_min_safetensors(&model_dir.join("vae/diffusion_pytorch_model.safetensors"));
    write_min_safetensors(&model_dir.join("text_encoder/model.safetensors"));
    std::fs::write(
        model_dir.join("unet/config.json"),
        r#"{"model_type":"unet"}"#,
    )
    .expect("unet config");
    std::fs::write(model_dir.join("vae/config.json"), r#"{"model_type":"vae"}"#)
        .expect("vae config");
    std::fs::write(
        model_dir.join("text_encoder/config.json"),
        r#"{"model_type":"clip_text_model"}"#,
    )
    .expect("text encoder config");
    std::fs::write(
        model_dir.join("scheduler/scheduler_config.json"),
        r#"{"scheduler":"euler"}"#,
    )
    .expect("scheduler config");
    std::fs::write(
        model_dir.join("tokenizer/tokenizer_config.json"),
        r#"{"model_type":"clip_tokenizer"}"#,
    )
    .expect("tokenizer config");
    std::fs::write(
        model_dir.join("tokenizer/tokenizer.json"),
        r#"{"tokenizer":"tiny-diffusion"}"#,
    )
    .expect("tokenizer");
    std::fs::write(
        model_dir.join("model_index.json"),
        r#"{
  "_class_name": "StableDiffusionPipeline",
  "scheduler": ["diffusers", "EulerDiscreteScheduler"],
  "unet": ["diffusers", "UNet2DConditionModel"],
  "vae": ["diffusers", "AutoencoderKL"],
  "text_encoder": ["transformers", "CLIPTextModel"],
  "tokenizer": ["transformers", "CLIPTokenizer"]
}"#,
    )
    .expect("model index");
}

fn write_min_safetensors(path: &Path) {
    let header = b"{}";
    let header_size = header.len() as u64;
    let mut content = header_size.to_le_bytes().to_vec();
    content.extend_from_slice(header);
    content.extend_from_slice(&[0; 64]);
    std::fs::write(path, content).expect("minimal safetensors fixture");
}

fn test_artifact_writer(temp: &TempDir) -> WorkflowArtifactWriter {
    let artifact_store = ArtifactStore::open(
        temp.path().join("artifacts"),
        test_runtime_artifact_policy(),
    )
    .expect("open artifact store");
    WorkflowArtifactWriter::new(artifact_store)
}

fn test_runtime_artifact_policy() -> ArtifactPolicy {
    ArtifactPolicy {
        policy_id: "embedded-runtime-image-session-test".to_string(),
        policy_version: 1,
        ttl_seconds: None,
        max_disk_bytes: None,
        max_memory_bytes: None,
        max_single_artifact_bytes: None,
        spill_threshold_bytes: None,
        delete_on_consume: false,
    }
}

#[tokio::test]
async fn test_runtime_run_and_session_execution() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let run_response = run_workflow_through_scheduler(
        &runtime,
        "runtime-text",
        vec![WorkflowPortBinding {
            node_id: "text-input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("hello"),
        }],
        Some(vec![WorkflowOutputTarget {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
        }]),
    )
    .await
    .expect("workflow run through scheduler");
    assert_eq!(run_response.outputs.len(), 1);
    assert_eq!(run_response.outputs[0].value, serde_json::json!("hello"));

    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await
        .expect("create session");

    let session_response = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: created.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("world"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect("run session");
    assert_eq!(session_response.outputs.len(), 1);
    assert_eq!(
        session_response.outputs[0].value,
        serde_json::json!("world")
    );

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: created.session_id,
        })
        .await
        .expect("close session");
}

#[tokio::test]
async fn scheduler_run_retains_detail_and_terminal_output_projection() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let workflow_service = workflow_service_with_artifact_store_and_ledger(&temp);
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        workflow_service.clone(),
        None,
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let response = run_workflow_through_scheduler(
        &runtime,
        "runtime-text",
        vec![WorkflowPortBinding {
            node_id: "text-input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("retained vertical text"),
        }],
        Some(vec![WorkflowOutputTarget {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
        }]),
    )
    .await
    .expect("workflow run through scheduler");
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(
        response.outputs[0].value,
        serde_json::json!("retained vertical text")
    );

    workflow_service
        .workflow_diagnostics_projection_refresh(
            pantograph_workflow_service::WorkflowDiagnosticsProjectionRefreshRequest {
                projections: vec![
                    pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::RunDetail,
                    pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::NodeStatus,
                    pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::IoArtifact,
                ],
                workflow_run_id: Some(response.workflow_run_id.clone()),
                workflow_id: Some("runtime-text".to_string()),
                reason: pantograph_workflow_service::WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
                batch_size: 50,
            },
        )
        .expect("refresh run detail projections");

    let detail = workflow_service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: response.workflow_run_id.clone(),
            projection_batch_size: Some(50),
        })
        .expect("run detail query");
    let run = detail.run.expect("run detail");
    assert_eq!(
        run.status,
        pantograph_workflow_service::RunListProjectionStatus::Completed
    );
    assert!(detail.node_statuses.is_empty());

    let artifacts = workflow_service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some(response.workflow_run_id.clone()),
            node_id: None,
            producer_node_id: None,
            consumer_node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            selected_backend_key: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(50),
            projection_batch_size: Some(50),
        })
        .expect("io artifact query")
        .artifacts;

    let workflow_input = artifacts
        .iter()
        .find(|artifact| {
            artifact.artifact_role == "workflow_input"
                && artifact.consumer_node_id.as_deref() == Some("text-input-1")
                && artifact.consumer_port_id.as_deref() == Some("text")
        })
        .expect("retained workflow input");
    assert_eq!(
        workflow_input.retention_state,
        pantograph_workflow_service::IoArtifactRetentionState::Retained
    );
    assert_eq!(
        workflow_service
            .read_artifact_body(pantograph_workflow_service::ArtifactReadRequest {
                artifact_id: workflow_input.artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("read retained workflow input")
            .body,
        b"retained vertical text"
    );

    let text_output_output = artifacts
        .iter()
        .find(|artifact| {
            artifact.artifact_role == "node_output"
                && artifact.producer_node_id.as_deref() == Some("text-output-1")
                && artifact.producer_port_id.as_deref() == Some("text")
        })
        .unwrap_or_else(|| {
            panic!(
                "retained text output node output; artifacts: {}",
                serde_json::to_string_pretty(&artifacts).expect("serialize artifacts")
            )
        });
    assert_eq!(
        workflow_service
            .read_artifact_body(pantograph_workflow_service::ArtifactReadRequest {
                artifact_id: text_output_output.artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("read retained text output output")
            .body,
        b"retained vertical text"
    );

    let workflow_output = artifacts
        .iter()
        .find(|artifact| {
            artifact.artifact_role == "workflow_output"
                && artifact.producer_node_id.as_deref() == Some("text-output-1")
                && artifact.producer_port_id.as_deref() == Some("text")
        })
        .expect("retained terminal workflow output");
    assert_eq!(
        workflow_service
            .read_artifact_body(pantograph_workflow_service::ArtifactReadRequest {
                artifact_id: workflow_output.artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("read retained workflow output")
            .body,
        b"retained vertical text"
    );
}

#[tokio::test]
async fn scheduler_session_event_sink_omits_legacy_task_completed_events() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));
    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await
        .expect("create session");
    let session_id = created.session_id.clone();
    let event_sink = Arc::new(node_engine::VecEventSink::new());

    let response = runtime
        .run_workflow_execution_session_with_event_sink(
            WorkflowExecutionSessionRunRequest {
                session_id: session_id.clone(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
            event_sink.clone(),
        )
        .await
        .expect("run session");
    assert_ne!(response.workflow_run_id, session_id);

    let events = event_sink.events();
    assert!(!events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::TaskCompleted { execution_id, .. }
            if execution_id == &session_id || execution_id == &response.workflow_run_id
    )));
}

#[tokio::test]
async fn embedded_workflow_host_run_workflow_returns_cancelled_for_precancelled_run_handle() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let run_handle = pantograph_workflow_service::WorkflowRunHandle::new();
    run_handle.cancel();

    let error = runtime
        .host()
        .run_workflow(
            "runtime-text",
            &[WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("hello"),
            }],
            Some(&[WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            WorkflowRunOptions {
                timeout_ms: None,
                workflow_execution_session_id: None,
                workflow_run_id: Some("pre-cancelled-run".to_string()),
            },
            run_handle,
        )
        .await
        .expect_err("pre-cancelled host run should return cancelled");

    match error {
        WorkflowServiceError::Cancelled(message) => {
            assert!(
                message.contains("cancelled before execution started"),
                "unexpected cancelled message: {message}"
            );
        }
        other => panic!("expected cancelled error, got {other:?}"),
    }
}

#[tokio::test]
async fn workflow_run_execution_session_rejects_human_input_workflow_without_execution_path() {
    let temp = TempDir::new().expect("temp dir");
    write_human_input_workflow(temp.path(), "interactive-human-input");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "interactive-human-input".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: false,
        })
        .await
        .expect("create interactive session");

    let error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: created.session_id,
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "human-input-1".to_string(),
                port_id: "value".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect_err(
            "interactive workflow execution session run should fail for non-streaming callers",
        );

    match error {
        WorkflowServiceError::CapabilityViolation(message) => {
            assert!(
                message.contains("unsupported=1") || message.contains("execution path"),
                "unexpected capability-violation message: {message}"
            );
        }
        other => panic!("expected scheduler execution-path rejection, got {other:?}"),
    }
}

#[tokio::test]
async fn retired_onnx_audio_workflow_fails_closed_before_python_adapter() {
    let temp = TempDir::new().expect("temp dir");
    write_mock_onnx_audio_workflow(temp.path(), "runtime-onnx-audio");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let python_runtime = Arc::new(MockMediaPythonRuntime {
        requests: Mutex::new(Vec::new()),
    });
    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        workflow_service_with_artifact_store(&temp),
        None,
        python_runtime.clone(),
    )
    .with_additional_runtime_capabilities(vec![onnx_python_sidecar_capability()])
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let error = run_workflow_through_scheduler_with_override(
        &runtime,
        "runtime-onnx-audio",
        vec![WorkflowPortBinding {
            node_id: "text-input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("a tiny painted robot"),
        }],
        Some(vec![WorkflowOutputTarget {
            node_id: "audio-output-1".to_string(),
            port_id: "audio".to_string(),
        }]),
        Some(WorkflowTechnicalFitOverride {
            runtime_id: None,
            runtime_variant_id: None,
            model_id: None,
            backend_key: Some("onnx-runtime".to_string()),
        }),
    )
    .await
    .expect_err("retired onnx workflow should fail stale graph validation");
    assert_retired_onnx_graph_rejected(&error);

    let requests = python_runtime.requests.lock().expect("requests lock");
    assert!(requests.is_empty());
}

#[tokio::test]
async fn retired_onnx_audio_workflow_with_gui_style_input_ids_fails_closed() {
    let temp = TempDir::new().expect("temp dir");
    write_mock_onnx_audio_workflow_with_prompt_node(
        temp.path(),
        "runtime-onnx-audio",
        "prompt-input",
    );

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let python_runtime = Arc::new(MockMediaPythonRuntime {
        requests: Mutex::new(Vec::new()),
    });
    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        workflow_service_with_artifact_store(&temp),
        None,
        python_runtime.clone(),
    )
    .with_additional_runtime_capabilities(vec![onnx_python_sidecar_capability()])
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let error = run_workflow_through_scheduler_with_override(
        &runtime,
        "runtime-onnx-audio",
        vec![WorkflowPortBinding {
            node_id: "prompt-input".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("a GUI style prompt node"),
        }],
        Some(vec![WorkflowOutputTarget {
            node_id: "audio-output-1".to_string(),
            port_id: "audio".to_string(),
        }]),
        Some(WorkflowTechnicalFitOverride {
            runtime_id: None,
            runtime_variant_id: None,
            model_id: None,
            backend_key: Some("onnx-runtime".to_string()),
        }),
    )
    .await
    .expect_err("retired onnx workflow should fail stale graph validation");
    assert_retired_onnx_graph_rejected(&error);

    let requests = python_runtime.requests.lock().expect("requests lock");
    assert!(requests.is_empty());
}

#[tokio::test]
async fn retired_onnx_audio_workflow_does_not_reconcile_python_sidecar_runtime() {
    let temp = TempDir::new().expect("temp dir");
    write_mock_onnx_audio_workflow(temp.path(), "runtime-onnx-audio");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        workflow_service_with_artifact_store(&temp),
        None,
        Arc::new(MockMediaPythonRuntime {
            requests: Mutex::new(Vec::new()),
        }),
    )
    .with_additional_runtime_capabilities(vec![onnx_python_sidecar_capability()])
    .with_runtime_registry(runtime_registry.clone());

    let error = run_workflow_through_scheduler_with_override(
        &runtime,
        "runtime-onnx-audio",
        vec![WorkflowPortBinding {
            node_id: "text-input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("a tiny painted robot"),
        }],
        Some(vec![WorkflowOutputTarget {
            node_id: "audio-output-1".to_string(),
            port_id: "audio".to_string(),
        }]),
        Some(WorkflowTechnicalFitOverride {
            runtime_id: None,
            runtime_variant_id: None,
            model_id: None,
            backend_key: Some("onnx-runtime".to_string()),
        }),
    )
    .await
    .expect_err("retired onnx workflow should fail stale graph validation");
    assert_retired_onnx_graph_rejected(&error);

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot
        .runtimes
        .iter()
        .all(|runtime| runtime.runtime_id != "onnx-runtime"));
}

fn assert_retired_onnx_graph_rejected(error: &WorkflowServiceError) {
    let stale_error = match error {
        WorkflowServiceError::WithDiagnostics { source, .. } => source.as_ref(),
        other => other,
    };
    let diagnostics = match stale_error {
        WorkflowServiceError::StaleWorkflowGraph {
            message,
            diagnostics,
        } => {
            assert!(
                message.contains("onnx-inference"),
                "stale graph message should identify retired onnx node type: {message}"
            );
            diagnostics
        }
        other => panic!("expected stale graph rejection, got {other:?}"),
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == pantograph_workflow_service::WorkflowGraphDiagnosticCode::UnknownNodeType
            && diagnostic.node_type.as_deref() == Some("onnx-inference")
    }));
}

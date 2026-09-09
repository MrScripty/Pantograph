use super::*;
use crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort;
use crate::runtime_host_load_target::RuntimeHostPumasLoadTargetResolver;
use crate::runtime_host_media_artifact_sink::WorkflowServiceRuntimeHostMediaArtifactSink;
use crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsResolver;
use async_trait::async_trait;
use inference::types::{EncodedImage, ImageGenerationResult};
use inference::{
    BackendExecutionContext, ImageGenerationBatchExecutionMemberResponse,
    ImageGenerationBatchExecutionRequest, ImageGenerationBatchExecutionResponse,
    ImageGenerationBatchExecutionState, ImageGenerationBatchMemberExecutionState,
    ImageGenerationExecutionPlan,
};
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
    SchedulerDispatchCandidateId, SchedulerEstimateHint, SchedulerEstimateHintKind,
    SchedulerReservationLeaseId, SchedulerResourceFitAssessment, SchedulerResourceFitState,
    SchedulerResourceKind, SchedulerResourceReservation, SchedulerTaskStateRecord,
};
use pantograph_workflow_service::workflow::{
    ValidatedWorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateFact,
    WorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateProvider,
    WorkflowRuntimeDispatchCandidateProviderError, WorkflowRuntimeDispatchCandidateSet,
    WorkflowRuntimeDispatchLoadState, WorkflowRuntimeDispatchSourceRefreshError,
    WorkflowRuntimeDispatchSourceRefresher,
    WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
            .with_runtime_host_execution_port(runtime_host_port.clone())
            .with_runtime_host_batch_execution_port(runtime_host_port)
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

fn text_runtime_inference_interface_snapshot_json() -> serde_json::Value {
    let mut interface = image_runtime_inference_interface_snapshot_json();
    interface["task_kind"] = serde_json::json!("text_generation");
    interface["outputs"][0]["port_id"] = serde_json::json!("text");
    interface["outputs"][0]["value_type"] =
        serde_json::json!({"category": "scalar", "kind": "string"});
    interface
}

fn make_text_runtime_inference_node(node: &mut GraphNode) {
    node.data["task_kind"] = serde_json::json!("text_generation");
    node.data["device"] = serde_json::json!("cpu");
    node.data["inference_interface_snapshot"] = text_runtime_inference_interface_snapshot_json();
}

fn dependent_text_image_session_graph(
    text_model_id: &str,
    text_selected_artifact_id: &str,
    image_model_id: &str,
    image_selected_artifact_id: &str,
) -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({}),
            },
            GraphNode {
                id: "text-infer".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: serde_json::json!({
                    "task_kind": "text_generation",
                    "runtime": "pytorch",
                    "device": "cpu",
                    "inference_interface_snapshot": text_runtime_inference_interface_snapshot_json(),
                    "pumas_model_ref": {
                        "model_id": text_model_id,
                        "revision": "main",
                        "selected_artifact_id": text_selected_artifact_id
                    }
                }),
            },
            GraphNode {
                id: "image-infer".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 400.0, y: 0.0 },
                data: serde_json::json!({
                    "task_kind": "image_generation",
                    "runtime": "pytorch",
                    "device": "cuda:0",
                    "inference_interface_snapshot": image_runtime_inference_interface_snapshot_json(),
                    "pumas_model_ref": {
                        "model_id": image_model_id,
                        "revision": "main",
                        "selected_artifact_id": image_selected_artifact_id
                    }
                }),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "prompt-to-text".to_string(),
                source: "prompt".to_string(),
                source_handle: "text".to_string(),
                target: "text-infer".to_string(),
                target_handle: "prompt".to_string(),
            },
            GraphEdge {
                id: "generated-text-to-image".to_string(),
                source: "text-infer".to_string(),
                source_handle: "text".to_string(),
                target: "image-infer".to_string(),
                target_handle: "prompt".to_string(),
            },
        ],
        derived_graph: None,
    }
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

fn runtime_source_context() -> pantograph_workflow_service::graph::WorkflowRuntimeSourceContext {
    pantograph_workflow_service::graph::WorkflowRuntimeSourceContext {
        operation_type: "image-generation.txt2img".to_string(),
        context_shape_key: "txt2img.1024x1024.steps30".to_string(),
        cancellation_mode: "per-run-fanout".to_string(),
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

fn runtime_dependency_environment_request(
    version: &WorkflowVersionRecord,
    node_id: &str,
    task_type: &str,
    model_ref: &PumasModelRef,
    binding_id: &str,
    device_id: &str,
) -> ValidatedDependencyEnvironmentRequest {
    let mut planning = image_runtime_dependency_planning_request(
        version,
        model_ref,
        vec![DependencyBindingId::parse(binding_id).expect("valid dependency binding id")],
    );
    planning.task_id = DependencyTaskId::parse(task_type).expect("valid dependency task id");
    planning.task_type = Some(planning.task_id.clone());
    planning.scheduler_intent.requested_device_id =
        Some(DeviceIntentId::parse(device_id).expect("valid dependency device id"));
    planning.caller_context.node_id = Some(node_id.to_string());
    let validated_planning = ValidatedDependencyPlanningRequest::try_from(planning.clone())
        .expect("valid planning request");
    let dependency_proof = produce_dependency_requirements_proof(&validated_planning, None)
        .expect("dependency requirements proof");
    ValidatedDependencyEnvironmentRequest::try_from(DependencyEnvironmentRequest {
        contract_version: 1,
        action: DependencyEnvironmentAction::Resolve,
        identity_key: dependency_proof.identity_key,
        planning_request: planning,
        dependency_requirements_id: Some(dependency_proof.dependency_requirements_id),
        environment_ref: None,
    })
    .expect("valid dependency environment request")
}

fn ready_dependency_environment_result(
    request: &ValidatedDependencyEnvironmentRequest,
    requirement_name: &str,
) -> DependencyEnvironmentResult {
    let mut result = ready_image_dependency_environment_result(request);
    let requirement_name = DependencyRequirementName::parse(requirement_name)
        .expect("valid dependency requirement name");
    result.requirements[0].name = requirement_name.clone();
    result.requirements[0].python.as_mut().unwrap().import_name =
        Some(requirement_name.as_str().to_string());
    for binding in &mut result.bindings {
        binding.requirement_name = requirement_name.clone();
    }
    result
}

fn runtime_validation_snapshot_node(
    template: &WorkflowExecutableValidationSnapshotNode,
    version: &WorkflowVersionRecord,
    node_id: &str,
    task_type: &str,
    model_ref: &PumasModelRef,
    binding_id: &str,
    device_id: &str,
) -> (
    WorkflowExecutableValidationSnapshotNode,
    ValidatedDependencyEnvironmentRequest,
) {
    let dependency_request = runtime_dependency_environment_request(
        version, node_id, task_type, model_ref, binding_id, device_id,
    );
    let planning = dependency_request.as_request().planning_request.clone();
    let dependency_proof = produce_dependency_requirements_proof(
        &ValidatedDependencyPlanningRequest::try_from(planning).expect("valid planning request"),
        None,
    )
    .expect("dependency requirements proof");
    let mut node = template.clone();
    node.node_id = WorkflowNodeId::parse(node_id).expect("valid validation node id");
    node.task_kind = InferenceTaskKind::parse(task_type).expect("valid validation task kind");
    node.model_ref = model_ref.clone();
    node.constraints.requested_runtime_id =
        Some(RuntimeIntentId::parse("pytorch").expect("valid runtime id"));
    node.constraints.requested_device_id =
        Some(DeviceIntentId::parse(device_id).expect("valid device id"));
    node.dependency_requirements_id = dependency_proof.dependency_requirements_id;
    node.selected_binding_ids = dependency_proof.identity_key.selected_binding_ids;
    node.dependency_override_fingerprint = dependency_proof.dependency_override_fingerprint;
    (node, dependency_request)
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
        let environment_ref = readiness_proof
            .preflight_result
            .environment_ref
            .clone()
            .ok_or_else(|| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: format!(
                    "runtime scheduler task '{}' has no environment ref",
                    task.task_id.as_str()
                ),
            })?;
        let reservation = SchedulerResourceReservation {
            reservation_lease_id: SchedulerReservationLeaseId::parse(format!(
                "reservation.embedded_runtime_session_test.{}",
                intent.node_id.as_str()
            ))
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
            candidate_id: SchedulerDispatchCandidateId::parse(format!(
                "candidate.embedded_runtime_session_test.{}",
                intent.node_id.as_str()
            ))
            .map_err(|error| {
                WorkflowRuntimeDispatchCandidateProviderError::Failed {
                    message: error.to_string(),
                }
            })?,
            selected_runtime_id,
            selected_runtime_variant_id: Some(
                if intent.task_type.as_str() == "text_generation" {
                    "pytorch.cpu"
                } else {
                    "pytorch.diffusers"
                }
                .parse()
                .unwrap(),
            ),
            selected_backend_key: "pytorch".to_string(),
            runtime_family: "test-runtime".to_string(),
            resolved_load_target: format!("test:{}", intent.model_ref.model_id),
            runtime_residency_key: format!("test-runtime:{}", intent.model_ref.model_id),
            loaded_runtime_memory_estimate_bytes: 1,
            runtime_load_state: WorkflowRuntimeDispatchLoadState::Loaded,
            runtime_instance_id: Some(format!(
                "runtime.embedded-session-test.{}.001",
                intent.node_id.as_str()
            )),
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
            outputs: self
                .graph
                .nodes
                .iter()
                .filter(|node| node.node_type == "llm-inference")
                .map(|node| {
                    let text = node.data["task_kind"] == "text_generation";
                    WorkflowIoNode {
                        node_id: node.id.clone(),
                        node_type: node.node_type.clone(),
                        name: None,
                        description: None,
                        ports: vec![WorkflowIoPort {
                            port_id: if text { "text" } else { "image" }.into(),
                            name: None,
                            description: None,
                            data_type: Some(
                                if text { "string" } else { "media_artifact_ref" }.into(),
                            ),
                            required: Some(false),
                            multiple: Some(false),
                        }],
                    }
                })
                .collect(),
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
            image_generation_batch: true,
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

    async fn stop(&mut self) -> Result<(), BackendError> {
        Ok(())
    }

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

    async fn generate_image_batch_from_execution_request(
        &self,
        request: ImageGenerationBatchExecutionRequest,
        _context: BackendExecutionContext,
    ) -> Result<ImageGenerationBatchExecutionResponse, BackendError> {
        let members = request
            .members
            .into_iter()
            .map(|member| ImageGenerationBatchExecutionMemberResponse {
                member_id: member.member_id,
                state: ImageGenerationBatchMemberExecutionState::Completed,
                result: Some(ImageGenerationResult {
                    images: vec![EncodedImage {
                        data_base64: "aGVsbG8=".to_string(),
                        mime_type: "image/png".to_string(),
                        width: member.plan.width,
                        height: member.plan.height,
                    }],
                    seed_used: member.plan.seed,
                    metadata: serde_json::Value::Null,
                }),
                diagnostics: Vec::new(),
            })
            .collect();
        Ok(ImageGenerationBatchExecutionResponse {
            batch_execution_id: request.batch_execution_id,
            state: ImageGenerationBatchExecutionState::Completed,
            members,
            diagnostics: Vec::new(),
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

#[tokio::test]
async fn selected_text_workflow_retains_completed_output_through_canonical_batch_host() {
    const MODEL_ID: &str = "llm/example/tiny-transformers";
    const SELECTED_ARTIFACT_ID: &str = "text-bundle";

    let temp = TempDir::new().expect("temp dir");
    let artifact_writer = test_artifact_writer(&temp);
    let workflow_service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_artifact_writer(artifact_writer.clone())
        .with_diagnostics_ledger(
            pantograph_workflow_service::SqliteDiagnosticsLedger::open_in_memory().unwrap(),
        );
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(TestRuntimeDispatchSourceRefresher::default());
    let reservation_lifecycle_port = Arc::new(TestReservationLifecyclePort::default());
    let target_directory = temp.path().join("selected-model");
    std::fs::create_dir_all(&target_directory).unwrap();
    let resolver = Arc::new(SelectedTextResolver(target_directory));
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let runtime_host_port = Arc::new(EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
        resolver.clone(),
        resolver,
        Arc::new(WorkflowServiceRuntimeHostMediaArtifactSink::new(
            artifact_writer,
        )),
        Arc::new(inference::InferenceGateway::with_backend(
            Box::new(SelectedWorkflowTextBackend(prompts.clone())),
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
            .with_runtime_host_execution_port(runtime_host_port.clone())
            .with_runtime_host_batch_execution_port(runtime_host_port)
            .with_reservation_lifecycle_port(reservation_lifecycle_port.clone()),
    );
    let workflow_id = "wf-selected-text-runtime-host";
    let workflow_semantic_version = "1.2.3";
    let mut graph = image_runtime_session_graph(MODEL_ID, SELECTED_ARTIFACT_ID);
    let infer = &mut graph.nodes[1];
    infer.data["task_kind"] = serde_json::json!("text_generation");
    infer.data["device"] = serde_json::json!("cpu");
    let mut interface = image_runtime_inference_interface_snapshot_json();
    interface["task_kind"] = serde_json::json!("text_generation");
    interface["outputs"][0]["port_id"] = serde_json::json!("text");
    interface["outputs"][0]["value_type"] =
        serde_json::json!({"category": "scalar", "kind": "string"});
    infer.data["inference_interface_snapshot"] = interface;
    let version = service
        .resolve_workflow_graph_version(workflow_id, workflow_semantic_version, &graph)
        .unwrap();
    let mut snapshot =
        image_runtime_validation_snapshot(&version, &graph, MODEL_ID, SELECTED_ARTIFACT_ID);
    let template = snapshot.nodes[0].clone();
    snapshot.nodes.clear();
    {
        let node_id = "infer";
        let mut node = template.clone();
        node.node_id = WorkflowNodeId::parse(node_id).unwrap();
        node.task_kind = InferenceTaskKind::parse("text_generation").unwrap();
        node.constraints.requested_device_id = Some(DeviceIntentId::parse("cpu").unwrap());
        let mut planning = image_runtime_dependency_planning_request(
            &version,
            &node.model_ref,
            vec![DependencyBindingId::parse("torch-transformers").unwrap()],
        );
        planning.task_id = DependencyTaskId::parse("text_generation").unwrap();
        planning.task_type = Some(planning.task_id.clone());
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
        let dependency_request =
            ValidatedDependencyEnvironmentRequest::try_from(DependencyEnvironmentRequest {
                contract_version: 1,
                action: DependencyEnvironmentAction::Resolve,
                identity_key: proof.identity_key,
                planning_request: planning,
                dependency_requirements_id: Some(proof.dependency_requirements_id),
                environment_ref: None,
            })
            .unwrap();
        let mut result = ready_image_dependency_environment_result(&dependency_request);
        result.requirements[0].name = DependencyRequirementName::parse("transformers").unwrap();
        result.requirements[0].python.as_mut().unwrap().import_name = Some("transformers".into());
        for binding in &mut result.bindings {
            binding.requirement_name = DependencyRequirementName::parse("transformers").unwrap();
        }
        dependency_readiness_provider
            .insert_snapshot(
                DependencyEnvironmentReadinessSnapshot::for_request(
                    &dependency_request,
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
                    value: serde_json::json!("  paint a red cube\n"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "infer".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("production embedded image runtime host should complete");

    assert_eq!(*prompts.lock().unwrap(), vec!["  paint a red cube\n"]);
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, "infer");
    assert_eq!(response.outputs[0].port_id, "text");
    assert_eq!(
        response.outputs[0].value,
        serde_json::json!("expanded:  paint a red cube\n")
    );
    service.workflow_diagnostics_projection_refresh(pantograph_workflow_service::WorkflowDiagnosticsProjectionRefreshRequest {
        projections: vec![pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::RunDetail, pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::IoArtifact],
        workflow_run_id: Some(response.workflow_run_id.clone()), workflow_id: Some(workflow_id.into()),
        reason: pantograph_workflow_service::WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh, batch_size: 50,
    }).unwrap();
    let detail = service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: response.workflow_run_id.clone(),
            projection_batch_size: Some(50),
        })
        .unwrap();
    assert_eq!(
        detail.run.unwrap().status,
        pantograph_workflow_service::RunListProjectionStatus::Completed
    );
    let artifacts = service
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
        .unwrap()
        .artifacts;
    {
        let (node, expected) = ("infer", "expanded:  paint a red cube\n");
        let artifact = artifacts
            .iter()
            .find(|artifact| {
                artifact.producer_node_id.as_deref() == Some(node)
                    && artifact.producer_port_id.as_deref() == Some("text")
            })
            .expect("retained generated node text");
        assert_eq!(
            artifact.retention_state,
            pantograph_workflow_service::IoArtifactRetentionState::Retained
        );
        assert_eq!(
            service
                .read_artifact_body(ArtifactReadRequest {
                    artifact_id: artifact.artifact_id.clone(),
                    byte_range_start: None,
                    byte_range_end_exclusive: None
                })
                .unwrap()
                .body,
            expected.as_bytes()
        );
    }
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn selected_text_workflow_retains_outputs_and_materializes_dependent_edge() {
    const MODEL_ID: &str = "llm/example/tiny-transformers";
    const SELECTED_ARTIFACT_ID: &str = "text-bundle";

    let temp = TempDir::new().expect("temp dir");
    let artifact_writer = test_artifact_writer(&temp);
    let workflow_service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_artifact_writer(artifact_writer.clone())
        .with_diagnostics_ledger(
            pantograph_workflow_service::SqliteDiagnosticsLedger::open_in_memory().unwrap(),
        );
    let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let dependency_readiness_work_queue = Arc::new(DependencyReadinessWorkQueue::new());
    let source_refresher = Arc::new(TestRuntimeDispatchSourceRefresher::default());
    let reservation_lifecycle_port = Arc::new(TestReservationLifecyclePort::default());
    let target_directory = temp.path().join("selected-model");
    std::fs::create_dir_all(&target_directory).unwrap();
    let resolver = Arc::new(SelectedTextResolver(target_directory));
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let runtime_host_port = Arc::new(EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
        resolver.clone(),
        resolver,
        Arc::new(WorkflowServiceRuntimeHostMediaArtifactSink::new(
            artifact_writer,
        )),
        Arc::new(inference::InferenceGateway::with_backend(
            Box::new(SelectedWorkflowTextBackend(prompts.clone())),
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
            .with_runtime_host_execution_port(runtime_host_port.clone())
            .with_runtime_host_batch_execution_port(runtime_host_port)
            .with_reservation_lifecycle_port(reservation_lifecycle_port.clone()),
    );
    let workflow_id = "wf-selected-text-dependent-runtime-host";
    let workflow_semantic_version = "1.2.3";
    let mut graph = image_runtime_session_graph(MODEL_ID, SELECTED_ARTIFACT_ID);
    make_text_runtime_inference_node(&mut graph.nodes[1]);
    let mut dependent = graph.nodes[1].clone();
    dependent.id = "infer-next".into();
    graph.nodes.push(dependent);
    graph.edges.push(GraphEdge {
        id: "generated-text-edge".into(),
        source: "infer".into(),
        source_handle: "text".into(),
        target: "infer-next".into(),
        target_handle: "prompt".into(),
    });
    let version = service
        .resolve_workflow_graph_version(workflow_id, workflow_semantic_version, &graph)
        .unwrap();
    let template =
        image_runtime_validation_snapshot(&version, &graph, MODEL_ID, SELECTED_ARTIFACT_ID).nodes
            [0]
        .clone();
    let model_ref = template.model_ref.clone();
    let mut snapshot =
        image_runtime_validation_snapshot(&version, &graph, MODEL_ID, SELECTED_ARTIFACT_ID);
    snapshot.nodes.clear();
    for node_id in ["infer", "infer-next"] {
        let (node, dependency_request) = runtime_validation_snapshot_node(
            &template,
            &version,
            node_id,
            "text_generation",
            &model_ref,
            "torch-transformers",
            "cpu",
        );
        dependency_readiness_provider
            .insert_snapshot(
                DependencyEnvironmentReadinessSnapshot::for_request(
                    &dependency_request,
                    ready_dependency_environment_result(&dependency_request, "transformers"),
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
    .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
        session_id: created.session_id,
        workflow_semantic_version: workflow_semantic_version.to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("  paint a red cube\n"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "infer-next".to_string(),
            port_id: "text".to_string(),
        }, WorkflowOutputTarget { node_id: "infer".to_string(), port_id: "text".to_string() }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    })
    .await
    .expect("dependent text workflow should complete");

    assert_eq!(
        *prompts.lock().unwrap(),
        vec![
            "  paint a red cube\n".to_string(),
            "expanded:  paint a red cube\n".to_string()
        ]
    );
    assert_eq!(response.outputs.len(), 2);
    assert_eq!(response.outputs[0].node_id, "infer-next");
    assert_eq!(response.outputs[0].port_id, "text");
    assert_eq!(
        response.outputs[0].value,
        serde_json::json!("expanded:expanded:  paint a red cube\n")
    );

    service
        .workflow_diagnostics_projection_refresh(
            pantograph_workflow_service::WorkflowDiagnosticsProjectionRefreshRequest {
                projections: vec![
                    pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::RunDetail,
                    pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::NodeStatus,
                    pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::IoArtifact,
                ],
                workflow_run_id: Some(response.workflow_run_id.clone()),
                workflow_id: Some(workflow_id.to_string()),
                reason: pantograph_workflow_service::WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
                batch_size: 50,
            },
        )
        .unwrap();
    let detail = service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: response.workflow_run_id.clone(),
            projection_batch_size: Some(50),
        })
        .unwrap();
    assert_eq!(
        detail.run.unwrap().status,
        pantograph_workflow_service::RunListProjectionStatus::Completed
    );
    let artifacts = service
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
        .unwrap()
        .artifacts;
    for (node, expected) in [
        ("infer", "expanded:  paint a red cube\n"),
        ("infer-next", "expanded:expanded:  paint a red cube\n"),
    ] {
        let artifact = artifacts
            .iter()
            .find(|artifact| {
                artifact.producer_node_id.as_deref() == Some(node)
                    && artifact.producer_port_id.as_deref() == Some("text")
            })
            .unwrap_or_else(|| panic!("retained generated node text for {node}"));
        assert_eq!(
            artifact.retention_state,
            pantograph_workflow_service::IoArtifactRetentionState::Retained
        );
        assert_eq!(
            service
                .read_artifact_body(ArtifactReadRequest {
                    artifact_id: artifact.artifact_id.clone(),
                    byte_range_start: None,
                    byte_range_end_exclusive: None,
                })
                .unwrap()
                .body,
            expected.as_bytes()
        );
    }
    assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn dependent_text_to_image_workflow_forwards_exact_text_and_retains_outputs() {
    let fixture = DependentTextImageFixture::new(false).await;
    let prompt = "  paint a red cube\n";
    let response = fixture
        .run(prompt)
        .await
        .expect("dependent text-to-image workflow should complete");

    assert_eq!(
        fixture.text_prompts.lock().unwrap().as_slice(),
        [prompt.to_string()]
    );
    assert_eq!(
        fixture.image_prompts.lock().unwrap().as_slice(),
        [format!("expanded:{prompt}")]
    );
    assert_eq!(response.outputs.len(), 2);
    assert_eq!(response.outputs[0].node_id, "image-infer");
    assert_eq!(response.outputs[0].port_id, "image");
    let image_artifact_id = response.outputs[0]
        .value
        .get("artifact_id")
        .and_then(serde_json::Value::as_str)
        .expect("image output should be a path-free artifact reference");
    assert!(!image_artifact_id.contains('/'));
    let expected_image = valid_test_image_bytes();
    let image_body = fixture
        .service
        .read_artifact_body(ArtifactReadRequest {
            artifact_id: image_artifact_id.to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect("retained image output");
    assert_eq!(image_body.body, expected_image);
    assert_eq!(image_body.response.media_type, "image/png");

    fixture
        .service
        .workflow_diagnostics_projection_refresh(
            pantograph_workflow_service::WorkflowDiagnosticsProjectionRefreshRequest {
                projections: vec![
                    pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::RunDetail,
                    pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::SchedulerTimeline,
                    pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::IoArtifact,
                ],
                workflow_run_id: Some(response.workflow_run_id.clone()),
                workflow_id: Some(fixture.workflow_id.clone()),
                reason: pantograph_workflow_service::WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
                batch_size: 50,
            },
        )
        .unwrap();
    let detail = fixture
        .service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: response.workflow_run_id.clone(),
            projection_batch_size: Some(50),
        })
        .unwrap();
    assert_eq!(
        detail.run.as_ref().unwrap().status,
        pantograph_workflow_service::RunListProjectionStatus::Completed
    );
    let timeline = fixture
        .service
        .workflow_scheduler_timeline_query(
            pantograph_workflow_service::WorkflowSchedulerTimelineQueryRequest {
                workflow_run_id: Some(response.workflow_run_id.clone()),
                limit: Some(50),
                ..Default::default()
            },
        )
        .expect("canonical scheduler attempt timeline");
    let completed = timeline
        .events
        .iter()
        .filter(|event| {
            event.scheduler_attempt_transition ==
        Some(pantograph_diagnostics_ledger::SchedulerTaskAttemptLifecycleTransition::Completed)
        })
        .collect::<Vec<_>>();
    let text_status = completed
        .iter()
        .find(|event| event.scheduler_task_id.as_deref() == Some("text-infer"))
        .expect("completed text producer attempt");
    let image_status = completed
        .iter()
        .find(|event| event.scheduler_task_id.as_deref() == Some("image-infer"))
        .expect("completed image consumer attempt");
    assert_eq!(
        text_status.workflow_run_id.as_str(),
        response.workflow_run_id
    );
    assert_eq!(image_status.workflow_run_id, text_status.workflow_run_id);
    assert_ne!(
        text_status.scheduler_task_id,
        image_status.scheduler_task_id
    );
    assert_ne!(
        text_status.scheduler_attempt_id,
        image_status.scheduler_attempt_id
    );

    let artifacts = fixture
        .service
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
        .unwrap()
        .artifacts;
    let generated_text = format!("expanded:{prompt}");
    let text_artifact = artifacts
        .iter()
        .find(|artifact| {
            artifact.producer_node_id.as_deref() == Some("text-infer")
                && artifact.producer_port_id.as_deref() == Some("text")
        })
        .unwrap_or_else(|| panic!("retained text producer artifact: {artifacts:?}"));
    assert_eq!(
        text_artifact.retention_state,
        pantograph_workflow_service::IoArtifactRetentionState::Retained
    );
    assert_eq!(
        fixture
            .service
            .read_artifact_body(ArtifactReadRequest {
                artifact_id: text_artifact.artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .unwrap()
            .body,
        generated_text.as_bytes()
    );
    let retained_image_reference = artifacts
        .iter()
        .find(|artifact| {
            artifact.producer_node_id.as_deref() == Some("image-infer")
                && artifact.producer_port_id.as_deref() == Some("image")
                && artifact.media_type.as_deref() == Some("application/json")
        })
        .unwrap_or_else(|| panic!("retained image output reference: {artifacts:?}"));
    assert_eq!(
        retained_image_reference.retention_state,
        pantograph_workflow_service::IoArtifactRetentionState::Retained
    );
    let reference_body = fixture
        .service
        .read_artifact_body(ArtifactReadRequest {
            artifact_id: retained_image_reference.artifact_id.clone(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect("retained image output reference body");
    let reference: serde_json::Value =
        serde_json::from_slice(&reference_body.body).expect("image artifact reference JSON");
    assert_eq!(reference, response.outputs[0].value);
    assert_eq!(reference["artifact_id"].as_str(), Some(image_artifact_id));
}

#[tokio::test]
async fn dependent_text_to_image_workflow_does_not_run_downstream_after_producer_failure() {
    let fixture = DependentTextImageFixture::new(true).await;
    let error = fixture
        .run("a producer failure must not reach image generation")
        .await
        .expect_err("producer failure should fail the composed workflow");

    assert_eq!(
        fixture.text_prompts.lock().unwrap().as_slice(),
        ["a producer failure must not reach image generation".to_string()]
    );
    assert!(
        fixture.image_prompts.lock().unwrap().is_empty(),
        "downstream image generation must not run after producer failure"
    );
    assert_eq!(
        error.code(),
        pantograph_workflow_service::WorkflowErrorCode::RuntimeNotReady
    );
    assert_eq!(
        fixture.text_failures.lock().unwrap().as_slice(),
        ["controlled producer failure"]
    );
    let plan = fixture
        .service
        .workflow_execution_session_bootstrap_recovery_plan()
        .expect("retryable run plan");
    let producer = plan
        .decisions
        .iter()
        .find(|decision| decision.task_id == "text-infer")
        .expect("producer task state");
    assert_eq!(
        producer.state_kind,
        Some(pantograph_scheduler::SchedulerTaskStateKind::RetryableFailed)
    );
    let downstream = plan
        .decisions
        .iter()
        .find(|decision| decision.task_id == "image-infer")
        .expect("downstream task state");
    assert_eq!(
        downstream.state_kind,
        Some(pantograph_scheduler::SchedulerTaskStateKind::AwaitingInputs)
    );
    assert_eq!(producer.workflow_run_id, downstream.workflow_run_id);
    assert!(!producer.workflow_run_id.is_empty());
}

#[tokio::test]
async fn dependent_text_to_image_keeps_original_response_pending_until_downstream_finishes() {
    let fixture = DependentTextImageFixture::new(false).await;
    fixture.image_gate.enabled.store(true, Ordering::SeqCst);
    let mut run = Box::pin(fixture.run("hold downstream completion"));
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        tokio::select! {
            () = fixture.image_gate.entered.notified() => {},
            result = &mut run => panic!("run completed before downstream gate: {result:?}"),
        }
    })
    .await
    .expect("downstream execution reaches gate");
    assert_eq!(
        fixture.text_prompts.lock().unwrap().as_slice(),
        ["hold downstream completion"]
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut run)
            .await
            .is_err(),
        "the original caller must remain pending after producer success"
    );
    let plan = fixture
        .service
        .workflow_execution_session_bootstrap_recovery_plan()
        .expect("in-flight run plan");
    assert_eq!(
        plan.decisions
            .iter()
            .find(|task| task.task_id == "text-infer")
            .expect("producer")
            .state_kind,
        Some(pantograph_scheduler::SchedulerTaskStateKind::Completed)
    );
    assert_eq!(
        plan.decisions
            .iter()
            .find(|task| task.task_id == "image-infer")
            .expect("downstream")
            .state_kind,
        Some(pantograph_scheduler::SchedulerTaskStateKind::Running)
    );
    fixture.image_gate.release.notify_one();
    let response = tokio::time::timeout(std::time::Duration::from_secs(2), run)
        .await
        .expect("downstream settles after release")
        .expect("same run response completes");
    assert_eq!(
        fixture.image_prompts.lock().unwrap().as_slice(),
        ["expanded:hold downstream completion"]
    );
    assert!(response
        .outputs
        .iter()
        .any(|output| output.node_id == "image-infer" && output.port_id == "image"));
}

#[tokio::test]
async fn dependent_text_to_image_resumes_only_downstream_after_readiness_proof_arrives() {
    let fixture = DependentTextImageFixture::new(false).await;
    let mut stale = fixture.image_readiness_snapshot.clone();
    stale.status = DependencyEnvironmentReadinessSnapshotStatus::Stale;
    fixture
        .dependency_readiness_provider
        .insert_snapshot(stale)
        .expect("withhold downstream readiness proof");
    let prompt = "resume this generated prompt exactly once";
    let error = fixture
        .run(prompt)
        .await
        .expect_err("downstream proof is pending");
    assert!(
        matches!(&error, WorkflowServiceError::RuntimeDependencyReadinessPending { task_ids, .. } if task_ids == &["image-infer".to_string()]),
        "unexpected pending result: {error}"
    );
    assert!(error.message().contains("image-infer"));
    assert_eq!(fixture.text_prompts.lock().unwrap().as_slice(), [prompt]);
    assert!(fixture.image_prompts.lock().unwrap().is_empty());

    let plan = fixture
        .service
        .workflow_execution_session_bootstrap_recovery_plan()
        .expect("paused run plan");
    assert_eq!(plan.blocking_decision_count, 0);
    let upstream = plan
        .decisions
        .iter()
        .find(|decision| decision.task_id == "text-infer")
        .expect("completed producer remains part of paused run");
    assert_eq!(upstream.decision_kind,
        pantograph_workflow_service::WorkflowExecutionSessionBootstrapRecoveryDecisionKind::NoopCompleted);
    let workflow_run_id = upstream.workflow_run_id.clone();
    let downstream = plan
        .decisions
        .iter()
        .find(|decision| decision.task_id == "image-infer")
        .expect("paused downstream task");
    assert_eq!(downstream.decision_kind,
        pantograph_workflow_service::WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeRuntimeDependencyReadiness);

    fixture
        .dependency_readiness_provider
        .insert_snapshot(fixture.image_readiness_snapshot.clone())
        .expect("admit downstream readiness proof");
    let recovered = fixture
        .runtime
        .recover_workflow_execution_session_bootstrap()
        .await
        .expect("composed recovery skips completed producer event and resumes downstream");
    assert_eq!(recovered.resumed_runs.len(), 1);
    let response = &recovered.resumed_runs[0];
    assert_eq!(response.workflow_run_id, workflow_run_id);
    assert_eq!(fixture.text_prompts.lock().unwrap().as_slice(), [prompt]);
    assert_eq!(
        fixture.image_prompts.lock().unwrap().as_slice(),
        [format!("expanded:{prompt}")]
    );
    assert!(
        recovered.final_plan.decisions.is_empty(),
        "completed run must leave no active scheduler tasks"
    );
    let image = response
        .outputs
        .iter()
        .find(|output| output.node_id == "image-infer" && output.port_id == "image")
        .expect("recovered image output");
    let artifact_id = image
        .value
        .get("artifact_id")
        .and_then(serde_json::Value::as_str)
        .expect("retained recovered artifact");
    let retained = fixture
        .service
        .read_artifact_body(ArtifactReadRequest {
            artifact_id: artifact_id.to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect("read recovered image");
    assert_eq!(retained.body, valid_test_image_bytes());
    let again = fixture
        .runtime
        .recover_workflow_execution_session_bootstrap()
        .await
        .expect("completed recovery is idempotent");
    assert!(again.resumed_runs.is_empty());
    assert_eq!(fixture.text_prompts.lock().unwrap().len(), 1);
    assert_eq!(fixture.image_prompts.lock().unwrap().len(), 1);
}

const VALID_TEST_IMAGE_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

fn valid_test_image_bytes() -> Vec<u8> {
    crate::media_base64::decode_base64(VALID_TEST_IMAGE_BASE64).expect("valid PNG fixture base64")
}

#[derive(Default)]
struct DependentImageGate {
    enabled: AtomicBool,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

struct DependentTextImageFixture {
    dependency_readiness_provider: DependencyEnvironmentReadinessSnapshotProvider,
    image_readiness_snapshot: DependencyEnvironmentReadinessSnapshot,
    image_gate: Arc<DependentImageGate>,
    text_failures: Arc<Mutex<Vec<String>>>,
    _temp: TempDir,
    service: Arc<WorkflowService>,
    runtime: pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime,
    workflow_id: String,
    session_id: String,
    text_prompts: Arc<Mutex<Vec<String>>>,
    image_prompts: Arc<Mutex<Vec<String>>>,
}

impl DependentTextImageFixture {
    async fn new(fail_text: bool) -> Self {
        const TEXT_MODEL_ID: &str = "llm/example/tiny-transformers";
        const TEXT_ARTIFACT_ID: &str = "text-bundle";
        const IMAGE_MODEL_ID: &str = "image/example/tiny-diffusion";
        const IMAGE_ARTIFACT_ID: &str = "image-bundle";

        let temp = TempDir::new().expect("temp dir");
        let artifact_writer = test_artifact_writer(&temp);
        let workflow_service = WorkflowService::with_ephemeral_attribution_store()
            .expect("service")
            .with_artifact_writer(artifact_writer.clone())
            .with_diagnostics_ledger(
                pantograph_workflow_service::SqliteDiagnosticsLedger::open_in_memory().unwrap(),
            );
        let dependency_readiness_provider = DependencyEnvironmentReadinessSnapshotProvider::new();
        let dependency_readiness_work_queue = Arc::new(DependencyReadinessWorkQueue::new());
        let source_refresher = Arc::new(TestRuntimeDispatchSourceRefresher::default());
        let reservation_lifecycle_port = Arc::new(TestReservationLifecyclePort::default());
        let text_path = temp.path().join("selected-text-model");
        let image_path = temp.path().join("selected-image-model");
        std::fs::create_dir_all(&text_path).expect("text model fixture directory");
        std::fs::create_dir_all(&image_path).expect("image model fixture directory");
        let resolver = Arc::new(DependentTextImageResolver {
            text_path,
            image_path,
        });
        let text_prompts = Arc::new(Mutex::new(Vec::new()));
        let image_prompts = Arc::new(Mutex::new(Vec::new()));
        let fail_text_flag = Arc::new(AtomicBool::new(fail_text));
        let image_gate = Arc::new(DependentImageGate::default());
        let text_failures = Arc::new(Mutex::new(Vec::new()));
        let runtime_host_port =
            Arc::new(EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                resolver.clone(),
                resolver,
                Arc::new(WorkflowServiceRuntimeHostMediaArtifactSink::new(
                    artifact_writer,
                )),
                Arc::new(inference::InferenceGateway::with_backend(
                    Box::new(DependentTextImageBackend {
                        text_prompts: text_prompts.clone(),
                        image_prompts: image_prompts.clone(),
                        fail_text: fail_text_flag,
                        image_gate: image_gate.clone(),
                        text_failures: text_failures.clone(),
                    }),
                    "PyTorch",
                )),
            ));
        let service = Arc::new(
            workflow_service
                .with_dependency_environment_provider(Arc::new(
                    dependency_readiness_provider.clone(),
                ))
                .with_dependency_readiness_work_queue(dependency_readiness_work_queue)
                .with_runtime_dispatch_source_refresher(source_refresher)
                .with_runtime_dispatch_candidate_provider(Arc::new(
                    TestRuntimeDispatchCandidateProvider,
                ))
                .with_runtime_host_execution_port(runtime_host_port.clone())
                .with_runtime_host_batch_execution_port(runtime_host_port)
                .with_reservation_lifecycle_port(reservation_lifecycle_port),
        );
        let workflow_id = "wf-dependent-text-image-runtime-host".to_string();
        let workflow_semantic_version = "1.2.3";
        let graph = dependent_text_image_session_graph(
            TEXT_MODEL_ID,
            TEXT_ARTIFACT_ID,
            IMAGE_MODEL_ID,
            IMAGE_ARTIFACT_ID,
        );
        let version = service
            .resolve_workflow_graph_version(&workflow_id, workflow_semantic_version, &graph)
            .expect("resolve workflow version");
        let mut snapshot =
            image_runtime_validation_snapshot(&version, &graph, IMAGE_MODEL_ID, IMAGE_ARTIFACT_ID);
        let template = snapshot.nodes[0].clone();
        let text_model_ref = PumasModelRef {
            model_id: TEXT_MODEL_ID.to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some(TEXT_ARTIFACT_ID.to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        };
        let image_model_ref = PumasModelRef {
            model_id: IMAGE_MODEL_ID.to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some(IMAGE_ARTIFACT_ID.to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        };
        let (text_node, text_request) = runtime_validation_snapshot_node(
            &template,
            &version,
            "text-infer",
            "text_generation",
            &text_model_ref,
            "torch-transformers",
            "cpu",
        );
        let (image_node, image_request) = runtime_validation_snapshot_node(
            &template,
            &version,
            "image-infer",
            "image_generation",
            &image_model_ref,
            "torch-diffusers",
            "cuda:0",
        );
        snapshot.nodes = vec![text_node, image_node];
        for (request, requirement_name) in [
            (&text_request, "transformers"),
            (&image_request, "diffusers"),
        ] {
            dependency_readiness_provider
                .insert_snapshot(
                    DependencyEnvironmentReadinessSnapshot::for_request(
                        request,
                        ready_dependency_environment_result(request, requirement_name),
                        DependencyEnvironmentReadinessSnapshotStatus::Fresh,
                    )
                    .expect("valid dependency readiness snapshot"),
                )
                .expect("insert readiness snapshot");
        }
        let image_readiness_snapshot = DependencyEnvironmentReadinessSnapshot::for_request(
            &image_request,
            ready_dependency_environment_result(&image_request, "diffusers"),
            DependencyEnvironmentReadinessSnapshotStatus::Fresh,
        )
        .expect("image readiness snapshot");
        service
            .store_workflow_executable_validation_snapshot(snapshot)
            .expect("store validation snapshot");

        let host = Arc::new(ImageRuntimeSessionHost::new(graph));
        let created = service
            .create_workflow_execution_session(
                host.as_ref(),
                WorkflowExecutionSessionCreateRequest {
                    workflow_id: workflow_id.clone(),
                    usage_profile: None,
                    keep_alive: false,
                },
            )
            .await
            .expect("create dependent session");
        let session_id = created.session_id;
        let runtime = pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(
            service.clone(),
            host,
        );
        Self {
            dependency_readiness_provider,
            image_readiness_snapshot,
            image_gate,
            text_failures,
            _temp: temp,
            service,
            runtime,
            workflow_id,
            session_id,
            text_prompts,
            image_prompts,
        }
    }

    async fn run(&self, prompt: &str) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.runtime
            .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
                session_id: self.session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "prompt".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!(prompt),
                }],
                output_targets: Some(vec![
                    WorkflowOutputTarget {
                        node_id: "image-infer".to_string(),
                        port_id: "image".to_string(),
                    },
                    WorkflowOutputTarget {
                        node_id: "text-infer".to_string(),
                        port_id: "text".to_string(),
                    },
                ]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            })
            .await
    }
}

struct DependentTextImageResolver {
    text_path: std::path::PathBuf,
    image_path: std::path::PathBuf,
}

#[async_trait]
impl crate::runtime_host_package_facts::RuntimeHostPackageFactsResolver
    for DependentTextImageResolver
{
    async fn resolve(
        &self,
        request: &pantograph_runtime_host_contracts::ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        inference::ResolvedModelPackageFacts,
        crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsError,
    > {
        let request = request.as_ref();
        let task_type = request.handoff.task_intent.task_type.as_str();
        let selected = &request
            .handoff
            .dispatch_decision
            .as_ref()
            .expect("controlled fixture dispatch decision")
            .selected_model_ref;
        let mut facts: inference::ResolvedModelPackageFacts = match task_type {
            "text_generation" => serde_json::from_str(include_str!(
                "../../../inference/tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
            ))
            .expect("text package facts fixture"),
            "image_generation" => serde_json::from_str(include_str!(
                "../../../inference/tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
            ))
            .expect("image package facts fixture"),
            other => panic!("unsupported dependent fixture task type: {other}"),
        };
        facts.model_ref = inference::PumasModelRef {
            model_id: selected.model_id.clone(),
            revision: selected.revision.clone(),
            selected_artifact_id: selected.selected_artifact_id.clone(),
            selected_artifact_path: selected.selected_artifact_path.clone(),
            migration_diagnostics: Vec::new(),
        };
        if task_type == "text_generation" {
            facts.custom_code.requires_custom_code = false;
            facts.custom_code.custom_code_sources.clear();
            facts.custom_code.auto_map_sources.clear();
        }
        Ok(facts)
    }
}

#[async_trait]
impl crate::runtime_host_load_target::RuntimeHostLoadTargetResolver for DependentTextImageResolver {
    async fn resolve(
        &self,
        request: &pantograph_runtime_host_contracts::ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        pumas_library::models::PumasArtifactLoadTarget,
        crate::runtime_host_load_target::RuntimeHostPumasLoadTargetError,
    > {
        let request = request.as_ref();
        let task_type = request.handoff.task_intent.task_type.as_str();
        let selected = &request
            .handoff
            .dispatch_decision
            .as_ref()
            .expect("controlled fixture dispatch decision")
            .selected_model_ref;
        let (path, artifact_kind, library_root_id) = match task_type {
            "text_generation" => (
                &self.text_path,
                pumas_library::models::PackageArtifactKind::HfCompatibleDirectory,
                "dependent-text-fixture",
            ),
            "image_generation" => (
                &self.image_path,
                pumas_library::models::PackageArtifactKind::DiffusersBundle,
                "dependent-image-fixture",
            ),
            other => panic!("unsupported dependent fixture task type: {other}"),
        };
        Ok(pumas_library::models::PumasArtifactLoadTarget {
            model_ref: pumas_library::models::PumasModelRef {
                model_id: selected.model_id.clone(),
                revision: selected.revision.clone(),
                selected_artifact_id: selected.selected_artifact_id.clone(),
                selected_artifact_path: selected.selected_artifact_path.clone(),
                ..Default::default()
            },
            artifact_kind,
            local_load_path: path.to_str().expect("fixture path is UTF-8").to_string(),
            load_path_kind: pumas_library::models::PumasArtifactLoadPathKind::Directory,
            library_root_id: Some(library_root_id.to_string()),
            storage_kind: StorageKind::LibraryOwned,
            validation_state: AssetValidationState::Valid,
            content_fingerprint: None,
            package_facts_contract_version: Some(inference::MODEL_PACKAGE_FACTS_CONTRACT_VERSION),
        })
    }
}

struct DependentTextImageBackend {
    image_gate: Arc<DependentImageGate>,
    text_failures: Arc<Mutex<Vec<String>>>,
    text_prompts: Arc<Mutex<Vec<String>>>,
    image_prompts: Arc<Mutex<Vec<String>>>,
    fail_text: Arc<AtomicBool>,
}

#[async_trait]
impl InferenceBackend for DependentTextImageBackend {
    fn name(&self) -> &'static str {
        "PyTorch"
    }

    fn description(&self) -> &'static str {
        "controlled dependent text-to-image backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            image_generation: true,
            image_generation_batch: true,
            ..BackendCapabilities::default()
        }
    }

    async fn start(
        &mut self,
        _: &BackendConfig,
        _: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome::default())
    }

    async fn stop(&mut self) -> Result<(), BackendError> {
        Ok(())
    }

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn load_selected_text(
        &mut self,
        request: &inference::InferenceExecutionRequest,
        target: &inference::PumasArtifactLoadTarget,
        decision: &inference::BackendExecutionDecision,
    ) -> Result<BackendStartOutcome, BackendError> {
        assert_eq!(request.model_ref.as_ref(), Some(&target.model_ref));
        assert_eq!(
            decision.selected_device_id.as_ref().unwrap().as_str(),
            "cpu"
        );
        Ok(BackendStartOutcome::default())
    }

    async fn finish_selected_text(&self, _: bool) -> Result<(), BackendError> {
        Ok(())
    }

    async fn chat_completion_stream(
        &self,
        request_json: String,
    ) -> Result<
        Pin<Box<dyn futures_util::Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
        BackendError,
    > {
        let request: serde_json::Value = serde_json::from_str(&request_json).unwrap();
        let prompt = request["messages"][0]["content"][0]["text"]
            .as_str()
            .expect("controlled text prompt")
            .to_string();
        self.text_prompts.lock().unwrap().push(prompt.clone());
        if self.fail_text.load(Ordering::SeqCst) {
            self.text_failures
                .lock()
                .unwrap()
                .push("controlled producer failure".to_string());
            return Err(BackendError::Inference(
                "controlled producer failure".to_string(),
            ));
        }
        Ok(Box::pin(stream::iter([Ok(ChatChunk {
            content: Some(format!("expanded:{prompt}")),
            done: true,
            usage: None,
            cache_handle_id: None,
        })])))
    }

    async fn embeddings(
        &self,
        _: Vec<String>,
        _: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Err(BackendError::NotReady)
    }

    async fn rerank(&self, _: RerankRequest) -> Result<RerankResponse, BackendError> {
        Err(BackendError::NotReady)
    }

    async fn generate_image_from_plan(
        &self,
        plan: ImageGenerationExecutionPlan,
        _: BackendExecutionContext,
    ) -> Result<ImageGenerationResult, BackendError> {
        self.image_prompts.lock().unwrap().push(plan.prompt);
        Ok(ImageGenerationResult {
            images: vec![EncodedImage {
                data_base64: VALID_TEST_IMAGE_BASE64.to_string(),
                mime_type: "image/png".to_string(),
                width: Some(1),
                height: Some(1),
            }],
            seed_used: plan.seed,
            metadata: serde_json::Value::Null,
        })
    }

    async fn generate_image_batch_from_execution_request(
        &self,
        request: ImageGenerationBatchExecutionRequest,
        context: BackendExecutionContext,
    ) -> Result<ImageGenerationBatchExecutionResponse, BackendError> {
        assert!(matches!(
            context.cancellation_snapshot().state,
            inference::InferenceExecutionCancellationState::Running
        ));
        if self.image_gate.enabled.load(Ordering::SeqCst) {
            self.image_gate.entered.notify_one();
            self.image_gate.release.notified().await;
        }
        let members = request
            .members
            .into_iter()
            .map(|member| {
                self.image_prompts
                    .lock()
                    .unwrap()
                    .push(member.plan.prompt.clone());
                ImageGenerationBatchExecutionMemberResponse {
                    member_id: member.member_id,
                    state: ImageGenerationBatchMemberExecutionState::Completed,
                    result: Some(ImageGenerationResult {
                        images: vec![EncodedImage {
                            data_base64: VALID_TEST_IMAGE_BASE64.to_string(),
                            mime_type: "image/png".to_string(),
                            width: Some(1),
                            height: Some(1),
                        }],
                        seed_used: member.plan.seed,
                        metadata: serde_json::Value::Null,
                    }),
                    diagnostics: Vec::new(),
                }
            })
            .collect();
        Ok(ImageGenerationBatchExecutionResponse {
            batch_execution_id: request.batch_execution_id,
            state: ImageGenerationBatchExecutionState::Completed,
            members,
            diagnostics: Vec::new(),
        })
    }
}

struct SelectedTextResolver(std::path::PathBuf);
#[async_trait]
impl crate::runtime_host_package_facts::RuntimeHostPackageFactsResolver for SelectedTextResolver {
    async fn resolve(
        &self,
        request: &pantograph_runtime_host_contracts::ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        inference::ResolvedModelPackageFacts,
        crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsError,
    > {
        let mut facts: inference::ResolvedModelPackageFacts = serde_json::from_str(include_str!("../../../inference/tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json")).unwrap();
        let selected = &request
            .as_ref()
            .handoff
            .dispatch_decision
            .as_ref()
            .unwrap()
            .selected_model_ref;
        facts.model_ref = inference::PumasModelRef {
            model_id: selected.model_id.clone(),
            revision: selected.revision.clone(),
            selected_artifact_id: selected.selected_artifact_id.clone(),
            selected_artifact_path: selected.selected_artifact_path.clone(),
            migration_diagnostics: vec![],
        };
        facts.custom_code.requires_custom_code = false;
        facts.custom_code.custom_code_sources.clear();
        facts.custom_code.auto_map_sources.clear();
        Ok(facts)
    }
}
#[async_trait]
impl crate::runtime_host_load_target::RuntimeHostLoadTargetResolver for SelectedTextResolver {
    async fn resolve(
        &self,
        request: &pantograph_runtime_host_contracts::ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        pumas_library::models::PumasArtifactLoadTarget,
        crate::runtime_host_load_target::RuntimeHostPumasLoadTargetError,
    > {
        let selected = &request
            .as_ref()
            .handoff
            .dispatch_decision
            .as_ref()
            .unwrap()
            .selected_model_ref;
        Ok(pumas_library::models::PumasArtifactLoadTarget {
            model_ref: pumas_library::models::PumasModelRef {
                model_id: selected.model_id.clone(),
                revision: selected.revision.clone(),
                selected_artifact_id: selected.selected_artifact_id.clone(),
                selected_artifact_path: selected.selected_artifact_path.clone(),
                ..Default::default()
            },
            artifact_kind: pumas_library::models::PackageArtifactKind::HfCompatibleDirectory,
            local_load_path: self.0.to_str().unwrap().into(),
            load_path_kind: pumas_library::models::PumasArtifactLoadPathKind::Directory,
            library_root_id: Some("selected-text-test".into()),
            storage_kind: StorageKind::LibraryOwned,
            validation_state: AssetValidationState::Valid,
            content_fingerprint: None,
            package_facts_contract_version: Some(inference::MODEL_PACKAGE_FACTS_CONTRACT_VERSION),
        })
    }
}
struct SelectedWorkflowTextBackend(Arc<Mutex<Vec<String>>>);
#[async_trait]
impl InferenceBackend for SelectedWorkflowTextBackend {
    fn name(&self) -> &'static str {
        "PyTorch"
    }
    fn description(&self) -> &'static str {
        "selected text workflow backend"
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }
    fn is_ready(&self) -> bool {
        true
    }
    fn base_url(&self) -> Option<String> {
        None
    }
    async fn health_check(&self) -> bool {
        true
    }
    async fn start(
        &mut self,
        _: &BackendConfig,
        _: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome::default())
    }
    async fn stop(&mut self) -> Result<(), BackendError> {
        Ok(())
    }
    async fn load_selected_text(
        &mut self,
        request: &inference::InferenceExecutionRequest,
        target: &inference::PumasArtifactLoadTarget,
        decision: &inference::BackendExecutionDecision,
    ) -> Result<BackendStartOutcome, BackendError> {
        assert_eq!(request.model_ref.as_ref(), Some(&target.model_ref));
        assert_eq!(
            decision.selected_device_id.as_ref().unwrap().as_str(),
            "cpu"
        );
        Ok(BackendStartOutcome::default())
    }
    async fn finish_selected_text(&self, _: bool) -> Result<(), BackendError> {
        Ok(())
    }
    async fn chat_completion_stream(
        &self,
        json: String,
    ) -> Result<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
        BackendError,
    > {
        let request: serde_json::Value = serde_json::from_str(&json).unwrap();
        let prompt = request["messages"][0]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        self.0.lock().unwrap().push(prompt.clone());
        Ok(Box::pin(futures_util::stream::iter([Ok(ChatChunk {
            content: Some(format!("expanded:{prompt}")),
            done: true,
            usage: None,
            cache_handle_id: None,
        })])))
    }
    async fn embeddings(
        &self,
        _: Vec<String>,
        _: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Err(BackendError::NotReady)
    }
    async fn rerank(
        &self,
        _: inference::RerankRequest,
    ) -> Result<inference::RerankResponse, BackendError> {
        Err(BackendError::NotReady)
    }
}

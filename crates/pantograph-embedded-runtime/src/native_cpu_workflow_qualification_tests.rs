// Actual hosted composition, native model load/forward and owned cleanup.
// This uses committed tiny untrained BERT weights, with no downloads or ready fixtures.
use super::*;
use crate::{EmbeddedDependencyReadinessAutoResumeConfig, EmbeddedRuntime, EmbeddedRuntimeConfig};
use pantograph_workflow_service::{
    WorkflowErrorCode, WorkflowExecutionSessionCloseRequest, WorkflowExecutionSessionCreateRequest,
    WorkflowExecutionSessionRunRequest, WorkflowOutputTarget, WorkflowPortBinding,
};
use std::time::Duration;

// Observe the result of the actual auto-resume port, without replacing its
// requirements, readiness, graph execution or cleanup behavior.
struct RecordingAutoResume {
    inner: crate::dependency_readiness_auto_resume::EmbeddedWorkflowServiceAutoResumePort,
    completed: Arc<std::sync::Mutex<Vec<pantograph_workflow_service::WorkflowRunResponse>>>,
    failures: Arc<std::sync::Mutex<Vec<WorkflowErrorCode>>>,
}

#[async_trait]
impl crate::DependencyReadinessAutoResumePort for RecordingAutoResume {
    fn dependency_readiness_resume_candidates(
        &self,
    ) -> Result<
        Vec<pantograph_workflow_service::WorkflowExecutionSessionResumeRequest>,
        WorkflowServiceError,
    > {
        self.inner.dependency_readiness_resume_candidates()
    }
    async fn resume_dependency_readiness(
        &self,
        request: pantograph_workflow_service::WorkflowExecutionSessionResumeRequest,
    ) -> Result<pantograph_workflow_service::WorkflowRunResponse, WorkflowServiceError> {
        let result = self.inner.resume_dependency_readiness(request).await;
        if let Ok(response) = &result {
            self.completed.lock().unwrap().push(response.clone());
        } else if let Err(error) = &result {
            if !matches!(
                error,
                WorkflowServiceError::RuntimeDependencyReadinessPending { .. }
            ) {
                self.failures.lock().unwrap().push(error.code());
            }
        }
        result
    }
}

// Explicit service-failure diagnostics accept only the closed error-code enum.
// Arbitrary service error messages can contain session or attribution details.
fn qualification_error_identifier(code: WorkflowErrorCode) -> String {
    format!("{code:?}")
}

#[test]
fn qualification_error_identifiers_omit_service_error_payloads() {
    let marker = "session_private_qualification_marker";
    for (error, expected) in [
        (
            WorkflowServiceError::InvalidRequest(marker.into()),
            "InvalidRequest",
        ),
        (
            WorkflowServiceError::SessionNotFound(marker.into()),
            "SessionNotFound",
        ),
        (
            WorkflowServiceError::Internal(marker.into()),
            "InternalError",
        ),
    ] {
        let identifier = qualification_error_identifier(error.code());
        assert_eq!(identifier, expected);
        assert!(!identifier.contains(marker));
    }
}

fn copy_fixture(source: &std::path::Path, destination: &std::path::Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[tokio::test]
#[ignore = "Requires the reviewed Pumas HF-directory owner fix; published 26a84 pin returns a file target"]
async fn empty_hosted_readiness_bootstrap_completes_actual_cpu_embedding_and_reuses_owner() {
    tokio::time::timeout(Duration::from_secs(45), qualify_cpu_workflow(true))
        .await
        .expect("bounded real CPU workflow qualification");
}

#[tokio::test]
#[ignore = "Requires the reviewed Pumas HF-directory owner fix; published 26a84 pin returns a file target"]
async fn empty_hosted_readiness_bootstrap_completes_cold_cpu_embedding_and_cleans_owner() {
    tokio::time::timeout(Duration::from_secs(45), qualify_cpu_workflow(false))
        .await
        .expect("bounded cold CPU workflow qualification");
}

async fn qualify_cpu_workflow(keep_alive: bool) {
    let temp = create_test_env();
    let model_id = "embedding/qualification/synthetic-bert-8";
    let workflow_id = "native-cpu-bootstrap";
    let model_dir = temp.path().join("shared-resources/models").join(model_id);
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../inference/tests/fixtures/candle_bert/bert-8");
    copy_fixture(&source, &model_dir);
    std::fs::write(model_dir.join("metadata.json"), serde_json::to_vec(&serde_json::json!({
        "schema_version":2,"model_id":model_id,"family":"qualification","model_type":"embedding",
        "official_name":"Synthetic-BERT-8","cleaned_name":"synthetic-bert-8","source_path":model_dir,
        "entry_path":model_dir,"storage_kind":"library_owned","selected_artifact_id":"main",
        "selected_artifact_files":["model.safetensors"],"import_state":"ready","validation_state":"valid",
        "pipeline_tag":"feature-extraction","task_type_primary":"embedding","input_modalities":["text"],
        "output_modalities":["embedding"],"recommended_backend":"candle","runtime_engine_hints":["candle"]
    })).unwrap()).unwrap();
    let api = Arc::new(
        crate::pumas_test_support::builder(temp.path())
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .unwrap(),
    );
    api.rebuild_model_index().await.unwrap();
    let registry = Arc::new(RuntimeRegistry::new());
    let gateway = Arc::new(inference::InferenceGateway::new());
    if keep_alive {
        gateway.switch_backend("Candle").await.unwrap();
    }
    let app_data = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data).unwrap();
    let output = EmbeddedWorkflowServiceComposition::resource_backed_hosted_startup(
        EmbeddedHostedStartupCompositionInput::new(EmbeddedHostedStartupConfig {
            runtime_registry: registry.clone(),
            runtime_registry_controller: gateway.clone(),
            gateway: gateway.clone(),
            pumas_selector_source: Some(EmbeddedHostedStartupPumasSelectorSource::Provided(
                Arc::new(PumasSelectorAccess::Owner(api)),
            )),
            project_root: temp.path().to_path_buf(),
            kv_cache_dir: temp.path().join("kv-cache"),
            dependency_readiness_runtime_handle: tokio::runtime::Handle::current(),
            max_loaded_sessions: Some(1),
            max_dispatch_source_snapshot_age_ms: 1_000,
        })
        .with_workflow_service(workflow_service_with_artifact_and_attribution_store(&temp))
        .with_dependency_inventory_app_data_dir(app_data.clone())
        .with_dependency_readiness_producer_config(
            EmbeddedDependencyReadinessSnapshotProducerConfig {
                poll_interval: Duration::from_millis(5),
            },
        ),
    )
    .await
    .unwrap();
    let service = output.workflow_service.clone();
    let readiness = output.dependency_readiness.clone();
    assert_eq!(readiness.snapshot_provider().snapshot_count(), 0);

    // No readiness result or payload is injected. Obtain the current descriptor
    // from normal graph validation, then author its actual ports for submission.
    let mut graph: WorkflowGraph = serde_json::from_value(serde_json::json!({"nodes":[
        {"id":"prompt","node_type":"text-input","position":{"x":0,"y":100},"data":{"text":"hello world"}},
        {"id":"infer","node_type":"llm-inference","position":{"x":300,"y":100},"data":{
            "task_kind":"embedding","runtime":"candle","device":"cpu",
            "runtime_source_context":{"operation_type":"embedding.text","context_shape_key":"embedding.one-text","cancellation_mode":"run_scoped"},
            "pumas_model_ref":{"model_id":model_id,"selected_artifact_id":"main"}}},
        {"id":"vectors","node_type":"vector-output","position":{"x":600,"y":100},"data":{}},
        {"id":"deps","node_type":"dependency-environment","position":{"x":300,"y":450},"data":{"mode":"manual"}}
    ],"edges":[
        {"id":"text-to-infer","source":"prompt","source_handle":"text","target":"infer","target_handle":"text"},
        {"id":"infer-to-vector","source":"infer","source_handle":"embedding","target":"vectors","target_handle":"vector"},
        {"id":"deps-to-infer","source":"deps","source_handle":"dependency_environment_sidecar","target":"infer","target_handle":"dependency_environment_sidecar"}
    ]})).unwrap();
    let node_registry = pantograph_workflow_service::graph::NodeRegistry::new();
    for node in &mut graph.nodes {
        if matches!(node.node_type.as_str(), "text-input" | "vector-output") {
            node.data["definition"] =
                serde_json::to_value(node_registry.get_definition(&node.node_type).unwrap())
                    .unwrap();
        }
    }
    let preview = service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
            workflow_id: None,
        })
        .await
        .unwrap();
    let validation = service
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: preview.session_id,
                graph_revision: preview.graph_revision.parse().unwrap(),
            },
        )
        .await
        .unwrap();
    let descriptor = &validation.node_projections[0].descriptor;
    let authored =
        pantograph_workflow_service::graph::authored_snapshot_from_descriptor(descriptor).unwrap();
    graph
        .nodes
        .iter_mut()
        .find(|n| n.id == "infer")
        .unwrap()
        .data["inference_interface_snapshot"] = serde_json::to_value(authored).unwrap();
    let workflow_dir = temp.path().join(".pantograph/workflows");
    std::fs::create_dir_all(&workflow_dir).unwrap();
    std::fs::write(workflow_dir.join(format!("{workflow_id}.json")), serde_json::to_vec(&serde_json::json!({"version":"1.0","metadata":{"name":"Native CPU qualification"},"graph":graph})).unwrap()).unwrap();
    let session = service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph,
            workflow_id: Some(workflow_id.into()),
        })
        .await
        .unwrap();
    let validation = service
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: session.session_id.clone(),
                graph_revision: session.graph_revision.parse().unwrap(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        validation.summary.summary.as_ref().unwrap().status,
        DraftGraphValidationStatus::Executable,
        "native fixture validation must be executable"
    );
    let validation_id = validation.summary.validation_session_id.unwrap();
    let action = service
        .workflow_graph_resolve_dependency_environment_action_intent(
            DependencyEnvironmentActionIntent {
                contract_version: 1,
                graph_session_id: session.session_id.parse().unwrap(),
                graph_revision: session.graph_revision.parse().unwrap(),
                validation_session_id: Some(validation_id.clone()),
                target_node_id: "deps".parse().unwrap(),
                action: DependencyEnvironmentAction::Resolve,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        action.status,
        DependencyEnvironmentActionIntentStatus::RequestReady,
        "native fixture dependency action must be ready"
    );
    let executable = service
        .publish_graph_session_executable_validation_snapshot(
            WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                workflow_id: workflow_id.into(),
                workflow_semantic_version: "1.0.0".into(),
                graph_session_id: session.session_id,
                validation_session_id: Some(validation_id),
                validation_snapshot_id: None,
            },
        )
        .await
        .unwrap();
    use pantograph_dependency_environment_service::DependencyRequirementsRegistry;
    let requirements_id = &executable.as_record().nodes[0].dependency_requirements_id;
    let requirements = readiness
        .requirements_registry()
        .lookup_requirements(requirements_id)
        .expect("authoritative payload published by graph resolve");
    assert!(
        requirements
            .payload
            .identity_key
            .selected_binding_ids
            .is_empty(),
        "authored empty choices preserved in identity"
    );
    assert_eq!(
        requirements.payload.selected_binding_ids.len(),
        2,
        "owner-selected actual bindings"
    );
    assert_eq!(
        readiness.snapshot_provider().snapshot_count(),
        0,
        "requirements resolution did not forge readiness"
    );
    assert!(registry.snapshot().reservations.is_empty());
    assert!(registry
        .snapshot()
        .runtimes
        .iter()
        .all(|r| r.models.is_empty()));
    let runtime = EmbeddedRuntime::from_hosted_composition(
        EmbeddedRuntimeConfig {
            app_data_dir: app_data,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![workflow_dir],
            max_loaded_sessions: Some(1),
        },
        gateway.clone(),
        output.shared_extensions,
        service,
        None,
        Some(registry.clone()),
        None,
    )
    .with_dependency_readiness_snapshot_producer(output.dependency_readiness_snapshot_producer);
    let completed = Arc::new(std::sync::Mutex::new(Vec::new()));
    let failures = Arc::new(std::sync::Mutex::new(Vec::new()));
    let auto_resume =
        crate::EmbeddedDependencyReadinessAutoResume::new(Arc::new(RecordingAutoResume {
            inner:
                crate::dependency_readiness_auto_resume::EmbeddedWorkflowServiceAutoResumePort::new(
                    runtime.workflow_service.clone(),
                    runtime.host(),
                ),
            completed: completed.clone(),
            failures: failures.clone(),
        }))
        .with_config(EmbeddedDependencyReadinessAutoResumeConfig {
            poll_interval: Duration::from_millis(5),
        })
        .spawn(tokio::runtime::Handle::current())
        .unwrap();
    let runtime = runtime.with_dependency_readiness_auto_resume(auto_resume);
    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: workflow_id.into(),
            usage_profile: None,
            keep_alive,
        })
        .await
        .unwrap();
    let golden: serde_json::Value =
        serde_json::from_slice(&std::fs::read(source.join("golden.json")).unwrap()).unwrap();
    let expected = golden["single_vectors"][0].as_array().unwrap();
    use pantograph_dependency_planning::{
        DependencyEnvironmentRequest, DependencyPlanningRequest,
        ValidatedDependencyEnvironmentRequest,
    };
    let probe_request =
        ValidatedDependencyEnvironmentRequest::try_from(DependencyEnvironmentRequest {
            contract_version: 1,
            action: DependencyEnvironmentAction::Check,
            identity_key: requirements.payload.identity_key.clone(),
            planning_request: serde_json::from_value::<DependencyPlanningRequest>(
                serde_json::to_value(&requirements.payload.identity_key).unwrap(),
            )
            .unwrap(),
            dependency_requirements_id: Some(requirements_id.clone()),
            environment_ref: None,
        })
        .unwrap();
    let mut instance = None;
    for run in 0..2 {
        let submitted = runtime
            .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.0.0".into(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "prompt".into(),
                    port_id: "text".into(),
                    value: serde_json::json!("hello world"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "vectors".into(),
                    port_id: "vector".into(),
                }]),
                override_selection: None,
                timeout_ms: Some(15_000),
                priority: None,
            })
            .await;
        let result = match submitted {
            Ok(result) => result,
            Err(WorkflowServiceError::RuntimeDependencyReadinessPending { .. }) => {
                tokio::time::timeout(Duration::from_secs(15), async {
                    loop {
                        if let Some(code) = failures.lock().unwrap().pop() {
                            panic!("actual auto-resume failed: {}", qualification_error_identifier(code));
                        }
                        if let Some(result) = completed.lock().unwrap().pop() { break result; }
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                }).await.unwrap_or_else(|_| {
                    let snapshot = registry.snapshot();
                    panic!(
                        "actual CPU run {run} never completed after owned resume: runtime_count={} reservation_count={} snapshot_count={}",
                        snapshot.runtimes.len(),
                        snapshot.reservations.len(),
                        readiness.snapshot_provider().snapshot_count()
                    )
                })
            }
            Err(error) => {
                let snapshot = registry.snapshot();
                panic!(
                    "actual CPU run {run}: code={} runtime_count={} reservation_count={}",
                    qualification_error_identifier(error.code()),
                    snapshot.runtimes.len(),
                    snapshot.reservations.len()
                )
            }
        };
        use pantograph_dependency_environment_service::DependencyEnvironmentProvider;
        let check = readiness.snapshot_provider().check(&probe_request);
        assert_eq!(
            check.readiness_state,
            pantograph_dependency_planning::DependencyEnvironmentReadinessState::Ready
        );
        assert!(
            result.outputs.len() == 1,
            "actual CPU workflow must produce one output"
        );
        let vector = result.outputs[0]
            .value
            .as_array()
            .expect("actual numeric vector");
        assert_eq!(vector.len(), expected.len());
        for (actual, expected) in vector.iter().zip(expected) {
            assert!((actual.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-4);
        }
        let snapshot = registry.snapshot();
        let candle = snapshot
            .runtimes
            .iter()
            .find(|r| r.runtime_id == "candle")
            .unwrap();
        if keep_alive {
            assert!(
                candle.runtime_instance_id.is_some(),
                "actual Candle lifecycle owner"
            );
            if run == 0 {
                instance = candle.runtime_instance_id.clone();
            } else {
                assert!(
                    candle.runtime_instance_id == instance,
                    "same native owner reused"
                );
                assert_eq!(
                    gateway.runtime_lifecycle_snapshot().await.runtime_reused,
                    Some(true)
                );
            }
            assert_eq!(
                snapshot.reservations.len(),
                1,
                "only retained session lease remains"
            );
        } else {
            assert_eq!(
                candle.status,
                pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
            );
            assert!(candle.runtime_instance_id.is_none());
            assert!(candle.models.is_empty());
            assert!(
                snapshot.reservations.is_empty(),
                "ephemeral session and task leases retired"
            );
        }
    }
    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: created.session_id,
        })
        .await
        .unwrap();
    assert!(
        registry.snapshot().reservations.is_empty(),
        "owned session/task leases released on close"
    );
    runtime.shutdown().await.unwrap();
    let snapshot = registry.snapshot();
    assert!(snapshot.reservations.is_empty());
    let candle = snapshot
        .runtimes
        .iter()
        .find(|r| r.runtime_id == "candle")
        .unwrap();
    assert_eq!(
        candle.status,
        pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
    );
    assert!(candle.runtime_instance_id.is_none());
    assert!(candle.models.is_empty());
}

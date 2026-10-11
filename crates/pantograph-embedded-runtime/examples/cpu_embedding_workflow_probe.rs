//! Isolated native workflow probe. This is not a GUI qualification.
//! Uses only the committed untrained BERT fixture and the published Pumas owner.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pantograph_diagnostics_ledger::{DiagnosticsLedgerRepository, SqliteDiagnosticsLedger};
use pantograph_embedded_runtime::{
    EmbeddedDependencyReadinessSnapshotProducerConfig, EmbeddedHostedStartupCompositionInput,
    EmbeddedHostedStartupConfig, EmbeddedHostedStartupPumasSelectorSource, EmbeddedRuntime,
    EmbeddedRuntimeConfig, EmbeddedWorkflowServiceComposition,
};
use pantograph_inference_interface_contracts::{
    DependencyEnvironmentAction, DependencyEnvironmentActionIntent,
    DependencyEnvironmentActionIntentStatus, DraftGraphValidationStatus,
};
use pantograph_runtime_registry::RuntimeRegistry;
use pantograph_workflow_service::graph::{
    authored_snapshot_from_descriptor, NodeRegistry, WorkflowGraph,
};
use pantograph_workflow_service::workflow::WorkflowGraphSessionExecutableValidationSnapshotPublishRequest;
use pantograph_workflow_service::{
    ArtifactPolicy, ArtifactStore, WorkflowExecutionSessionCloseRequest,
    WorkflowExecutionSessionCreateRequest, WorkflowExecutionSessionRunRequest,
    WorkflowGraphCurrentValidationRefreshRequest, WorkflowGraphEditSessionCloseRequest,
    WorkflowGraphEditSessionCreateRequest, WorkflowOutputTarget, WorkflowPortBinding,
    WorkflowService, WorkflowServiceError,
};
use serde_json::{json, Value};
use workflow_nodes::setup::PumasSelectorAccess;

type ProbeResult<T> = Result<T, Box<dyn std::error::Error>>;
const MODEL_ID: &str = "embedding/qualification/synthetic-bert-8";
const WORKFLOW_ID: &str = "cpu-embedding-probe";

#[path = "cpu_embedding_workflow_probe/browser_host.rs"]
mod browser_host;
#[path = "cpu_embedding_workflow_probe/target_observation.rs"]
mod target_observation;

fn save(root: &Path, name: &str, value: &impl serde::Serialize) -> ProbeResult<()> {
    std::fs::write(root.join(name), serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn copy_fixture(source: &Path, destination: &Path) -> ProbeResult<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_fixture(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn seed_fixture(
    root: &Path,
    model_id: &str,
    selected_files: Option<&[&str]>,
) -> ProbeResult<PathBuf> {
    let model_dir = root.join("shared-resources/models").join(model_id);
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../inference/tests/fixtures/candle_bert/bert-8");
    copy_fixture(&source, &model_dir)?;
    save(
        &model_dir,
        "metadata.json",
        &json!({
            "schema_version": 2, "model_id": model_id, "family": "qualification",
            "model_type": "embedding", "official_name": "Synthetic-BERT-8",
            "cleaned_name": "synthetic-bert-8", "source_path": model_dir,
            "entry_path": model_dir, "storage_kind": "library_owned",
            "selected_artifact_id": "main", "selected_artifact_files": selected_files,
            "import_state": "ready", "validation_state": "valid",
            "pipeline_tag": "feature-extraction", "task_type_primary": "embedding",
            "input_modalities": ["text"], "output_modalities": ["embedding"],
            "recommended_backend": "candle", "runtime_engine_hints": ["candle"]
        }),
    )?;
    Ok(model_dir)
}

async fn create_runtime(
    root: &Path,
) -> ProbeResult<(EmbeddedRuntime, Arc<pumas_library::PumasApi>)> {
    seed_fixture(root, MODEL_ID, Some(&["model.safetensors"]))?;
    for directory in [
        "launcher-data/metadata",
        "launcher-data/cache",
        "launcher-data/logs",
        "app-data",
    ] {
        std::fs::create_dir_all(root.join(directory))?;
    }
    let api = Arc::new(
        pumas_library::PumasApi::builder(root)
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await?,
    );
    api.rebuild_model_index().await?;
    let registry = Arc::new(RuntimeRegistry::new());
    let gateway = Arc::new(inference::InferenceGateway::new());
    let service = WorkflowService::with_ephemeral_attribution_store()?
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open(root.join("diagnostics.db"))?)
        .with_artifact_store(ArtifactStore::open(
            root.join("artifacts"),
            ArtifactPolicy {
                policy_id: "cpu-embedding-probe".into(),
                policy_version: 1,
                ttl_seconds: None,
                max_disk_bytes: Some(1024 * 1024),
                max_memory_bytes: Some(1024 * 1024),
                max_single_artifact_bytes: Some(65536),
                spill_threshold_bytes: None,
                delete_on_consume: false,
            },
        )?);
    let composition = EmbeddedWorkflowServiceComposition::resource_backed_hosted_startup(
        EmbeddedHostedStartupCompositionInput::new(EmbeddedHostedStartupConfig {
            runtime_registry: registry.clone(),
            runtime_registry_controller: gateway.clone(),
            gateway: gateway.clone(),
            pumas_selector_source: Some(EmbeddedHostedStartupPumasSelectorSource::Provided(
                Arc::new(PumasSelectorAccess::Owner(api.clone())),
            )),
            project_root: root.to_path_buf(),
            kv_cache_dir: root.join("kv-cache"),
            dependency_readiness_runtime_handle: tokio::runtime::Handle::current(),
            max_loaded_sessions: Some(1),
            max_dispatch_source_snapshot_age_ms: 1000,
        })
        .with_workflow_service(service)
        .with_dependency_inventory_app_data_dir(root.join("app-data"))
        .with_dependency_readiness_producer_config(
            EmbeddedDependencyReadinessSnapshotProducerConfig {
                // Bound this interactive probe independently of the background default (60s).
                poll_interval: Duration::from_millis(50),
            },
        ),
    )
    .await?;
    Ok((
        EmbeddedRuntime::from_hosted_composition(
            EmbeddedRuntimeConfig::new(root.join("app-data"), root.to_path_buf()),
            gateway,
            composition.shared_extensions,
            composition.workflow_service,
            None,
            Some(registry),
            None,
        )
        .with_dependency_readiness_snapshot_producer(
            composition.dependency_readiness_snapshot_producer,
        ),
        api,
    ))
}

async fn author_workflow(
    runtime: &EmbeddedRuntime,
    root: &Path,
    stage: &mut &str,
    publish: bool,
) -> ProbeResult<String> {
    let service = runtime.workflow_service();
    *stage = "graph_authoring";
    let mut graph: WorkflowGraph = serde_json::from_value(json!({"nodes": [
        {"id":"prompt", "node_type":"text-input", "position":{"x":0,"y":100}, "data":{"text":"hello world"}},
        {"id":"infer", "node_type":"llm-inference", "position":{"x":300,"y":100}, "data":{
            "label":"CPU Embedding", "modelName":"Untrained BERT-8",
            "task_kind":"embedding", "runtime":"candle", "device":"cpu",
            "runtime_source_context":{"operation_type":"embedding.text", "context_shape_key":"embedding.one-text", "cancellation_mode":"run_scoped"},
            "pumas_model_ref":{"model_id":MODEL_ID, "selected_artifact_id":"main"}}},
        {"id":"vectors", "node_type":"vector-output", "position":{"x":740,"y":100}, "data":{}},
        {"id":"deps", "node_type":"dependency-environment", "position":{"x":300,"y":450}, "data":{"mode":"manual"}}
    ], "edges": [
        {"id":"text-to-infer", "source":"prompt", "source_handle":"text", "target":"infer", "target_handle":"text"},
        {"id":"infer-to-vector", "source":"infer", "source_handle":"embedding", "target":"vectors", "target_handle":"vector"},
        {"id":"deps-to-infer", "source":"deps", "source_handle":"dependency_environment_sidecar", "target":"infer", "target_handle":"dependency_environment_sidecar"}
    ]}))?;
    let registry = NodeRegistry::new();
    for node in &mut graph.nodes {
        if matches!(node.node_type.as_str(), "text-input" | "vector-output") {
            node.data["definition"] = serde_json::to_value(
                registry
                    .get_definition(&node.node_type)
                    .ok_or("missing built-in node definition")?,
            )?;
        }
    }
    let preview = service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
            workflow_id: None,
        })
        .await?;
    let validation = service
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: preview.session_id.clone(),
                graph_revision: preview.graph_revision.parse()?,
            },
        )
        .await?;
    save(root, "descriptor-validation.json", &validation)?;
    let projection = validation
        .node_projections
        .iter()
        .find(|node| node.node_id.as_str() == "infer")
        .ok_or("backend did not return an inference descriptor")?;
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "infer")
        .ok_or("missing inference node")?
        .data["inference_interface_snapshot"] =
        serde_json::to_value(authored_snapshot_from_descriptor(&projection.descriptor)?)?;
    service
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: preview.session_id,
        })
        .await?;
    service.workflow_graph_save(
        &pantograph_workflow_service::FileSystemWorkflowGraphStore::new(root),
        pantograph_workflow_service::WorkflowGraphSaveRequest {
            name: WORKFLOW_ID.into(),
            graph: graph.clone(),
        },
    )?;
    let session = service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph,
            workflow_id: Some(WORKFLOW_ID.into()),
        })
        .await?;
    *stage = "executable_validation";
    let validation = service
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: session.session_id.clone(),
                graph_revision: session.graph_revision.parse()?,
            },
        )
        .await?;
    save(root, "executable-validation.json", &validation)?;
    if validation
        .summary
        .summary
        .as_ref()
        .map(|summary| &summary.status)
        != Some(&DraftGraphValidationStatus::Executable)
    {
        return Err(
            "backend executable validation is blocking; inspect executable-validation.json".into(),
        );
    }
    let validation_id = validation
        .summary
        .validation_session_id
        .ok_or("missing validation identity")?;
    *stage = "dependency_resolve";
    let action = service
        .workflow_graph_resolve_dependency_environment_action_intent(
            DependencyEnvironmentActionIntent {
                contract_version: 1,
                graph_session_id: session.session_id.parse()?,
                graph_revision: session.graph_revision.parse()?,
                validation_session_id: Some(validation_id.clone()),
                target_node_id: "deps".parse()?,
                action: DependencyEnvironmentAction::Resolve,
            },
        )
        .await?;
    save(root, "dependency-resolve.json", &action)?;
    if action.status != DependencyEnvironmentActionIntentStatus::RequestReady {
        return Err(
            "backend dependency Resolve is blocking; inspect dependency-resolve.json".into(),
        );
    }
    if !publish {
        return Ok(session.session_id);
    }
    *stage = "publish_validation";
    let published = service
        .publish_graph_session_executable_validation_snapshot(
            WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                workflow_id: WORKFLOW_ID.into(),
                workflow_semantic_version: "1.0.0".into(),
                graph_session_id: session.session_id.clone(),
                validation_session_id: Some(validation_id),
                validation_snapshot_id: None,
            },
        )
        .await?;
    save(root, "published-validation.json", published.as_record())?;
    // Keep authoring context alive while its dependency work is being checked,
    // just as the open editor does. Close it after the execution session.
    Ok(session.session_id)
}

async fn execute(runtime: &EmbeddedRuntime, root: &Path, stage: &mut &str) -> ProbeResult<Value> {
    let graph_session_id = author_workflow(runtime, root, stage, true).await?;
    *stage = "session_create";
    let session = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: WORKFLOW_ID.into(),
            usage_profile: None,
            keep_alive: false,
        })
        .await?;
    *stage = "scheduler_submit_and_runtime_dispatch";
    let started = Instant::now();
    let execution = async {
        let mut result = runtime
            .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
                session_id: session.session_id.clone(),
                workflow_semantic_version: "1.0.0".into(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "prompt".into(),
                    port_id: "text".into(),
                    value: json!("hello world"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "vectors".into(),
                    port_id: "vector".into(),
                }]),
                override_selection: None,
                timeout_ms: Some(15000),
                priority: None,
            })
            .await;
        // Only resume the existing scheduler run. Never submit replacement work or readiness facts.
        while matches!(
            result,
            Err(WorkflowServiceError::RuntimeDependencyReadinessPending { .. })
        ) {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let candidates = runtime
                .workflow_service()
                .workflow_execution_session_runtime_dependency_readiness_resume_candidates()?;
            if let Some(request) = candidates
                .into_iter()
                .find(|item| item.session_id == session.session_id)
            {
                result = runtime
                    .resume_workflow_execution_session_runtime_dependency_readiness(request)
                    .await;
            }
        }
        result
    };
    let result = tokio::time::timeout(Duration::from_secs(30), execution).await;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let close = runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: session.session_id,
        })
        .await;
    save(
        root,
        "session-close.json",
        &json!({"ok": close.is_ok(), "error": close.as_ref().err().map(ToString::to_string)}),
    )?;
    let graph_close = runtime
        .workflow_service()
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: graph_session_id,
        })
        .await;
    let response = result??;
    close?;
    graph_close?;
    *stage = "output_verification";
    let vector = response
        .outputs
        .iter()
        .find(|port| port.node_id == "vectors" && port.port_id == "vector")
        .and_then(|port| port.value.as_array())
        .ok_or("missing vector output")?;
    let golden: Value = serde_json::from_slice(&std::fs::read(
        root.join("shared-resources/models")
            .join(MODEL_ID)
            .join("golden.json"),
    )?)?;
    let expected = golden["single_vectors"][0]
        .as_array()
        .ok_or("missing independent reference vector")?;
    if vector.len() != expected.len()
        || vector.iter().zip(expected).any(|(actual, expected)| {
            match (actual.as_f64(), expected.as_f64()) {
                (Some(actual), Some(expected)) => {
                    !actual.is_finite() || (actual - expected).abs() > 1e-4
                }
                _ => true,
            }
        })
    {
        return Err(
            "actual scheduler output does not match the independent fixture reference".into(),
        );
    }
    Ok(
        json!({"scope":"native_workflow_only", "backend":"candle", "device":"cpu_f32",
        "model_id":MODEL_ID, "model_training":"untrained", "submit_to_response_wall_ms":elapsed_ms,
        "response":response, "reference_tolerance":1e-4}),
    )
}

#[tokio::main]
async fn main() -> ProbeResult<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() > 2
        || args
            .get(1)
            .is_some_and(|mode| mode != "--inspect-targets" && mode != "--serve-stdio")
    {
        return Err(
            "usage: cpu_embedding_workflow_probe <new absolute directory> [--inspect-targets|--serve-stdio]"
                .into(),
        );
    }
    let root = PathBuf::from(args.first().ok_or("pass a new absolute probe directory")?);
    if !root.is_absolute()
        || std::env::var_os("PUMAS_REGISTRY_DB_PATH")
            .map(PathBuf::from)
            .as_deref()
            != Some(root.join("registry.db").as_path())
    {
        return Err("set PUMAS_REGISTRY_DB_PATH to <new absolute probe directory>/registry.db; existing catalogs are never opened".into());
    }
    std::fs::create_dir(&root)?; // Refuse existing directories, including previous evidence.
    if args.get(1).is_some_and(|mode| mode == "--inspect-targets") {
        return target_observation::inspect(&root).await;
    }
    let (runtime, library) = create_runtime(&root).await?;
    if args.get(1).is_some_and(|mode| mode == "--serve-stdio") {
        return browser_host::serve(runtime, library, &root).await;
    }
    let mut stage = "initialization";
    let result = tokio::time::timeout(
        Duration::from_secs(60),
        execute(&runtime, &root, &mut stage),
    )
    .await;
    let shutdown = tokio::time::timeout(Duration::from_secs(10), runtime.shutdown()).await;
    let shutdown_ok = matches!(shutdown, Ok(Ok(())));
    let ledger = SqliteDiagnosticsLedger::open(root.join("diagnostics.db"))?;
    let events = ledger.diagnostic_events_after(0, 500)?;
    save(&root, "diagnostics.json", &events)?;
    let weights = std::fs::read(
        root.join("shared-resources/models")
            .join(MODEL_ID)
            .join("model.safetensors"),
    )?;
    let report = match &result {
        Ok(Ok(output)) if shutdown_ok => json!({"status":"passed", "stage":stage, "output":output}),
        Ok(Err(error)) => json!({"status":"blocked", "stage":stage, "error":error.to_string()}),
        _ => {
            json!({"status":"failed", "stage":stage, "error":"execution or shutdown did not complete successfully"})
        }
    };
    let report = json!({"qualification":report, "shutdown_ok":shutdown_ok, "editor_qualified":false,
        "model_acquisition_bytes":0, "model_weights_bytes":weights.len(),
        "model_weights_blake3":blake3::hash(&weights).to_hex().to_string(),
        "diagnostic_events_saved":events.len(), "diagnostics_json_may_be_truncated":events.len() == 500});
    save(&root, "result.json", &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report["qualification"]["status"] != "passed" {
        return Err(
            "native workflow probe did not pass; evidence is retained in the probe directory"
                .into(),
        );
    }
    Ok(())
}

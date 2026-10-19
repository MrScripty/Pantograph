//! Bounded public CPU text workflow probe. Synthetic fixtures do not qualify T3.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use pantograph_diagnostics_ledger::{
    DiagnosticEventPayload, DiagnosticsLedgerRepository, NodeExecutionProjectionStatus,
    SchedulerTaskAttemptLifecycleTransition, SqliteDiagnosticsLedger,
};
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
    ArtifactPolicy, ArtifactStore, WorkflowExecutionSessionActiveTaskCancelRequest,
    WorkflowExecutionSessionCloseRequest, WorkflowExecutionSessionCreateRequest,
    WorkflowExecutionSessionRunRequest, WorkflowGraphCurrentValidationRefreshRequest,
    WorkflowGraphEditSessionCloseRequest, WorkflowGraphEditSessionCreateRequest,
    WorkflowOutputTarget, WorkflowPortBinding, WorkflowService, WorkflowServiceError,
};
use serde_json::{json, Value};
use workflow_nodes::setup::PumasSelectorAccess;

type ProbeResult<T> = Result<T, Box<dyn std::error::Error>>;
const MODEL_ID: &str = "llm/qualification/local-text";
const WORKFLOW_ID: &str = "cpu-text-probe";
const PROMPT: &str = "The future of science is";
const MAX_NEW_TOKENS: u64 = 24;
const TEMPERATURE: f64 = 0.0;

fn save(root: &Path, name: &str, value: &impl serde::Serialize) -> ProbeResult<()> {
    std::fs::write(root.join(name), serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

// Copy only ordinary local Transformers inputs, never Python/model remote code.
fn seed_model(root: &Path, source: &Path, synthetic: bool) -> ProbeResult<()> {
    if !source.is_absolute() || !source.is_dir() {
        return Err("model source must be an existing absolute directory".into());
    }
    let model = root.join("shared-resources/models").join(MODEL_ID);
    std::fs::create_dir_all(&model)?;
    let mut total = 0u64;
    let mut files = Vec::new();
    for name in [
        "config.json",
        "model.safetensors",
        "tokenizer.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
        "generation_config.json",
        "vocab.json",
        "merges.txt",
        "chat_template.jinja",
    ] {
        let path = source.join(name);
        if !path.exists() {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_file() {
            return Err("model inputs must be regular files, not symlinks".into());
        }
        total = total
            .checked_add(metadata.len())
            .ok_or("model size overflow")?;
        if total > 600 * 1024 * 1024 {
            return Err("model exceeds 600 MiB acquisition/input budget".into());
        }
        std::fs::copy(&path, model.join(name))?;
        let bytes = std::fs::read(model.join(name))?;
        files.push(json!({"file":name,"bytes":bytes.len(),"blake3":blake3::hash(&bytes).to_hex().to_string()}));
    }
    for name in ["config.json", "model.safetensors", "tokenizer_config.json"] {
        if !model.join(name).is_file() {
            return Err(format!("missing model input: {name}").into());
        }
    }
    save(
        root,
        "model-inputs.json",
        &json!({"training":if synthetic {"untrained_synthetic"} else {"external_unverified"},"bytes":total,"files":files,"t3_qualified":false}),
    )?;
    save(
        &model,
        "metadata.json",
        &json!({
            "schema_version":2,"model_id":MODEL_ID,"family":"qualification","model_type":"llm",
            "official_name":"Local CPU text probe","cleaned_name":"local-text","source_path":model,"entry_path":model,
            "storage_kind":"library_owned","selected_artifact_id":"main","selected_artifact_files":["model.safetensors"],
            "import_state":"ready","validation_state":"valid","pipeline_tag":"text-generation","task_type_primary":"text_generation",
            "input_modalities":["text"],"output_modalities":["text"],"recommended_backend":"transformers","runtime_engine_hints":["transformers"]
        }),
    )?;
    Ok(())
}

#[derive(Default)]
struct Sessions {
    graphs: Vec<String>,
    execution: Option<String>,
}
impl Sessions {
    async fn close(&self, runtime: &EmbeddedRuntime, root: &Path) -> ProbeResult<()> {
        let mut results = Vec::new();
        if let Some(session_id) = &self.execution {
            let result = runtime
                .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
                    session_id: session_id.clone(),
                })
                .await;
            results.push(json!({"kind":"execution","ok":result.is_ok(),"error":result.err().map(|e|e.to_string())}));
        }
        for session_id in self.graphs.iter().rev() {
            let result = runtime
                .workflow_service()
                .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                    session_id: session_id.clone(),
                })
                .await;
            results.push(json!({"kind":"graph","ok":result.is_ok(),"error":result.err().map(|e|e.to_string())}));
        }
        save(root, "session-close.json", &results)?;
        if results.iter().any(|r| r["ok"] != true) {
            return Err("session cleanup failed".into());
        }
        Ok(())
    }
}
async fn create_runtime(
    root: &Path,
) -> ProbeResult<(
    EmbeddedRuntime,
    Arc<pumas_library::PumasApi>,
    Arc<RuntimeRegistry>,
)> {
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
    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(inference::backend::pytorch::PyTorchBackend::new()),
        "PyTorch",
    ));
    let service = WorkflowService::new()
        .with_attribution_store(
            pantograph_runtime_attribution::SqliteAttributionStore::open(
                root.join("attribution.db"),
            )?,
        )
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open(root.join("diagnostics.db"))?)
        .with_artifact_store(ArtifactStore::open(
            root.join("artifacts"),
            ArtifactPolicy {
                policy_id: "cpu-text-probe".into(),
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
            Some(registry.clone()),
            None,
        )
        .with_dependency_readiness_snapshot_producer(
            composition.dependency_readiness_snapshot_producer,
        ),
        api,
        registry,
    ))
}

async fn author_workflow(
    runtime: &EmbeddedRuntime,
    root: &Path,
    stage: &mut &str,
    sessions: &mut Sessions,
) -> ProbeResult<String> {
    *stage = "graph_authoring";
    let graph: WorkflowGraph = serde_json::from_value(json!({"nodes": [
        {"id":"prompt", "node_type":"text-input", "position":{"x":0,"y":100}, "data":{"text":PROMPT}},
        {"id":"infer", "node_type":"llm-inference", "position":{"x":300,"y":100}, "data":{
            "label":"CPU Text", "modelName":"Local Text",
            "task_kind":"text_generation", "runtime":"pytorch", "device":"cpu",
            "runtime_source_context":{"operation_type":"text_generation.prompt", "context_shape_key":"text.single-prompt", "cancellation_mode":"run_scoped"},
            "pumas_model_ref":{"model_id":MODEL_ID, "selected_artifact_id":"main"}}},
        {"id":"text-result", "node_type":"text-output", "position":{"x":740,"y":100}, "data":{}},
        {"id":"token-cap", "node_type":"number-input", "position":{"x":0,"y":250}, "data":{"value":MAX_NEW_TOKENS}},
        {"id":"temperature", "node_type":"selection-input", "position":{"x":0,"y":350}, "data":{"value":TEMPERATURE}},
        {"id":"deps", "node_type":"dependency-environment", "position":{"x":300,"y":450}, "data":{"mode":"manual"}}
    ], "edges": [
        {"id":"text-to-infer", "source":"prompt", "source_handle":"text", "target":"infer", "target_handle":"prompt"},
        {"id":"cap-to-infer", "source":"token-cap", "source_handle":"value", "target":"infer", "target_handle":"max_new_tokens"},
        {"id":"temperature-to-infer", "source":"temperature", "source_handle":"value", "target":"infer", "target_handle":"temperature"},
        {"id":"infer-to-text", "source":"infer", "source_handle":"text", "target":"text-result", "target_handle":"text"},
        {"id":"deps-to-infer", "source":"deps", "source_handle":"dependency_environment_sidecar", "target":"infer", "target_handle":"dependency_environment_sidecar"}
    ]}))?;
    author_graph(runtime, root, stage, graph, sessions).await
}

async fn author_graph(
    runtime: &EmbeddedRuntime,
    root: &Path,
    stage: &mut &str,
    mut graph: WorkflowGraph,
    sessions: &mut Sessions,
) -> ProbeResult<String> {
    let service = runtime.workflow_service();
    let registry = NodeRegistry::new();
    for node in &mut graph.nodes {
        if node.node_type != "llm-inference" {
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
    sessions.graphs.push(preview.session_id.clone());
    let validation = service
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: preview.session_id.clone(),
                graph_revision: preview.graph_revision.parse()?,
            },
        )
        .await?;
    save(root, "descriptor-validation.json", &validation)?;
    for node in graph
        .nodes
        .iter_mut()
        .filter(|node| node.node_type == "llm-inference")
    {
        let projection = validation
            .node_projections
            .iter()
            .find(|projection| projection.node_id.as_str() == node.id)
            .ok_or("backend did not return an inference descriptor")?;
        node.data["inference_interface_snapshot"] =
            serde_json::to_value(authored_snapshot_from_descriptor(&projection.descriptor)?)?;
    }
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
    sessions.graphs.push(session.session_id.clone());
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
    for action_kind in [
        DependencyEnvironmentAction::Resolve,
        DependencyEnvironmentAction::Check,
    ] {
        let dependency_node = "deps";
        let action = service
            .workflow_graph_resolve_dependency_environment_action_intent(
                DependencyEnvironmentActionIntent {
                    contract_version: 1,
                    graph_session_id: session.session_id.parse()?,
                    graph_revision: session.graph_revision.parse()?,
                    validation_session_id: Some(validation_id.clone()),
                    target_node_id: dependency_node.parse()?,
                    action: action_kind,
                },
            )
            .await?;
        save(root, &format!("dependency-{action_kind:?}.json"), &action)?;
        if action.status != DependencyEnvironmentActionIntentStatus::RequestReady {
            return Err(
                "backend dependency action is blocking; inspect dependency-Resolve.json and dependency-Check.json".into(),
            );
        }
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

fn readiness_pending(error: &WorkflowServiceError) -> bool {
    match error {
        WorkflowServiceError::RuntimeDependencyReadinessPending { .. } => true,
        WorkflowServiceError::WithDiagnostics { source, .. }
        | WorkflowServiceError::WithRuntimeDiagnosticPhase { source, .. } => {
            readiness_pending(source)
        }
        _ => false,
    }
}

// The public ledger owns these observations. Never infer cancellation from a generic run error.
fn observe_negative(
    root: &Path,
    mode: &str,
    run_id: &str,
    cancellation_attempt: Option<&str>,
) -> ProbeResult<Value> {
    let events = SqliteDiagnosticsLedger::open(root.join("diagnostics.db"))?
        .diagnostic_events_after(0, 500)?;
    if events.len() == 500 {
        return Err("negative evidence may be truncated".into());
    }
    let mut cancelled = false;
    let mut matched = Vec::new();
    let mut terminal_attempts = 0;
    for event in &events {
        if event.workflow_run_id.as_ref().map(|id| id.as_str()) != Some(run_id)
            || event.node_id.as_deref() != Some("infer")
        {
            continue;
        }
        match serde_json::from_str::<DiagnosticEventPayload>(&event.payload_json)? {
            DiagnosticEventPayload::NodeExecutionStatus(payload)
                if payload.status == NodeExecutionProjectionStatus::Cancelled =>
            {
                cancelled = true;
                matched.push(serde_json::to_value(event)?);
            }
            DiagnosticEventPayload::NodeExecutionStatus(payload)
                if payload.status == NodeExecutionProjectionStatus::Completed
                    || (mode == "cancel"
                        && payload.status == NodeExecutionProjectionStatus::Failed) =>
            {
                return Err("negative runtime observation contains completed node".into())
            }
            DiagnosticEventPayload::SchedulerTaskAttemptLifecycleChanged(payload)
                if payload.scheduler_task_id == "infer" =>
            {
                if matches!(
                    payload.transition,
                    SchedulerTaskAttemptLifecycleTransition::Started
                        | SchedulerTaskAttemptLifecycleTransition::Redispatched
                ) {
                    continue;
                }
                terminal_attempts += 1;
                let message = payload.error_summary.as_deref().unwrap_or("");
                let identified = if mode == "failure" {
                    message.contains("pytorch_worker_model_load_failed")
                        && message.contains("Error while deserializing header: header too large")
                } else {
                    cancellation_attempt == Some(payload.scheduler_attempt_id.as_str())
                        && (payload.transition
                            == SchedulerTaskAttemptLifecycleTransition::Cancelled
                            || message.contains("execution cancelled before completion"))
                };
                let terminal_expected = payload.transition
                    == SchedulerTaskAttemptLifecycleTransition::Failed
                    || (mode == "cancel"
                        && payload.transition
                            == SchedulerTaskAttemptLifecycleTransition::Cancelled);
                if !terminal_expected || payload.ended_at_ms.is_none() || !identified {
                    return Err("unexpected or contradictory runtime terminal evidence".into());
                }
                matched.push(serde_json::to_value(event)?);
            }
            _ => {}
        }
    }
    if terminal_attempts != 1
        || (mode == "cancel" && (!cancelled || cancellation_attempt.is_none()))
        || (mode == "failure" && cancelled)
    {
        return Err("missing identity-bound runtime failure/cancellation observation".into());
    }
    let observation =
        json!({"runtime_effect_observed":true,"workflow_run_id":run_id,"events":matched});
    save(root, "negative-runtime-observation.json", &observation)?;
    Ok(observation)
}

async fn execute(
    runtime: &EmbeddedRuntime,
    root: &Path,
    stage: &mut &str,
    sessions: &mut Sessions,
    mode: &str,
) -> ProbeResult<Value> {
    author_workflow(runtime, root, stage, sessions).await?;
    if mode == "failure" {
        // Damage only this disposable copy after package/graph resolution.
        std::fs::write(
            root.join("shared-resources/models")
                .join(MODEL_ID)
                .join("model.safetensors"),
            b"invalid safetensors qualification fixture",
        )?;
    }
    *stage = "session_create";
    let session = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: WORKFLOW_ID.into(),
            usage_profile: None,
            keep_alive: false,
        })
        .await?;
    sessions.execution = Some(session.session_id.clone());
    *stage = "scheduler_dispatch";
    let mut result = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id.clone(),
            workflow_semantic_version: "1.0.0".into(),
            inputs: vec![
                WorkflowPortBinding {
                    node_id: "prompt".into(),
                    port_id: "text".into(),
                    value: json!(PROMPT),
                },
                WorkflowPortBinding {
                    node_id: "token-cap".into(),
                    port_id: "value".into(),
                    value: json!(MAX_NEW_TOKENS),
                },
                WorkflowPortBinding {
                    node_id: "temperature".into(),
                    port_id: "value".into(),
                    value: json!(TEMPERATURE),
                },
            ],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-result".into(),
                port_id: "text".into(),
            }]),
            override_selection: None,
            timeout_ms: Some(60000),
            priority: None,
        })
        .await;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut resumes = Vec::new();
    let mut cancellation_observed = false;
    let mut cancellation_attempt = None;
    while result.as_ref().err().is_some_and(readiness_pending) {
        if tokio::time::Instant::now() >= deadline {
            return Err("readiness deadline exceeded".into());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
        let requests = runtime
            .workflow_service()
            .workflow_execution_session_runtime_dependency_readiness_resume_candidates()?;
        if let Some(request) = requests
            .into_iter()
            .find(|r| r.session_id == session.session_id)
        {
            save(root, "readiness-resume-request.json", &request)?;
            resumes.push(request.workflow_run_id.clone());
            let resume =
                runtime.resume_workflow_execution_session_runtime_dependency_readiness(request);
            tokio::pin!(resume);
            result = loop {
                tokio::select! {
                    outcome = &mut resume => break outcome,
                    _ = tokio::time::sleep(Duration::from_millis(1)), if mode == "cancel" && !cancellation_observed => {
                        let active = runtime.workflow_service().workflow_execution_session_bootstrap_recovery_report()?;
                        if let Some(run) = active.active_runs.iter().find(|run|run.session_id==session.session_id) {
                            if let Some(task) = run.runtime_tasks.iter().find(|task|task.state_kind==Some(pantograph_scheduler::SchedulerTaskStateKind::Running)) {
                                save(root,"running-before-cancel.json",&active)?;
                                let cancelled = runtime.workflow_service().workflow_cancel_active_execution_session_task(WorkflowExecutionSessionActiveTaskCancelRequest {
                                    session_id:session.session_id.clone(),workflow_run_id:run.workflow_run_id.clone(),task_id:task.task_id.clone(),reason:Some("bounded CPU text probe cancellation".into()),
                                }).await?;
                                save(root,"cancellation-request.json",&cancelled)?;
                                cancellation_observed = cancelled.ok;
                                cancellation_attempt = Some(cancelled.active_attempt_id);
                            }
                        }
                    }
                }
            };
        } else {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
    save(root, "readiness-resumed-runs.json", &resumes)?;
    let scheduler = runtime
        .workflow_service()
        .workflow_get_scheduler_snapshot(serde_json::from_value(
            json!({"session_id": session.session_id}),
        )?)
        .await?;
    save(root, "scheduler-after-run.json", &scheduler)?;
    match result {
        Err(error) if matches!(mode, "failure" | "cancel") => {
            if !scheduler.items.is_empty() {
                return Err("terminal run still retained in session queue".into());
            }
            if mode == "cancel" && !cancellation_observed {
                return Err("runtime cancellation was not acknowledged".into());
            }
            let run_id = resumes
                .first()
                .ok_or("missing original workflow run identity")?;
            if resumes.iter().any(|id| id != run_id) {
                return Err("readiness resume changed run identity".into());
            }
            let observation =
                observe_negative(root, mode, run_id, cancellation_attempt.as_deref())?;
            let public_projection = if mode == "cancel" {
                error.code() == pantograph_workflow_service::WorkflowErrorCode::Cancelled
            } else {
                error.runtime_diagnostic_phase_hint()==Some(pantograph_workflow_service::WorkflowRuntimeDiagnosticPhaseHint::RuntimeModelLoad) && error.to_string().contains("pytorch_worker_model_load_failed")
            };
            let public_error = json!({"code":format!("{:?}",error.code()),"phase":format!("{:?}",error.runtime_diagnostic_phase_hint()),"message":error.to_string()});
            save(root, "public-negative-result.json", &public_error)?;
            Ok(
                json!({"scope":"runtime_effect_observation_only","runtime_effect_observed":true,"public_result_projection_qualified":public_projection,"public_error":public_error,"observation":observation}),
            )
        }
        Err(error) => Err(error.into()),
        Ok(response) if mode == "generate" => {
            save(root, "response.json", &response)?;
            let text = response
                .outputs
                .iter()
                .find(|p| p.node_id == "text-result" && p.port_id == "text")
                .and_then(|p| p.value.as_str())
                .ok_or("missing text completion output")?;
            if text.trim().is_empty() {
                return Err("empty text completion".into());
            }
            *stage = "output_verified";
            Ok(
                json!({"scope":"native_cpu_text_integration","text":text,"runtime_effect_observed":true,"public_result_projection_qualified":true}),
            )
        }
        Ok(_) => Err("requested failure/cancellation scenario completed instead".into()),
    }
}

#[tokio::main]
async fn main() -> ProbeResult<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 || !matches!(args[2].as_str(), "generate" | "failure" | "cancel") {
        return Err("usage: cpu_text_generation_workflow_probe <new absolute evidence root> <absolute local model directory|--synthetic> <generate|failure|cancel>".into());
    }
    let root = PathBuf::from(&args[0]);
    if !root.is_absolute()
        || std::env::var_os("PUMAS_REGISTRY_DB_PATH")
            .map(PathBuf::from)
            .as_deref()
            != Some(root.join("registry.db").as_path())
    {
        return Err("set PUMAS_REGISTRY_DB_PATH=<new absolute evidence root>/registry.db; existing catalogs are never opened".into());
    }
    let synthetic = args[1] == "--synthetic";
    let source = if synthetic {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../inference/tests/fixtures/tiny_chat_gpt2/model-11")
    } else {
        PathBuf::from(&args[1])
    };
    std::fs::create_dir(&root)?;
    seed_model(&root, &source, synthetic)?;
    save(
        &root,
        "linked-package-observation.json",
        &inference::backend::pytorch::observe_text_package_presence()?,
    )?;
    let (runtime, _library, registry) = create_runtime(&root).await?;
    let mut sessions = Sessions::default();
    let mut stage = "initialization";
    let result = tokio::time::timeout(
        Duration::from_secs(120),
        execute(&runtime, &root, &mut stage, &mut sessions, &args[2]),
    )
    .await;
    let outcome_save = save(
        &root,
        "execution-outcome.json",
        &match &result {
            Ok(Ok(value)) => value.clone(),
            Ok(Err(error)) => json!({"code":format!("{:?}",error),"error":error.to_string()}),
            Err(_) => json!({"error":"watchdog expired"}),
        },
    );
    // A watchdog expiry is a failure, never cancellation evidence. Always run owner cleanup.
    let close =
        tokio::time::timeout(Duration::from_secs(30), sessions.close(&runtime, &root)).await;
    let shutdown = tokio::time::timeout(Duration::from_secs(30), runtime.shutdown()).await;
    outcome_save?;
    let registry_after = registry.snapshot();
    save(&root, "registry-after-shutdown.json", &registry_after)?;
    let cleanup_ok = matches!(close, Ok(Ok(())))
        && matches!(shutdown, Ok(Ok(())))
        && registry_after.reservations.is_empty()
        && registry_after.runtimes.iter().all(|runtime| {
            matches!(
                runtime.status,
                pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
                    | pantograph_runtime_registry::RuntimeRegistryStatus::Failed
            ) && runtime.models.is_empty()
                && runtime.runtime_instance_id.is_none()
                && runtime.active_reservation_ids.is_empty()
                && runtime.active_reservation_claims.is_empty()
                && !runtime.resident_resources_uncertain
                && runtime.model_resource_residency.is_none()
        });
    let events = SqliteDiagnosticsLedger::open(root.join("diagnostics.db"))?
        .diagnostic_events_after(0, 500)?;
    save(&root, "diagnostics.json", &events)?;
    let output = match &result {
        Ok(Ok(value)) => value.clone(),
        Ok(Err(error)) => json!({"error":error.to_string()}),
        Err(_) => json!({"error":"probe watchdog expired"}),
    };
    let passed = matches!(result, Ok(Ok(_))) && cleanup_ok;
    let public_projection = output["public_result_projection_qualified"] == true;
    let report = json!({"probe_passed":passed,"runtime_effect_observed":matches!(result,Ok(Ok(_))),"public_result_projection_qualified":public_projection,"integration_passed":passed && public_projection,"stage":stage,"scenario":args[2],"synthetic":synthetic,"t3_qualified":false,
        "cleanup_ok":cleanup_ok,"shutdown_ok":matches!(shutdown,Ok(Ok(()))),"output":output,"diagnostics_may_be_truncated":events.len()==500,
        "limits":{"device":"cpu","max_new_tokens":MAX_NEW_TOKENS,"temperature":TEMPERATURE,"max_model_bytes":600*1024*1024,"timeout_ms":60000,"max_loaded_sessions":1}});
    save(&root, "result.json", &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !passed {
        return Err("probe failed; evidence retained in supplied directory".into());
    }
    Ok(())
}

//! Loopback command adapter to the real embedded runtime. No replacement executor.
use super::*;
use pantograph_workflow_service::*;
use std::io::{BufRead, Write};

struct BrowserHost<'a> {
    runtime: &'a EmbeddedRuntime,
    library: Arc<pumas_library::PumasApi>,
    root: &'a Path,
    rich: bool,
}

fn required_string<'a>(args: &'a Value, key: &str) -> ProbeResult<&'a str> {
    args[key]
        .as_str()
        .ok_or_else(|| format!("{key} is required").into())
}

fn workflow_id(rich: bool) -> &'static str {
    if rich {
        rich_workflow::WORKFLOW
    } else {
        WORKFLOW_ID
    }
}

fn require_workflow(id: &str, rich: bool) -> ProbeResult<()> {
    if id != workflow_id(rich) {
        return Err(WorkflowServiceError::CapabilityViolation(
            "This host owns only the committed CPU embedding fixture workflow".into(),
        )
        .into());
    }
    Ok(())
}

fn graph_path(path: &str, rich: bool) -> ProbeResult<String> {
    let id = workflow_id(rich);
    if path == id || path == format!(".pantograph/workflows/{id}.json") {
        Ok(format!(".pantograph/workflows/{id}.json"))
    } else {
        Err(WorkflowServiceError::CapabilityViolation(
            "Workflow path is outside this fixture host".into(),
        )
        .into())
    }
}

fn require_fixture_graph(graph: &WorkflowGraph, rich: bool) -> ProbeResult<()> {
    if rich {
        return rich_workflow::validate(graph)
            .map_err(|error| WorkflowServiceError::CapabilityViolation(error.to_string()).into());
    }
    let expected = [
        ("prompt", "text-input"),
        ("infer", "llm-inference"),
        ("vectors", "vector-output"),
        ("deps", "dependency-environment"),
    ];
    if graph.nodes.len() != expected.len()
        || expected.iter().any(|(id, kind)| {
            !graph
                .nodes
                .iter()
                .any(|node| node.id == *id && node.node_type == *kind)
        })
    {
        return Err(WorkflowServiceError::CapabilityViolation(
            "Only fixture nodes are supported".into(),
        )
        .into());
    }
    let infer = &graph
        .nodes
        .iter()
        .find(|node| node.id == "infer")
        .unwrap()
        .data;
    if infer["pumas_model_ref"]["model_id"] != MODEL_ID
        || infer["pumas_model_ref"]["selected_artifact_id"] != "main"
        || infer["runtime"] != "candle"
        || infer["device"] != "cpu"
        || infer["task_kind"] != "embedding"
    {
        return Err(WorkflowServiceError::CapabilityViolation(
            "Only the untrained BERT-8 Candle CPU binding is supported".into(),
        )
        .into());
    }
    Ok(())
}

impl BrowserHost<'_> {
    fn refresh_projections(&self) -> ProbeResult<()> {
        use WorkflowDiagnosticsProjectionKind::*;
        let response = self
            .runtime
            .workflow_service()
            .workflow_diagnostics_projection_refresh(
                WorkflowDiagnosticsProjectionRefreshRequest {
                    projections: vec![
                        RunList,
                        RunDetail,
                        NodeStatus,
                        SchedulerTimeline,
                        IoArtifact,
                        LibraryUsage,
                    ],
                    workflow_run_id: None,
                    workflow_id: None,
                    reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
                    batch_size: 500,
                },
            )?;
        if !response.failed.is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "Projection refresh failed: {:?}",
                response.failed
            ))
            .into());
        }
        Ok(())
    }

    async fn run(&self, request: WorkflowExecutionSessionRunRequest) -> ProbeResult<Value> {
        let session_id = request.session_id.clone();
        let started = Instant::now();
        let result = tokio::time::timeout(Duration::from_secs(20), async {
            let mut result = self.runtime.run_workflow_execution_session(request).await;
            while matches!(
                result,
                Err(WorkflowServiceError::RuntimeDependencyReadinessPending { .. })
            ) {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let candidates = self
                    .runtime
                    .workflow_service()
                    .workflow_execution_session_runtime_dependency_readiness_resume_candidates()?;
                if let Some(request) = candidates
                    .into_iter()
                    .find(|item| item.session_id == session_id)
                {
                    result = self
                        .runtime
                        .resume_workflow_execution_session_runtime_dependency_readiness(request)
                        .await;
                }
            }
            result
        })
        .await?;
        self.refresh_projections()?;
        let response = result?;
        save(
            self.root,
            "browser-last-run.json",
            &json!({
                "scope":"live_browser_host_native_result", "model_training":"untrained",
                "backend":"candle", "backends":vec!["candle"],
                "model_ids":if self.rich { vec![MODEL_ID, rich_workflow::SECOND_MODEL] } else { vec![MODEL_ID] },
                "device":"cpu_f32", "model_id":MODEL_ID, "image_generation_executed":false,
                "model_acquisition_bytes":0, "submit_to_response_wall_ms":started.elapsed().as_secs_f64()*1000.0,
                "response":response,
            }),
        )?;
        Ok(serde_json::to_value(response)?)
    }

    async fn command(&self, command: &str, args: Value) -> ProbeResult<Value> {
        let service = self.runtime.workflow_service();
        let request = args.get("request").cloned().unwrap_or_else(|| json!({}));
        let store = FileSystemWorkflowGraphStore::new(self.root);
        self.refresh_projections()?;
        macro_rules! query {
            ($method:ident) => {
                Ok(serde_json::to_value(
                    service.$method(serde_json::from_value(request)?)?,
                )?)
            };
        }
        macro_rules! action {
            ($method:ident) => {
                Ok(serde_json::to_value(
                    service.$method(serde_json::from_value(request)?).await?,
                )?)
            };
        }
        match command {
            "workflow_browser_host_capabilities" => Ok(json!({"model_library_read":true,
                "model_library_write":false,"model_search":false,"graph_edit":true,
                "description": if self.rich { "Untrained BERT-8/BERT-12 CPU fixtures: text composition and two embedding branches. Text generation unqualified; image unavailable. Zero model downloads." } else { "Untrained BERT-8 fixture: edit prompt, save and submit through the real Candle CPU scheduler. No model downloads." }})),
            "get_node_definitions" => {
                Ok(serde_json::to_value(NodeRegistry::new().all_definitions())?)
            }
            "list_workflows" => Ok(serde_json::to_value(
                service.workflow_graph_list(&store)?.workflows,
            )?),
            "load_workflow" => Ok(serde_json::to_value(service.workflow_graph_load(
                &store,
                WorkflowGraphLoadRequest {
                    path: graph_path(required_string(&args, "path")?, self.rich)?,
                },
            )?)?),
            "save_workflow" => {
                require_workflow(required_string(&args, "name")?, self.rich)?;
                let graph = serde_json::from_value(args["graph"].clone())?;
                require_fixture_graph(&graph, self.rich)?;
                Ok(json!(
                    service
                        .workflow_graph_save(
                            &store,
                            WorkflowGraphSaveRequest {
                                name: workflow_id(self.rich).into(),
                                graph
                            }
                        )?
                        .path
                ))
            }
            "create_workflow_execution_session" => {
                require_workflow(required_string(&args, "workflowId")?, self.rich)?;
                let graph = serde_json::from_value(args["graph"].clone())?;
                require_fixture_graph(&graph, self.rich)?;
                Ok(serde_json::to_value(
                    service
                        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                            graph,
                            workflow_id: Some(workflow_id(self.rich).into()),
                        })
                        .await?,
                )?)
            }
            "remove_execution" => {
                service
                    .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                        session_id: required_string(&args, "executionId")?.into(),
                    })
                    .await?;
                Ok(Value::Null)
            }
            "get_execution_graph" => Ok(serde_json::to_value(
                service
                    .workflow_graph_get_edit_session_graph(WorkflowGraphEditSessionGraphRequest {
                        session_id: required_string(&args, "executionId")?.into(),
                    })
                    .await?
                    .graph,
            )?),
            "get_undo_redo_state" => Ok(serde_json::to_value(
                service
                    .workflow_graph_get_undo_redo_state(WorkflowGraphUndoRedoStateRequest {
                        session_id: required_string(&args, "executionId")?.into(),
                    })
                    .await?,
            )?),
            "update_node_data" => {
                let data = args["data"].as_object().ok_or("data must be an object")?;
                let prompt = (args["nodeId"] == "prompt"
                    || (self.rich && args["nodeId"] == "context"))
                    && data.keys().all(|key| key == "text")
                    && data.get("text").is_some_and(Value::is_string);
                // These are ordinary authored UI fields. Readiness remains an
                // owner-produced service fact; authored status never admits work.
                let dependency_state = (args["nodeId"] == "deps"
                    || (self.rich && args["nodeId"] == "context-deps"))
                    && data.keys().all(|key| {
                        matches!(
                            key.as_str(),
                            "mode"
                                | "selected_binding_ids"
                                | "dependency_requirements"
                                | "dependency_status"
                                | "environment_ref"
                                | "manual_overrides"
                                | "dependency_override_patches"
                                | "activity_log"
                        )
                    });
                if !prompt && !dependency_state {
                    return Err(WorkflowServiceError::CapabilityViolation(
                        "This fixture host only edits prompt and dependency UI state".into(),
                    )
                    .into());
                }
                Ok(serde_json::to_value(
                    service
                        .workflow_graph_update_node_data(WorkflowGraphUpdateNodeDataRequest {
                            session_id: required_string(&args, "executionId")?.into(),
                            node_id: required_string(&args, "nodeId")?.into(),
                            data: args["data"].clone(),
                        })
                        .await?,
                )?)
            }
            "update_node_position_in_execution" => Ok(serde_json::to_value(
                service
                    .workflow_graph_update_node_position(WorkflowGraphUpdateNodePositionRequest {
                        session_id: required_string(&args, "executionId")?.into(),
                        node_id: required_string(&args, "nodeId")?.into(),
                        position: serde_json::from_value(args["position"].clone())?,
                    })
                    .await?,
            )?),
            "current_graph_validation_summary" => {
                action!(workflow_graph_current_validation_summary)
            }
            "current_graph_validation_projection" => {
                action!(workflow_graph_current_validation_projection)
            }
            "refresh_current_graph_validation_summary" => {
                action!(workflow_graph_refresh_current_validation_summary)
            }
            "graph_validation_lifecycle_event_snapshot" => Ok(serde_json::to_value(
                service
                    .workflow_graph_validation_lifecycle_event_snapshot(required_string(
                        &args,
                        "graphSessionId",
                    )?)
                    .await?,
            )?),
            "resolve_dependency_environment_action_intent" => {
                if !matches!(request["action"].as_str(), Some("resolve" | "check")) {
                    return Err(WorkflowServiceError::CapabilityViolation(
                        "The fixture host supports dependency Resolve and Check only".into(),
                    )
                    .into());
                }
                action!(workflow_graph_resolve_dependency_environment_action_intent)
            }
            "publish_graph_session_executable_validation_snapshot" => {
                let request: WorkflowGraphSessionExecutableValidationSnapshotPublishRequest =
                    serde_json::from_value(request)?;
                require_workflow(&request.workflow_id, self.rich)?;
                Ok(serde_json::to_value(
                    service
                        .publish_graph_session_executable_validation_snapshot(request)
                        .await?
                        .as_record(),
                )?)
            }
            "workflow_create_execution_session" => {
                let request: WorkflowExecutionSessionCreateRequest =
                    serde_json::from_value(request)?;
                require_workflow(&request.workflow_id, self.rich)?;
                Ok(serde_json::to_value(
                    self.runtime
                        .create_workflow_execution_session(request)
                        .await?,
                )?)
            }
            "workflow_run_execution_session" => self.run(serde_json::from_value(request)?).await,
            "workflow_close_execution_session" => {
                let response = self
                    .runtime
                    .close_workflow_execution_session(serde_json::from_value(request)?)
                    .await?;
                save(self.root, "browser-session-close.json", &response)?;
                Ok(serde_json::to_value(response)?)
            }
            "workflow_run_list_query" => query!(workflow_run_list_query),
            "workflow_run_detail_query" => query!(workflow_run_detail_query),
            "workflow_run_inspection_query" => query!(workflow_run_inspection_query),
            "workflow_node_status_query" => query!(workflow_node_status_query),
            "workflow_scheduler_timeline_query" => query!(workflow_scheduler_timeline_query),
            "workflow_io_artifact_query" => query!(workflow_io_artifact_query),
            "workflow_library_usage_query" => query!(workflow_library_usage_query),
            "workflow_run_graph_query" => query!(workflow_run_graph_query),
            "workflow_scheduler_estimate_query" => query!(workflow_scheduler_estimate_query),
            "workflow_local_network_status_query" => query!(workflow_local_network_status_query),
            "workflow_projection_rebuild" => query!(workflow_projection_rebuild),
            "workflow_artifact_descriptor" => query!(artifact_descriptor),
            "workflow_read_artifact_body" => query!(read_artifact_body),
            "query_port_options" => self.port_options(args).await,
            "list_model_library_updates_since" => Ok(serde_json::to_value(
                self.library
                    .list_model_library_updates_since(
                        args["cursor"].as_str(),
                        args["limit"].as_u64().unwrap_or(100).min(250) as usize,
                    )
                    .await?,
            )?),
            _ => Err(WorkflowServiceError::CapabilityViolation(format!(
                "Command '{command}' is unavailable in the CPU fixture host"
            ))
            .into()),
        }
    }

    async fn port_options(&self, args: Value) -> ProbeResult<Value> {
        let node_type = required_string(&args, "nodeType")?;
        let port_id = required_string(&args, "portId")?;
        if node_type != "puma-lib" || port_id != "pumas_model_ref" {
            return Err(WorkflowServiceError::CapabilityViolation(
                "Only the isolated Pumas selector is exposed".into(),
            )
            .into());
        }
        let mut extensions = node_engine::ExecutorExtensions::new();
        extensions.set(node_engine::extension_keys::PUMAS_API, self.library.clone());
        extensions.set(
            workflow_nodes::setup::PUMAS_SELECTOR_ACCESS,
            Arc::new(PumasSelectorAccess::Owner(self.library.clone())),
        );
        let query = node_engine::PortOptionsQuery {
            search: args["search"].as_str().map(str::to_owned),
            limit: Some(args["limit"].as_u64().unwrap_or(250).min(250) as usize),
            offset: args["offset"].as_u64().map(|value| value as usize),
            context: args
                .get("context")
                .filter(|v| !v.is_null())
                .map(|v| serde_json::from_value(v.clone()))
                .transpose()?,
        };
        Ok(serde_json::to_value(
            node_engine::NodeRegistry::with_builtins()
                .query_port_options(node_type, port_id, &query, &extensions)
                .await
                .map_err(|error| WorkflowServiceError::Internal(error.to_string()))?,
        )?)
    }
}

pub(super) async fn serve(
    runtime: EmbeddedRuntime,
    library: Arc<pumas_library::PumasApi>,
    root: &Path,
    rich: bool,
) -> ProbeResult<()> {
    let mut stage = "startup";
    let initial = if rich {
        super::author_graph(
            &runtime,
            root,
            &mut stage,
            rich_workflow::graph()?,
            rich_workflow::WORKFLOW,
            &["context-deps", "deps"],
            false,
        )
        .await?
    } else {
        super::author_workflow(&runtime, root, &mut stage, false).await?
    };
    runtime
        .workflow_service()
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: initial,
        })
        .await?;
    let host = BrowserHost {
        runtime: &runtime,
        library,
        root,
        rich,
    };
    for line in std::io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line?)?;
        let result = host
            .command(
                request["command"].as_str().unwrap_or(""),
                request.get("args").cloned().unwrap_or_else(|| json!({})),
            )
            .await;
        let response = match result {
            Ok(value) => json!({"ok":true,"value":value}),
            Err(error) => {
                let details = if let Some(error) = error.downcast_ref::<WorkflowServiceError>() {
                    json!({"code":error.code(),"message":error.message(),"details":error.details(),"diagnostics":error.diagnostics()})
                } else {
                    json!({"code":"invalid_request","message":error.to_string(),"details":null,"diagnostics":null})
                };
                json!({"ok":false,"error":details})
            }
        };
        println!("{}", serde_json::to_string(&response)?);
        std::io::stdout().flush()?;
    }
    let shutdown = runtime.shutdown().await;
    save(
        root,
        "browser-shutdown.json",
        &json!({"ok":shutdown.is_ok(),"error":shutdown.as_ref().err().map(ToString::to_string)}),
    )?;
    shutdown?;
    Ok(())
}

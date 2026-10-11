//! Explicit read-only application commands for the loopback demonstration host.
use super::*;
use serde_json::Value;

impl DemoRuntime {
    pub(super) async fn dispatch(&self, request: Value) -> Value {
        let command = request["command"].as_str().unwrap_or("");
        let args = request.get("args").cloned().unwrap_or_else(|| json!({}));
        let result = if command == "run" {
            self.run(args).await
        } else {
            self.command(command, args).await
        };
        match result {
            Ok(value) => json!({"ok": true, "value": value}),
            Err(error) => {
                let details = if let Some(error) = error.downcast_ref::<WorkflowServiceError>() {
                    json!({"code": error.code(), "message": error.message(),
                        "details": error.details(), "diagnostics": error.diagnostics()})
                } else {
                    json!({"code": "invalid_request", "message": error.to_string(), "details": null, "diagnostics": null})
                };
                json!({"ok": false, "error": details})
            }
        }
    }

    fn refresh_projections(&self) -> DemoResult<()> {
        use WorkflowDiagnosticsProjectionKind::*;
        let response = self.service.workflow_diagnostics_projection_refresh(
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

    async fn command(&self, command: &str, args: Value) -> DemoResult<Value> {
        let service = &self.service;
        let request = args.get("request").cloned().unwrap_or_else(|| json!({}));
        self.refresh_projections()?;
        macro_rules! query {
            ($method:ident) => {
                Ok(serde_json::to_value(
                    service.$method(serde_json::from_value(request)?)?,
                )?)
            };
        }
        match command {
            "workflow_browser_host_capabilities" => Ok(json!({
                "model_library_read": true, "model_library_write": false,
                "model_search": false, "graph_edit": false,
                "description": "Local demo host: real scheduler and diagnostics; isolated model library."
            })),
            "workflow_scheduler_estimate_query" => query!(workflow_scheduler_estimate_query),
            "list_workflows" => Ok(serde_json::to_value(
                service
                    .workflow_graph_list(&FileSystemWorkflowGraphStore::new(
                        self._directory.path(),
                    ))?
                    .workflows,
            )?),
            "load_workflow" => {
                let path = demo_graph_path(args["path"].as_str())?;
                Ok(serde_json::to_value(service.workflow_graph_load(
                    &FileSystemWorkflowGraphStore::new(self._directory.path()),
                    WorkflowGraphLoadRequest { path },
                )?)?)
            }
            "workflow_graph_inspect" => {
                let mut request: WorkflowGraphInspectionRequest = serde_json::from_value(request)?;
                request.path = demo_graph_path(Some(&request.path))?;
                Ok(serde_json::to_value(service.workflow_graph_inspect(
                    &FileSystemWorkflowGraphStore::new(self._directory.path()),
                    request,
                )?)?)
            }
            "workflow_run_list_query" => query!(workflow_run_list_query),
            "workflow_run_detail_query" => query!(workflow_run_detail_query),
            "workflow_run_inspection_query" => query!(workflow_run_inspection_query),
            "workflow_node_status_query" => query!(workflow_node_status_query),
            "workflow_scheduler_timeline_query" => query!(workflow_scheduler_timeline_query),
            "workflow_io_artifact_query" => query!(workflow_io_artifact_query),
            "workflow_library_usage_query" => query!(workflow_library_usage_query),
            "workflow_run_graph_query" => query!(workflow_run_graph_query),
            "workflow_projection_rebuild" => query!(workflow_projection_rebuild),
            "workflow_artifact_descriptor" => query!(artifact_descriptor),
            "workflow_read_artifact_body" => query!(read_artifact_body),
            "get_node_definitions" => {
                Ok(serde_json::to_value(NodeRegistry::new().all_definitions())?)
            }
            "get_node_definition" => Ok(serde_json::to_value(
                NodeRegistry::new()
                    .get_definition(args["nodeType"].as_str().ok_or("nodeType is required")?),
            )?),
            "query_port_options" => {
                let node_type = args["nodeType"].as_str().ok_or("nodeType is required")?;
                let port_id = args["portId"].as_str().ok_or("portId is required")?;
                if node_type != "puma-lib" || port_id != "pumas_model_ref" {
                    return Err(WorkflowServiceError::CapabilityViolation(
                        "This host exposes only the isolated model-library selector".into(),
                    )
                    .into());
                }
                let library = self
                    .library
                    .as_ref()
                    .ok_or("Isolated model library is unavailable")?;
                let mut extensions = node_engine::ExecutorExtensions::new();
                extensions.set(node_engine::extension_keys::PUMAS_API, library.clone());
                extensions.set(
                    workflow_nodes::setup::PUMAS_SELECTOR_ACCESS,
                    Arc::new(workflow_nodes::setup::PumasSelectorAccess::Owner(
                        library.clone(),
                    )),
                );
                let query = node_engine::PortOptionsQuery {
                    search: args["search"].as_str().map(str::to_owned),
                    limit: Some(args["limit"].as_u64().unwrap_or(250).min(250) as usize),
                    offset: args["offset"].as_u64().map(|value| value as usize),
                    context: args
                        .get("context")
                        .filter(|v| !v.is_null())
                        .map(|value| serde_json::from_value(value.clone()))
                        .transpose()?,
                };
                let options = node_engine::NodeRegistry::with_builtins()
                    .query_port_options(node_type, port_id, &query, &extensions)
                    .await
                    .map_err(|error| WorkflowServiceError::Internal(error.to_string()))?;
                service.workflow_library_asset_access_record(
                    WorkflowLibraryAssetAccessRecordRequest {
                        asset_id: "pumas://models".into(),
                        operation: LibraryAssetOperation::Access,
                        cache_status: Some(LibraryAssetCacheStatus::Unknown),
                        network_bytes: None,
                        source_instance_id: Some("loopback-model-selector".into()),
                    },
                )?;
                Ok(serde_json::to_value(options)?)
            }
            "list_model_library_updates_since" => Ok(serde_json::to_value(
                self.library
                    .as_ref()
                    .ok_or("Isolated model library is unavailable")?
                    .list_model_library_updates_since(
                        args["cursor"].as_str(),
                        args["limit"].as_u64().unwrap_or(100).min(250) as usize,
                    )
                    .await?,
            )?),
            _ => Err(WorkflowServiceError::CapabilityViolation(format!(
                "Command '{command}' is unavailable in the read-only loopback demo host"
            ))
            .into()),
        }
    }
}

fn demo_graph_path(path: Option<&str>) -> Result<String, WorkflowServiceError> {
    match path {
        Some("scheduler-demo" | ".pantograph/workflows/scheduler-demo.json") => {
            Ok(".pantograph/workflows/scheduler-demo.json".into())
        }
        _ => Err(WorkflowServiceError::CapabilityViolation(
            "Only the built-in scheduler-demo graph is available in this loopback host".into(),
        )),
    }
}

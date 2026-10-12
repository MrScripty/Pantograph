//! Thin adapters to the same graph-edit and scheduler-session APIs as the native host.
use super::*;
use serde_json::Value;

impl DemoRuntime {
    pub(super) async fn graph_command(
        &self,
        command: &str,
        args: Value,
    ) -> DemoResult<Option<Value>> {
        let service = &self.service;
        let request = args.get("request").cloned().unwrap_or_else(|| json!({}));
        let edit = edit_request(&args);
        macro_rules! graph_query {
            ($method:ident, $value:expr) => {
                Some(serde_json::to_value(
                    service.$method(serde_json::from_value($value)?).await?,
                )?)
            };
        }
        macro_rules! host_query {
            ($method:ident) => {
                Some(serde_json::to_value(
                    service
                        .$method(&self.host, serde_json::from_value(request)?)
                        .await?,
                )?)
            };
        }
        Ok(match command {
            "validate_workflow_connection" => Some(json!(validate_workflow_connection(
                &serde_json::from_value(args["sourceType"].clone())?,
                &serde_json::from_value(args["targetType"].clone())?,
            ))),
            "create_workflow_execution_session" => {
                let graph: WorkflowGraph = serde_json::from_value(args["graph"].clone())?;
                validate_demo_graph(&graph)?;
                Some(serde_json::to_value(
                    service
                        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                            graph,
                            workflow_id: args["workflowId"].as_str().map(str::to_owned),
                        })
                        .await?,
                )?)
            }
            "get_execution_graph" => Some(serde_json::to_value(
                service
                    .workflow_graph_get_edit_session_graph(serde_json::from_value(edit)?)
                    .await?
                    .graph,
            )?),
            "get_undo_redo_state" => {
                let state = service
                    .workflow_graph_get_undo_redo_state(serde_json::from_value(edit)?)
                    .await?;
                Some(serde_json::to_value(UndoRedoState {
                    can_undo: state.can_undo,
                    can_redo: state.can_redo,
                    undo_count: state.undo_count,
                })?)
            }
            "update_node_data" => graph_query!(workflow_graph_update_node_data, edit),
            "update_node_position_in_execution" => {
                graph_query!(workflow_graph_update_node_position, edit)
            }
            "undo_workflow" => graph_query!(workflow_graph_undo, edit),
            "redo_workflow" => graph_query!(workflow_graph_redo, edit),
            "remove_execution" => {
                service
                    .workflow_graph_close_edit_session(serde_json::from_value(edit)?)
                    .await?;
                Some(Value::Null)
            }
            "save_workflow" => {
                let name = args["name"].as_str().ok_or("name is required")?;
                super::commands::validate_demo_name(name)?;
                if name == "scheduler-demo" {
                    return Err(WorkflowServiceError::CapabilityViolation("scheduler-demo is the fixed JSON API example; save your edited graph under another name".into()).into());
                }
                let graph: WorkflowGraph = serde_json::from_value(args["graph"].clone())?;
                validate_demo_graph(&graph)?;
                service.workflow_graph_save(
                    &FileSystemWorkflowGraphStore::new(self._directory.path()),
                    WorkflowGraphSaveRequest {
                        name: name.into(),
                        graph,
                    },
                )?;
                Some(json!(format!(".pantograph/workflows/{name}.json")))
            }
            "current_graph_validation_summary" => {
                graph_query!(workflow_graph_current_validation_summary, request)
            }
            "current_graph_validation_projection" => {
                graph_query!(workflow_graph_current_validation_projection, request)
            }
            "refresh_current_graph_validation_summary" => {
                graph_query!(workflow_graph_refresh_current_validation_summary, request)
            }
            "start_current_graph_validation_task" => {
                graph_query!(workflow_graph_start_current_validation_task, request)
            }
            "graph_validation_lifecycle_event_snapshot" => Some(serde_json::to_value(
                service
                    .workflow_graph_validation_lifecycle_event_snapshot(
                        args["graphSessionId"]
                            .as_str()
                            .ok_or("graphSessionId is required")?,
                    )
                    .await?,
            )?),
            "publish_graph_session_executable_validation_snapshot" => Some(serde_json::to_value(
                service
                    .publish_graph_session_executable_validation_snapshot(serde_json::from_value(
                        request,
                    )?)
                    .await?
                    .into_inner(),
            )?),
            "workflow_create_execution_session" => host_query!(create_workflow_execution_session),
            "workflow_run_execution_session" => host_query!(run_workflow_execution_session),
            "workflow_close_execution_session" => host_query!(close_workflow_execution_session),
            "workflow_get_capabilities" => host_query!(workflow_get_capabilities),
            "workflow_get_execution_session_status" => {
                graph_query!(workflow_get_execution_session_status, request)
            }
            "workflow_list_execution_session_queue" => {
                graph_query!(workflow_list_execution_session_queue, request)
            }
            "workflow_get_scheduler_snapshot" => {
                graph_query!(workflow_get_scheduler_snapshot, request)
            }
            _ => None,
        })
    }
}

fn edit_request(args: &Value) -> Value {
    json!({"session_id": args["executionId"], "node_id": args["nodeId"], "data": args["data"], "position": args["position"]})
}

fn validate_demo_graph(graph: &WorkflowGraph) -> Result<(), WorkflowServiceError> {
    if graph.nodes.len() > 32 || graph.edges.len() > 64 {
        return Err(WorkflowServiceError::CapabilityViolation(
            "The loopback demo allows at most 32 nodes and 64 edges".into(),
        ));
    }
    for node in &graph.nodes {
        if !matches!(
            node.node_type.as_str(),
            "text-input" | "text-output" | "selection-input" | "json-filter"
        ) {
            return Err(WorkflowServiceError::CapabilityViolation(format!(
                "Node type '{}' is not supported by this deterministic loopback host",
                node.node_type
            )));
        }
    }
    Ok(())
}

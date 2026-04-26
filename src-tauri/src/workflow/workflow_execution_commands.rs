pub use super::workflow_edit_session::{
    add_edge_to_execution, add_node_to_execution, connect_anchors_in_execution,
    create_group_in_execution, create_workflow_execution_session, delete_selection_from_execution,
    get_connection_candidates, get_execution_graph, get_undo_redo_state,
    insert_node_and_connect_in_execution, insert_node_on_edge_in_execution,
    preview_node_insert_on_edge_in_execution, redo_workflow, remove_edge_from_execution,
    remove_edges_from_execution, remove_execution, remove_node_from_execution, undo_workflow,
    ungroup_in_execution, update_group_ports_in_execution, update_node_data,
    update_node_position_in_execution,
};
pub use super::workflow_execution_runtime::{
    RunWorkflowExecutionSessionInput, WorkflowEditSessionRunResponse,
    WorkflowExecutionRuntimeState, run_workflow_execution_session,
};

#[cfg(test)]
mod tests {
    use pantograph_embedded_runtime::workflow_runtime::RuntimeEventProjection;

    #[test]
    fn runtime_event_projection_keeps_backend_owned_model_targets() {
        let projection = RuntimeEventProjection {
            active_runtime_snapshot: inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("pytorch".to_string()),
                runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: false,
                last_error: None,
            },
            embedding_runtime_snapshot: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama_cpp_embedding".to_string()),
                runtime_instance_id: Some("embedding-1".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            trace_runtime_metrics: pantograph_workflow_service::WorkflowTraceRuntimeMetrics {
                runtime_id: Some("pytorch".to_string()),
                observed_runtime_ids: vec!["pytorch".to_string()],
                runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
                model_target: Some("/models/sidecar.safetensors".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            active_model_target: Some("/models/sidecar.safetensors".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
        };

        assert_eq!(
            projection.active_model_target.as_deref(),
            Some("/models/sidecar.safetensors")
        );
        assert_eq!(
            projection.embedding_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
        assert_eq!(
            projection
                .embedding_runtime_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.runtime_id.as_deref()),
            Some("llama_cpp_embedding")
        );
    }
}

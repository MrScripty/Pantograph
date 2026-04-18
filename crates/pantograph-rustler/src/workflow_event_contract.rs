use node_engine::{EventError, WorkflowEvent};

pub(crate) fn serialize_workflow_event_json(
    event: &WorkflowEvent,
) -> std::result::Result<String, EventError> {
    serde_json::to_string(event).map_err(|e| EventError {
        message: format!("Serialization error: {}", e),
    })
}

#[cfg(test)]
mod tests {
    use super::serialize_workflow_event_json;

    #[test]
    fn preserves_graph_modified_contract() {
        let json = serialize_workflow_event_json(&node_engine::WorkflowEvent::GraphModified {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            dirty_tasks: vec!["node-a".to_string(), "node-b".to_string()],
            occurred_at_ms: Some(123),
        })
        .expect("serialize graph-modified event");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("parse graph-modified event");

        assert_eq!(value["type"], "graphModified");
        assert_eq!(value["workflowId"], "wf-1");
        assert_eq!(value["executionId"], "exec-1");
        assert_eq!(value["dirtyTasks"], serde_json::json!(["node-a", "node-b"]));
    }

    #[test]
    fn preserves_waiting_for_input_contract() {
        let json = serialize_workflow_event_json(&node_engine::WorkflowEvent::WaitingForInput {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            task_id: "human-input-1".to_string(),
            prompt: Some("Need approval".to_string()),
            occurred_at_ms: Some(456),
        })
        .expect("serialize waiting-for-input event");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("parse waiting-for-input event");

        assert_eq!(value["type"], "waitingForInput");
        assert_eq!(value["workflowId"], "wf-1");
        assert_eq!(value["executionId"], "exec-1");
        assert_eq!(value["taskId"], "human-input-1");
        assert_eq!(value["prompt"], "Need approval");
    }
}

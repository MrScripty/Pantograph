use std::collections::HashMap;

use crate::error::Result;
use crate::events::{EventSink, WorkflowEvent, unix_timestamp_ms};
use crate::types::NodeId;

pub(super) fn emit_task_started(event_sink: &dyn EventSink, task_id: NodeId, execution_id: String) {
    let _ = event_sink.send(WorkflowEvent::TaskStarted {
        task_id,
        execution_id,
        occurred_at_ms: Some(unix_timestamp_ms()),
    });
}

pub(super) fn emit_waiting_for_input(
    event_sink: &dyn EventSink,
    workflow_id: String,
    execution_id: String,
    task_id: NodeId,
    prompt: Option<String>,
) {
    let _ = event_sink.send(WorkflowEvent::WaitingForInput {
        workflow_id,
        execution_id,
        task_id,
        prompt,
        occurred_at_ms: Some(unix_timestamp_ms()),
    });
}

pub(super) fn emit_task_completed(
    event_sink: &dyn EventSink,
    task_id: NodeId,
    execution_id: String,
    outputs: &HashMap<String, serde_json::Value>,
) -> Result<()> {
    let _ = event_sink.send(WorkflowEvent::TaskCompleted {
        task_id,
        execution_id,
        output: Some(serde_json::to_value(outputs)?),
        occurred_at_ms: Some(unix_timestamp_ms()),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::VecEventSink;

    #[test]
    fn emit_waiting_for_input_preserves_prompt_and_ids() {
        let sink = VecEventSink::new();

        emit_waiting_for_input(
            &sink,
            "workflow-a".to_string(),
            "exec-1".to_string(),
            "approval".to_string(),
            Some("Approve deployment?".to_string()),
        );

        let events = sink.events();
        assert!(matches!(
            events.as_slice(),
            [WorkflowEvent::WaitingForInput {
                workflow_id,
                execution_id,
                task_id,
                prompt: Some(prompt),
                ..
            }] if workflow_id == "workflow-a"
                && execution_id == "exec-1"
                && task_id == "approval"
                && prompt == "Approve deployment?"
        ));
    }

    #[test]
    fn emit_task_completed_serializes_outputs() {
        let sink = VecEventSink::new();
        let outputs = HashMap::from([("out".to_string(), serde_json::json!("value"))]);

        emit_task_completed(&sink, "node-a".to_string(), "exec-1".to_string(), &outputs)
            .expect("emit completed");

        let events = sink.events();
        assert!(matches!(
            events.as_slice(),
            [WorkflowEvent::TaskCompleted {
                task_id,
                execution_id,
                output: Some(output),
                ..
            }] if task_id == "node-a"
                && execution_id == "exec-1"
                && output == &serde_json::json!({ "out": "value" })
        ));
    }
}

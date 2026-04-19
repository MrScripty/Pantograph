use std::collections::HashMap;

use super::super::events::WorkflowEvent as TauriWorkflowEvent;

/// A value that flows through a port.
type PortValue = serde_json::Value;

pub(super) fn translated_execution_id(event: &TauriWorkflowEvent) -> &str {
    match event {
        TauriWorkflowEvent::Started { execution_id, .. }
        | TauriWorkflowEvent::NodeStarted { execution_id, .. }
        | TauriWorkflowEvent::NodeProgress { execution_id, .. }
        | TauriWorkflowEvent::NodeStream { execution_id, .. }
        | TauriWorkflowEvent::NodeCompleted { execution_id, .. }
        | TauriWorkflowEvent::NodeError { execution_id, .. }
        | TauriWorkflowEvent::Completed { execution_id, .. }
        | TauriWorkflowEvent::Failed { execution_id, .. }
        | TauriWorkflowEvent::Cancelled { execution_id, .. }
        | TauriWorkflowEvent::GraphModified { execution_id, .. }
        | TauriWorkflowEvent::WaitingForInput { execution_id, .. }
        | TauriWorkflowEvent::IncrementalExecutionStarted { execution_id, .. }
        | TauriWorkflowEvent::RuntimeSnapshot { execution_id, .. }
        | TauriWorkflowEvent::SchedulerSnapshot { execution_id, .. }
        | TauriWorkflowEvent::DiagnosticsSnapshot { execution_id, .. } => execution_id,
    }
}

pub(super) fn translate_node_event(event: node_engine::WorkflowEvent) -> TauriWorkflowEvent {
    match event {
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id,
            execution_id,
            ..
        } => TauriWorkflowEvent::Started {
            workflow_id,
            node_count: 0,
            execution_id,
        },

        node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id,
            execution_id,
            ..
        } => TauriWorkflowEvent::Completed {
            workflow_id,
            outputs: HashMap::new(),
            execution_id,
        },

        node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id,
            execution_id,
            error,
            ..
        } => TauriWorkflowEvent::Failed {
            workflow_id,
            error,
            execution_id,
        },

        node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id,
            execution_id,
            error,
            ..
        } => TauriWorkflowEvent::Cancelled {
            workflow_id,
            error,
            execution_id,
        },

        node_engine::WorkflowEvent::TaskStarted {
            task_id,
            execution_id,
            ..
        } => TauriWorkflowEvent::NodeStarted {
            node_id: task_id,
            node_type: String::new(),
            execution_id,
        },

        node_engine::WorkflowEvent::TaskCompleted {
            task_id,
            execution_id,
            output,
            ..
        } => {
            let outputs: HashMap<String, PortValue> = output
                .and_then(|value| value.as_object().cloned())
                .map(|object| object.into_iter().collect())
                .unwrap_or_default();

            TauriWorkflowEvent::NodeCompleted {
                node_id: task_id,
                outputs,
                execution_id,
            }
        }

        node_engine::WorkflowEvent::TaskFailed {
            task_id,
            execution_id,
            error,
            ..
        } => TauriWorkflowEvent::NodeError {
            node_id: task_id,
            error,
            execution_id,
        },

        node_engine::WorkflowEvent::TaskProgress {
            task_id,
            execution_id,
            progress,
            message,
            detail,
            ..
        } => TauriWorkflowEvent::NodeProgress {
            node_id: task_id,
            progress,
            message,
            detail,
            execution_id,
        },

        node_engine::WorkflowEvent::TaskStream {
            task_id,
            execution_id,
            port,
            data,
            ..
        } => TauriWorkflowEvent::NodeStream {
            node_id: task_id,
            port,
            chunk: data,
            execution_id,
        },

        node_engine::WorkflowEvent::WaitingForInput {
            workflow_id,
            execution_id,
            task_id,
            prompt,
            ..
        } => TauriWorkflowEvent::WaitingForInput {
            workflow_id,
            execution_id,
            node_id: task_id,
            message: prompt,
        },

        node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            memory_impact,
            ..
        } => TauriWorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            graph: None,
            dirty_tasks,
            memory_impact,
        },

        node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id,
            tasks,
            ..
        } => TauriWorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id,
            task_ids: tasks,
        },
    }
}

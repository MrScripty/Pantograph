use std::collections::HashMap;

use super::super::events::WorkflowEvent as TauriWorkflowEvent;

/// A value that flows through a port.
type PortValue = serde_json::Value;

pub(super) fn translated_workflow_run_id(event: &TauriWorkflowEvent) -> &str {
    match event {
        TauriWorkflowEvent::Started {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::NodeStarted {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::NodeProgress {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::NodeStream {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::NodeCompleted {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::NodeError {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::Completed {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::Failed {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::Cancelled {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::GraphModified {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::WaitingForInput {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::IncrementalExecutionStarted {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::RuntimeSnapshot {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::SchedulerSnapshot {
            workflow_run_id, ..
        }
        | TauriWorkflowEvent::DiagnosticsSnapshot {
            workflow_run_id, ..
        } => workflow_run_id,
    }
}

pub(super) fn translate_node_event(event: node_engine::WorkflowEvent) -> TauriWorkflowEvent {
    match event {
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id,
            execution_id: workflow_run_id,
            ..
        } => TauriWorkflowEvent::Started {
            workflow_id,
            node_count: 0,
            workflow_run_id,
        },

        node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id,
            execution_id: workflow_run_id,
            ..
        } => TauriWorkflowEvent::Completed {
            workflow_id,
            outputs: HashMap::new(),
            workflow_run_id,
        },

        node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id,
            execution_id: workflow_run_id,
            error,
            ..
        } => TauriWorkflowEvent::Failed {
            workflow_id,
            error,
            workflow_run_id,
        },

        node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id,
            execution_id: workflow_run_id,
            error,
            ..
        } => TauriWorkflowEvent::Cancelled {
            workflow_id,
            error,
            workflow_run_id,
        },

        node_engine::WorkflowEvent::TaskStarted {
            task_id,
            execution_id: workflow_run_id,
            ..
        } => TauriWorkflowEvent::NodeStarted {
            node_id: task_id,
            node_type: String::new(),
            workflow_run_id,
        },

        node_engine::WorkflowEvent::TaskCompleted {
            task_id,
            execution_id: workflow_run_id,
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
                workflow_run_id,
            }
        }

        node_engine::WorkflowEvent::TaskFailed {
            task_id,
            execution_id: workflow_run_id,
            error,
            ..
        } => TauriWorkflowEvent::NodeError {
            node_id: task_id,
            error,
            workflow_run_id,
        },

        node_engine::WorkflowEvent::TaskProgress {
            task_id,
            execution_id: workflow_run_id,
            progress,
            message,
            detail,
            ..
        } => TauriWorkflowEvent::NodeProgress {
            node_id: task_id,
            progress,
            message,
            detail,
            workflow_run_id,
        },

        node_engine::WorkflowEvent::TaskStream {
            task_id,
            execution_id: workflow_run_id,
            port,
            data,
            ..
        } => TauriWorkflowEvent::NodeStream {
            node_id: task_id,
            port,
            chunk: data,
            workflow_run_id,
        },

        node_engine::WorkflowEvent::WaitingForInput {
            workflow_id,
            execution_id: workflow_run_id,
            task_id,
            prompt,
            ..
        } => TauriWorkflowEvent::WaitingForInput {
            workflow_id,
            workflow_run_id,
            node_id: task_id,
            message: prompt,
        },

        node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id: workflow_run_id,
            dirty_tasks,
            memory_impact,
            ..
        } => TauriWorkflowEvent::GraphModified {
            workflow_id,
            workflow_run_id,
            graph: None,
            dirty_tasks,
            memory_impact,
        },

        node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id: workflow_run_id,
            tasks,
            ..
        } => TauriWorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            workflow_run_id,
            task_ids: tasks,
        },
    }
}

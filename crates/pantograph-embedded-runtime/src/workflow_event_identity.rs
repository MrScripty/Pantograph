use std::sync::Arc;

struct WorkflowRunEventSink {
    inner: Arc<dyn node_engine::EventSink>,
    workflow_run_id: String,
}

impl node_engine::EventSink for WorkflowRunEventSink {
    fn send(&self, event: node_engine::WorkflowEvent) -> Result<(), node_engine::EventError> {
        self.inner.send(workflow_event_with_execution_id(
            event,
            &self.workflow_run_id,
        ))
    }
}

pub(crate) fn workflow_run_event_sink(
    inner: Option<Arc<dyn node_engine::EventSink>>,
    workflow_run_id: &str,
) -> Option<Arc<dyn node_engine::EventSink>> {
    inner.map(|inner| {
        Arc::new(WorkflowRunEventSink {
            inner,
            workflow_run_id: workflow_run_id.to_string(),
        }) as Arc<dyn node_engine::EventSink>
    })
}

fn workflow_event_with_execution_id(
    event: node_engine::WorkflowEvent,
    workflow_run_id: &str,
) -> node_engine::WorkflowEvent {
    match event {
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id,
            execution_id: workflow_run_id.to_string(),
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id,
            execution_id: workflow_run_id.to_string(),
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id,
            error,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id,
            execution_id: workflow_run_id.to_string(),
            error,
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id,
            error,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id,
            execution_id: workflow_run_id.to_string(),
            error,
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::WaitingForInput {
            workflow_id,
            task_id,
            prompt,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::WaitingForInput {
            workflow_id,
            execution_id: workflow_run_id.to_string(),
            task_id,
            prompt,
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::TaskStarted {
            task_id,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::TaskStarted {
            task_id,
            execution_id: workflow_run_id.to_string(),
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::TaskInputsResolved {
            task_id,
            input,
            cache_status,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::TaskInputsResolved {
            task_id,
            execution_id: workflow_run_id.to_string(),
            input,
            cache_status,
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::TaskCompleted {
            task_id,
            output,
            cache_status,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::TaskCompleted {
            task_id,
            execution_id: workflow_run_id.to_string(),
            output,
            cache_status,
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::TaskFailed {
            task_id,
            error,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::TaskFailed {
            task_id,
            execution_id: workflow_run_id.to_string(),
            error,
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::TaskProgress {
            task_id,
            progress,
            message,
            detail,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::TaskProgress {
            task_id,
            execution_id: workflow_run_id.to_string(),
            progress,
            message,
            detail,
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::TaskStream {
            task_id,
            port,
            data,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::TaskStream {
            task_id,
            execution_id: workflow_run_id.to_string(),
            port,
            data,
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            dirty_tasks,
            memory_impact,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id: workflow_run_id.to_string(),
            dirty_tasks,
            memory_impact,
            occurred_at_ms,
        },
        node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            tasks,
            occurred_at_ms,
            ..
        } => node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id: workflow_run_id.to_string(),
            tasks,
            occurred_at_ms,
        },
    }
}

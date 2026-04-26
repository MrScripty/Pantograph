use pantograph_workflow_service::{
    WorkflowGraph, WorkflowTraceEvent, WorkflowTraceGraphContext, WorkflowTraceNodeRecord,
    WorkflowTraceSummary,
};

use super::overlay::{DiagnosticsNodeOverlay, DiagnosticsRunOverlay};
use super::types::{
    diagnostics_node_status, diagnostics_run_status, DiagnosticsNodeTrace, DiagnosticsRunTrace,
    DiagnosticsTimingExpectation, DiagnosticsTraceRuntimeMetrics,
};
use crate::workflow::events::WorkflowEvent;

pub(crate) fn graph_trace_context(graph: &WorkflowGraph) -> WorkflowTraceGraphContext {
    WorkflowTraceGraphContext {
        graph_fingerprint: graph
            .derived_graph
            .as_ref()
            .map(|derived| derived.graph_fingerprint.clone()),
        node_count_at_start: graph.nodes.len(),
        node_types_by_id: graph
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.node_type.clone()))
            .collect(),
    }
}

pub(crate) fn node_engine_workflow_trace_event(
    event: &node_engine::WorkflowEvent,
) -> Option<(WorkflowTraceEvent, u64)> {
    let occurred_at_ms = event.occurred_at_ms()?;
    let trace_event = match event {
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id,
            execution_id: workflow_run_id,
            ..
        } => WorkflowTraceEvent::RunStarted {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            node_count: 0,
        },
        node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id,
            execution_id: workflow_run_id,
            ..
        } => WorkflowTraceEvent::RunCompleted {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
        },
        node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id,
            execution_id: workflow_run_id,
            error,
            ..
        } => WorkflowTraceEvent::RunFailed {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            error: error.clone(),
        },
        node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id,
            execution_id: workflow_run_id,
            error,
            ..
        } => WorkflowTraceEvent::RunCancelled {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            error: error.clone(),
        },
        node_engine::WorkflowEvent::WaitingForInput {
            workflow_id,
            execution_id: workflow_run_id,
            task_id,
            ..
        } => WorkflowTraceEvent::WaitingForInput {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            node_id: task_id.clone(),
        },
        node_engine::WorkflowEvent::TaskStarted {
            task_id,
            execution_id: workflow_run_id,
            ..
        } => WorkflowTraceEvent::NodeStarted {
            workflow_run_id: workflow_run_id.clone(),
            node_id: task_id.clone(),
            node_type: None,
        },
        node_engine::WorkflowEvent::TaskCompleted {
            task_id,
            execution_id: workflow_run_id,
            ..
        } => WorkflowTraceEvent::NodeCompleted {
            workflow_run_id: workflow_run_id.clone(),
            node_id: task_id.clone(),
        },
        node_engine::WorkflowEvent::TaskFailed {
            task_id,
            execution_id: workflow_run_id,
            error,
            ..
        } => WorkflowTraceEvent::NodeFailed {
            workflow_run_id: workflow_run_id.clone(),
            node_id: task_id.clone(),
            error: error.clone(),
        },
        node_engine::WorkflowEvent::TaskProgress {
            task_id,
            execution_id: workflow_run_id,
            detail,
            ..
        } => WorkflowTraceEvent::NodeProgress {
            workflow_run_id: workflow_run_id.clone(),
            node_id: task_id.clone(),
            detail: detail.clone(),
        },
        node_engine::WorkflowEvent::TaskStream {
            task_id,
            execution_id: workflow_run_id,
            ..
        } => WorkflowTraceEvent::NodeStream {
            workflow_run_id: workflow_run_id.clone(),
            node_id: task_id.clone(),
        },
        node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id: workflow_run_id,
            dirty_tasks,
            memory_impact,
            ..
        } => WorkflowTraceEvent::GraphModified {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            dirty_tasks: dirty_tasks.clone(),
            memory_impact: memory_impact.clone(),
        },
        node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id: workflow_run_id,
            tasks,
            ..
        } => WorkflowTraceEvent::IncrementalExecutionStarted {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            task_ids: tasks.clone(),
        },
    };

    Some((trace_event, occurred_at_ms))
}

pub(crate) fn workflow_trace_event(event: &WorkflowEvent) -> Option<WorkflowTraceEvent> {
    match event {
        WorkflowEvent::Started {
            workflow_id,
            node_count,
            workflow_run_id,
        } => Some(WorkflowTraceEvent::RunStarted {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            node_count: *node_count,
        }),
        WorkflowEvent::NodeStarted {
            node_id,
            node_type,
            workflow_run_id,
        } => Some(WorkflowTraceEvent::NodeStarted {
            workflow_run_id: workflow_run_id.clone(),
            node_id: node_id.clone(),
            node_type: (!node_type.trim().is_empty()).then(|| node_type.clone()),
        }),
        WorkflowEvent::NodeProgress {
            node_id,
            workflow_run_id,
            detail,
            ..
        } => Some(WorkflowTraceEvent::NodeProgress {
            workflow_run_id: workflow_run_id.clone(),
            node_id: node_id.clone(),
            detail: detail.clone(),
        }),
        WorkflowEvent::NodeStream {
            node_id,
            workflow_run_id,
            ..
        } => Some(WorkflowTraceEvent::NodeStream {
            workflow_run_id: workflow_run_id.clone(),
            node_id: node_id.clone(),
        }),
        WorkflowEvent::NodeCompleted {
            node_id,
            workflow_run_id,
            ..
        } => Some(WorkflowTraceEvent::NodeCompleted {
            workflow_run_id: workflow_run_id.clone(),
            node_id: node_id.clone(),
        }),
        WorkflowEvent::NodeError {
            node_id,
            error,
            workflow_run_id,
        } => Some(WorkflowTraceEvent::NodeFailed {
            workflow_run_id: workflow_run_id.clone(),
            node_id: node_id.clone(),
            error: error.clone(),
        }),
        WorkflowEvent::Cancelled {
            workflow_id,
            error,
            workflow_run_id,
        } => Some(WorkflowTraceEvent::RunCancelled {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            error: error.clone(),
        }),
        WorkflowEvent::Completed {
            workflow_id,
            workflow_run_id,
            ..
        } => Some(WorkflowTraceEvent::RunCompleted {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
        }),
        WorkflowEvent::Failed {
            workflow_id,
            error,
            workflow_run_id,
        } => Some(WorkflowTraceEvent::RunFailed {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            error: error.clone(),
        }),
        WorkflowEvent::WaitingForInput {
            workflow_id,
            workflow_run_id,
            node_id,
            ..
        } => Some(WorkflowTraceEvent::WaitingForInput {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            node_id: node_id.clone(),
        }),
        WorkflowEvent::GraphModified {
            workflow_id,
            workflow_run_id,
            dirty_tasks,
            memory_impact,
            ..
        } => Some(WorkflowTraceEvent::GraphModified {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            dirty_tasks: dirty_tasks.clone(),
            memory_impact: memory_impact.clone(),
        }),
        WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            workflow_run_id,
            task_ids,
            ..
        } => Some(WorkflowTraceEvent::IncrementalExecutionStarted {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            task_ids: task_ids.clone(),
        }),
        WorkflowEvent::RuntimeSnapshot {
            workflow_id,
            workflow_run_id,
            captured_at_ms,
            capabilities,
            trace_runtime_metrics,
            error,
            ..
        } => Some(WorkflowTraceEvent::RuntimeSnapshotCaptured {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            captured_at_ms: *captured_at_ms,
            runtime: trace_runtime_metrics.as_ref().clone(),
            capabilities: capabilities.as_ref().clone(),
            error: error.clone(),
        }),
        WorkflowEvent::SchedulerSnapshot {
            workflow_id,
            workflow_run_id,
            session_id,
            captured_at_ms,
            session,
            items,
            diagnostics,
            error,
            ..
        } => Some(WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: workflow_run_id.clone(),
            workflow_id: workflow_id.clone(),
            session_id: session_id.clone(),
            captured_at_ms: *captured_at_ms,
            session: session.clone(),
            items: items.clone(),
            diagnostics: diagnostics.clone(),
            error: error.clone(),
        }),
        WorkflowEvent::DiagnosticsSnapshot { .. } => None,
    }
}

pub(crate) fn diagnostics_run_trace(
    trace: &WorkflowTraceSummary,
    overlay: Option<DiagnosticsRunOverlay>,
) -> DiagnosticsRunTrace {
    let DiagnosticsRunOverlay {
        last_updated_at_ms,
        last_dirty_tasks: overlay_last_dirty_tasks,
        last_incremental_task_ids: overlay_last_incremental_task_ids,
        last_graph_memory_impact: overlay_last_graph_memory_impact,
        nodes_by_id,
        events,
    } = overlay.unwrap_or_else(|| DiagnosticsRunOverlay::new(trace.started_at_ms));

    let last_dirty_tasks = if overlay_last_dirty_tasks.is_empty() {
        trace.last_dirty_tasks.clone()
    } else {
        overlay_last_dirty_tasks
    };
    let last_incremental_task_ids = if overlay_last_incremental_task_ids.is_empty() {
        trace.last_incremental_task_ids.clone()
    } else {
        overlay_last_incremental_task_ids
    };
    let last_graph_memory_impact =
        overlay_last_graph_memory_impact.or_else(|| trace.last_graph_memory_impact.clone());

    DiagnosticsRunTrace {
        workflow_run_id: trace.workflow_run_id.clone(),
        session_id: trace.session_id.clone(),
        workflow_id: trace.workflow_id.clone(),
        graph_fingerprint_at_start: trace.graph_fingerprint.clone(),
        node_count_at_start: trace.node_count_at_start,
        status: diagnostics_run_status(trace.status),
        started_at_ms: trace.started_at_ms,
        ended_at_ms: trace.ended_at_ms,
        duration_ms: trace.duration_ms,
        last_updated_at_ms: last_updated_at_ms
            .max(trace.ended_at_ms.unwrap_or(trace.started_at_ms)),
        error: trace.last_error.clone(),
        waiting_for_input: trace.waiting_for_input,
        runtime: DiagnosticsTraceRuntimeMetrics::from(&trace.runtime),
        event_count: trace.event_count,
        stream_event_count: trace.stream_event_count,
        last_dirty_tasks,
        last_incremental_task_ids,
        last_graph_memory_impact,
        timing_expectation: trace
            .timing_expectation
            .as_ref()
            .map(DiagnosticsTimingExpectation::from),
        nodes: trace
            .nodes
            .iter()
            .map(|node| {
                let overlay = nodes_by_id.get(&node.node_id).cloned();
                (node.node_id.clone(), diagnostics_node_trace(node, overlay))
            })
            .collect(),
        events,
    }
}

fn diagnostics_node_trace(
    node: &WorkflowTraceNodeRecord,
    overlay: Option<DiagnosticsNodeOverlay>,
) -> DiagnosticsNodeTrace {
    let overlay = overlay.unwrap_or_default();
    DiagnosticsNodeTrace {
        node_id: node.node_id.clone(),
        node_type: node.node_type.clone(),
        status: diagnostics_node_status(node.status),
        started_at_ms: node.started_at_ms,
        ended_at_ms: node.ended_at_ms,
        duration_ms: node.duration_ms,
        last_progress: overlay.last_progress,
        last_message: overlay.last_message,
        last_progress_detail: overlay
            .last_progress_detail
            .or_else(|| node.last_progress_detail.clone()),
        timing_expectation: node
            .timing_expectation
            .as_ref()
            .map(DiagnosticsTimingExpectation::from),
        stream_event_count: node.stream_event_count,
        event_count: node.event_count,
        error: node.last_error.clone(),
    }
}

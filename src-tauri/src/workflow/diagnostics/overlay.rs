use std::collections::{BTreeMap, HashSet};

use node_engine::GraphMemoryImpactSummary;
use pantograph_workflow_service::WorkflowTraceSnapshotResponse;

use super::trace::diagnostics_run_trace;
use super::types::{
    DiagnosticsEventRecord, DiagnosticsRuntimeSnapshot, DiagnosticsRuntimeSnapshotInput,
    DiagnosticsSchedulerSnapshot, WorkflowDiagnosticsProjection,
    WorkflowDiagnosticsProjectionContext,
};
use crate::workflow::events::WorkflowEvent;

#[derive(Debug, Clone, Default)]
pub(crate) struct DiagnosticsNodeOverlay {
    pub(crate) last_progress: Option<f32>,
    pub(crate) last_message: Option<String>,
    pub(crate) last_progress_detail: Option<node_engine::TaskProgressDetail>,
}

#[derive(Debug, Clone)]
pub(crate) struct DiagnosticsRunOverlay {
    pub(crate) last_updated_at_ms: u64,
    pub(crate) last_dirty_tasks: Vec<String>,
    pub(crate) last_incremental_task_ids: Vec<String>,
    pub(crate) last_graph_memory_impact: Option<GraphMemoryImpactSummary>,
    pub(crate) nodes_by_id: BTreeMap<String, DiagnosticsNodeOverlay>,
    pub(crate) events: Vec<DiagnosticsEventRecord>,
}

impl DiagnosticsRunOverlay {
    pub(crate) fn new(timestamp_ms: u64) -> Self {
        Self {
            last_updated_at_ms: timestamp_ms,
            last_dirty_tasks: Vec::new(),
            last_incremental_task_ids: Vec::new(),
            last_graph_memory_impact: None,
            nodes_by_id: BTreeMap::new(),
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowDiagnosticsState {
    pub(crate) overlays_by_workflow_run_id: BTreeMap<String, DiagnosticsRunOverlay>,
    pub(crate) runtime: DiagnosticsRuntimeSnapshot,
    pub(crate) scheduler: DiagnosticsSchedulerSnapshot,
    pub(crate) retained_event_limit: usize,
}

impl WorkflowDiagnosticsState {
    pub(crate) fn new(retained_event_limit: usize) -> Self {
        Self {
            overlays_by_workflow_run_id: BTreeMap::new(),
            runtime: DiagnosticsRuntimeSnapshot::default(),
            scheduler: DiagnosticsSchedulerSnapshot::default(),
            retained_event_limit,
        }
    }

    pub(crate) fn snapshot(
        &self,
        traces: &WorkflowTraceSnapshotResponse,
    ) -> WorkflowDiagnosticsProjection {
        let run_order = traces
            .traces
            .iter()
            .map(|trace| trace.workflow_run_id.clone())
            .collect::<Vec<_>>();
        let runs_by_id = traces
            .traces
            .iter()
            .map(|trace| {
                let overlay = self
                    .overlays_by_workflow_run_id
                    .get(&trace.workflow_run_id)
                    .cloned();
                (
                    trace.workflow_run_id.clone(),
                    diagnostics_run_trace(trace, overlay),
                )
            })
            .collect();

        WorkflowDiagnosticsProjection {
            context: WorkflowDiagnosticsProjectionContext::default(),
            runs_by_id,
            run_order,
            runtime: self.runtime.clone(),
            scheduler: self.scheduler.clone(),
            workflow_timing_history: None,
            current_session_state: None,
            retained_event_limit: self.retained_event_limit,
        }
    }

    #[cfg(test)]
    pub(crate) fn clear_history(&mut self) {
        self.overlays_by_workflow_run_id.clear();
    }

    pub(crate) fn prune_overlays(&mut self, traces: &WorkflowTraceSnapshotResponse) {
        let retained_workflow_run_ids = traces
            .traces
            .iter()
            .map(|trace| trace.workflow_run_id.as_str())
            .collect::<HashSet<_>>();
        self.overlays_by_workflow_run_id
            .retain(|workflow_run_id, _| {
                retained_workflow_run_ids.contains(workflow_run_id.as_str())
            });
    }
}

pub(crate) fn record_diagnostics_overlay(
    state: &mut WorkflowDiagnosticsState,
    event: &WorkflowEvent,
    timestamp_ms: u64,
) {
    if matches!(event, WorkflowEvent::RuntimeSnapshot { .. }) {
        apply_runtime_event(state, event, timestamp_ms);
    }
    if matches!(event, WorkflowEvent::SchedulerSnapshot { .. }) {
        apply_scheduler_event(state, event, timestamp_ms);
    }

    let Some(workflow_run_id) = event_workflow_run_id(event) else {
        return;
    };

    let overlay = state
        .overlays_by_workflow_run_id
        .entry(workflow_run_id.clone())
        .or_insert_with(|| DiagnosticsRunOverlay::new(timestamp_ms));
    overlay.last_updated_at_ms = timestamp_ms;

    if let Some(node_id) = event_node_id(event) {
        let node_overlay = overlay.nodes_by_id.entry(node_id).or_default();
        match event {
            WorkflowEvent::NodeStarted { .. } => {
                node_overlay.last_progress = None;
                node_overlay.last_message = None;
                node_overlay.last_progress_detail = None;
            }
            WorkflowEvent::NodeProgress {
                progress,
                message,
                detail,
                ..
            } => {
                if let Some(detail) = detail.clone() {
                    node_overlay.last_progress_detail = Some(detail);
                } else {
                    node_overlay.last_progress = Some(*progress);
                    node_overlay.last_message = message.clone();
                }
            }
            WorkflowEvent::WaitingForInput { message, .. } => {
                node_overlay.last_message = message
                    .clone()
                    .or_else(|| Some("Waiting for input".to_string()));
            }
            _ => {}
        }
    }

    match event {
        WorkflowEvent::GraphModified {
            dirty_tasks,
            memory_impact,
            ..
        } => {
            overlay.last_dirty_tasks = dirty_tasks.clone();
            overlay.last_graph_memory_impact = memory_impact.clone();
        }
        WorkflowEvent::IncrementalExecutionStarted { task_ids, .. } => {
            overlay.last_incremental_task_ids = task_ids.clone();
        }
        _ => {}
    }

    let sequence = overlay.events.len() + 1;
    overlay.events.push(DiagnosticsEventRecord {
        id: format!("{}-{}", workflow_run_id, sequence),
        sequence,
        timestamp_ms,
        event_type: event_type_name(event).to_string(),
        workflow_run_id,
        workflow_id: event_workflow_id(event),
        node_id: event_node_id(event),
        summary: summarize_event(event),
        payload: event_payload(event),
    });
    if overlay.events.len() > state.retained_event_limit {
        let excess = overlay.events.len() - state.retained_event_limit;
        overlay.events.drain(0..excess);
    }
}

pub(crate) fn event_workflow_run_id(event: &WorkflowEvent) -> Option<String> {
    match event {
        WorkflowEvent::Started {
            workflow_run_id, ..
        }
        | WorkflowEvent::NodeStarted {
            workflow_run_id, ..
        }
        | WorkflowEvent::NodeProgress {
            workflow_run_id, ..
        }
        | WorkflowEvent::NodeStream {
            workflow_run_id, ..
        }
        | WorkflowEvent::NodeCompleted {
            workflow_run_id, ..
        }
        | WorkflowEvent::NodeError {
            workflow_run_id, ..
        }
        | WorkflowEvent::Completed {
            workflow_run_id, ..
        }
        | WorkflowEvent::Failed {
            workflow_run_id, ..
        }
        | WorkflowEvent::Cancelled {
            workflow_run_id, ..
        }
        | WorkflowEvent::GraphModified {
            workflow_run_id, ..
        }
        | WorkflowEvent::WaitingForInput {
            workflow_run_id, ..
        }
        | WorkflowEvent::IncrementalExecutionStarted {
            workflow_run_id, ..
        }
        | WorkflowEvent::RuntimeSnapshot {
            workflow_run_id, ..
        }
        | WorkflowEvent::SchedulerSnapshot {
            workflow_run_id, ..
        }
        | WorkflowEvent::DiagnosticsSnapshot {
            workflow_run_id, ..
        } => Some(workflow_run_id.clone()),
    }
}

fn apply_runtime_event(
    state: &mut WorkflowDiagnosticsState,
    event: &WorkflowEvent,
    timestamp_ms: u64,
) {
    if let WorkflowEvent::RuntimeSnapshot {
        workflow_id,
        capabilities,
        active_model_target,
        embedding_model_target,
        active_runtime_snapshot,
        embedding_runtime_snapshot,
        managed_runtimes,
        error,
        ..
    } = event
    {
        state.runtime =
            DiagnosticsRuntimeSnapshot::from_capabilities(DiagnosticsRuntimeSnapshotInput {
                workflow_id: workflow_id.clone(),
                capabilities: capabilities.as_ref().clone(),
                last_error: error.clone(),
                active_model_target: active_model_target.clone(),
                embedding_model_target: embedding_model_target.clone(),
                active_runtime_snapshot: active_runtime_snapshot.clone(),
                embedding_runtime_snapshot: embedding_runtime_snapshot.clone(),
                managed_runtimes: managed_runtimes.clone(),
                captured_at_ms: timestamp_ms,
            });
    }
}

fn apply_scheduler_event(
    state: &mut WorkflowDiagnosticsState,
    event: &WorkflowEvent,
    timestamp_ms: u64,
) {
    if let WorkflowEvent::SchedulerSnapshot {
        workflow_id,
        workflow_run_id,
        session_id,
        session,
        items,
        diagnostics,
        error,
        ..
    } = event
    {
        state.scheduler = DiagnosticsSchedulerSnapshot {
            workflow_id: workflow_id.clone(),
            session_id: Some(session_id.clone()),
            workflow_run_id: Some(workflow_run_id.clone()),
            captured_at_ms: Some(timestamp_ms),
            session: session.clone(),
            items: items.clone(),
            diagnostics: diagnostics.clone(),
            last_error: error.clone(),
        };
    }
}

fn event_type_name(event: &WorkflowEvent) -> &'static str {
    match event {
        WorkflowEvent::Started { .. } => "Started",
        WorkflowEvent::NodeStarted { .. } => "NodeStarted",
        WorkflowEvent::NodeProgress { .. } => "NodeProgress",
        WorkflowEvent::NodeStream { .. } => "NodeStream",
        WorkflowEvent::NodeCompleted { .. } => "NodeCompleted",
        WorkflowEvent::NodeError { .. } => "NodeError",
        WorkflowEvent::Completed { .. } => "Completed",
        WorkflowEvent::Failed { .. } => "Failed",
        WorkflowEvent::Cancelled { .. } => "Cancelled",
        WorkflowEvent::GraphModified { .. } => "GraphModified",
        WorkflowEvent::WaitingForInput { .. } => "WaitingForInput",
        WorkflowEvent::IncrementalExecutionStarted { .. } => "IncrementalExecutionStarted",
        WorkflowEvent::RuntimeSnapshot { .. } => "RuntimeSnapshot",
        WorkflowEvent::SchedulerSnapshot { .. } => "SchedulerSnapshot",
        WorkflowEvent::DiagnosticsSnapshot { .. } => "DiagnosticsSnapshot",
    }
}

fn event_workflow_id(event: &WorkflowEvent) -> Option<String> {
    match event {
        WorkflowEvent::Started { workflow_id, .. }
        | WorkflowEvent::Completed { workflow_id, .. }
        | WorkflowEvent::Failed { workflow_id, .. }
        | WorkflowEvent::Cancelled { workflow_id, .. }
        | WorkflowEvent::GraphModified { workflow_id, .. }
        | WorkflowEvent::WaitingForInput { workflow_id, .. }
        | WorkflowEvent::IncrementalExecutionStarted { workflow_id, .. } => {
            Some(workflow_id.clone())
        }
        WorkflowEvent::RuntimeSnapshot { workflow_id, .. } => Some(workflow_id.clone()),
        WorkflowEvent::SchedulerSnapshot { workflow_id, .. } => workflow_id.clone(),
        _ => None,
    }
}

fn event_node_id(event: &WorkflowEvent) -> Option<String> {
    match event {
        WorkflowEvent::NodeStarted { node_id, .. }
        | WorkflowEvent::NodeProgress { node_id, .. }
        | WorkflowEvent::NodeStream { node_id, .. }
        | WorkflowEvent::NodeCompleted { node_id, .. }
        | WorkflowEvent::NodeError { node_id, .. }
        | WorkflowEvent::WaitingForInput { node_id, .. } => Some(node_id.clone()),
        _ => None,
    }
}

fn event_payload(event: &WorkflowEvent) -> serde_json::Value {
    match event {
        WorkflowEvent::NodeStream {
            node_id,
            port,
            chunk,
            workflow_run_id,
        } => {
            return serde_json::json!({
                "node_id": node_id,
                "port": port,
                "chunk": diagnostics_safe_value_for_key(Some(port), chunk),
                "workflow_run_id": workflow_run_id,
            });
        }
        WorkflowEvent::NodeCompleted {
            node_id,
            outputs,
            workflow_run_id,
        } => {
            let outputs = outputs
                .iter()
                .map(|(port, value)| {
                    (
                        port.clone(),
                        diagnostics_safe_value_for_key(Some(port), value),
                    )
                })
                .collect::<serde_json::Map<_, _>>();
            return serde_json::json!({
                "node_id": node_id,
                "outputs": outputs,
                "workflow_run_id": workflow_run_id,
            });
        }
        WorkflowEvent::Completed {
            workflow_id,
            outputs,
            workflow_run_id,
        } => {
            let outputs = outputs
                .iter()
                .map(|(node_id, node_outputs)| {
                    let node_outputs = node_outputs
                        .iter()
                        .map(|(port, value)| {
                            (
                                port.clone(),
                                diagnostics_safe_value_for_key(Some(port), value),
                            )
                        })
                        .collect::<serde_json::Map<_, _>>();
                    (node_id.clone(), serde_json::Value::Object(node_outputs))
                })
                .collect::<serde_json::Map<_, _>>();
            return serde_json::json!({
                "workflow_id": workflow_id,
                "outputs": outputs,
                "workflow_run_id": workflow_run_id,
            });
        }
        _ => {}
    }

    match serde_json::to_value(event) {
        Ok(serde_json::Value::Object(mut value)) => {
            value.remove("data").unwrap_or(serde_json::Value::Null)
        }
        Ok(_) | Err(_) => serde_json::Value::Null,
    }
}

fn diagnostics_safe_value_for_key(
    key: Option<&str>,
    value: &serde_json::Value,
) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => serde_json::Value::Object(
            object
                .iter()
                .map(|(field, field_value)| {
                    let sanitized = if is_inline_body_key(field) {
                        redacted_inline_body(field_value)
                    } else {
                        diagnostics_safe_value_for_key(Some(field), field_value)
                    };
                    (field.clone(), sanitized)
                })
                .collect(),
        ),
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .iter()
                .map(|value| diagnostics_safe_value_for_key(key, value))
                .collect(),
        ),
        serde_json::Value::String(value)
            if is_inline_body_key(key.unwrap_or_default())
                || is_inline_media_string_key(key.unwrap_or_default(), value)
                || data_url_base64_media_type(value).is_some() =>
        {
            redacted_inline_body(&serde_json::Value::String(value.clone()))
        }
        _ => value.clone(),
    }
}

fn is_inline_body_key(key: &str) -> bool {
    matches!(
        key,
        "audio_base64"
            | "body"
            | "content"
            | "data"
            | "encoded_body"
            | "image_base64"
            | "payload"
            | "video_base64"
    ) || key.ends_with("_base64")
}

fn is_inline_media_string_key(key: &str, value: &str) -> bool {
    matches!(key, "audio" | "image" | "video") && is_probably_base64_body(value)
}

fn is_probably_base64_body(value: &str) -> bool {
    let value = value.trim();
    value.len() >= 128
        && value.len() % 4 == 0
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
}

fn redacted_inline_body(value: &serde_json::Value) -> serde_json::Value {
    let mut redacted = serde_json::Map::new();
    redacted.insert(
        "diagnostics_redacted".to_string(),
        serde_json::Value::Bool(true),
    );
    redacted.insert(
        "reason".to_string(),
        serde_json::Value::String("inline_content_body".to_string()),
    );
    match value {
        serde_json::Value::String(value) => {
            redacted.insert(
                "original_type".to_string(),
                serde_json::Value::String("string".to_string()),
            );
            redacted.insert(
                "character_length".to_string(),
                serde_json::Value::Number(serde_json::Number::from(value.len())),
            );
            if let Some(media_type) = data_url_base64_media_type(value) {
                redacted.insert(
                    "media_type".to_string(),
                    serde_json::Value::String(media_type.to_string()),
                );
                redacted.insert(
                    "encoding".to_string(),
                    serde_json::Value::String("base64".to_string()),
                );
            }
        }
        serde_json::Value::Array(values) => {
            redacted.insert(
                "original_type".to_string(),
                serde_json::Value::String("array".to_string()),
            );
            redacted.insert(
                "item_count".to_string(),
                serde_json::Value::Number(serde_json::Number::from(values.len())),
            );
        }
        serde_json::Value::Object(object) => {
            redacted.insert(
                "original_type".to_string(),
                serde_json::Value::String("object".to_string()),
            );
            redacted.insert(
                "field_count".to_string(),
                serde_json::Value::Number(serde_json::Number::from(object.len())),
            );
        }
        other => {
            redacted.insert(
                "original_type".to_string(),
                serde_json::Value::String(value_type_name(other).to_string()),
            );
        }
    }
    serde_json::Value::Object(redacted)
}

fn data_url_base64_media_type(value: &str) -> Option<&str> {
    value
        .strip_prefix("data:")
        .and_then(|value| value.split_once(";base64,"))
        .map(|(media_type, _)| media_type)
        .filter(|media_type| !media_type.trim().is_empty())
}

fn value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn summarize_event(event: &WorkflowEvent) -> String {
    match event {
        WorkflowEvent::Started { node_count, .. } => {
            format!("Workflow started ({} nodes)", node_count)
        }
        WorkflowEvent::NodeStarted { node_id, .. } => format!("Node {} started", node_id),
        WorkflowEvent::NodeProgress {
            node_id,
            progress,
            message,
            ..
        } => message
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                format!("Node {} progress {}%", node_id, (progress * 100.0).round())
            }),
        WorkflowEvent::NodeStream { node_id, port, .. } => {
            format!("Node {} streamed on {}", node_id, port)
        }
        WorkflowEvent::NodeCompleted { node_id, .. } => format!("Node {} completed", node_id),
        WorkflowEvent::NodeError { node_id, error, .. } => {
            format!("Node {} failed: {}", node_id, error)
        }
        WorkflowEvent::Completed { .. } => "Workflow completed".to_string(),
        WorkflowEvent::Failed { error, .. } => format!("Workflow failed: {}", error),
        WorkflowEvent::Cancelled { error, .. } => format!("Workflow cancelled: {}", error),
        WorkflowEvent::GraphModified { dirty_tasks, .. } if !dirty_tasks.is_empty() => {
            format!("Graph modified; dirty tasks: {}", dirty_tasks.join(", "))
        }
        WorkflowEvent::GraphModified { .. } => "Graph modified".to_string(),
        WorkflowEvent::WaitingForInput {
            node_id, message, ..
        } => message
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("Waiting for input on {}", node_id)),
        WorkflowEvent::IncrementalExecutionStarted { task_ids, .. } if !task_ids.is_empty() => {
            format!("Incremental execution for {}", task_ids.join(", "))
        }
        WorkflowEvent::IncrementalExecutionStarted { .. } => {
            "Incremental execution started".to_string()
        }
        WorkflowEvent::RuntimeSnapshot { error, .. } => error
            .clone()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("Runtime snapshot failed: {}", value))
            .unwrap_or_else(|| "Runtime snapshot captured".to_string()),
        WorkflowEvent::SchedulerSnapshot { items, error, .. } => error
            .clone()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("Scheduler snapshot failed: {}", value))
            .unwrap_or_else(|| {
                format!("Scheduler snapshot captured ({} queue items)", items.len())
            }),
        WorkflowEvent::DiagnosticsSnapshot { .. } => "Diagnostics snapshot captured".to_string(),
    }
}

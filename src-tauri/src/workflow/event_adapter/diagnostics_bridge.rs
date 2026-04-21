use super::super::diagnostics::{SharedWorkflowDiagnosticsStore, node_engine_workflow_trace_event};
use super::super::events::WorkflowEvent as TauriWorkflowEvent;
use super::translation::{translate_node_event, translated_execution_id};

pub(super) fn translate_node_event_with_diagnostics(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    event: node_engine::WorkflowEvent,
) -> (TauriWorkflowEvent, TauriWorkflowEvent) {
    let trace_event = node_engine_workflow_trace_event(&event);
    let tauri_event = translate_node_event(event);
    let source_execution_id = translated_execution_id(&tauri_event).to_string();
    let diagnostics_snapshot = trace_event
        .map(|(trace_event, occurred_at_ms)| {
            diagnostics_store.record_trace_event_with_overlay(
                &trace_event,
                &tauri_event,
                occurred_at_ms,
            )
        })
        .unwrap_or_else(|| diagnostics_store.record_workflow_event_now(&tauri_event))
        .with_source_execution_id(Some(source_execution_id.clone()));
    let diagnostics_event =
        TauriWorkflowEvent::diagnostics_snapshot(source_execution_id, diagnostics_snapshot);

    (tauri_event, diagnostics_event)
}

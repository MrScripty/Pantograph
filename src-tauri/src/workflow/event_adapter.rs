//! Event adapter for converting node-engine events to Tauri channel events.
//!
//! The stable `workflow::event_adapter` facade remains in this file while the
//! translation and diagnostics-bridge helpers live in focused submodules.

mod diagnostics_bridge;
mod translation;

#[cfg(test)]
mod tests;

use node_engine::{EventError, EventSink};
use pantograph_workflow_service::WorkflowGraph;
use tauri::ipc::Channel;

use super::diagnostics::SharedWorkflowDiagnosticsStore;
use super::events::WorkflowEvent as TauriWorkflowEvent;
use diagnostics_bridge::translate_node_event_with_diagnostics;

/// Adapter that converts node-engine `WorkflowEvent`s to Tauri workflow events
/// and sends them through a Tauri channel to the frontend.
pub struct TauriEventAdapter {
    channel: Channel<TauriWorkflowEvent>,
    workflow_id: String,
    workflow_name: Option<String>,
    execution_graph: Option<WorkflowGraph>,
    diagnostics_store: SharedWorkflowDiagnosticsStore,
}

impl TauriEventAdapter {
    /// Create a new adapter with the given Tauri channel and diagnostics store.
    pub fn new(
        channel: Channel<TauriWorkflowEvent>,
        workflow_id: impl Into<String>,
        diagnostics_store: SharedWorkflowDiagnosticsStore,
    ) -> Self {
        Self {
            channel,
            workflow_id: workflow_id.into(),
            workflow_name: None,
            execution_graph: None,
            diagnostics_store,
        }
    }

    /// Attach the display name that belongs to runtime execution events.
    pub fn with_workflow_name(mut self, workflow_name: Option<String>) -> Self {
        self.workflow_name = workflow_name;
        self
    }

    /// Attach the graph that belongs to runtime execution events.
    pub fn with_execution_graph(mut self, graph: WorkflowGraph) -> Self {
        self.execution_graph = Some(graph);
        self
    }

    fn prepare_event_for_diagnostics(
        &self,
        event: node_engine::WorkflowEvent,
    ) -> node_engine::WorkflowEvent {
        let event = workflow_event_with_id(event, &self.workflow_id);
        let execution_id = node_engine_execution_id(&event);
        self.diagnostics_store.set_execution_metadata(
            execution_id,
            Some(self.workflow_id.clone()),
            self.workflow_name.clone(),
        );
        if let Some(graph) = &self.execution_graph {
            self.diagnostics_store
                .set_execution_graph(execution_id, graph);
        }
        event
    }
}

impl EventSink for TauriEventAdapter {
    fn send(&self, event: node_engine::WorkflowEvent) -> Result<(), EventError> {
        let event = self.prepare_event_for_diagnostics(event);
        let (tauri_event, diagnostics_event) =
            translate_node_event_with_diagnostics(&self.diagnostics_store, event);

        self.channel
            .send(tauri_event)
            .map_err(|_| EventError::channel_closed())
            .and_then(|_| {
                self.channel
                    .send(diagnostics_event)
                    .map_err(|_| EventError::channel_closed())
            })
    }
}

fn node_engine_execution_id(event: &node_engine::WorkflowEvent) -> &str {
    match event {
        node_engine::WorkflowEvent::WorkflowStarted { execution_id, .. }
        | node_engine::WorkflowEvent::WorkflowCompleted { execution_id, .. }
        | node_engine::WorkflowEvent::WorkflowFailed { execution_id, .. }
        | node_engine::WorkflowEvent::WorkflowCancelled { execution_id, .. }
        | node_engine::WorkflowEvent::WaitingForInput { execution_id, .. }
        | node_engine::WorkflowEvent::TaskStarted { execution_id, .. }
        | node_engine::WorkflowEvent::TaskCompleted { execution_id, .. }
        | node_engine::WorkflowEvent::TaskFailed { execution_id, .. }
        | node_engine::WorkflowEvent::TaskProgress { execution_id, .. }
        | node_engine::WorkflowEvent::TaskStream { execution_id, .. }
        | node_engine::WorkflowEvent::GraphModified { execution_id, .. }
        | node_engine::WorkflowEvent::IncrementalExecutionStarted { execution_id, .. } => {
            execution_id
        }
    }
}

fn workflow_event_with_id(
    mut event: node_engine::WorkflowEvent,
    workflow_id: &str,
) -> node_engine::WorkflowEvent {
    match &mut event {
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::WaitingForInput {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::GraphModified {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id: event_id,
            ..
        } => {
            *event_id = workflow_id.to_string();
        }
        node_engine::WorkflowEvent::TaskStarted { .. }
        | node_engine::WorkflowEvent::TaskCompleted { .. }
        | node_engine::WorkflowEvent::TaskFailed { .. }
        | node_engine::WorkflowEvent::TaskProgress { .. }
        | node_engine::WorkflowEvent::TaskStream { .. } => {}
    }

    event
}

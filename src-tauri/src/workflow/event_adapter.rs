//! Event adapter for converting node-engine events to Tauri channel events.
//!
//! The stable `workflow::event_adapter` facade remains in this file while the
//! translation and diagnostics-bridge helpers live in focused submodules.

mod diagnostics_bridge;
mod translation;

#[cfg(test)]
mod tests;

use node_engine::{EventError, EventSink};
use tauri::ipc::Channel;

use super::diagnostics::SharedWorkflowDiagnosticsStore;
use super::events::WorkflowEvent as TauriWorkflowEvent;
use diagnostics_bridge::translate_node_event_with_diagnostics;

/// Adapter that converts node-engine `WorkflowEvent`s to Tauri workflow events
/// and sends them through a Tauri channel to the frontend.
pub struct TauriEventAdapter {
    channel: Channel<TauriWorkflowEvent>,
    diagnostics_store: SharedWorkflowDiagnosticsStore,
}

impl TauriEventAdapter {
    /// Create a new adapter with the given Tauri channel and diagnostics store.
    pub fn new(
        channel: Channel<TauriWorkflowEvent>,
        workflow_id: impl Into<String>,
        diagnostics_store: SharedWorkflowDiagnosticsStore,
    ) -> Self {
        let _ = workflow_id.into();
        Self {
            channel,
            diagnostics_store,
        }
    }
}

impl EventSink for TauriEventAdapter {
    fn send(&self, event: node_engine::WorkflowEvent) -> Result<(), EventError> {
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

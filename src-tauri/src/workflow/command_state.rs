//! Operation-specific bundles of Tauri-injected state; no client payload fields.

use tauri::ipc::{CommandArg, CommandItem, InvokeError};
use tauri::{Runtime, State};

use crate::agent::rag::SharedRagManager;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};
use super::commands::{
    SharedExtensions, SharedNodeRegistry, SharedWorkflowDiagnosticsStore, SharedWorkflowService,
};

pub struct WorkflowRunCommandState<'r> {
    pub gateway: State<'r, SharedGateway>,
    pub runtime_registry: State<'r, SharedRuntimeRegistry>,
    pub extensions: State<'r, SharedExtensions>,
    pub rag_manager: State<'r, SharedRagManager>,
    pub workflow_service: State<'r, SharedWorkflowService>,
    pub diagnostics_store: State<'r, SharedWorkflowDiagnosticsStore>,
}

impl<'r, 'de: 'r, R: Runtime> CommandArg<'de, R> for WorkflowRunCommandState<'r> {
    fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError> {
        // Match the original command's extraction order and macro-generated keys.
        Ok(Self {
            gateway: managed_state(&command, "gateway")?,
            runtime_registry: managed_state(&command, "runtimeRegistry")?,
            extensions: managed_state(&command, "extensions")?,
            rag_manager: managed_state(&command, "ragManager")?,
            workflow_service: managed_state(&command, "workflowService")?,
            diagnostics_store: managed_state(&command, "diagnosticsStore")?,
        })
    }
}

pub struct PortOptionsCommandState<'r> {
    pub registry: State<'r, SharedNodeRegistry>,
    pub extensions: State<'r, SharedExtensions>,
    pub workflow_service: State<'r, SharedWorkflowService>,
}

impl<'r, 'de: 'r, R: Runtime> CommandArg<'de, R> for PortOptionsCommandState<'r> {
    fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError> {
        Ok(Self {
            registry: managed_state(&command, "registry")?,
            extensions: managed_state(&command, "extensions")?,
            workflow_service: managed_state(&command, "workflowService")?,
        })
    }
}

fn managed_state<'r, 'de: 'r, T: Send + Sync + 'static, R: Runtime>(
    command: &CommandItem<'de, R>,
    key: &'static str,
) -> Result<State<'r, T>, InvokeError> {
    State::from_command(CommandItem {
        plugin: command.plugin,
        name: command.name,
        key,
        message: command.message,
        acl: command.acl,
    })
}

#[cfg(test)]
#[path = "command_state_tests.rs"]
mod tests;

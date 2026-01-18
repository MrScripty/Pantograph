//! Tauri commands for workflow operations
//!
//! These commands expose the workflow engine to the frontend.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::{command, ipc::Channel, State};
use uuid::Uuid;

use crate::agent::rag::SharedRagManager;
use crate::llm::gateway::SharedGateway;

use super::engine::{WorkflowEngine, WorkflowResult};
use super::events::WorkflowEvent;
use super::node::ExecutionContext;
use super::registry::NodeRegistry;
use super::types::{NodeDefinition, PortDataType, WorkflowFile, WorkflowGraph, WorkflowMetadata};
use super::validation::validate_connection as validate_connection_internal;

/// Get the workflows directory path
fn get_workflows_dir() -> Result<PathBuf, String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let workflows_dir = project_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflows_dir)
        .map_err(|e| format!("Failed to create workflows directory: {}", e))?;
    Ok(workflows_dir)
}

/// Execute a workflow graph
///
/// Validates the graph, executes nodes in topological order, and
/// streams events to the frontend via the provided channel.
#[command]
pub async fn execute_workflow(
    graph: WorkflowGraph,
    gateway: State<'_, SharedGateway>,
    rag_manager: State<'_, SharedRagManager>,
    channel: Channel<WorkflowEvent>,
) -> Result<WorkflowResult, String> {
    // Get project root - use CARGO_MANIFEST_DIR like main.rs does
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // Create execution context
    let context = ExecutionContext {
        project_root,
        abort_signal: Arc::new(AtomicBool::new(false)),
        gateway: gateway.inner().clone(),
        rag_manager: rag_manager.inner().clone(),
        execution_id: Uuid::new_v4().to_string(),
    };

    // Create engine and execute
    let engine = WorkflowEngine::new();

    engine
        .execute(graph, context, channel)
        .await
        .map_err(|e| e.to_string())
}

/// Validate a connection between two port types
///
/// Returns true if the source type can connect to the target type.
/// Used by the frontend to provide real-time connection validation.
#[command]
pub fn validate_workflow_connection(source_type: PortDataType, target_type: PortDataType) -> bool {
    validate_connection_internal(&source_type, &target_type)
}

/// Get all available node definitions
///
/// Returns the complete catalog of node types that can be used
/// in workflow graphs. Used to populate the node palette.
#[command]
pub fn get_node_definitions() -> Vec<NodeDefinition> {
    NodeRegistry::new().all_definitions()
}

/// Get node definitions grouped by category
///
/// Returns node definitions organized by their category (input, processing, etc.)
/// for easier display in the node palette.
#[command]
pub fn get_node_definitions_by_category() -> std::collections::HashMap<String, Vec<NodeDefinition>>
{
    NodeRegistry::new().definitions_by_category()
}

/// Get a single node definition by type
///
/// Returns the definition for a specific node type, or None if not found.
#[command]
pub fn get_node_definition(node_type: String) -> Option<NodeDefinition> {
    NodeRegistry::new().get_definition(&node_type).cloned()
}

// --- Workflow Persistence ---

/// Save a workflow to disk
///
/// Saves the workflow graph with metadata to a JSON file in the workflows directory.
/// Returns the path to the saved file.
#[command]
pub fn save_workflow(name: String, graph: WorkflowGraph) -> Result<String, String> {
    let workflows_dir = get_workflows_dir()?;

    // Sanitize the name for filesystem
    let safe_name: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect();

    let file_path = workflows_dir.join(format!("{}.json", safe_name));

    // Check if file exists and update modified time, otherwise create new
    let workflow_file = if file_path.exists() {
        // Load existing to preserve created time
        let existing = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read existing workflow: {}", e))?;
        let mut existing: WorkflowFile = serde_json::from_str(&existing)
            .map_err(|e| format!("Failed to parse existing workflow: {}", e))?;

        existing.metadata.name = name;
        existing.metadata.modified = chrono::Utc::now().to_rfc3339();
        existing.graph = graph;
        existing
    } else {
        WorkflowFile::new(name, graph)
    };

    let json = serde_json::to_string_pretty(&workflow_file)
        .map_err(|e| format!("Failed to serialize workflow: {}", e))?;

    fs::write(&file_path, json)
        .map_err(|e| format!("Failed to write workflow file: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

/// Load a workflow from disk
///
/// Loads a workflow file from the given path.
#[command]
pub fn load_workflow(path: String) -> Result<WorkflowFile, String> {
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read workflow file: {}", e))?;

    let workflow: WorkflowFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse workflow file: {}", e))?;

    Ok(workflow)
}

/// List all saved workflows
///
/// Returns metadata for all workflows in the workflows directory.
#[command]
pub fn list_workflows() -> Result<Vec<WorkflowMetadata>, String> {
    let workflows_dir = get_workflows_dir()?;

    let mut workflows = Vec::new();

    let entries = fs::read_dir(&workflows_dir)
        .map_err(|e| format!("Failed to read workflows directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "json") {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    if let Ok(workflow) = serde_json::from_str::<WorkflowFile>(&content) {
                        workflows.push(workflow.metadata);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read workflow file {:?}: {}", path, e);
                }
            }
        }
    }

    // Sort by modified date descending (most recent first)
    workflows.sort_by(|a, b| b.modified.cmp(&a.modified));

    Ok(workflows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_connection() {
        assert!(validate_workflow_connection(
            PortDataType::String,
            PortDataType::String
        ));
        assert!(validate_workflow_connection(
            PortDataType::String,
            PortDataType::Prompt
        ));
        assert!(validate_workflow_connection(
            PortDataType::Any,
            PortDataType::Image
        ));
        assert!(!validate_workflow_connection(
            PortDataType::Image,
            PortDataType::String
        ));
    }

    #[test]
    fn test_get_node_definitions() {
        let defs = get_node_definitions();
        assert!(!defs.is_empty());

        // Check for some expected nodes
        assert!(defs.iter().any(|d| d.node_type == "text-input"));
        assert!(defs.iter().any(|d| d.node_type == "llm-inference"));
        assert!(defs.iter().any(|d| d.node_type == "text-output"));
    }

    #[test]
    fn test_get_node_definitions_by_category() {
        let grouped = get_node_definitions_by_category();

        assert!(grouped.contains_key("input"));
        assert!(grouped.contains_key("processing"));
        assert!(grouped.contains_key("output"));
    }

    #[test]
    fn test_get_node_definition() {
        let def = get_node_definition("text-input".to_string());
        assert!(def.is_some());
        assert_eq!(def.unwrap().node_type, "text-input");

        let missing = get_node_definition("nonexistent".to_string());
        assert!(missing.is_none());
    }
}

//! Tool nodes - file operations and other tools
//!
//! These nodes wrap existing RIG tools to provide file operations
//! within workflows.

use std::collections::HashMap;

use async_trait::async_trait;
use tauri::ipc::Channel;
use tokio::fs;

use crate::workflow::events::WorkflowEvent;
use crate::workflow::node::{ExecutionContext, InputsExt, Node, NodeError, NodeInputs, NodeOutputs};
use crate::workflow::types::{
    ExecutionMode, NodeCategory, NodeDefinition, PortDataType, PortDefinition,
};

/// Read file node - reads content from a file
///
/// Reads the content of a file relative to the project root.
pub struct ReadFileNode {
    id: String,
    definition: NodeDefinition,
}

impl ReadFileNode {
    /// Create a new read file node instance
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            definition: Self::definition(),
        }
    }

    /// Get the node type definition
    pub fn definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "read-file".to_string(),
            category: NodeCategory::Tool,
            label: "Read File".to_string(),
            description: "Read content from a file in the project".to_string(),
            inputs: vec![PortDefinition::required(
                "path",
                "File Path",
                PortDataType::String,
            )],
            outputs: vec![
                PortDefinition::required("content", "Content", PortDataType::String),
                PortDefinition::optional("exists", "Exists", PortDataType::Boolean),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Node for ReadFileNode {
    fn definition(&self) -> &NodeDefinition {
        &self.definition
    }

    fn id(&self) -> &str {
        &self.id
    }

    async fn execute(
        &self,
        inputs: NodeInputs,
        context: &ExecutionContext,
        channel: &Channel<WorkflowEvent>,
    ) -> Result<NodeOutputs, NodeError> {
        let path = inputs.get_string("path")?;

        // Resolve path relative to project root
        let full_path = context.project_root.join(path);

        let _ = channel.send(WorkflowEvent::node_progress(
            &self.id,
            0.5,
            Some(format!("Reading: {}", path)),
        ));

        // Check if file exists
        let exists = full_path.exists();
        let content = if exists {
            fs::read_to_string(&full_path)
                .await
                .map_err(|e| NodeError::Io(e))?
        } else {
            String::new()
        };

        let mut outputs = HashMap::new();
        outputs.insert("content".to_string(), serde_json::json!(content));
        outputs.insert("exists".to_string(), serde_json::json!(exists));

        Ok(outputs)
    }
}

/// Write file node - writes content to a file
///
/// Writes content to a file relative to the project root.
/// Creates parent directories if needed.
pub struct WriteFileNode {
    id: String,
    definition: NodeDefinition,
}

impl WriteFileNode {
    /// Create a new write file node instance
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            definition: Self::definition(),
        }
    }

    /// Get the node type definition
    pub fn definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "write-file".to_string(),
            category: NodeCategory::Tool,
            label: "Write File".to_string(),
            description: "Write content to a file in the project".to_string(),
            inputs: vec![
                PortDefinition::required("path", "File Path", PortDataType::String),
                PortDefinition::required("content", "Content", PortDataType::String),
            ],
            outputs: vec![
                PortDefinition::required("success", "Success", PortDataType::Boolean),
                PortDefinition::optional("path", "Written Path", PortDataType::String),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Node for WriteFileNode {
    fn definition(&self) -> &NodeDefinition {
        &self.definition
    }

    fn id(&self) -> &str {
        &self.id
    }

    async fn execute(
        &self,
        inputs: NodeInputs,
        context: &ExecutionContext,
        channel: &Channel<WorkflowEvent>,
    ) -> Result<NodeOutputs, NodeError> {
        let path = inputs.get_string("path")?;
        let content = inputs.get_string("content")?;

        // Resolve path relative to project root
        let full_path = context.project_root.join(path);

        let _ = channel.send(WorkflowEvent::node_progress(
            &self.id,
            0.5,
            Some(format!("Writing: {}", path)),
        ));

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| NodeError::Io(e))?;
        }

        // Write the file
        fs::write(&full_path, content)
            .await
            .map_err(|e| NodeError::Io(e))?;

        let mut outputs = HashMap::new();
        outputs.insert("success".to_string(), serde_json::json!(true));
        outputs.insert("path".to_string(), serde_json::json!(path));

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_definition() {
        let def = ReadFileNode::definition();
        assert_eq!(def.node_type, "read-file");
        assert_eq!(def.category, NodeCategory::Tool);
        assert!(def.inputs.iter().any(|p| p.id == "path" && p.required));
    }

    #[test]
    fn test_write_file_definition() {
        let def = WriteFileNode::definition();
        assert_eq!(def.node_type, "write-file");
        assert!(def.inputs.iter().any(|p| p.id == "path" && p.required));
        assert!(def.inputs.iter().any(|p| p.id == "content" && p.required));
    }
}

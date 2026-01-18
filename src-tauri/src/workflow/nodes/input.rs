//! Input nodes - sources of data for workflows
//!
//! These nodes provide the starting data for workflow execution,
//! such as user text input or image captures.

use std::collections::HashMap;

use async_trait::async_trait;
use tauri::ipc::Channel;

use crate::workflow::events::WorkflowEvent;
use crate::workflow::node::{ExecutionContext, InputsExt, Node, NodeError, NodeInputs, NodeOutputs};
use crate::workflow::types::{
    ExecutionMode, NodeCategory, NodeDefinition, PortDataType, PortDefinition,
};

/// Text input node - provides user-entered text
///
/// The text value is stored in the node's data and passed through
/// as an output.
pub struct TextInputNode {
    id: String,
    definition: NodeDefinition,
}

impl TextInputNode {
    /// Create a new text input node instance
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            definition: Self::definition(),
        }
    }

    /// Get the node type definition
    pub fn definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "text-input".to_string(),
            category: NodeCategory::Input,
            label: "Text Input".to_string(),
            description: "Provides user-entered text as input to the workflow".to_string(),
            inputs: vec![
                // Text can be provided via node data or connected from upstream
                PortDefinition::optional("text", "Text", PortDataType::String),
            ],
            outputs: vec![PortDefinition::required("text", "Text", PortDataType::String)],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Node for TextInputNode {
    fn definition(&self) -> &NodeDefinition {
        &self.definition
    }

    fn id(&self) -> &str {
        &self.id
    }

    async fn execute(
        &self,
        inputs: NodeInputs,
        _context: &ExecutionContext,
        _channel: &Channel<WorkflowEvent>,
    ) -> Result<NodeOutputs, NodeError> {
        // Get text from inputs (either from node data or upstream connection)
        let text = inputs
            .get_string_opt("text")
            .unwrap_or_default()
            .to_string();

        let mut outputs = HashMap::new();
        outputs.insert("text".to_string(), serde_json::json!(text));

        Ok(outputs)
    }
}

/// Image input node - provides image data (base64 encoded)
///
/// The image is captured from the canvas or provided via node data.
/// Outputs both the image data and the capture bounds.
pub struct ImageInputNode {
    id: String,
    definition: NodeDefinition,
}

impl ImageInputNode {
    /// Create a new image input node instance
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            definition: Self::definition(),
        }
    }

    /// Get the node type definition
    pub fn definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "image-input".to_string(),
            category: NodeCategory::Input,
            label: "Image Input".to_string(),
            description: "Provides image data (base64 encoded) from canvas capture".to_string(),
            inputs: vec![
                PortDefinition::optional("image_base64", "Image (Base64)", PortDataType::String),
                PortDefinition::optional("bounds", "Capture Bounds", PortDataType::Json),
            ],
            outputs: vec![
                PortDefinition::required("image", "Image", PortDataType::Image),
                PortDefinition::optional("bounds", "Bounds", PortDataType::Json),
            ],
            execution_mode: ExecutionMode::Manual,
        }
    }
}

#[async_trait]
impl Node for ImageInputNode {
    fn definition(&self) -> &NodeDefinition {
        &self.definition
    }

    fn id(&self) -> &str {
        &self.id
    }

    async fn execute(
        &self,
        inputs: NodeInputs,
        _context: &ExecutionContext,
        _channel: &Channel<WorkflowEvent>,
    ) -> Result<NodeOutputs, NodeError> {
        // Get image base64 from inputs
        let image_base64 = inputs
            .get_string("image_base64")
            .map_err(|_| NodeError::MissingInput("image_base64".to_string()))?;

        // Get optional bounds
        let bounds = inputs
            .get("bounds")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        let mut outputs = HashMap::new();
        outputs.insert("image".to_string(), serde_json::json!(image_base64));
        outputs.insert("bounds".to_string(), bounds);

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    // Note: Full integration tests require a mock Channel and ExecutionContext

    #[test]
    fn test_text_input_definition() {
        let def = TextInputNode::definition();
        assert_eq!(def.node_type, "text-input");
        assert_eq!(def.category, NodeCategory::Input);
        assert_eq!(def.outputs.len(), 1);
        assert_eq!(def.outputs[0].id, "text");
    }

    #[test]
    fn test_image_input_definition() {
        let def = ImageInputNode::definition();
        assert_eq!(def.node_type, "image-input");
        assert_eq!(def.category, NodeCategory::Input);
        assert_eq!(def.outputs.len(), 2);
    }
}

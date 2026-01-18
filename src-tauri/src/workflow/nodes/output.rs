//! Output nodes - display and preview nodes
//!
//! These nodes are endpoints in workflows that display results
//! or trigger UI updates in the frontend.

use std::collections::HashMap;

use async_trait::async_trait;
use tauri::ipc::Channel;

use crate::workflow::events::WorkflowEvent;
use crate::workflow::node::{ExecutionContext, InputsExt, Node, NodeError, NodeInputs, NodeOutputs};
use crate::workflow::types::{
    ExecutionMode, NodeCategory, NodeDefinition, PortDataType, PortDefinition,
};

/// Text output node - displays text result
///
/// Simply passes through the text input for display in the UI.
pub struct TextOutputNode {
    id: String,
    definition: NodeDefinition,
}

impl TextOutputNode {
    /// Create a new text output node instance
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            definition: Self::definition(),
        }
    }

    /// Get the node type definition
    pub fn definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "text-output".to_string(),
            category: NodeCategory::Output,
            label: "Text Output".to_string(),
            description: "Displays text result in the workflow output".to_string(),
            inputs: vec![PortDefinition::required("text", "Text", PortDataType::String)],
            outputs: vec![
                // Pass through for chaining
                PortDefinition::optional("text", "Text", PortDataType::String),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Node for TextOutputNode {
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
        channel: &Channel<WorkflowEvent>,
    ) -> Result<NodeOutputs, NodeError> {
        let text = inputs.get_string("text")?;

        // Emit stream event so the frontend can display the text
        let _ = channel.send(WorkflowEvent::node_stream(
            &self.id,
            "text",
            serde_json::json!({
                "type": "text",
                "content": text
            }),
        ));

        let mut outputs = HashMap::new();
        outputs.insert("text".to_string(), serde_json::json!(text));

        Ok(outputs)
    }
}

/// Component preview node - renders a component on the canvas
///
/// Emits a special event that tells the frontend to render
/// the specified component in the preview area.
pub struct ComponentPreviewNode {
    id: String,
    definition: NodeDefinition,
}

impl ComponentPreviewNode {
    /// Create a new component preview node instance
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            definition: Self::definition(),
        }
    }

    /// Get the node type definition
    pub fn definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "component-preview".to_string(),
            category: NodeCategory::Output,
            label: "Component Preview".to_string(),
            description: "Renders a Svelte component on the canvas preview".to_string(),
            inputs: vec![
                PortDefinition::required("component", "Component Path", PortDataType::Component),
                PortDefinition::optional("props", "Props", PortDataType::Json),
            ],
            outputs: vec![PortDefinition::optional(
                "rendered",
                "Rendered",
                PortDataType::Boolean,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Node for ComponentPreviewNode {
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
        channel: &Channel<WorkflowEvent>,
    ) -> Result<NodeOutputs, NodeError> {
        let component_path = inputs.get_string("component")?;
        let props = inputs
            .get("props")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // Emit special preview event for the frontend
        let _ = channel.send(WorkflowEvent::node_stream(
            &self.id,
            "preview",
            serde_json::json!({
                "type": "component_preview",
                "path": component_path,
                "props": props
            }),
        ));

        let mut outputs = HashMap::new();
        outputs.insert("rendered".to_string(), serde_json::json!(true));

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_output_definition() {
        let def = TextOutputNode::definition();
        assert_eq!(def.node_type, "text-output");
        assert_eq!(def.category, NodeCategory::Output);
        assert_eq!(def.inputs.len(), 1);
        assert!(def.inputs[0].required);
    }

    #[test]
    fn test_component_preview_definition() {
        let def = ComponentPreviewNode::definition();
        assert_eq!(def.node_type, "component-preview");
        assert_eq!(def.category, NodeCategory::Output);
        assert_eq!(def.inputs.len(), 2);
    }
}

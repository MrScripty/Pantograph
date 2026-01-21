//! Task descriptor trait and metadata types
//!
//! This module provides the `TaskDescriptor` trait that allows tasks to
//! self-describe their metadata (ports, category, label, etc.).
//!
//! This creates a single source of truth for node definitions - the task
//! implementation defines both its behavior AND its metadata.

use serde::{Deserialize, Serialize};

use crate::types::{ExecutionMode, NodeCategory, PortDataType};

/// Trait for tasks that can describe their metadata
///
/// Implementing this trait allows a task to provide its metadata
/// for UI rendering and validation without requiring a separate
/// definition in a registry.
///
/// # Example
///
/// ```ignore
/// use node_engine::{TaskDescriptor, TaskMetadata, PortMetadata};
/// use node_engine::{NodeCategory, ExecutionMode, PortDataType};
///
/// impl TaskDescriptor for MyTask {
///     fn descriptor() -> TaskMetadata {
///         TaskMetadata {
///             node_type: "my-task".to_string(),
///             category: NodeCategory::Processing,
///             label: "My Task".to_string(),
///             description: "Does something useful".to_string(),
///             inputs: vec![
///                 PortMetadata::required("input", "Input", PortDataType::String),
///             ],
///             outputs: vec![
///                 PortMetadata::optional("output", "Output", PortDataType::String),
///             ],
///             execution_mode: ExecutionMode::Reactive,
///         }
///     }
/// }
/// ```
pub trait TaskDescriptor {
    /// Get the static metadata for this task type
    fn descriptor() -> TaskMetadata
    where
        Self: Sized;
}

/// Complete metadata for a task type
///
/// This describes everything needed to render a node in the UI
/// and validate connections between nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMetadata {
    /// Unique type identifier (e.g., "llm-inference")
    pub node_type: String,
    /// Category for UI grouping
    pub category: NodeCategory,
    /// Human-readable label
    pub label: String,
    /// Description of what the task does
    pub description: String,
    /// Input port definitions
    pub inputs: Vec<PortMetadata>,
    /// Output port definitions
    pub outputs: Vec<PortMetadata>,
    /// Execution mode
    pub execution_mode: ExecutionMode,
}

/// Metadata for a port (input or output)
///
/// Describes a single port on a node, including its data type
/// and connection constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortMetadata {
    /// Port identifier (used in context keys)
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Data type
    pub data_type: PortDataType,
    /// Whether this input is required
    pub required: bool,
    /// Whether multiple connections are allowed
    pub multiple: bool,
}

impl PortMetadata {
    /// Create a new port metadata
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        data_type: PortDataType,
        required: bool,
        multiple: bool,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            data_type,
            required,
            multiple,
        }
    }

    /// Create a required port
    pub fn required(
        id: impl Into<String>,
        label: impl Into<String>,
        data_type: PortDataType,
    ) -> Self {
        Self::new(id, label, data_type, true, false)
    }

    /// Create an optional port
    pub fn optional(
        id: impl Into<String>,
        label: impl Into<String>,
        data_type: PortDataType,
    ) -> Self {
        Self::new(id, label, data_type, false, false)
    }

    /// Set this port to accept multiple connections
    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_metadata_required() {
        let port = PortMetadata::required("input", "Input", PortDataType::String);
        assert_eq!(port.id, "input");
        assert_eq!(port.label, "Input");
        assert!(port.required);
        assert!(!port.multiple);
    }

    #[test]
    fn test_port_metadata_optional() {
        let port = PortMetadata::optional("output", "Output", PortDataType::String);
        assert_eq!(port.id, "output");
        assert!(!port.required);
        assert!(!port.multiple);
    }

    #[test]
    fn test_port_metadata_multiple() {
        let port = PortMetadata::optional("tools", "Tools", PortDataType::Tools).multiple();
        assert!(port.multiple);
    }

    #[test]
    fn test_task_metadata_serialization() {
        let metadata = TaskMetadata {
            node_type: "test-task".to_string(),
            category: NodeCategory::Processing,
            label: "Test Task".to_string(),
            description: "A test task".to_string(),
            inputs: vec![PortMetadata::required("input", "Input", PortDataType::String)],
            outputs: vec![PortMetadata::optional("output", "Output", PortDataType::String)],
            execution_mode: ExecutionMode::Reactive,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("test-task"));
        assert!(json.contains("nodeType")); // camelCase
    }
}

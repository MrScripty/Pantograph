//! Workflow Engine - Node-based visual programming system
//!
//! This module provides a complete workflow execution engine where all node
//! execution, graph traversal, and validation happens in Rust. The Svelte
//! frontend is purely the visual layer.
//!
//! ## Architecture
//!
//! ```text
//! Frontend (Svelte)          Backend (Rust)
//! ┌─────────────────┐        ┌─────────────────────────────┐
//! │ NodeGraph.svelte│◄──────►│ workflow/                   │
//! │ (display only)  │        │ ├── engine.rs (execution)   │
//! └─────────────────┘        │ ├── registry.rs (nodes)     │
//!                            │ ├── validation.rs (checks)  │
//!                            │ └── nodes/*.rs (logic)      │
//!                            └─────────────────────────────┘
//! ```

pub mod commands;
pub mod engine;
pub mod events;
pub mod node;
pub mod nodes;
pub mod registry;
pub mod types;
pub mod validation;

// Re-export commonly used types
pub use commands::{
    execute_workflow, get_node_definitions, list_workflows, load_workflow, save_workflow,
    validate_workflow_connection,
};
pub use engine::{WorkflowEngine, WorkflowError, WorkflowResult};
pub use events::WorkflowEvent;
pub use node::{ExecutionContext, Node, NodeError, NodeInputs, NodeOutputs, PortValue};
pub use registry::NodeRegistry;
pub use types::{
    ExecutionMode, GraphEdge, GraphNode, NodeCategory, NodeDefinition, PortDataType,
    PortDefinition, Position, Viewport, WorkflowFile, WorkflowGraph, WorkflowMetadata,
};
pub use validation::{validate_connection, ValidationError, WorkflowValidator};

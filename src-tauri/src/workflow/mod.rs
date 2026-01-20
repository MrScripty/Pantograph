//! Workflow Engine - Node-based visual programming system
//!
//! This module provides a complete workflow execution engine using node-engine.
//! All node execution, graph traversal, and validation happens in Rust. The Svelte
//! frontend is purely the visual layer.
//!
//! ## Architecture
//!
//! ```text
//! Frontend (Svelte)          Backend (Rust)
//! ┌─────────────────┐        ┌─────────────────────────────────┐
//! │ NodeGraph.svelte│◄──────►│ workflow/                       │
//! │ (display only)  │        │ ├── commands.rs (Tauri commands)│
//! └─────────────────┘        │ ├── execution_manager.rs        │
//!                            │ ├── task_executor.rs            │
//!                            │ └── event_adapter.rs            │
//!                            │                                 │
//!                            │ node-engine crate:              │
//!                            │ ├── engine.rs (demand-driven)   │
//!                            │ ├── tasks/*.rs (node logic)     │
//!                            │ └── undo.rs (undo/redo)         │
//!                            └─────────────────────────────────┘
//! ```

pub mod commands;
pub mod event_adapter;
pub mod events;
pub mod execution_manager;
pub mod registry;
pub mod task_executor;
pub mod types;
pub mod validation;

// Re-export commonly used types
pub use commands::{
    get_node_definitions, list_workflows, load_workflow, save_workflow,
    validate_workflow_connection,
};
pub use event_adapter::TauriEventAdapter;
pub use events::WorkflowEvent;
pub use execution_manager::{ExecutionManager, ExecutionState, SharedExecutionManager, UndoRedoState};
pub use registry::NodeRegistry;
pub use task_executor::PantographTaskExecutor;
pub use types::{
    ExecutionMode, GraphEdge, GraphNode, NodeCategory, NodeDefinition, PortDataType,
    PortDefinition, Position, Viewport, WorkflowFile, WorkflowGraph, WorkflowMetadata,
};
pub use validation::{validate_connection, ValidationError, WorkflowValidator};

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
pub mod connection_intent;
pub mod event_adapter;
pub mod events;
pub mod execution_manager;
pub mod groups;
pub mod headless_workflow_commands;
pub mod model_dependencies;
pub mod model_dependency_commands;
pub mod orchestration;
pub mod python_runtime;
pub mod registry;
pub mod task_executor;
pub mod types;
pub mod validation;
pub mod workflow_definition_commands;
pub mod workflow_execution_commands;
pub mod workflow_model_review_commands;
pub mod workflow_persistence_commands;
pub mod workflow_port_query_commands;

// Re-export types used by main.rs
pub use execution_manager::{ExecutionManager, SharedExecutionManager};
pub use model_dependencies::SharedModelDependencyResolver;
pub use orchestration::SharedOrchestrationStore;

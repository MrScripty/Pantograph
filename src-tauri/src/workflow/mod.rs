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
//! └─────────────────┘        │ ├── event_adapter.rs            │
//!                            │ └── workflow_execution_*.rs     │
//!                            │                                 │
//!                            │ node-engine crate:              │
//!                            │ ├── engine.rs (demand-driven)   │
//!                            │ ├── tasks/*.rs (node logic)     │
//!                            │ └── undo.rs (undo/redo)         │
//!                            └─────────────────────────────────┘
//! ```

pub mod commands;
pub mod dependency_environment_commands;
pub mod diagnostics;
pub mod event_adapter;
pub mod events;
pub mod groups;
mod headless_diagnostics;
pub mod headless_diagnostics_transport;
mod headless_runtime;
pub mod headless_workflow_commands;
pub mod model_dependencies;
pub mod model_dependency_commands;
pub mod orchestration;
pub mod puma_lib_commands;
pub mod python_runtime;
pub mod runtime_shutdown;
pub mod types;
pub mod workflow_definition_commands;
mod workflow_edit_session;
pub mod workflow_execution_commands;
mod workflow_execution_runtime;
pub mod workflow_execution_tauri_commands;
pub mod workflow_model_review_commands;
pub mod workflow_port_query_commands;

// Re-export types used by main.rs
pub use diagnostics::WorkflowDiagnosticsStore;
pub use model_dependencies::SharedModelDependencyResolver;
pub use orchestration::SharedOrchestrationStore;

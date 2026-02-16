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
pub mod groups;
pub mod orchestration;
pub mod registry;
pub mod task_executor;
pub mod types;
pub mod validation;

// Re-export types used by main.rs
pub use execution_manager::{ExecutionManager, SharedExecutionManager};
pub use orchestration::SharedOrchestrationStore;

//! Node Engine - Graph-based workflow execution for Pantograph
//!
//! This crate provides a demand-driven, lazy evaluation workflow engine
//! built on top of graph-flow. It supports:
//!
//! - Async task execution with parallel scheduling
//! - Human-in-the-loop with WaitForInput
//! - Conditional branching and loops (GoTo/GoBack)
//! - Compressed snapshot-based undo/redo
//! - Demand-driven lazy evaluation (only compute what's needed)
//!
//! # Architecture
//!
//! The engine is built on graph-flow's Task model, with custom extensions:
//!
//! - `DemandEngine`: Pull-based lazy evaluation with version-tracked caching
//! - `UndoStack`: Compressed immutable snapshots for undo/redo
//! - `EventSink`: Generic event streaming (not tied to Tauri)
//!
//! # Example
//!
//! ```ignore
//! use node_engine::tasks::InferenceTask;
//! use graph_flow::GraphBuilder;
//!
//! let graph = GraphBuilder::new("my_workflow")
//!     .add_task(InferenceTask::new("inference_1"))
//!     .build()
//!     .unwrap();
//! ```

pub mod engine;
pub mod error;
pub mod events;
pub mod tasks;
pub mod types;
pub mod undo;

// Re-export key types
pub use engine::DemandEngine;
pub use error::{NodeEngineError, Result};
pub use events::{EventSink, WorkflowEvent};
pub use undo::UndoStack;

// Re-export graph-flow types that consumers will need
pub use graph_flow::{Context, GraphBuilder, GraphError, NextAction, Task, TaskResult};

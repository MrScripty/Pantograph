//! Node Engine - Graph-based workflow execution for Pantograph
//!
//! This crate provides the **framework** for a demand-driven, lazy evaluation
//! workflow engine built on top of graph-flow. It supports:
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
//! - `TaskDescriptor`: Self-describing tasks with port metadata
//!
//! # Task Implementations
//!
//! Task implementations have moved to the `workflow-nodes` crate.
//! This crate provides only the framework:
//!
//! - `TaskDescriptor` trait for self-describing tasks
//! - `TaskMetadata` and `PortMetadata` for node definitions
//! - `ContextKeys` helper for building context keys
//! - Type definitions (`PortDataType`, `NodeCategory`, etc.)
//!
//! # Example
//!
//! ```ignore
//! use node_engine::{TaskDescriptor, TaskMetadata, PortMetadata, PortDataType, NodeCategory};
//!
//! impl TaskDescriptor for MyTask {
//!     fn descriptor() -> TaskMetadata {
//!         TaskMetadata {
//!             node_type: "my-task".to_string(),
//!             category: NodeCategory::Processing,
//!             label: "My Task".to_string(),
//!             description: "Does something".to_string(),
//!             inputs: vec![
//!                 PortMetadata::required("input", "Input", PortDataType::String),
//!             ],
//!             outputs: vec![
//!                 PortMetadata::optional("output", "Output", PortDataType::String),
//!             ],
//!             execution_mode: ExecutionMode::Reactive,
//!         }
//!     }
//! }
//! ```

pub mod builder;
pub mod descriptor;
pub mod engine;
pub mod error;
pub mod events;
pub mod extensions;
pub mod groups;
pub mod orchestration;
pub mod registry;
pub mod tasks;
pub mod types;
pub mod undo;
pub mod validation;

// Re-export key types from engine
pub use engine::{CacheStats, CachedOutput, DemandEngine, TaskExecutor, WorkflowExecutor};
pub use error::{NodeEngineError, Result};
pub use extensions::{extension_keys, ExecutorExtensions};
pub use events::{
    BroadcastEventSink, CallbackEventSink, CompositeEventSink, EventError, EventSink,
    NullEventSink, VecEventSink, WorkflowEvent,
};
pub use types::{
    EdgeId, ExecutionMode, GraphEdge, GraphNode, NodeCategory, NodeDefinition, NodeId,
    PortDataType, PortDefinition, PortId, WorkflowGraph,
};
pub use undo::UndoStack;

// Re-export group types
pub use groups::{
    CreateGroupResult, GroupOperations, GroupValidationError, NodeGroup, PortMapping,
};

// Re-export descriptor types
pub use descriptor::{PortMetadata, TaskDescriptor, TaskMetadata};

// Re-export ContextKeys helper (only framework type from tasks module)
pub use tasks::ContextKeys;

// Re-export registry types
pub use registry::{
    CallbackNodeExecutor, NodeExecutor, NodeExecutorFactory, NodeRegistry, RegistryTaskExecutor,
    SyncCallbackNodeExecutor,
};

// Re-export orchestration types
pub use orchestration::{
    ConditionConfig, DataGraphConfig, DataGraphExecutor, LoopConfig, NodeExecutionResult,
    OrchestrationContext, OrchestrationEdge, OrchestrationEdgeId, OrchestrationEvent,
    OrchestrationExecutor, OrchestrationGraph, OrchestrationGraphId, OrchestrationGraphMetadata,
    OrchestrationNode, OrchestrationNodeId, OrchestrationNodeType, OrchestrationResult,
    OrchestrationStore,
};

// Re-export builder types
pub use builder::{OrchestrationBuilder, WorkflowBuilder};

// Re-export validation types
pub use validation::{validate_orchestration, validate_workflow, ValidationError};

// Re-export graph-flow types that consumers will need
pub use graph_flow::{Context, GraphBuilder, GraphError, NextAction, Task, TaskResult};

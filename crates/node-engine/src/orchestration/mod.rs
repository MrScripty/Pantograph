//! Orchestration layer for control flow execution.
//!
//! This module provides a two-level workflow system:
//! - **Orchestration Graphs**: High-level control flow (sequences, conditions, loops)
//! - **Data Graphs**: Low-level computation (LLM inference, validation, etc.)
//!
//! Orchestration graphs define the execution order and control flow between
//! data graphs. Each node in an orchestration graph can reference a data graph
//! that performs the actual computation.
//!
//! # Node Types
//!
//! - **Start**: Entry point of the orchestration (exactly one per graph)
//! - **End**: Exit point of the orchestration (can have multiple)
//! - **Condition**: Branch based on a boolean condition
//! - **Loop**: Iterate with max iterations and exit conditions
//! - **DataGraph**: Execute a referenced data graph
//! - **Merge**: Combine multiple execution paths
//!
//! # Example
//!
//! ```ignore
//! use node_engine::orchestration::{
//!     OrchestrationGraph, OrchestrationNode, OrchestrationNodeType,
//!     OrchestrationEdge, OrchestrationExecutor,
//! };
//!
//! // Create a simple orchestration: Start -> DataGraph -> End
//! let mut graph = OrchestrationGraph::new("my-orch", "My Orchestration");
//!
//! graph.nodes.push(OrchestrationNode::new("start", OrchestrationNodeType::Start, (0.0, 0.0)));
//! graph.nodes.push(OrchestrationNode::with_config(
//!     "generate",
//!     OrchestrationNodeType::DataGraph,
//!     (100.0, 0.0),
//!     serde_json::json!({"dataGraphId": "code-generation"}),
//! ));
//! graph.nodes.push(OrchestrationNode::new("end", OrchestrationNodeType::End, (200.0, 0.0)));
//!
//! graph.edges.push(OrchestrationEdge::new("e1", "start", "next", "generate", "input"));
//! graph.edges.push(OrchestrationEdge::new("e2", "generate", "next", "end", "input"));
//!
//! // Execute with a data graph executor
//! let executor = OrchestrationExecutor::new(my_data_executor);
//! let result = executor.execute(&graph, initial_data, &event_sink).await?;
//! ```

pub mod executor;
pub mod nodes;
pub mod store;
pub mod types;

// Re-export commonly used types
pub use executor::{DataGraphExecutor, OrchestrationEvent, OrchestrationExecutor};
pub use nodes::{NodeExecutionResult, OrchestrationContext};
pub use store::{OrchestrationGraphMetadata, OrchestrationStore};
pub use types::{
    ConditionConfig, DataGraphConfig, LoopConfig, OrchestrationEdge, OrchestrationEdgeId,
    OrchestrationGraph, OrchestrationGraphId, OrchestrationNode, OrchestrationNodeId,
    OrchestrationNodeType, OrchestrationResult,
};

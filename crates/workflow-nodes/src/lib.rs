//! Workflow Nodes
//!
//! Task/node implementations for the Pantograph workflow engine.
//! Each node is an atomic building block that can be composed into workflows.
//!
//! # Categories
//!
//! - **Input**: Nodes that accept user input or external data
//! - **Output**: Nodes that display or export results
//! - **Processing**: Nodes that transform data (LLM, embedding, etc.)
//! - **Storage**: Nodes for file and database operations
//! - **Control**: Nodes for control flow (loops, conditionals)

pub mod control;
pub mod input;
pub mod output;
pub mod processing;
pub mod storage;
pub mod system;

// Re-export all tasks for convenience
pub use control::*;
pub use input::*;
pub use output::*;
pub use processing::*;
pub use storage::*;
pub use system::*;

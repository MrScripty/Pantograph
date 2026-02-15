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
pub mod setup;
pub mod storage;
pub mod system;
pub mod tool;

// Re-export all tasks for convenience
pub use control::*;
pub use input::*;
pub use output::*;
pub use processing::*;
pub use setup::{setup_extensions, setup_extensions_with_path};
pub use storage::*;
pub use system::*;
pub use tool::*;

#[cfg(test)]
mod tests {
    use node_engine::NodeRegistry;

    #[test]
    fn test_inventory_collects_all_builtins() {
        let registry = NodeRegistry::with_builtins();
        let all = registry.all_metadata();

        #[cfg(feature = "desktop")]
        assert_eq!(all.len(), 25, "Expected 25 built-in nodes with desktop feature");
        #[cfg(not(feature = "desktop"))]
        assert_eq!(all.len(), 23, "Expected 23 built-in nodes without desktop feature");

        // Spot-check known types
        assert!(registry.has_node_type("text-input"));
        assert!(registry.has_node_type("llm-inference"));
        assert!(registry.has_node_type("conditional"));
        assert!(registry.has_node_type("text-output"));
        assert!(registry.has_node_type("process"));
        assert!(registry.has_node_type("llamacpp-inference"));
        assert!(registry.has_node_type("puma-lib"));
        assert!(registry.has_node_type("agent-tools"));
    }
}

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
        assert_eq!(
            all.len(),
            45,
            "Expected 45 built-in nodes with desktop feature"
        );
        #[cfg(not(feature = "desktop"))]
        assert_eq!(
            all.len(),
            42,
            "Expected 42 built-in nodes without desktop feature"
        );

        // Spot-check known types
        assert!(registry.has_node_type("boolean-input"));
        assert!(registry.has_node_type("number-input"));
        assert!(registry.has_node_type("text-input"));
        assert!(registry.has_node_type("vector-input"));
        assert!(registry.has_node_type("selection-input"));
        assert!(registry.has_node_type("llm-inference"));
        assert!(registry.has_node_type("conditional"));
        assert!(registry.has_node_type("text-output"));
        assert!(registry.has_node_type("vector-output"));
        assert!(registry.has_node_type("image-output"));
        assert!(registry.has_node_type("diffusion-inference"));
        assert!(registry.has_node_type("audio-input"));
        assert!(registry.has_node_type("audio-output"));
        assert!(registry.has_node_type("audio-generation"));
        assert!(registry.has_node_type("depth-estimation"));
        assert!(registry.has_node_type("process"));
        assert!(registry.has_node_type("llamacpp-inference"));
        assert!(registry.has_node_type("reranker"));
        assert!(registry.has_node_type("puma-lib"));
        assert!(registry.has_node_type("agent-tools"));
        assert!(registry.has_node_type("unload-model"));
        assert!(registry.has_node_type("pytorch-inference"));
        assert!(registry.has_node_type("kv-cache-save"));
        assert!(registry.has_node_type("kv-cache-load"));
        assert!(registry.has_node_type("kv-cache-truncate"));
        assert!(registry.has_node_type("masked-text-input"));
        assert!(registry.has_node_type("expand-settings"));
        assert!(registry.has_node_type("dependency-environment"));

        #[cfg(feature = "desktop")]
        assert!(registry.has_node_type("point-cloud-output"));
    }
}

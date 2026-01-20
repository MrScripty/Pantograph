//! Task implementations for workflow nodes
//!
//! Each task corresponds to a node type and implements graph-flow's Task trait.
//! Tasks communicate via the shared Context, storing inputs and outputs
//! with well-defined key patterns.
//!
//! # Key Conventions
//!
//! - Inputs: `{task_id}.input.{port_name}`
//! - Outputs: `{task_id}.output.{port_name}`
//! - Streaming: `{task_id}.stream.{port_name}`
//!
//! # Example
//!
//! ```ignore
//! // Set input for inference task
//! context.set("inference_1.input.prompt", "Hello, world!").await?;
//!
//! // After execution, get output
//! let response: String = context.get("inference_1.output.response").await?;
//! ```

pub mod inference;
pub mod human_input;

pub use inference::InferenceTask;
pub use human_input::HumanInputTask;

/// Helper for building context keys
pub struct ContextKeys;

impl ContextKeys {
    /// Build an input key: `{task_id}.input.{port}`
    pub fn input(task_id: &str, port: &str) -> String {
        format!("{}.input.{}", task_id, port)
    }

    /// Build an output key: `{task_id}.output.{port}`
    pub fn output(task_id: &str, port: &str) -> String {
        format!("{}.output.{}", task_id, port)
    }

    /// Build a stream key: `{task_id}.stream.{port}`
    pub fn stream(task_id: &str, port: &str) -> String {
        format!("{}.stream.{}", task_id, port)
    }

    /// Build a metadata key: `{task_id}.meta.{field}`
    pub fn meta(task_id: &str, field: &str) -> String {
        format!("{}.meta.{}", task_id, field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_keys() {
        assert_eq!(
            ContextKeys::input("task1", "prompt"),
            "task1.input.prompt"
        );
        assert_eq!(
            ContextKeys::output("task1", "response"),
            "task1.output.response"
        );
        assert_eq!(
            ContextKeys::stream("task1", "chunks"),
            "task1.stream.chunks"
        );
    }
}

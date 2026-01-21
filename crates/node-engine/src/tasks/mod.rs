//! Context key helpers for tasks
//!
//! Task implementations have moved to the `workflow-nodes` crate.
//! This module provides only the `ContextKeys` helper for building
//! context key strings.
//!
//! # Key Conventions
//!
//! - Inputs: `{task_id}.input.{port_name}`
//! - Outputs: `{task_id}.output.{port_name}`
//! - Streaming: `{task_id}.stream.{port_name}`
//! - Metadata: `{task_id}.meta.{field_name}`
//!
//! # Example
//!
//! ```ignore
//! use node_engine::ContextKeys;
//!
//! // Build keys for a task
//! let prompt_key = ContextKeys::input("inference_1", "prompt");
//! let response_key = ContextKeys::output("inference_1", "response");
//! ```

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
        assert_eq!(
            ContextKeys::meta("task1", "config"),
            "task1.meta.config"
        );
    }
}

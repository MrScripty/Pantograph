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
//! # Task Categories
//!
//! ## Input Tasks
//! - [`TextInputTask`] - Simple text passthrough
//! - [`ImageInputTask`] - Base64 image handling
//! - [`HumanInputTask`] - Interactive user input (pauses execution)
//!
//! ## Output Tasks
//! - [`TextOutputTask`] - Text display with streaming
//! - [`ComponentPreviewTask`] - Svelte component rendering
//!
//! ## Processing Tasks
//! - [`InferenceTask`] - LLM text completion
//! - [`VisionAnalysisTask`] - Image analysis with vision models
//! - [`RagSearchTask`] - Semantic document search
//!
//! ## Tool Tasks
//! - [`ReadFileTask`] - File reading
//! - [`WriteFileTask`] - File writing
//!
//! ## Control Flow Tasks
//! - [`ToolLoopTask`] - Multi-turn agent loop with tool calling
//!
//! # Example
//!
//! ```ignore
//! // Set input for inference task
//! context.set("inference_1.input.prompt", "Hello, world!").await;
//!
//! // After execution, get output
//! let response: String = context.get("inference_1.output.response").await?;
//! ```

// Input tasks
pub mod text_input;
pub mod image_input;
pub mod human_input;

// Output tasks
pub mod text_output;
pub mod component_preview;

// Processing tasks
pub mod inference;
pub mod vision_analysis;
pub mod rag_search;

// Tool tasks
pub mod read_file;
pub mod write_file;

// Control flow tasks
pub mod tool_loop;

// Re-exports - Input tasks
pub use text_input::TextInputTask;
pub use image_input::{ImageInputTask, ImageBounds};
pub use human_input::HumanInputTask;

// Re-exports - Output tasks
pub use text_output::TextOutputTask;
pub use component_preview::ComponentPreviewTask;

// Re-exports - Processing tasks
pub use inference::{InferenceTask, InferenceConfig};
pub use vision_analysis::{VisionAnalysisTask, VisionConfig};
pub use rag_search::{RagSearchTask, RagConfig, RagDocument};

// Re-exports - Tool tasks
pub use read_file::ReadFileTask;
pub use write_file::WriteFileTask;

// Re-exports - Control flow tasks
pub use tool_loop::{ToolLoopTask, ToolLoopConfig, ToolDefinition, ToolCall};

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

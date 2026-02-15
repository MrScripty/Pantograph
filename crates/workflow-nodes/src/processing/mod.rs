//! Processing nodes
//!
//! Nodes that transform, analyze, or generate data.

mod embedding;
mod inference;
mod json_filter;
mod llamacpp_inference;
mod ollama_inference;
mod unload_model;
mod validator;
mod vision_analysis;

pub use embedding::{EmbeddingConfig, EmbeddingTask};
pub use inference::{InferenceConfig, InferenceTask, ToolCall as InferenceToolCall, ToolDefinition as InferenceToolDefinition};
pub use json_filter::{JsonFilterConfig, JsonFilterTask};
pub use llamacpp_inference::LlamaCppInferenceTask;
pub use ollama_inference::OllamaInferenceTask;
pub use unload_model::UnloadModelTask;
pub use validator::{ValidationResult, ValidatorConfig, ValidatorTask};
pub use vision_analysis::{VisionAnalysisTask, VisionConfig};

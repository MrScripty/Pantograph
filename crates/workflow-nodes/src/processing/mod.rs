//! Processing nodes
//!
//! Nodes that transform, analyze, or generate data.

mod audio_generation;
mod depth_estimation;
mod diffusion_inference;
mod embedding;
mod expand_settings;
mod inference;
mod json_filter;
mod llamacpp_inference;
mod ollama_inference;
mod pytorch_inference;
mod unload_model;
mod validator;
mod vision_analysis;

pub use audio_generation::AudioGenerationTask;
pub use depth_estimation::DepthEstimationTask;
pub use diffusion_inference::DiffusionInferenceTask;
pub use embedding::{EmbeddingConfig, EmbeddingTask};
pub use expand_settings::ExpandSettingsTask;
pub use inference::{
    InferenceConfig, InferenceTask, ToolCall as InferenceToolCall,
    ToolDefinition as InferenceToolDefinition,
};
pub use json_filter::{JsonFilterConfig, JsonFilterTask};
pub use llamacpp_inference::LlamaCppInferenceTask;
pub use ollama_inference::OllamaInferenceTask;
pub use pytorch_inference::PyTorchInferenceTask;
pub use unload_model::UnloadModelTask;
pub use validator::{ValidationResult, ValidatorConfig, ValidatorTask};
pub use vision_analysis::{VisionAnalysisTask, VisionConfig};

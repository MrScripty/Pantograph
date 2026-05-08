//! Processing nodes
//!
//! Nodes that transform, analyze, or generate data.

mod audio_generation;
mod dependency_environment;
mod depth_estimation;
mod expand_settings;
mod inference;
mod json_filter;
mod onnx_inference;
mod unload_model;
mod validator;
mod vision_analysis;

pub use audio_generation::AudioGenerationTask;
pub use dependency_environment::DependencyEnvironmentTask;
pub use depth_estimation::DepthEstimationTask;
pub use expand_settings::ExpandSettingsTask;
pub use inference::{
    InferenceConfig, InferenceTask, ToolCall as InferenceToolCall,
    ToolDefinition as InferenceToolDefinition,
};
pub use json_filter::{JsonFilterConfig, JsonFilterTask};
pub use onnx_inference::OnnxInferenceTask;
pub use unload_model::UnloadModelTask;
pub use validator::{ValidationResult, ValidatorConfig, ValidatorTask};
pub use vision_analysis::{VisionAnalysisTask, VisionConfig};

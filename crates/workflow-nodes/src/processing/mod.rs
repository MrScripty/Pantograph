//! Processing nodes
//!
//! Nodes that transform, analyze, or generate data.

pub const DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID: &str = "dependency_environment_sidecar";

mod dependency_environment;
mod inference;
mod json_filter;
mod unload_model;
mod validator;

pub use dependency_environment::DependencyEnvironmentTask;
pub use inference::{
    InferenceTask, ToolCall as InferenceToolCall, ToolDefinition as InferenceToolDefinition,
};
pub use json_filter::{JsonFilterConfig, JsonFilterTask};
pub use unload_model::UnloadModelTask;
pub use validator::{ValidationResult, ValidatorConfig, ValidatorTask};

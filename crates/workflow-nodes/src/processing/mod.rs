//! Processing nodes
//!
//! Nodes that transform, analyze, or generate data.

mod embedding;
mod inference;
mod vision_analysis;

pub use embedding::{EmbeddingConfig, EmbeddingTask};
pub use inference::{InferenceConfig, InferenceTask};
pub use vision_analysis::{VisionAnalysisTask, VisionConfig};

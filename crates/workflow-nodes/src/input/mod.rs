//! Input nodes
//!
//! Nodes that accept user input or external data.

mod audio_input;
mod human_input;
mod image_input;
#[cfg(feature = "desktop")]
mod linked_input;
mod masked_text_input;
mod model_provider;
mod puma_lib;
mod selection_input;
mod text_input;
mod vector_input;

pub use audio_input::AudioInputTask;
pub use human_input::HumanInputTask;
pub use image_input::{ImageBounds, ImageInputTask};
#[cfg(feature = "desktop")]
pub use linked_input::LinkedInputTask;
pub use masked_text_input::{MaskedTextInputTask, TextSegment};
pub use model_provider::{ModelInfo, ModelProviderTask};
pub use puma_lib::PumaLibTask;
pub use selection_input::SelectionInputTask;
pub use text_input::TextInputTask;
pub use vector_input::VectorInputTask;

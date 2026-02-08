//! Input nodes
//!
//! Nodes that accept user input or external data.

mod human_input;
mod image_input;
#[cfg(feature = "desktop")]
mod linked_input;
mod model_provider;
mod puma_lib;
mod text_input;

pub use human_input::HumanInputTask;
pub use image_input::{ImageBounds, ImageInputTask};
#[cfg(feature = "desktop")]
pub use linked_input::LinkedInputTask;
pub use model_provider::{ModelInfo, ModelProviderTask};
pub use puma_lib::PumaLibTask;
pub use text_input::TextInputTask;

//! Input nodes
//!
//! Nodes that accept user input or external data.

mod human_input;
mod image_input;
mod text_input;

pub use human_input::HumanInputTask;
pub use image_input::{ImageBounds, ImageInputTask};
pub use text_input::TextInputTask;

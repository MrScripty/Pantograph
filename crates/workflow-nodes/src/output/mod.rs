//! Output nodes
//!
//! Nodes that display or export results.

#[cfg(feature = "desktop")]
mod component_preview;
mod text_output;

#[cfg(feature = "desktop")]
pub use component_preview::ComponentPreviewTask;
pub use text_output::TextOutputTask;

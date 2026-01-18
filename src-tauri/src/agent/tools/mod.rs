//! Agent tools for component generation
//!
//! This module provides the tools available to the LLM agent:
//! - File operations (read/write Svelte components)
//! - List operations (components, templates)
//! - Tailwind color palette

mod error;
mod list;
mod read;
mod tailwind;
mod validation;
mod write;

// Re-export read tool
pub use read::ReadGuiFileTool;

// Re-export write tool
pub use write::{WriteGuiFileArgs, WriteGuiFileTool};

// Re-export list tools
pub use list::{
    ListComponentsTool,
    ListTemplatesTool,
    ReadTemplateTool,
};

// Re-export tailwind tool
pub use tailwind::GetTailwindColorsTool;

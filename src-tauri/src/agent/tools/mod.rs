//! Agent tools for component generation and documentation search
//!
//! This module provides the tools available to the LLM agent:
//! - File operations (read/write Svelte components)
//! - List operations (components, templates)
//! - Tailwind color palette
//! - Documentation search (keyword and vector-based)

mod docs_search;
mod error;
mod list;
mod read;
mod tailwind;
mod validation;
mod write;

// Re-export error type
pub use error::ToolError;

// Re-export validation constants and helpers
pub use validation::{capitalize_first, MATHML_ELEMENTS, STANDARD_HTML_ELEMENTS, SVG_ELEMENTS};

// Re-export read tool
pub use read::{ReadGuiFileArgs, ReadGuiFileTool};

// Re-export write tool
pub use write::{WriteGuiFileArgs, WriteGuiFileTool};

// Re-export list tools
pub use list::{
    ListComponentsArgs, ListComponentsTool,
    ListTemplatesArgs, ListTemplatesTool,
    ReadTemplateArgs, ReadTemplateTool,
};

// Re-export tailwind tool
pub use tailwind::{GetTailwindColorsArgs, GetTailwindColorsTool};

// Re-export docs search tools
pub use docs_search::{
    SearchSvelteDocsArgs, SearchSvelteDocsTool,
    SearchSvelteDocsVectorArgs, SearchSvelteDocsVectorTool,
    VectorDocResult, VectorDocSearchOutput,
};

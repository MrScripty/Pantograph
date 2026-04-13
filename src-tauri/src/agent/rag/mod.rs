//! RAG (Retrieval Augmented Generation) module for Svelte documentation
//!
//! This module provides semantic search capabilities using LanceDB for persistent
//! vector storage and a local embedding model.

mod error;
mod lancedb;
mod manager;
mod types;

// Re-export data types
pub use types::{DatabaseInfo, IndexingProgress, RagStatus, SvelteDoc};

// Re-export manager
pub use manager::{RagManager, SharedRagManager, create_rag_manager};

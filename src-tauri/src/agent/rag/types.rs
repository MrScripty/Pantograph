//! Data types for RAG operations

use rig::Embed;
use serde::{Deserialize, Serialize};

use crate::agent::docs_index::IndexEntry;

/// A Svelte documentation entry prepared for embedding
#[derive(Embed, Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct SvelteDoc {
    /// Unique identifier
    pub id: String,
    /// Display title
    pub title: String,
    /// Section name
    pub section: String,
    /// Brief summary
    pub summary: String,
    /// Full content - this is what gets embedded
    #[embed]
    pub content: String,
}

impl From<&IndexEntry> for SvelteDoc {
    fn from(entry: &IndexEntry) -> Self {
        Self {
            id: entry.id.clone(),
            title: entry.title.clone(),
            section: entry.section.clone(),
            summary: entry.summary.clone(),
            // Combine title and content for better semantic matching
            content: format!("{}\n\n{}", entry.title, entry.content),
        }
    }
}

/// Progress information for indexing operations
#[derive(Debug, Clone, Serialize)]
pub struct IndexingProgress {
    pub current: usize,
    pub total: usize,
    pub status: String,
}

/// Status of the RAG system
#[derive(Debug, Clone, Serialize, Default)]
pub struct RagStatus {
    /// Whether documentation is downloaded
    pub docs_available: bool,
    /// Number of documentation files
    pub docs_count: usize,
    /// Whether the embedding server is reachable
    pub vectorizer_available: bool,
    /// URL of the embedding server (if configured)
    pub vectorizer_url: Option<String>,
    /// Whether vectors have been indexed
    pub vectors_indexed: bool,
    /// Number of indexed vectors
    pub vectors_count: usize,
    /// Current indexing progress (if indexing)
    pub indexing_progress: Option<IndexingProgress>,
}

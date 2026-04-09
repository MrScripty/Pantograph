use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Minimal document payload required by host-side RAG search execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagDocument {
    pub id: String,
    pub title: String,
    pub section: String,
    pub summary: String,
    pub content: String,
}

#[async_trait]
pub trait RagBackend: Send + Sync {
    async fn search_as_docs(&self, query: &str, limit: usize) -> Result<Vec<RagDocument>, String>;
}

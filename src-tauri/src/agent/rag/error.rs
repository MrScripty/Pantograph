//! Error types for RAG operations

use rig::embeddings::{EmbedError, EmbeddingError};
use rig::vector_store::VectorStoreError;
use thiserror::Error;

/// Errors that can occur during RAG operations
#[derive(Debug, Error)]
pub enum RagError {
    #[error("Embedding server not available")]
    ServerNotAvailable,
    #[error("Embedding error: {0}")]
    Embedding(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Documents not available")]
    DocsNotAvailable,
    #[error("Client error: {0}")]
    Client(String),
    #[error("LanceDB error: {0}")]
    LanceDb(String),
}

impl From<String> for RagError {
    fn from(s: String) -> Self {
        RagError::Client(s)
    }
}

impl From<EmbeddingError> for RagError {
    fn from(e: EmbeddingError) -> Self {
        RagError::Embedding(e.to_string())
    }
}

impl From<VectorStoreError> for RagError {
    fn from(e: VectorStoreError) -> Self {
        RagError::Embedding(e.to_string())
    }
}

impl From<EmbedError> for RagError {
    fn from(e: EmbedError) -> Self {
        RagError::Embedding(e.to_string())
    }
}

impl From<lancedb::Error> for RagError {
    fn from(e: lancedb::Error) -> Self {
        RagError::LanceDb(e.to_string())
    }
}

//! RAG (Retrieval Augmented Generation) manager for Svelte documentation
//!
//! Provides semantic search capabilities using RIG's InMemoryVectorStore
//! and a local embedding model (e.g., Qwen3-Embedding-0.6B via llama.cpp).

use std::path::PathBuf;
use std::sync::Arc;

use rig::embeddings::EmbeddingsBuilder;
use rig::embeddings::EmbeddingError;
use rig::embeddings::EmbedError;
use rig::vector_store::in_memory_store::InMemoryVectorStore;
use rig::vector_store::VectorStoreError;
use rig::vector_store::VectorStoreIndex;
use rig::vector_store::VectorSearchRequest;
use rig::Embed;
use rig::prelude::EmbeddingsClient;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;

use super::docs::DocsManager;
use super::docs_index::IndexEntry;
use super::embeddings::{check_embedding_server, create_embedding_client, get_embedding_model_name};

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

/// Manager for RAG operations
pub struct RagManager {
    /// Path to store embeddings
    store_path: PathBuf,
    /// URL of the embedding server
    embedding_url: Option<String>,
    /// Current status
    status: RagStatus,
    /// Vector store (if initialized)
    vector_store: Option<InMemoryVectorStore<SvelteDoc>>,
    /// Cached documents (for search results)
    docs: Vec<SvelteDoc>,
}

impl RagManager {
    /// Create a new RAG manager
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            store_path: app_data_dir.join("svelte-docs"),
            embedding_url: None,
            status: RagStatus::default(),
            vector_store: None,
            docs: Vec::new(),
        }
    }

    /// Set the embedding server URL
    pub fn set_embedding_url(&mut self, url: String) {
        self.embedding_url = Some(url.clone());
        self.status.vectorizer_url = Some(url);
    }

    /// Get the current status
    pub fn status(&self) -> &RagStatus {
        &self.status
    }

    /// Check if the embedding server is available
    pub async fn check_vectorizer(&mut self) -> bool {
        if let Some(url) = &self.embedding_url {
            let available = check_embedding_server(url).await;
            self.status.vectorizer_available = available;
            available
        } else {
            self.status.vectorizer_available = false;
            false
        }
    }

    /// Update docs status from DocsManager
    pub fn update_docs_status(&mut self, docs_manager: &DocsManager) {
        let status = docs_manager.get_status();
        self.status.docs_available = status.available;
        self.status.docs_count = status.doc_count;
    }

    /// Check if we need to re-index
    #[allow(dead_code)]
    pub fn needs_indexing(&self, current_version: &str) -> bool {
        let version_file = self.store_path.join("embeddings-version.txt");
        if !version_file.exists() {
            return true;
        }

        match std::fs::read_to_string(&version_file) {
            Ok(stored_version) => stored_version.trim() != current_version,
            Err(_) => true,
        }
    }

    /// Index all documentation entries
    pub async fn index_documents(
        &mut self,
        entries: &[IndexEntry],
        version: &str,
        on_progress: impl Fn(IndexingProgress),
    ) -> Result<(), RagError> {
        log::info!("Validating embedding URL: {:?}", self.embedding_url);
        let embedding_url = self
            .embedding_url
            .as_ref()
            .ok_or(RagError::ServerNotAvailable)?;

        // Check server availability
        log::info!("Checking embedding server health at: {}", embedding_url);
        let server_available = check_embedding_server(embedding_url).await;
        log::info!("Embedding server health check result: {}", server_available);
        if !server_available {
            log::error!("Embedding server health check failed - server not responding");
            return Err(RagError::ServerNotAvailable);
        }

        let total = entries.len();
        on_progress(IndexingProgress {
            current: 0,
            total,
            status: "Preparing documents...".to_string(),
        });

        // Convert entries to SvelteDoc
        let docs: Vec<SvelteDoc> = entries.iter().map(SvelteDoc::from).collect();

        on_progress(IndexingProgress {
            current: 0,
            total,
            status: "Connecting to embedding server...".to_string(),
        });

        // Create embedding client
        let client = create_embedding_client(embedding_url)?;
        let model_name = get_embedding_model_name(embedding_url).await;
        log::info!("Created embedding client for model: {}", model_name);
        let embedding_model = client.embedding_model(&model_name);

        on_progress(IndexingProgress {
            current: 0,
            total,
            status: "Generating embeddings...".to_string(),
        });

        // Build embeddings
        log::info!("Sending {} documents to embedding server for vectorization", docs.len());
        let embeddings = EmbeddingsBuilder::new(embedding_model)
            .documents(docs.clone())?
            .build()
            .await?;
        log::info!("Received {} embeddings from server", embeddings.len());

        on_progress(IndexingProgress {
            current: total,
            total,
            status: "Building vector index...".to_string(),
        });

        // Create vector store
        let vector_store =
            InMemoryVectorStore::from_documents_with_id_f(embeddings, |doc| doc.id.clone());

        // Save to disk
        self.save_embeddings(&docs, &vector_store, version).await?;

        // Update state
        self.vector_store = Some(vector_store);
        self.docs = docs;
        self.status.vectors_indexed = true;
        self.status.vectors_count = total;

        on_progress(IndexingProgress {
            current: total,
            total,
            status: "Complete".to_string(),
        });

        Ok(())
    }

    /// Save embeddings to disk for persistence
    async fn save_embeddings(
        &self,
        docs: &[SvelteDoc],
        _store: &InMemoryVectorStore<SvelteDoc>,
        version: &str,
    ) -> Result<(), RagError> {
        // Create directory if needed
        tokio::fs::create_dir_all(&self.store_path).await?;

        // Save version
        let version_path = self.store_path.join("embeddings-version.txt");
        tokio::fs::write(&version_path, version).await?;

        // Save documents (we'll regenerate embeddings on load)
        let docs_path = self.store_path.join("embedded-docs.json");
        let docs_json = serde_json::to_string_pretty(docs)?;
        tokio::fs::write(&docs_path, docs_json).await?;

        log::info!("Saved {} embedded documents to disk", docs.len());
        Ok(())
    }

    /// Load embeddings from disk
    pub async fn load_from_disk(&mut self) -> Result<bool, RagError> {
        let docs_path = self.store_path.join("embedded-docs.json");

        if !docs_path.exists() {
            return Ok(false);
        }

        // Load documents
        let docs_json = tokio::fs::read_to_string(&docs_path).await?;
        let docs: Vec<SvelteDoc> = serde_json::from_str(&docs_json)?;

        if docs.is_empty() {
            return Ok(false);
        }

        // Check if we have embedding server to regenerate the index
        let embedding_url = match &self.embedding_url {
            Some(url) => url.clone(),
            None => return Ok(false),
        };

        if !check_embedding_server(&embedding_url).await {
            // Server not available, but we have the docs
            self.docs = docs;
            self.status.vectors_count = self.docs.len();
            // Can't search without embeddings, but status shows we have cached docs
            return Ok(false);
        }

        // Regenerate embeddings
        let client = create_embedding_client(&embedding_url)?;
        let model_name = get_embedding_model_name(&embedding_url).await;
        let embedding_model = client.embedding_model(&model_name);

        let embeddings = EmbeddingsBuilder::new(embedding_model)
            .documents(docs.clone())?
            .build()
            .await?;

        let vector_store =
            InMemoryVectorStore::from_documents_with_id_f(embeddings, |doc| doc.id.clone());

        self.vector_store = Some(vector_store);
        self.docs = docs;
        self.status.vectors_indexed = true;
        self.status.vectors_count = self.docs.len();

        log::info!("Loaded {} embedded documents from disk", self.docs.len());
        Ok(true)
    }

    /// Perform semantic search
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SvelteDoc>, RagError> {
        let embedding_url = self
            .embedding_url
            .as_ref()
            .ok_or(RagError::ServerNotAvailable)?;

        let store = self
            .vector_store
            .as_ref()
            .ok_or(RagError::DocsNotAvailable)?;

        // Create embedding client for the query
        let client = create_embedding_client(embedding_url)?;
        let model_name = get_embedding_model_name(embedding_url).await;
        let embedding_model = client.embedding_model(&model_name);

        // Create index and search
        // Clone store since index() takes ownership
        let index = store.clone().index(embedding_model);

        let request = VectorSearchRequest::builder()
            .query(query)
            .samples(limit as u64)
            .build()?;

        let results: Vec<(f64, String, SvelteDoc)> = index
            .top_n::<SvelteDoc>(request)
            .await?;

        // Extract just the documents (results are (score, id, doc) tuples)
        let docs: Vec<SvelteDoc> = results.into_iter().map(|(_, _, doc)| doc).collect();

        Ok(docs)
    }

    /// Check if semantic search is available
    #[allow(dead_code)]
    pub fn is_search_available(&self) -> bool {
        self.vector_store.is_some() && self.embedding_url.is_some()
    }

    /// Clear the vector store and cached data
    pub async fn clear_cache(&mut self) -> Result<(), RagError> {
        self.vector_store = None;
        self.docs.clear();
        self.status.vectors_indexed = false;
        self.status.vectors_count = 0;

        // Remove cached files
        let version_path = self.store_path.join("embeddings-version.txt");
        let docs_path = self.store_path.join("embedded-docs.json");

        if version_path.exists() {
            tokio::fs::remove_file(&version_path).await?;
        }
        if docs_path.exists() {
            tokio::fs::remove_file(&docs_path).await?;
        }

        log::info!("Cleared RAG cache");
        Ok(())
    }
}

/// Thread-safe wrapper for RagManager
pub type SharedRagManager = Arc<RwLock<RagManager>>;

/// Create a new shared RAG manager
pub fn create_rag_manager(app_data_dir: PathBuf) -> SharedRagManager {
    Arc::new(RwLock::new(RagManager::new(app_data_dir)))
}

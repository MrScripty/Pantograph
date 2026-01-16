//! RAG (Retrieval Augmented Generation) manager for Svelte documentation
//!
//! Provides semantic search capabilities using LanceDB for persistent vector storage
//! and a local embedding model (e.g., Qwen3-Embedding-0.6B via llama.cpp).
//!
//! Documents are chunked at H2/H3 header boundaries for finer-grained retrieval.

use std::path::PathBuf;
use std::sync::Arc;

// Arrow types - must match lancedb's arrow version (56.2)
use arrow_array::types::Float64Type;
use arrow_array::{
    ArrayRef, BooleanArray, FixedSizeListArray, Int32Array, RecordBatch, RecordBatchIterator,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures_util::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use rig::embeddings::{EmbedError, Embedding, EmbeddingError, EmbeddingModel, EmbeddingsBuilder};
use rig::one_or_many::OneOrMany;
use rig::prelude::EmbeddingsClient;
use rig::vector_store::VectorStoreError;
use rig::Embed;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;

use super::chunker::{chunk_document, ChunkConfig};
use super::docs::DocsManager;
use super::docs_index::IndexEntry;
use super::embeddings::{check_embedding_server, create_embedding_client, get_embedding_model_name};
use super::types::DocChunk;

use crate::config::EmbeddingMemoryMode;
use crate::llm::{BackendConfig, SharedGateway};

/// Number of chunks to process per embedding batch for progress updates.
/// Small enough for frequent UI updates, large enough to minimize HTTP overhead.
const EMBEDDING_BATCH_SIZE: usize = 10;

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

/// Default embedding dimensions for common models
const DEFAULT_EMBEDDING_DIM: i32 = 1024; // Qwen3-Embedding-0.6B uses 1024 dimensions

/// LanceDB table name for doc chunks
const CHUNKS_TABLE_NAME: &str = "doc_chunks";

/// Manager for RAG operations
pub struct RagManager {
    /// Path to store LanceDB data
    store_path: PathBuf,
    /// URL of the embedding server
    embedding_url: Option<String>,
    /// Current status
    status: RagStatus,
    /// LanceDB connection (if initialized)
    db: Option<lancedb::Connection>,
    /// Cached embedding dimension (detected from first embedding)
    embedding_dim: Option<i32>,
    /// Chunking configuration
    chunk_config: ChunkConfig,
    /// Embedding backend config for sequential mode switching
    embedding_config: Option<BackendConfig>,
}

impl RagManager {
    /// Create a new RAG manager
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            store_path: app_data_dir.join("lancedb"),
            embedding_url: None,
            status: RagStatus::default(),
            db: None,
            embedding_dim: None,
            chunk_config: ChunkConfig::default(),
            embedding_config: None,
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

    /// Set the embedding backend config for sequential mode
    ///
    /// This config is used when we need to temporarily switch the main
    /// backend to embedding mode for search operations.
    pub fn set_embedding_config(&mut self, config: BackendConfig) {
        self.embedding_config = Some(config);
    }

    /// Get the stored embedding config
    pub fn embedding_config(&self) -> Option<&BackendConfig> {
        self.embedding_config.as_ref()
    }

    // ─── GATEWAY-AWARE SEARCH METHODS ──────────────────────────────────

    /// Perform semantic search with gateway support for different memory modes
    ///
    /// This method handles all three memory modes:
    /// - CpuParallel/GpuParallel: Uses the dedicated embedding server
    /// - Sequential: Temporarily switches the main backend to embedding mode
    ///
    /// # Arguments
    /// * `query` - The search query text
    /// * `limit` - Maximum number of results
    /// * `gateway` - The inference gateway
    /// * `app` - Tauri app handle (needed for sequential mode)
    /// * `mode` - Current embedding memory mode
    pub async fn search_with_gateway(
        &self,
        query: &str,
        limit: usize,
        gateway: &SharedGateway,
        app: &tauri::AppHandle,
        mode: EmbeddingMemoryMode,
    ) -> Result<Vec<DocChunk>, RagError> {
        match mode {
            EmbeddingMemoryMode::CpuParallel | EmbeddingMemoryMode::GpuParallel => {
                // Parallel mode: use the dedicated embedding server URL
                let url = gateway.embedding_url().await
                    .ok_or(RagError::ServerNotAvailable)?;
                self.search_with_url(&url, query, limit).await
            }
            EmbeddingMemoryMode::Sequential => {
                // Sequential mode: need to swap models
                self.search_with_swap(query, limit, gateway, app).await
            }
        }
    }

    /// Perform search using a specific embedding server URL
    ///
    /// This is the core search implementation that creates embeddings
    /// from the given URL and queries LanceDB.
    pub async fn search_with_url(
        &self,
        embedding_url: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<DocChunk>, RagError> {
        let db = self.db.as_ref().ok_or(RagError::DocsNotAvailable)?;

        // Open the chunks table
        let table = db.open_table(CHUNKS_TABLE_NAME).execute().await?;

        // Create embedding for query
        let client = create_embedding_client(embedding_url)?;
        let model_name = get_embedding_model_name(embedding_url).await;
        let embedding_model = client.embedding_model(&model_name);

        // Get query embedding
        let query_embedding = EmbeddingModel::embed_text(&embedding_model, query)
            .await
            .map_err(|e| RagError::Embedding(e.to_string()))?;

        // Perform vector search
        let mut results = table
            .vector_search(query_embedding.vec.clone())
            .map_err(|e| RagError::LanceDb(e.to_string()))?
            .limit(limit)
            .execute()
            .await?;

        // Convert results to DocChunks
        let mut chunks = Vec::new();
        while let Some(batch_result) = results.try_next().await.transpose() {
            let batch = batch_result.map_err(|e| RagError::LanceDb(e.to_string()))?;

            // Helper to extract string column
            fn get_string_col<'a>(
                batch: &'a RecordBatch,
                name: &str,
            ) -> Result<&'a StringArray, RagError> {
                batch
                    .column_by_name(name)
                    .ok_or_else(|| RagError::LanceDb(format!("Missing {} column", name)))?
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| RagError::LanceDb(format!("{} column has wrong type", name)))
            }

            // Helper to extract i32 column
            fn get_i32_col<'a>(
                batch: &'a RecordBatch,
                name: &str,
            ) -> Result<&'a Int32Array, RagError> {
                batch
                    .column_by_name(name)
                    .ok_or_else(|| RagError::LanceDb(format!("Missing {} column", name)))?
                    .as_any()
                    .downcast_ref::<Int32Array>()
                    .ok_or_else(|| RagError::LanceDb(format!("{} column has wrong type", name)))
            }

            // Helper to extract bool column
            fn get_bool_col<'a>(
                batch: &'a RecordBatch,
                name: &str,
            ) -> Result<&'a BooleanArray, RagError> {
                batch
                    .column_by_name(name)
                    .ok_or_else(|| RagError::LanceDb(format!("Missing {} column", name)))?
                    .as_any()
                    .downcast_ref::<BooleanArray>()
                    .ok_or_else(|| RagError::LanceDb(format!("{} column has wrong type", name)))
            }

            // Extract columns
            let ids = get_string_col(&batch, "id")?;
            let doc_ids = get_string_col(&batch, "doc_id")?;
            let titles = get_string_col(&batch, "title")?;
            let doc_titles = get_string_col(&batch, "doc_title")?;
            let sections = get_string_col(&batch, "section")?;
            let chunk_indices = get_i32_col(&batch, "chunk_index")?;
            let total_chunks_col = get_i32_col(&batch, "total_chunks")?;
            let header_contexts = get_string_col(&batch, "header_context")?;
            let contents = get_string_col(&batch, "content")?;
            let has_codes = get_bool_col(&batch, "has_code")?;

            for i in 0..batch.num_rows() {
                chunks.push(DocChunk {
                    id: ids.value(i).to_string(),
                    doc_id: doc_ids.value(i).to_string(),
                    title: titles.value(i).to_string(),
                    doc_title: doc_titles.value(i).to_string(),
                    section: sections.value(i).to_string(),
                    chunk_index: chunk_indices.value(i) as u32,
                    total_chunks: total_chunks_col.value(i) as u32,
                    header_context: header_contexts.value(i).to_string(),
                    content: contents.value(i).to_string(),
                    has_code: has_codes.value(i),
                });
            }
        }

        Ok(chunks)
    }

    /// Perform search with model swapping (sequential mode)
    ///
    /// This temporarily switches the main backend to embedding mode,
    /// performs the search, then switches back to inference mode.
    async fn search_with_swap(
        &self,
        query: &str,
        limit: usize,
        gateway: &SharedGateway,
        app: &tauri::AppHandle,
    ) -> Result<Vec<DocChunk>, RagError> {
        let was_inference = gateway.is_inference_mode().await;
        let restore_config = gateway.last_inference_config().await;
        let embedding_config = self.embedding_config.as_ref()
            .ok_or(RagError::ServerNotAvailable)?;

        // Switch to embedding mode if currently in inference mode
        if was_inference {
            log::info!("Sequential mode: switching to embedding model for search");
            gateway.start(embedding_config, app).await
                .map_err(|e| RagError::Client(format!("Failed to switch to embedding mode: {}", e)))?;

            // Wait a moment for the server to initialize
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // Get the embedding URL (now from main server)
        let url = gateway.base_url().await
            .ok_or(RagError::ServerNotAvailable)?;

        // Perform search
        let result = self.search_with_url(&url, query, limit).await;

        // Restore inference mode
        if was_inference {
            log::info!("Sequential mode: restoring inference model");
            if let Some(config) = restore_config {
                if let Err(e) = gateway.start(&config, app).await {
                    log::error!("Failed to restore inference mode: {}", e);
                }
            }
        }

        result
    }

    /// Update docs status from DocsManager
    pub fn update_docs_status(&mut self, docs_manager: &DocsManager) {
        let status = docs_manager.get_status();
        self.status.docs_available = status.available;
        self.status.docs_count = status.doc_count;
    }

    /// Initialize or connect to LanceDB
    async fn ensure_db(&mut self) -> Result<&lancedb::Connection, RagError> {
        if self.db.is_none() {
            // Create directory if needed
            tokio::fs::create_dir_all(&self.store_path).await?;

            let db_path = self.store_path.to_string_lossy().to_string();
            log::info!("Connecting to LanceDB at: {}", db_path);

            let db = lancedb::connect(&db_path).execute().await?;
            self.db = Some(db);
        }
        Ok(self.db.as_ref().unwrap())
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

    /// Create Arrow schema for the chunks table
    fn create_schema(embedding_dim: i32) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("doc_id", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("doc_title", DataType::Utf8, false),
            Field::new("section", DataType::Utf8, false),
            Field::new("chunk_index", DataType::Int32, false),
            Field::new("total_chunks", DataType::Int32, false),
            Field::new("header_context", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("has_code", DataType::Boolean, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float64, true)),
                    embedding_dim,
                ),
                false,
            ),
        ]))
    }

    /// Convert embeddings to Arrow RecordBatch
    fn embeddings_to_record_batch(
        embeddings: &[(DocChunk, Embedding)],
        embedding_dim: i32,
    ) -> Result<RecordBatch, RagError> {
        let ids: Vec<&str> = embeddings.iter().map(|(c, _)| c.id.as_str()).collect();
        let doc_ids: Vec<&str> = embeddings.iter().map(|(c, _)| c.doc_id.as_str()).collect();
        let titles: Vec<&str> = embeddings.iter().map(|(c, _)| c.title.as_str()).collect();
        let doc_titles: Vec<&str> = embeddings
            .iter()
            .map(|(c, _)| c.doc_title.as_str())
            .collect();
        let sections: Vec<&str> = embeddings.iter().map(|(c, _)| c.section.as_str()).collect();
        let chunk_indices: Vec<i32> = embeddings
            .iter()
            .map(|(c, _)| c.chunk_index as i32)
            .collect();
        let total_chunks: Vec<i32> = embeddings
            .iter()
            .map(|(c, _)| c.total_chunks as i32)
            .collect();
        let header_contexts: Vec<&str> = embeddings
            .iter()
            .map(|(c, _)| c.header_context.as_str())
            .collect();
        let contents: Vec<&str> = embeddings.iter().map(|(c, _)| c.content.as_str()).collect();
        let has_codes: Vec<bool> = embeddings.iter().map(|(c, _)| c.has_code).collect();

        // Build vector array - flatten all embeddings into a single Vec<f64>
        let vectors_flat: Vec<f64> = embeddings
            .iter()
            .flat_map(|(_, emb)| emb.vec.iter().copied())
            .collect();

        let vectors_array = FixedSizeListArray::from_iter_primitive::<Float64Type, _, _>(
            vectors_flat
                .chunks(embedding_dim as usize)
                .map(|chunk| Some(chunk.iter().copied().map(Some).collect::<Vec<_>>())),
            embedding_dim,
        );

        let schema = Self::create_schema(embedding_dim);

        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(ids)) as ArrayRef,
                Arc::new(StringArray::from(doc_ids)) as ArrayRef,
                Arc::new(StringArray::from(titles)) as ArrayRef,
                Arc::new(StringArray::from(doc_titles)) as ArrayRef,
                Arc::new(StringArray::from(sections)) as ArrayRef,
                Arc::new(Int32Array::from(chunk_indices)) as ArrayRef,
                Arc::new(Int32Array::from(total_chunks)) as ArrayRef,
                Arc::new(StringArray::from(header_contexts)) as ArrayRef,
                Arc::new(StringArray::from(contents)) as ArrayRef,
                Arc::new(BooleanArray::from(has_codes)) as ArrayRef,
                Arc::new(vectors_array) as ArrayRef,
            ],
        )
        .map_err(|e| RagError::LanceDb(format!("Failed to create RecordBatch: {}", e)))?;

        Ok(batch)
    }

    /// Index all documentation entries by chunking them first
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
            .ok_or(RagError::ServerNotAvailable)?
            .clone();

        // Check server availability
        log::info!("Checking embedding server health at: {}", embedding_url);
        let server_available = check_embedding_server(&embedding_url).await;
        log::info!("Embedding server health check result: {}", server_available);
        if !server_available {
            log::error!("Embedding server health check failed - server not responding");
            return Err(RagError::ServerNotAvailable);
        }

        on_progress(IndexingProgress {
            current: 0,
            total: entries.len(),
            status: "Chunking documents...".to_string(),
        });

        // Chunk all documents
        let mut all_chunks: Vec<DocChunk> = Vec::new();
        for entry in entries {
            let chunks = chunk_document(
                &entry.id,
                &entry.title,
                &entry.section,
                &entry.content,
                &self.chunk_config,
            );
            all_chunks.extend(chunks);
        }

        let total_chunks = all_chunks.len();
        log::info!(
            "Chunked {} documents into {} chunks",
            entries.len(),
            total_chunks
        );

        on_progress(IndexingProgress {
            current: 0,
            total: total_chunks,
            status: "Connecting to embedding server...".to_string(),
        });

        // Create embedding client
        let client = create_embedding_client(&embedding_url)?;
        let model_name = get_embedding_model_name(&embedding_url).await;
        log::info!("Created embedding client for model: {}", model_name);
        let embedding_model = client.embedding_model(&model_name);

        on_progress(IndexingProgress {
            current: 0,
            total: total_chunks,
            status: format!("Generating embeddings for {} chunks...", total_chunks),
        });

        // Process chunks in batches for progress updates
        log::info!(
            "Processing {} chunks in batches of {} for embedding",
            all_chunks.len(),
            EMBEDDING_BATCH_SIZE
        );

        let mut all_embeddings: Vec<(DocChunk, Embedding)> = Vec::with_capacity(total_chunks);
        let mut processed = 0;

        for batch in all_chunks.chunks(EMBEDDING_BATCH_SIZE) {
            // Build embeddings for this batch
            let batch_vec: Vec<DocChunk> = batch.to_vec();
            let raw_batch: Vec<(DocChunk, OneOrMany<Embedding>)> =
                EmbeddingsBuilder::new(embedding_model.clone())
                    .documents(batch_vec)?
                    .build()
                    .await?;

            // Flatten OneOrMany to single embeddings
            for (chunk, embs) in raw_batch {
                all_embeddings.push((chunk, embs.first().clone()));
            }

            processed += batch.len();

            // Send progress update after each batch
            on_progress(IndexingProgress {
                current: processed,
                total: total_chunks,
                status: format!("Embedded {}/{} chunks...", processed, total_chunks),
            });
        }

        let embeddings = all_embeddings;
        log::info!("Received {} embeddings from server", embeddings.len());

        // Detect embedding dimension from first embedding
        let embedding_dim = if let Some((_, emb)) = embeddings.first() {
            emb.vec.len() as i32
        } else {
            DEFAULT_EMBEDDING_DIM
        };
        self.embedding_dim = Some(embedding_dim);
        log::info!("Detected embedding dimension: {}", embedding_dim);

        on_progress(IndexingProgress {
            current: total_chunks,
            total: total_chunks,
            status: "Storing vectors in LanceDB...".to_string(),
        });

        // Ensure DB is connected
        let db = self.ensure_db().await?;

        // Drop existing table if it exists
        let table_names = db.table_names().execute().await?;
        if table_names.contains(&CHUNKS_TABLE_NAME.to_string()) {
            log::info!("Dropping existing chunks table");
            db.drop_table(CHUNKS_TABLE_NAME, &[]).await?;
        }

        // Create RecordBatch from embeddings
        let batch = Self::embeddings_to_record_batch(&embeddings, embedding_dim)?;
        let schema = Self::create_schema(embedding_dim);

        // Create table with embeddings
        log::info!("Creating LanceDB table with {} vectors", embeddings.len());
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        db.create_table(CHUNKS_TABLE_NAME, Box::new(batches))
            .execute()
            .await?;

        // Save version for cache invalidation
        let version_path = self.store_path.join("embeddings-version.txt");
        tokio::fs::write(&version_path, version).await?;

        // Update status
        self.status.vectors_indexed = true;
        self.status.vectors_count = total_chunks;

        on_progress(IndexingProgress {
            current: total_chunks,
            total: total_chunks,
            status: "Complete".to_string(),
        });

        log::info!(
            "Successfully indexed {} chunks in LanceDB ({}D vectors)",
            total_chunks,
            embedding_dim
        );
        Ok(())
    }

    /// Load existing index from LanceDB (no re-embedding required!)
    pub async fn load_from_disk(&mut self) -> Result<bool, RagError> {
        // Connect to LanceDB
        let db = self.ensure_db().await?;

        // Check if table exists
        let table_names = db.table_names().execute().await?;
        if !table_names.contains(&CHUNKS_TABLE_NAME.to_string()) {
            log::info!("No existing chunks table found in LanceDB");
            return Ok(false);
        }

        // Open existing table and count rows
        let table = db.open_table(CHUNKS_TABLE_NAME).execute().await?;
        let count = table.count_rows(None).await?;

        if count == 0 {
            return Ok(false);
        }

        // Update status - vectors are already indexed!
        self.status.vectors_indexed = true;
        self.status.vectors_count = count;

        log::info!(
            "Loaded existing LanceDB index with {} vectors (no re-embedding needed)",
            count
        );
        Ok(true)
    }

    /// Perform semantic search - returns relevant chunks
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<DocChunk>, RagError> {
        let embedding_url = self
            .embedding_url
            .as_ref()
            .ok_or(RagError::ServerNotAvailable)?;

        let db = self.db.as_ref().ok_or(RagError::DocsNotAvailable)?;

        // Open the chunks table
        let table = db.open_table(CHUNKS_TABLE_NAME).execute().await?;

        // Create embedding for query
        let client = create_embedding_client(embedding_url)?;
        let model_name = get_embedding_model_name(embedding_url).await;
        let embedding_model = client.embedding_model(&model_name);

        // Get query embedding - use the EmbeddingModel trait method
        let query_embedding = EmbeddingModel::embed_text(&embedding_model, query)
            .await
            .map_err(|e| RagError::Embedding(e.to_string()))?;

        // Perform vector search
        let mut results = table
            .vector_search(query_embedding.vec.clone())
            .map_err(|e| RagError::LanceDb(e.to_string()))?
            .limit(limit)
            .execute()
            .await?;

        // Convert results to DocChunks
        let mut chunks = Vec::new();
        while let Some(batch) = results.try_next().await? {

            // Helper to extract string column
            fn get_string_col<'a>(
                batch: &'a RecordBatch,
                name: &str,
            ) -> Result<&'a StringArray, RagError> {
                batch
                    .column_by_name(name)
                    .ok_or_else(|| RagError::LanceDb(format!("Missing {} column", name)))?
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| RagError::LanceDb(format!("{} column has wrong type", name)))
            }

            // Helper to extract i32 column
            fn get_i32_col<'a>(
                batch: &'a RecordBatch,
                name: &str,
            ) -> Result<&'a Int32Array, RagError> {
                batch
                    .column_by_name(name)
                    .ok_or_else(|| RagError::LanceDb(format!("Missing {} column", name)))?
                    .as_any()
                    .downcast_ref::<Int32Array>()
                    .ok_or_else(|| RagError::LanceDb(format!("{} column has wrong type", name)))
            }

            // Helper to extract bool column
            fn get_bool_col<'a>(
                batch: &'a RecordBatch,
                name: &str,
            ) -> Result<&'a BooleanArray, RagError> {
                batch
                    .column_by_name(name)
                    .ok_or_else(|| RagError::LanceDb(format!("Missing {} column", name)))?
                    .as_any()
                    .downcast_ref::<BooleanArray>()
                    .ok_or_else(|| RagError::LanceDb(format!("{} column has wrong type", name)))
            }

            // Extract columns
            let ids = get_string_col(&batch, "id")?;
            let doc_ids = get_string_col(&batch, "doc_id")?;
            let titles = get_string_col(&batch, "title")?;
            let doc_titles = get_string_col(&batch, "doc_title")?;
            let sections = get_string_col(&batch, "section")?;
            let chunk_indices = get_i32_col(&batch, "chunk_index")?;
            let total_chunks_col = get_i32_col(&batch, "total_chunks")?;
            let header_contexts = get_string_col(&batch, "header_context")?;
            let contents = get_string_col(&batch, "content")?;
            let has_codes = get_bool_col(&batch, "has_code")?;

            for i in 0..batch.num_rows() {
                chunks.push(DocChunk {
                    id: ids.value(i).to_string(),
                    doc_id: doc_ids.value(i).to_string(),
                    title: titles.value(i).to_string(),
                    doc_title: doc_titles.value(i).to_string(),
                    section: sections.value(i).to_string(),
                    chunk_index: chunk_indices.value(i) as u32,
                    total_chunks: total_chunks_col.value(i) as u32,
                    header_context: header_contexts.value(i).to_string(),
                    content: contents.value(i).to_string(),
                    has_code: has_codes.value(i),
                });
            }
        }

        Ok(chunks)
    }

    /// Perform semantic search and convert to legacy SvelteDoc format for backwards compatibility
    pub async fn search_as_docs(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SvelteDoc>, RagError> {
        let chunks = self.search(query, limit).await?;

        // Convert chunks to SvelteDoc format
        let docs: Vec<SvelteDoc> = chunks
            .into_iter()
            .map(|chunk| SvelteDoc {
                id: chunk.doc_id,
                title: chunk.doc_title,
                section: chunk.section,
                summary: chunk.header_context,
                content: chunk.content,
            })
            .collect();

        Ok(docs)
    }

    /// Check if semantic search is available
    #[allow(dead_code)]
    pub fn is_search_available(&self) -> bool {
        self.db.is_some() && self.embedding_url.is_some() && self.status.vectors_indexed
    }

    /// Clear the vector store and cached data
    pub async fn clear_cache(&mut self) -> Result<(), RagError> {
        // Drop table if DB is connected
        if let Some(db) = &self.db {
            let table_names = db.table_names().execute().await?;
            if table_names.contains(&CHUNKS_TABLE_NAME.to_string()) {
                db.drop_table(CHUNKS_TABLE_NAME, &[]).await?;
            }
        }

        self.status.vectors_indexed = false;
        self.status.vectors_count = 0;

        // Remove version file
        let version_path = self.store_path.join("embeddings-version.txt");
        if version_path.exists() {
            tokio::fs::remove_file(&version_path).await?;
        }

        // Also remove old chunk files if they exist (legacy cleanup)
        let chunks_path = self.store_path.parent().map(|p| p.join("svelte-docs"));
        if let Some(path) = chunks_path {
            if path.exists() {
                let _ = tokio::fs::remove_dir_all(&path).await;
            }
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

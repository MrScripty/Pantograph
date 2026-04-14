//! RAG (Retrieval Augmented Generation) commands.

use super::resolve_embedding_model_path;
use super::shared::{SharedAppConfig, get_project_data_dir};
use crate::agent::DocsManager;
use crate::agent::rag::{DatabaseInfo, IndexingProgress, RagStatus, SharedRagManager};
use crate::llm::gateway::SharedGateway;
use crate::llm::startup::{
    build_resolved_embedding_request, capture_inference_restore_config, restore_inference_runtime,
    restore_inference_runtime_best_effort,
};
use tauri::{AppHandle, State, command, ipc::Channel};

/// Event sent during document indexing
#[derive(Clone, serde::Serialize)]
pub struct IndexingEvent {
    pub current: usize,
    pub total: usize,
    pub status: String,
    pub done: bool,
    pub error: Option<String>,
}

impl From<IndexingProgress> for IndexingEvent {
    fn from(progress: IndexingProgress) -> Self {
        Self {
            current: progress.current,
            total: progress.total,
            status: progress.status,
            done: false,
            error: None,
        }
    }
}

#[command]
pub async fn get_rag_status(
    _app: AppHandle,
    rag_manager: State<'_, SharedRagManager>,
) -> Result<RagStatus, String> {
    let project_data_dir = get_project_data_dir()?;
    let docs_manager = DocsManager::new(project_data_dir);

    let mut manager = rag_manager.write().await;
    manager.update_docs_status(&docs_manager);

    // Load existing vectors from disk if not already loaded
    if !manager.status().vectors_indexed {
        if let Err(e) = manager.load_from_disk().await {
            log::warn!("Failed to load vectors from disk: {}", e);
        }
    }

    Ok(manager.status().clone())
}

#[command]
pub async fn check_embedding_server(url: String) -> Result<bool, String> {
    Ok(crate::agent::check_embedding_server(&url).await)
}

#[command]
pub async fn set_embedding_server_url(
    rag_manager: State<'_, SharedRagManager>,
    url: String,
) -> Result<bool, String> {
    let mut manager = rag_manager.write().await;
    manager.set_embedding_url(url);
    let available = manager.check_vectorizer().await;
    Ok(available)
}

#[command]
pub async fn index_rag_documents(
    _app: AppHandle,
    rag_manager: State<'_, SharedRagManager>,
    channel: Channel<IndexingEvent>,
) -> Result<(), String> {
    let project_data_dir = get_project_data_dir()?;
    let docs_manager = DocsManager::new(project_data_dir);

    // Ensure docs are available
    docs_manager
        .ensure_docs_available()
        .await
        .map_err(|e| format!("Failed to ensure docs available: {}", e))?;

    // Load the search index
    let index = docs_manager
        .load_index()
        .map_err(|e| format!("Failed to load search index: {}", e))?;

    let version = docs_manager
        .get_status()
        .version
        .unwrap_or_else(|| "unknown".to_string());

    // Create a progress callback that sends to the channel
    let channel_clone = channel.clone();
    let on_progress = move |progress: IndexingProgress| {
        channel_clone.send(IndexingEvent::from(progress)).ok();
    };

    // Index documents
    let mut manager = rag_manager.write().await;
    match manager
        .index_documents(&index.entries, &version, on_progress)
        .await
    {
        Ok(()) => {
            channel
                .send(IndexingEvent {
                    current: index.entries.len(),
                    total: index.entries.len(),
                    status: "Complete".to_string(),
                    done: true,
                    error: None,
                })
                .ok();
            Ok(())
        }
        Err(e) => {
            channel
                .send(IndexingEvent {
                    current: 0,
                    total: 0,
                    status: "Failed".to_string(),
                    done: true,
                    error: Some(e.to_string()),
                })
                .ok();
            Err(e.to_string())
        }
    }
}

#[command]
pub async fn load_rag_from_disk(rag_manager: State<'_, SharedRagManager>) -> Result<bool, String> {
    let mut manager = rag_manager.write().await;
    manager
        .load_from_disk()
        .await
        .map_err(|e| format!("Failed to load RAG from disk: {}", e))
}

#[command]
pub async fn clear_rag_cache(rag_manager: State<'_, SharedRagManager>) -> Result<(), String> {
    let mut manager = rag_manager.write().await;
    manager
        .clear_cache()
        .await
        .map_err(|e| format!("Failed to clear RAG cache: {}", e))
}

#[command]
pub async fn search_rag(
    rag_manager: State<'_, SharedRagManager>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<crate::agent::SvelteDoc>, String> {
    let manager = rag_manager.read().await;
    // Use the backwards-compatible method that returns SvelteDoc
    manager
        .search_as_docs(&query, limit.unwrap_or(3))
        .await
        .map_err(|e| format!("RAG search failed: {}", e))
}

/// Index documents with automatic mode switching
/// If in inference mode, switches to embedding mode, indexes, then switches back
#[command]
pub async fn index_docs_with_switch(
    _app: AppHandle,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    channel: Channel<IndexingEvent>,
) -> Result<(), String> {
    log::info!("========== INDEX_DOCS_WITH_SWITCH CALLED ==========");
    let config_guard = config.read().await;

    // Check we have embedding model configured
    let embedding_model_path = config_guard
        .models
        .embedding_model_path
        .as_ref()
        .ok_or_else(|| "Embedding model path not configured".to_string())?
        .clone();
    log::info!("Embedding model path: {:?}", embedding_model_path);

    // Check if we need to restore VLM mode after
    let restore_config = capture_inference_restore_config(&gateway).await;
    let restore_vlm = restore_config.is_some();
    log::info!("Restore VLM after indexing: {}", restore_vlm);
    let resolved_embedding_model_path = resolve_embedding_model_path(&embedding_model_path)?;

    let device = config_guard.device.clone();
    let candle_model_path = config_guard
        .models
        .candle_embedding_model_path
        .as_ref()
        .map(std::path::PathBuf::from);
    drop(config_guard);

    // Send progress: switching to embedding mode
    channel
        .send(IndexingEvent {
            current: 0,
            total: 0,
            status: "Switching to embedding mode...".to_string(),
            done: false,
            error: None,
        })
        .ok();

    let backend_name = gateway.current_backend_name().await;
    log::info!("Current backend for embedding: {}", backend_name);
    let embedding_config = gateway
        .build_embedding_start_config(build_resolved_embedding_request(
            Some(resolved_embedding_model_path.clone()),
            candle_model_path,
            &device,
            Some("nomic-embed-text".to_string()),
        ))
        .await
        .map_err(|e| e.to_string())?;

    gateway
        .start(&embedding_config)
        .await
        .map_err(|e| format!("Failed to start embedding server: {}", e))?;

    // Update RAG manager with embedding URL from the gateway
    // All backends now expose an HTTP API (llama.cpp sidecar, Ollama daemon, Candle's Axum server)
    let embedding_url = match gateway.base_url().await {
        Some(url) => url,
        None => {
            // Backend has no HTTP API (e.g., Candle)
            // Restore VLM mode if needed and return error
            restore_inference_runtime_best_effort(
                &gateway,
                restore_config.clone(),
                "Failed to restore VLM mode after RAG embedding startup fallback",
            )
            .await;
            return Err(format!(
                "The {} backend does not support RAG indexing through the GUI. \
                 It runs in-process without an HTTP API. \
                 Please use llama.cpp or Ollama for RAG/embedding functionality.",
                backend_name
            ));
        }
    };
    log::info!("Embedding URL set: {:?}", embedding_url);
    {
        let mut rag_guard = rag_manager.write().await;
        rag_guard.set_embedding_url(embedding_url);
    }

    // Load docs and index
    let project_data_dir = get_project_data_dir()?;
    let docs_manager = DocsManager::new(project_data_dir);

    docs_manager
        .ensure_docs_available()
        .await
        .map_err(|e| format!("Failed to ensure docs available: {}", e))?;

    let index = docs_manager
        .load_index()
        .map_err(|e| format!("Failed to load search index: {}", e))?;

    let version = docs_manager
        .get_status()
        .version
        .unwrap_or_else(|| "unknown".to_string());
    log::info!(
        "Loaded {} documents from search index (version: {})",
        index.entries.len(),
        version
    );

    // Create progress callback
    let channel_clone = channel.clone();
    let on_progress = move |progress: IndexingProgress| {
        channel_clone.send(IndexingEvent::from(progress)).ok();
    };

    // Index documents
    log::info!(
        "Starting index_documents() with {} docs",
        index.entries.len()
    );
    let index_result = {
        let mut manager = rag_manager.write().await;
        manager
            .index_documents(&index.entries, &version, on_progress)
            .await
    };

    match index_result {
        Ok(()) => {
            channel
                .send(IndexingEvent {
                    current: index.entries.len(),
                    total: index.entries.len(),
                    status: "Indexing complete".to_string(),
                    done: false,
                    error: None,
                })
                .ok();
        }
        Err(e) => {
            log::error!("Failed to index documents: {:?}", e);
            channel
                .send(IndexingEvent {
                    current: 0,
                    total: 0,
                    status: "Indexing failed".to_string(),
                    done: true,
                    error: Some(e.to_string()),
                })
                .ok();

            restore_inference_runtime_best_effort(
                &gateway,
                restore_config.clone(),
                "Failed to restore VLM mode after RAG indexing failure",
            )
            .await;

            return Err(e.to_string());
        }
    }

    // Restore VLM mode if we were in it before
    if restore_vlm {
        channel
            .send(IndexingEvent {
                current: index.entries.len(),
                total: index.entries.len(),
                status: "Switching back to VLM mode...".to_string(),
                done: false,
                error: None,
            })
            .ok();

        restore_inference_runtime(&gateway, restore_config, "Failed to restore VLM mode").await?;
    }

    channel
        .send(IndexingEvent {
            current: index.entries.len(),
            total: index.entries.len(),
            status: "Complete".to_string(),
            done: true,
            error: None,
        })
        .ok();

    Ok(())
}

/// List all available vector databases
#[command]
pub async fn list_vector_databases(
    rag_manager: State<'_, SharedRagManager>,
) -> Result<Vec<DatabaseInfo>, String> {
    let manager = rag_manager.read().await;
    manager
        .list_databases()
        .await
        .map_err(|e| format!("Failed to list databases: {}", e))
}

/// Create a new vector database
#[command]
pub async fn create_vector_database(
    rag_manager: State<'_, SharedRagManager>,
    name: String,
) -> Result<String, String> {
    let manager = rag_manager.read().await;
    manager
        .create_database(&name)
        .await
        .map_err(|e| format!("Failed to create database: {}", e))
}

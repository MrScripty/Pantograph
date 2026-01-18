//! RAG (Retrieval Augmented Generation) commands.

use super::shared::{get_project_data_dir, SharedAppConfig};
use crate::agent::rag::{IndexingProgress, RagStatus, SharedRagManager};
use crate::agent::DocsManager;
use crate::llm::gateway::SharedGateway;
use crate::llm::BackendConfig;
use tauri::{command, ipc::Channel, AppHandle, State};

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
    app: AppHandle,
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
    let restore_vlm = gateway.is_inference_mode().await;
    log::info!("Restore VLM after indexing: {}", restore_vlm);

    // Save the last inference config for potential restoration
    let last_inference_config = gateway.last_inference_config().await;

    let device = config_guard.device.clone();
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

    // Build embedding config based on which backend is active
    let backend_name = gateway.current_backend_name().await;
    log::info!("Current backend for embedding: {}", backend_name);

    let embedding_config = match backend_name.as_str() {
        "Ollama" => {
            // Ollama uses model names, not file paths
            // Default to nomic-embed-text for embeddings
            BackendConfig {
                model_name: Some("nomic-embed-text".to_string()),
                embedding_mode: true,
                ..Default::default()
            }
        }
        "Candle" => {
            // Candle uses local SafeTensors model directories (not GGUF files)
            // Get the path from config (user must download model manually from HuggingFace)
            let config_guard = config.read().await;
            let candle_path = config_guard.models.candle_embedding_model_path.clone()
                .ok_or_else(|| {
                    "Candle embedding model path not configured. \
                     Download a SafeTensors model from HuggingFace (e.g., BAAI/bge-small-en-v1.5) \
                     and set the path in Settings.".to_string()
                })?;
            drop(config_guard);

            BackendConfig {
                model_path: Some(std::path::PathBuf::from(&candle_path)),
                embedding_mode: true,
                ..Default::default()
            }
        }
        _ => {
            // llama.cpp and others use file paths (GGUF format)
            BackendConfig {
                model_path: Some(std::path::PathBuf::from(&embedding_model_path)),
                device: Some(device.device.clone()),
                gpu_layers: Some(device.gpu_layers),
                embedding_mode: true,
                ..Default::default()
            }
        }
    };

    gateway
        .start(&embedding_config, &app)
        .await
        .map_err(|e| format!("Failed to start embedding server: {}", e))?;

    // Update RAG manager with embedding URL from the gateway
    // All backends now expose an HTTP API (llama.cpp sidecar, Ollama daemon, Candle's Axum server)
    let embedding_url = match gateway.base_url().await {
        Some(url) => url,
        None => {
            // Backend has no HTTP API (e.g., Candle)
            // Restore VLM mode if needed and return error
            if restore_vlm {
                if let Some(inference_config) = last_inference_config.clone() {
                    let _ = gateway.start(&inference_config, &app).await;
                }
            }
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

            // Try to restore VLM mode even on error
            if restore_vlm {
                if let Some(inference_config) = last_inference_config.clone() {
                    let _ = gateway.start(&inference_config, &app).await;
                }
            }

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

        if let Some(inference_config) = last_inference_config {
            gateway
                .start(&inference_config, &app)
                .await
                .map_err(|e| format!("Failed to restore VLM mode: {}", e))?;
        }
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

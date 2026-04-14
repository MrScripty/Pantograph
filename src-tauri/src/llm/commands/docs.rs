//! Documentation and chunking commands.

use super::shared::get_project_data_dir;
use crate::agent::docs::DocsStatus;
use crate::agent::docs_index::SearchIndex;
use crate::agent::types::{ChunkPreview, DocInfo};
use crate::agent::{preview_chunks, ChunkConfig, DocsManager};
use tauri::{command, AppHandle, Manager};

#[command]
pub async fn get_svelte_docs_status(_app: AppHandle) -> Result<DocsStatus, String> {
    let project_data_dir = get_project_data_dir()?;
    let docs_manager = DocsManager::new(project_data_dir);
    Ok(docs_manager.get_status())
}

#[command]
pub async fn update_svelte_docs(_app: AppHandle) -> Result<DocsStatus, String> {
    let project_data_dir = get_project_data_dir()?;
    let docs_manager = DocsManager::new(project_data_dir);

    log::info!("Downloading Svelte 5 documentation...");
    docs_manager
        .download_docs()
        .await
        .map_err(|e| format!("Failed to download docs: {}", e))?;

    log::info!("Building search index...");
    docs_manager
        .build_index()
        .await
        .map_err(|e| format!("Failed to build index: {}", e))?;

    Ok(docs_manager.get_status())
}

/// List all documents available for chunking
#[command]
pub async fn list_chunkable_docs(app: AppHandle) -> Result<Vec<DocInfo>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let docs_dir = app_data_dir.join("svelte-docs");

    if !docs_dir.exists() {
        return Ok(Vec::new());
    }

    // Build index to get document info
    let index = SearchIndex::build_from_docs(&docs_dir)
        .map_err(|e| format!("Failed to build index: {}", e))?;

    let docs: Vec<DocInfo> = index
        .entries
        .iter()
        .map(|entry| DocInfo {
            id: entry.id.clone(),
            title: entry.title.clone(),
            section: entry.section.clone(),
            char_count: entry.content.len(),
        })
        .collect();

    Ok(docs)
}

/// Preview how a document would be chunked
#[command]
pub async fn preview_doc_chunks(app: AppHandle, doc_id: String) -> Result<ChunkPreview, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let docs_dir = app_data_dir.join("svelte-docs");

    if !docs_dir.exists() {
        return Err("Documentation not downloaded yet".to_string());
    }

    // Build index to find the document
    let index = SearchIndex::build_from_docs(&docs_dir)
        .map_err(|e| format!("Failed to build index: {}", e))?;

    // Find the document by ID
    let entry = index
        .entries
        .iter()
        .find(|e| e.id == doc_id)
        .ok_or_else(|| format!("Document not found: {}", doc_id))?;

    // Generate chunk preview
    let config = ChunkConfig::default();
    let preview = preview_chunks(
        &entry.id,
        &entry.title,
        &entry.section,
        &entry.content,
        &config,
    );

    Ok(preview)
}

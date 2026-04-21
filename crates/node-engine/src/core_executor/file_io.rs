use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{NodeEngineError, Result};

pub(crate) async fn execute_read_file(
    project_root: Option<&PathBuf>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let path = inputs
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing path input".to_string()))?;

    let allowed_root = match project_root {
        Some(root) => root.clone(),
        None => std::env::current_dir().map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to resolve current directory: {e}"))
        })?,
    };
    let full_path =
        crate::path_validation::resolve_path_within_root(path, &allowed_root).map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Invalid read path '{}': {}", path, e))
        })?;

    let content = tokio::fs::read_to_string(&full_path)
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

    let mut outputs = HashMap::new();
    outputs.insert("content".to_string(), serde_json::json!(content));
    outputs.insert(
        "path".to_string(),
        serde_json::json!(full_path.display().to_string()),
    );
    Ok(outputs)
}

pub(crate) async fn execute_write_file(
    project_root: Option<&PathBuf>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let path = inputs
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing path input".to_string()))?;

    let content = inputs
        .get("content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing content input".to_string()))?;

    let allowed_root = match project_root {
        Some(root) => root.clone(),
        None => std::env::current_dir().map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to resolve current directory: {e}"))
        })?,
    };
    let full_path =
        crate::path_validation::resolve_path_within_root(path, &allowed_root).map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Invalid write path '{}': {}", path, e))
        })?;

    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to create directories: {}", e))
        })?;
    }

    tokio::fs::write(&full_path, content)
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

    let mut outputs = HashMap::new();
    outputs.insert("success".to_string(), serde_json::json!(true));
    outputs.insert(
        "path".to_string(),
        serde_json::json!(full_path.display().to_string()),
    );
    Ok(outputs)
}

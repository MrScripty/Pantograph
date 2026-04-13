//! LLM server lifecycle management commands.

use super::config::list_devices;
use super::shared::SharedAppConfig;
use crate::agent::rag::SharedRagManager;
use crate::config::{EmbeddingMemoryMode, ServerModeInfo};
use crate::llm::BackendConfig;
use crate::llm::gateway::SharedGateway;
use reqwest::Url;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State, command};

fn derive_models_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate
            .to_string_lossy()
            .ends_with("shared-resources/models")
        {
            return Some(candidate.to_path_buf());
        }
        current = candidate.parent();
    }
    None
}

fn find_gguf_files_in_dir(dir: &Path, limit: usize) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        format!(
            "Cannot read embedding model directory '{}': {}",
            dir.display(),
            e
        )
    })?;

    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
        {
            matches.push(path);
            if matches.len() >= limit {
                break;
            }
        }
    }

    Ok(matches)
}

fn find_model_files_by_name(
    models_root: &Path,
    file_name: &std::ffi::OsStr,
    limit: usize,
) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    let mut stack = vec![models_root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.is_file() && path.file_name() == Some(file_name) {
                matches.push(path);
                if matches.len() >= limit {
                    return matches;
                }
            }
        }
    }

    matches
}

pub(crate) fn resolve_embedding_model_path(model_path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(model_path);
    if candidate.is_file() {
        return Ok(candidate);
    }
    if candidate.is_dir() {
        let matches = find_gguf_files_in_dir(&candidate, 8)?;
        return match matches.len() {
            0 => Err(format!(
                "Embedding model directory '{}' contains no .gguf files. Select a GGUF embedding model in Puma-Lib.",
                model_path
            )),
            1 => Ok(matches[0].clone()),
            _ => {
                let list = matches
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(format!(
                    "Embedding model directory '{}' contains multiple .gguf files: {}. Select a single GGUF file path.",
                    model_path, list
                ))
            }
        };
    }

    let file_name = candidate
        .file_name()
        .ok_or_else(|| format!("Embedding model path is invalid: {}", model_path))?;
    let Some(models_root) = derive_models_root(&candidate) else {
        return Err(format!(
            "Embedding model file not found: {}. Update Model Configuration with a valid GGUF file path.",
            model_path
        ));
    };

    let matches = find_model_files_by_name(&models_root, file_name, 8);
    match matches.len() {
        0 => Err(format!(
            "Embedding model file not found: {}. Could not find '{}' under '{}'. Update Model Configuration.",
            model_path,
            file_name.to_string_lossy(),
            models_root.display()
        )),
        1 => {
            log::warn!(
                "Embedding model path '{}' was missing. Using discovered file '{}'",
                model_path,
                matches[0].display()
            );
            Ok(matches[0].clone())
        }
        _ => {
            let list = matches
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!(
                "Embedding model file not found at '{}', and multiple candidates matched '{}': {}. Update Model Configuration explicitly.",
                model_path,
                file_name.to_string_lossy(),
                list
            ))
        }
    }
}

fn validate_external_server_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("External server URL is required".to_string());
    }

    let parsed = Url::parse(trimmed)
        .map_err(|e| format!("Invalid external server URL '{}': {}", trimmed, e))?;
    match parsed.scheme() {
        "http" | "https" => Ok(trimmed.trim_end_matches('/').to_string()),
        other => Err(format!(
            "Unsupported external server URL scheme '{}'. Use http or https.",
            other
        )),
    }
}

#[command]
pub async fn connect_to_server(
    gateway: State<'_, SharedGateway>,
    url: String,
) -> Result<ServerModeInfo, String> {
    let external_url = validate_external_server_url(&url)?;

    if gateway.current_backend_name().await != "llama.cpp" {
        gateway
            .switch_backend("llama.cpp")
            .await
            .map_err(|e| e.to_string())?;
    }

    let backend_config = BackendConfig {
        external_url: Some(external_url),
        ..Default::default()
    };

    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;

    Ok(gateway.mode_info().await)
}

#[command]
pub async fn start_sidecar_llm(
    _app: AppHandle,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    model_path: String,
    mmproj_path: String,
) -> Result<ServerModeInfo, String> {
    let config_guard = config.read().await;
    let device = config_guard.device.clone();
    drop(config_guard);

    let backend_config = BackendConfig {
        model_path: Some(std::path::PathBuf::from(&model_path)),
        mmproj_path: Some(std::path::PathBuf::from(&mmproj_path)),
        device: Some(device.device),
        gpu_layers: Some(device.gpu_layers),
        embedding_mode: false,
        ..Default::default()
    };

    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;

    Ok(gateway.mode_info().await)
}

#[command]
pub async fn get_llm_status(gateway: State<'_, SharedGateway>) -> Result<ServerModeInfo, String> {
    Ok(gateway.mode_info().await)
}

#[command]
pub async fn stop_llm(gateway: State<'_, SharedGateway>) -> Result<(), String> {
    gateway.stop().await;
    Ok(())
}

#[command]
pub async fn get_server_mode(gateway: State<'_, SharedGateway>) -> Result<ServerModeInfo, String> {
    Ok(gateway.mode_info().await)
}

#[command]
pub async fn start_sidecar_inference(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
) -> Result<ServerModeInfo, String> {
    let backend_name = gateway.current_backend_name().await;
    log::info!("Starting sidecar inference with backend: {}", backend_name);

    let config_guard = config.read().await;

    // Extract config values we'll need after dropping the guard
    let embedding_model_path = config_guard.models.embedding_model_path.clone();
    let embedding_memory_mode = config_guard.embedding_memory_mode.clone();

    // Build backend-specific config
    let backend_config = match backend_name.as_str() {
        "Ollama" => {
            // Ollama uses model names, not file paths
            let model_name = config_guard.models.ollama_vlm_model.as_ref()
                .ok_or_else(|| "Ollama VLM model not configured. Set a model like 'llava:13b' or 'qwen2-vl:7b' in Model Configuration.".to_string())?;
            BackendConfig {
                model_name: Some(model_name.clone()),
                embedding_mode: false,
                ..Default::default()
            }
        }
        _ => {
            // llama.cpp and others use file paths
            let model_path = config_guard
                .models
                .vlm_model_path
                .as_ref()
                .ok_or_else(|| "VLM model path not configured".to_string())?;
            let mmproj_path = config_guard
                .models
                .vlm_mmproj_path
                .as_ref()
                .ok_or_else(|| "VLM mmproj path not configured".to_string())?;

            BackendConfig {
                model_path: Some(std::path::PathBuf::from(model_path)),
                mmproj_path: Some(std::path::PathBuf::from(mmproj_path)),
                device: Some(config_guard.device.device.clone()),
                gpu_layers: Some(config_guard.device.gpu_layers),
                embedding_mode: false,
                ..Default::default()
            }
        }
    };
    drop(config_guard);

    // Start the main LLM server
    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Started sidecar in inference mode");

    // Start embedding server for parallel modes (if embedding model is configured)
    if let Some(ref emb_path) = embedding_model_path {
        if embedding_memory_mode != EmbeddingMemoryMode::Sequential {
            let resolved_embedding_path = match resolve_embedding_model_path(emb_path) {
                Ok(path) => path,
                Err(e) => {
                    log::warn!(
                        "Failed to resolve configured embedding model path '{}': {}",
                        emb_path,
                        e
                    );
                    return Ok(gateway.mode_info().await);
                }
            };

            // Get device info for VRAM checking
            let devices = list_devices(app.clone()).await.unwrap_or_default();

            match gateway
                .start_embedding_server(
                    &resolved_embedding_path.to_string_lossy(),
                    embedding_memory_mode.clone(),
                    &devices,
                )
                .await
            {
                Ok(()) => {
                    // Set embedding URL in RAG manager so search() will work
                    if let Some(url) = gateway.embedding_url().await {
                        let mut rag = rag_manager.write().await;
                        rag.set_embedding_url(url);
                        log::info!("Embedding server started and RAG manager configured");
                    }
                }
                Err(e) => {
                    // Log but don't fail - embedding server is optional
                    log::warn!(
                        "Failed to start embedding server: {}. Vector search may not work.",
                        e
                    );
                }
            }
        } else {
            log::info!("Sequential embedding mode: embedding server will start on-demand");
        }
    }

    Ok(gateway.mode_info().await)
}

#[command]
pub async fn start_sidecar_embedding(
    _app: AppHandle,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
) -> Result<ServerModeInfo, String> {
    let config_guard = config.read().await;

    let model_path = config_guard
        .models
        .embedding_model_path
        .as_ref()
        .ok_or_else(|| "Embedding model path not configured".to_string())?;
    let resolved_model_path = resolve_embedding_model_path(model_path)?;

    let backend_config = BackendConfig {
        model_path: Some(resolved_model_path),
        device: Some(config_guard.device.device.clone()),
        gpu_layers: Some(config_guard.device.gpu_layers),
        embedding_mode: true,
        ..Default::default()
    };
    drop(config_guard);

    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Started sidecar in embedding mode");
    Ok(gateway.mode_info().await)
}

#[cfg(test)]
mod tests {
    use super::validate_external_server_url;

    #[test]
    fn validates_external_server_urls() {
        assert_eq!(
            validate_external_server_url(" http://127.0.0.1:1234/ ").as_deref(),
            Ok("http://127.0.0.1:1234")
        );
        assert!(validate_external_server_url("").is_err());
        assert!(validate_external_server_url("ftp://127.0.0.1").is_err());
    }
}

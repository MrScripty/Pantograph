use std::path::{Path, PathBuf};

use reqwest::Url;

use crate::config::{AppConfig, DeviceConfig};
use crate::llm::{EmbeddingStartRequest, InferenceStartRequest};

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

pub(crate) fn validate_external_server_url(url: &str) -> Result<String, String> {
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

pub(crate) fn build_external_inference_request(url: &str) -> Result<InferenceStartRequest, String> {
    Ok(InferenceStartRequest {
        external_url: Some(validate_external_server_url(url)?),
        ..InferenceStartRequest::default()
    })
}

pub(crate) fn build_explicit_llamacpp_inference_request(
    model_path: &str,
    mmproj_path: &str,
    device: &DeviceConfig,
) -> InferenceStartRequest {
    InferenceStartRequest {
        external_url: None,
        file_model_path: Some(PathBuf::from(model_path)),
        mmproj_path: Some(PathBuf::from(mmproj_path)),
        ollama_model_name: None,
        device: Some(device.device.clone()),
        gpu_layers: Some(device.gpu_layers),
    }
}

pub(crate) fn build_configured_inference_request(config: &AppConfig) -> InferenceStartRequest {
    InferenceStartRequest {
        external_url: None,
        file_model_path: config.models.vlm_model_path.as_ref().map(PathBuf::from),
        mmproj_path: config.models.vlm_mmproj_path.as_ref().map(PathBuf::from),
        ollama_model_name: config.models.ollama_vlm_model.clone(),
        device: Some(config.device.device.clone()),
        gpu_layers: Some(config.device.gpu_layers),
    }
}

pub(crate) fn build_resolved_embedding_request(
    gguf_model_path: Option<PathBuf>,
    candle_model_path: Option<PathBuf>,
    device: &DeviceConfig,
    ollama_model_name: Option<String>,
) -> EmbeddingStartRequest {
    EmbeddingStartRequest {
        gguf_model_path,
        candle_model_path,
        ollama_model_name,
        device: Some(device.device.clone()),
        gpu_layers: Some(device.gpu_layers),
    }
}

pub(crate) fn build_configured_embedding_request(
    config: &AppConfig,
) -> Result<EmbeddingStartRequest, String> {
    Ok(build_resolved_embedding_request(
        config
            .models
            .embedding_model_path
            .as_deref()
            .map(resolve_embedding_model_path)
            .transpose()?,
        config
            .models
            .candle_embedding_model_path
            .as_ref()
            .map(PathBuf::from),
        &config.device,
        None,
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::config::{AppConfig, DeviceConfig, ModelConfig};

    use super::{
        build_configured_embedding_request, build_configured_inference_request,
        build_explicit_llamacpp_inference_request, build_external_inference_request,
        build_resolved_embedding_request, resolve_embedding_model_path,
        validate_external_server_url,
    };

    #[test]
    fn validates_external_server_urls() {
        assert_eq!(
            validate_external_server_url(" http://127.0.0.1:1234/ ").as_deref(),
            Ok("http://127.0.0.1:1234")
        );
        assert!(validate_external_server_url("").is_err());
        assert!(validate_external_server_url("ftp://127.0.0.1").is_err());
    }

    #[test]
    fn builds_external_inference_request_with_normalized_url() {
        let request = build_external_inference_request("http://127.0.0.1:8080/")
            .expect("external request should build");
        assert_eq!(
            request.external_url.as_deref(),
            Some("http://127.0.0.1:8080")
        );
        assert!(request.file_model_path.is_none());
    }

    #[test]
    fn builds_configured_inference_request_from_app_config() {
        let config = AppConfig {
            models: ModelConfig {
                vlm_model_path: Some("/models/qwen.gguf".to_string()),
                vlm_mmproj_path: Some("/models/qwen.mmproj".to_string()),
                embedding_model_path: None,
                candle_embedding_model_path: None,
                ollama_vlm_model: Some("qwen2.5vl".to_string()),
            },
            device: DeviceConfig {
                device: "Vulkan0".to_string(),
                gpu_layers: 99,
            },
            ..AppConfig::default()
        };

        let request = build_configured_inference_request(&config);
        assert_eq!(
            request.file_model_path.as_deref(),
            Some(Path::new("/models/qwen.gguf"))
        );
        assert_eq!(
            request.mmproj_path.as_deref(),
            Some(Path::new("/models/qwen.mmproj"))
        );
        assert_eq!(request.ollama_model_name.as_deref(), Some("qwen2.5vl"));
        assert_eq!(request.device.as_deref(), Some("Vulkan0"));
        assert_eq!(request.gpu_layers, Some(99));
    }

    #[test]
    fn builds_explicit_llamacpp_inference_request_from_inputs() {
        let request = build_explicit_llamacpp_inference_request(
            "/models/main.gguf",
            "/models/main.mmproj",
            &DeviceConfig {
                device: "cuda".to_string(),
                gpu_layers: -1,
            },
        );

        assert_eq!(
            request.file_model_path.as_deref(),
            Some(Path::new("/models/main.gguf"))
        );
        assert_eq!(
            request.mmproj_path.as_deref(),
            Some(Path::new("/models/main.mmproj"))
        );
        assert_eq!(request.device.as_deref(), Some("cuda"));
        assert_eq!(request.gpu_layers, Some(-1));
    }

    #[test]
    fn builds_resolved_embedding_request_from_runtime_inputs() {
        let request = build_resolved_embedding_request(
            Some(PathBuf::from("/models/embed.gguf")),
            Some(PathBuf::from("/models/candle")),
            &DeviceConfig {
                device: "Vulkan0".to_string(),
                gpu_layers: 24,
            },
            Some("nomic-embed-text".to_string()),
        );

        assert_eq!(
            request.gguf_model_path.as_deref(),
            Some(Path::new("/models/embed.gguf"))
        );
        assert_eq!(
            request.candle_model_path.as_deref(),
            Some(Path::new("/models/candle"))
        );
        assert_eq!(
            request.ollama_model_name.as_deref(),
            Some("nomic-embed-text")
        );
        assert_eq!(request.device.as_deref(), Some("Vulkan0"));
        assert_eq!(request.gpu_layers, Some(24));
    }

    #[test]
    fn resolves_embedding_model_file_and_builds_embedding_request() {
        let temp_dir =
            std::env::temp_dir().join(format!("pantograph-startup-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("temp directory should be created");
        let model_path = temp_dir.join("embed.gguf");
        fs::write(&model_path, b"gguf").expect("embedding model file should be written");

        let resolved = resolve_embedding_model_path(
            model_path
                .to_str()
                .expect("temporary embedding path should be utf-8"),
        )
        .expect("embedding model path should resolve");
        assert_eq!(resolved, model_path);

        let config = AppConfig {
            models: ModelConfig {
                embedding_model_path: Some(
                    model_path
                        .to_str()
                        .expect("temporary embedding path should be utf-8")
                        .to_string(),
                ),
                candle_embedding_model_path: Some("/models/candle".to_string()),
                ..ModelConfig::default()
            },
            device: DeviceConfig {
                device: "auto".to_string(),
                gpu_layers: 12,
            },
            ..AppConfig::default()
        };

        let request =
            build_configured_embedding_request(&config).expect("embedding request should build");
        assert_eq!(
            request.gguf_model_path.as_deref(),
            Some(model_path.as_path())
        );
        assert_eq!(
            request.candle_model_path.as_deref(),
            Some(Path::new("/models/candle"))
        );
        assert_eq!(request.device.as_deref(), Some("auto"));
        assert_eq!(request.gpu_layers, Some(12));

        fs::remove_file(&model_path).expect("temporary embedding model file should be removed");
        fs::remove_dir(&temp_dir).expect("temporary test directory should be removed");
    }
}

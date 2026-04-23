use std::pin::Pin;

use futures_util::{Stream, StreamExt};

use super::{BackendConfig, BackendError, ChatChunk};
use crate::config::DeviceConfig;
use crate::constants::defaults;
use crate::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use crate::server::ServerMode;
use crate::types::{RerankResponse, RerankResult};
use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};

pub fn normalize_rerank_results(
    json: serde_json::Value,
    documents: &[String],
    return_documents: bool,
) -> Result<RerankResponse, BackendError> {
    let (items, metadata) = if let Some(results) = json
        .get("results")
        .and_then(|value| value.as_array())
        .cloned()
    {
        let mut metadata = json;
        if let Some(object) = metadata.as_object_mut() {
            object.remove("results");
        }
        (results, metadata)
    } else if let Some(results) = json.as_array() {
        (results.clone(), serde_json::Value::Null)
    } else {
        return Err(BackendError::Inference(
            "Invalid rerank response format".to_string(),
        ));
    };

    let mut normalized = Vec::with_capacity(items.len());
    for item in items {
        let index = item
            .get("index")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| BackendError::Inference("Missing rerank result index".to_string()))?
            as usize;
        let score = item
            .get("score")
            .or_else(|| item.get("relevance_score"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| BackendError::Inference("Missing rerank score".to_string()))?
            as f32;
        let document = if return_documents {
            item.get("document")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| documents.get(index).cloned())
        } else {
            None
        };
        normalized.push(RerankResult {
            index,
            score,
            document,
        });
    }

    Ok(RerankResponse {
        results: normalized,
        metadata,
    })
}

pub async fn post_rerank_request(
    http_client: &reqwest::Client,
    url: &str,
    request: &serde_json::Value,
    documents: &[String],
    return_documents: bool,
) -> Result<RerankResponse, BackendError> {
    let response = http_client
        .post(url)
        .json(request)
        .send()
        .await
        .map_err(BackendError::Http)?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(BackendError::Inference(format!(
            "Rerank API error {}: {}",
            status, body
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| BackendError::Inference(format!("Failed to parse response: {}", e)))?;
    normalize_rerank_results(json, documents, return_documents)
}

pub fn parse_sse_stream(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>> {
    let stream = response.bytes_stream().map(|result| match result {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        return Ok(ChatChunk {
                            content: None,
                            done: true,
                        });
                    }

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = json
                            .get("choices")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("delta"))
                            .and_then(|d| d.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            return Ok(ChatChunk {
                                content: Some(content.to_string()),
                                done: false,
                            });
                        }
                    }
                }
            }

            Ok(ChatChunk {
                content: None,
                done: false,
            })
        }
        Err(e) => Err(BackendError::Http(e)),
    });

    Box::pin(stream)
}

pub fn kv_cache_runtime_fingerprint_for_mode(
    mode: &ServerMode,
    active_config: Option<&BackendConfig>,
) -> Result<KvCacheRuntimeFingerprint, BackendError> {
    let (model_path, mmproj_path, device) = match mode {
        ServerMode::SidecarInference {
            model_path,
            mmproj_path,
            device,
            ..
        } => (model_path.as_str(), mmproj_path.as_deref(), device),
        ServerMode::External { .. } => {
            return Err(BackendError::Inference(
                "KV cache reuse is not supported for external llama.cpp runtimes".to_string(),
            ));
        }
        _ => {
            return Err(BackendError::Inference(
                "KV cache reuse requires llama.cpp inference mode".to_string(),
            ));
        }
    };

    let context_size = active_config
        .and_then(|config| config.context_size)
        .unwrap_or(defaults::CONTEXT_SIZE);

    Ok(KvCacheRuntimeFingerprint {
        runtime_id: canonical_runtime_id("llama.cpp"),
        backend_key: canonical_runtime_backend_key("llama.cpp"),
        tokenizer_fingerprint: format!(
            "llamacpp:{}:{}:{}:{}",
            model_path, device.device, device.gpu_layers, context_size
        ),
        prompt_format_fingerprint: Some(if mmproj_path.is_some() {
            "llamacpp_completion_multimodal".to_string()
        } else {
            "llamacpp_completion".to_string()
        }),
        runtime_build_fingerprint: Some(format!("ctx-{}", context_size)),
    })
}

pub fn kv_cache_model_fingerprint_for_mode(
    mode: &ServerMode,
    active_config: Option<&BackendConfig>,
) -> Result<ModelFingerprint, BackendError> {
    let (model_path, mmproj_path, device) = match mode {
        ServerMode::SidecarInference {
            model_path,
            mmproj_path,
            device,
            ..
        } => (model_path.as_str(), mmproj_path.as_deref(), device),
        ServerMode::External { .. } => {
            return Err(BackendError::Inference(
                "KV cache model fingerprint is not supported for external llama.cpp runtimes"
                    .to_string(),
            ));
        }
        _ => {
            return Err(BackendError::Inference(
                "KV cache model fingerprint requires llama.cpp inference mode".to_string(),
            ));
        }
    };

    let context_size = active_config
        .and_then(|config| config.context_size)
        .unwrap_or(defaults::CONTEXT_SIZE);

    Ok(ModelFingerprint {
        model_id: model_path.to_string(),
        config_hash: format!(
            "llamacpp:{}:{}:{}:{}:{}",
            model_path,
            mmproj_path.unwrap_or("none"),
            device.device,
            device.gpu_layers,
            context_size
        ),
    })
}

pub fn sidecar_device_config(config: &BackendConfig) -> DeviceConfig {
    DeviceConfig {
        device: config.device.clone().unwrap_or_else(|| "auto".to_string()),
        gpu_layers: config.gpu_layers.unwrap_or(-1),
    }
}

pub fn map_sidecar_start_error(error: String) -> BackendError {
    if error.to_lowercase().contains("out of memory") || error.to_lowercase().contains("oom") {
        BackendError::OutOfMemory(error)
    } else {
        BackendError::StartupFailed(error)
    }
}

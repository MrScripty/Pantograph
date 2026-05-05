use std::pin::Pin;

use futures_util::{Stream, StreamExt};
use serde_json::{Map, Value};

use super::{BackendConfig, BackendError, ChatChunk};
use crate::config::DeviceConfig;
use crate::constants::defaults;
use crate::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use crate::model_contracts::{
    GenerationOptions, OptionCompatibilityDiagnostic, OptionSupportState,
};
use crate::server::ServerMode;
use crate::types::{InferenceUsage, RerankResponse, RerankResult};
use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LlamaCppGenerationOptionMapping {
    pub(super) request_fields: Map<String, Value>,
    pub(super) diagnostics: Vec<OptionCompatibilityDiagnostic>,
}

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

#[allow(dead_code)]
pub(super) fn llama_cpp_generation_option_mapping(
    options: &GenerationOptions,
) -> LlamaCppGenerationOptionMapping {
    let mut request_fields = Map::new();
    let mut diagnostics = Vec::new();

    map_llama_option(
        &mut request_fields,
        &mut diagnostics,
        "length.max_new_tokens",
        "max_tokens",
        options.length.max_new_tokens,
        OptionSupportState::Mapped,
    );
    push_unsupported_if_requested(
        &mut diagnostics,
        "length.min_new_tokens",
        options.length.min_new_tokens.is_some(),
        "llama.cpp OpenAI-compatible requests do not expose min_new_tokens",
    );
    push_unsupported_if_requested(
        &mut diagnostics,
        "length.max_length",
        options.length.max_length.is_some(),
        "llama.cpp request mapping uses max_tokens/max_new_tokens rather than max_length",
    );
    map_llama_option(
        &mut request_fields,
        &mut diagnostics,
        "sampling.temperature",
        "temperature",
        options.sampling.temperature,
        OptionSupportState::Honored,
    );
    map_llama_option(
        &mut request_fields,
        &mut diagnostics,
        "sampling.top_p",
        "top_p",
        options.sampling.top_p,
        OptionSupportState::Honored,
    );
    map_llama_option(
        &mut request_fields,
        &mut diagnostics,
        "sampling.top_k",
        "top_k",
        options.sampling.top_k,
        OptionSupportState::Honored,
    );
    map_llama_option(
        &mut request_fields,
        &mut diagnostics,
        "sampling.repetition_penalty",
        "repeat_penalty",
        options.sampling.repetition_penalty,
        OptionSupportState::Mapped,
    );
    map_llama_option(
        &mut request_fields,
        &mut diagnostics,
        "sampling.seed",
        "seed",
        options.sampling.seed,
        OptionSupportState::Honored,
    );
    push_unsupported_if_requested(
        &mut diagnostics,
        "search.num_beams",
        options.search.num_beams.is_some(),
        "llama.cpp does not expose beam search through the current request mapping",
    );
    push_unsupported_if_requested(
        &mut diagnostics,
        "search.num_return_sequences",
        options.search.num_return_sequences.is_some(),
        "llama.cpp does not expose multiple return sequences through the current request mapping",
    );
    if !options.stopping.stop_strings.is_empty() {
        request_fields.insert(
            "stop".to_string(),
            serde_json::json!(options.stopping.stop_strings),
        );
        diagnostics.push(llama_option_diagnostic(
            "stopping.stop_strings",
            OptionSupportState::Mapped,
            Some("mapped to llama.cpp stop".to_string()),
        ));
    }
    push_unsupported_if_requested(
        &mut diagnostics,
        "stopping.eos_token_ids",
        !options.stopping.eos_token_ids.is_empty(),
        "llama.cpp OpenAI-compatible requests do not accept explicit EOS token ids",
    );
    push_unsupported_if_requested(
        &mut diagnostics,
        "cache.use_cache",
        options.cache.use_cache.is_some(),
        "cache.use_cache is controlled by Pantograph runtime/KV policy for llama.cpp",
    );
    if options.cache.kv_cache_checkpoint_requested == Some(true) {
        diagnostics.push(llama_option_diagnostic(
            "cache.kv_cache_checkpoint_requested",
            OptionSupportState::Mapped,
            Some(
                "handled by Pantograph KV-cache publication outside llama.cpp request fields"
                    .to_string(),
            ),
        ));
    }
    push_unsupported_if_requested(
        &mut diagnostics,
        "output.return_logprobs",
        options.output.return_logprobs == Some(true),
        "logprob output is not exposed by the current llama.cpp mapping",
    );
    push_unsupported_if_requested(
        &mut diagnostics,
        "output.return_token_ids",
        options.output.return_token_ids == Some(true),
        "token-id output is not exposed by the current llama.cpp mapping",
    );
    push_unsupported_if_requested(
        &mut diagnostics,
        "special_tokens.bos_token_id",
        options.special_tokens.bos_token_id.is_some(),
        "llama.cpp request mapping does not accept BOS token overrides",
    );
    push_unsupported_if_requested(
        &mut diagnostics,
        "special_tokens.eos_token_id",
        options.special_tokens.eos_token_id.is_some(),
        "llama.cpp request mapping does not accept EOS token overrides",
    );
    push_unsupported_if_requested(
        &mut diagnostics,
        "special_tokens.pad_token_id",
        options.special_tokens.pad_token_id.is_some(),
        "llama.cpp request mapping does not accept PAD token overrides",
    );
    let invalid_backend_extension_paths = options
        .backend_extension_scope_diagnostics()
        .into_iter()
        .map(|diagnostic| {
            let path = diagnostic.option_path.clone();
            diagnostics.push(diagnostic);
            path
        })
        .collect::<Vec<_>>();
    for (key, value) in &options.backend_extensions {
        let option_path = format!("backend_extensions.{key}");
        if invalid_backend_extension_paths.contains(&option_path) {
            continue;
        }
        if let Some(llama_key) = key.strip_prefix("llama.cpp:") {
            request_fields.insert(llama_key.to_string(), value.clone());
            diagnostics.push(llama_option_diagnostic(
                option_path,
                OptionSupportState::Mapped,
                Some(format!("mapped to llama.cpp extension key {llama_key}")),
            ));
        } else {
            diagnostics.push(llama_option_diagnostic(
                option_path,
                OptionSupportState::Unsupported,
                Some("backend extension is not scoped to llama.cpp".to_string()),
            ));
        }
    }

    LlamaCppGenerationOptionMapping {
        request_fields,
        diagnostics,
    }
}

#[allow(dead_code)]
fn map_llama_option<T: serde::Serialize>(
    request_fields: &mut Map<String, Value>,
    diagnostics: &mut Vec<OptionCompatibilityDiagnostic>,
    option_path: &'static str,
    llama_key: &'static str,
    value: Option<T>,
    state: OptionSupportState,
) {
    if let Some(value) = value {
        request_fields.insert(llama_key.to_string(), serde_json::json!(value));
        diagnostics.push(llama_option_diagnostic(
            option_path,
            state,
            Some(format!("mapped to llama.cpp {llama_key}")),
        ));
    }
}

#[allow(dead_code)]
fn push_unsupported_if_requested(
    diagnostics: &mut Vec<OptionCompatibilityDiagnostic>,
    option_path: &'static str,
    requested: bool,
    message: &'static str,
) {
    if requested {
        diagnostics.push(llama_option_diagnostic(
            option_path,
            OptionSupportState::Unsupported,
            Some(message.to_string()),
        ));
    }
}

#[allow(dead_code)]
fn llama_option_diagnostic(
    option_path: impl Into<String>,
    state: OptionSupportState,
    message: Option<String>,
) -> OptionCompatibilityDiagnostic {
    OptionCompatibilityDiagnostic {
        option_path: option_path.into(),
        state,
        backend_key: Some("llama_cpp".to_string()),
        message,
    }
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
                    if let Some(chunk) = chat_chunk_from_sse_data(data) {
                        return Ok(chunk);
                    }
                }
            }

            Ok(ChatChunk {
                content: None,
                done: false,
                usage: None,
            })
        }
        Err(e) => Err(BackendError::Http(e)),
    });

    Box::pin(stream)
}

fn chat_chunk_from_sse_data(data: &str) -> Option<ChatChunk> {
    if data == "[DONE]" {
        return Some(ChatChunk {
            content: None,
            done: true,
            usage: None,
        });
    }

    let json = serde_json::from_str::<serde_json::Value>(data).ok()?;
    let content = json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("delta"))
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str())
        .map(ToOwned::to_owned);
    let usage = inference_usage_from_openai_payload(json.get("usage"));

    if content.is_none() && usage.is_none() {
        return None;
    }

    Some(ChatChunk {
        content,
        done: false,
        usage,
    })
}

fn inference_usage_from_openai_payload(
    value: Option<&serde_json::Value>,
) -> Option<InferenceUsage> {
    let usage = value?;
    let usage = InferenceUsage {
        prompt_tokens: bounded_u32_field(usage, "prompt_tokens"),
        completion_tokens: bounded_u32_field(usage, "completion_tokens"),
        total_tokens: bounded_u32_field(usage, "total_tokens"),
    };

    if usage.prompt_tokens.is_none()
        && usage.completion_tokens.is_none()
        && usage.total_tokens.is_none()
    {
        None
    } else {
        Some(usage)
    }
}

fn bounded_u32_field(value: &serde_json::Value, field: &str) -> Option<u32> {
    value
        .get(field)
        .and_then(|field_value| field_value.as_u64())
        .and_then(|count| u32::try_from(count).ok())
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
    if let Some(message) = crate::process::strip_managed_binary_spawn_error(&error) {
        BackendError::ManagedBinary(message)
    } else if error.to_lowercase().contains("out of memory") || error.to_lowercase().contains("oom")
    {
        BackendError::OutOfMemory(error)
    } else {
        BackendError::StartupFailed(error)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::model_contracts::{
        CacheGenerationOptions, GenerationOptions, LengthGenerationOptions, OptionSupportState,
        OutputGenerationOptions, SamplingGenerationOptions, SearchGenerationOptions,
        SpecialTokenGenerationOptions, StoppingGenerationOptions,
    };

    #[test]
    fn map_sidecar_start_error_preserves_managed_binary_failures() {
        let error = crate::process::managed_binary_spawn_error("llama.cpp is not ready for launch");

        assert!(matches!(
            map_sidecar_start_error(error),
            BackendError::ManagedBinary(message)
                if message == "llama.cpp is not ready for launch"
        ));
    }

    #[test]
    fn llama_cpp_sse_parser_keeps_usage_only_chunks() {
        let chunk = chat_chunk_from_sse_data(
            r#"{"choices":[],"usage":{"prompt_tokens":7,"completion_tokens":5,"total_tokens":12}}"#,
        )
        .expect("usage-only chunk should parse");

        assert_eq!(chunk.content, None);
        assert!(!chunk.done);
        let usage = chunk.usage.expect("usage should be retained");
        assert_eq!(usage.prompt_tokens, Some(7));
        assert_eq!(usage.completion_tokens, Some(5));
        assert_eq!(usage.total_tokens, Some(12));
    }

    #[test]
    fn llama_cpp_sse_parser_keeps_content_and_bounded_usage() {
        let chunk = chat_chunk_from_sse_data(
            r#"{"choices":[{"delta":{"content":"hi"}}],"usage":{"prompt_tokens":3,"completion_tokens":18446744073709551615,"total_tokens":4}}"#,
        )
        .expect("content chunk should parse");

        assert_eq!(chunk.content.as_deref(), Some("hi"));
        assert!(!chunk.done);
        let usage = chunk.usage.expect("usage should be retained");
        assert_eq!(usage.prompt_tokens, Some(3));
        assert_eq!(usage.completion_tokens, None);
        assert_eq!(usage.total_tokens, Some(4));
    }

    #[test]
    fn llama_cpp_generation_options_map_request_fields_and_report_all_requested_options() {
        let options = GenerationOptions {
            length: LengthGenerationOptions {
                max_new_tokens: Some(128),
                min_new_tokens: Some(8),
                ..Default::default()
            },
            sampling: SamplingGenerationOptions {
                temperature: Some(0.7),
                top_p: Some(0.9),
                top_k: Some(40),
                repetition_penalty: Some(1.1),
                seed: Some(42),
            },
            search: SearchGenerationOptions {
                num_beams: Some(4),
                ..Default::default()
            },
            stopping: StoppingGenerationOptions {
                stop_strings: vec!["END".to_string()],
                eos_token_ids: vec![2],
            },
            cache: CacheGenerationOptions {
                use_cache: Some(true),
                kv_cache_checkpoint_requested: Some(true),
            },
            output: OutputGenerationOptions {
                return_logprobs: Some(true),
                ..Default::default()
            },
            special_tokens: SpecialTokenGenerationOptions {
                eos_token_id: Some(2),
                ..Default::default()
            },
            backend_extensions: [
                ("llama.cpp:mirostat".to_string(), serde_json::json!(2)),
                (
                    "transformers:renormalize_logits".to_string(),
                    serde_json::json!(true),
                ),
                ("raw_top_k".to_string(), serde_json::json!(40)),
            ]
            .into_iter()
            .collect(),
        };

        let mapping = llama_cpp_generation_option_mapping(&options);

        assert_eq!(mapping.request_fields["max_tokens"], serde_json::json!(128));
        assert_eq!(mapping.request_fields["top_k"], serde_json::json!(40));
        let repeat_penalty = mapping.request_fields["repeat_penalty"]
            .as_f64()
            .expect("repeat penalty is numeric");
        assert!((repeat_penalty - 1.1).abs() < 0.000_001);
        assert_eq!(mapping.request_fields["stop"], serde_json::json!(["END"]));
        assert_eq!(mapping.request_fields["mirostat"], serde_json::json!(2));
        assert!(mapping.diagnostics.iter().any(|diagnostic| {
            diagnostic.option_path == "length.min_new_tokens"
                && diagnostic.state == OptionSupportState::Unsupported
        }));
        assert!(mapping.diagnostics.iter().any(|diagnostic| {
            diagnostic.option_path == "cache.kv_cache_checkpoint_requested"
                && diagnostic.state == OptionSupportState::Mapped
        }));
        assert!(mapping.diagnostics.iter().any(|diagnostic| {
            diagnostic.option_path == "backend_extensions.transformers:renormalize_logits"
                && diagnostic.state == OptionSupportState::Unsupported
        }));
        assert!(mapping.diagnostics.iter().any(|diagnostic| {
            diagnostic.option_path == "backend_extensions.raw_top_k"
                && diagnostic.state == OptionSupportState::Rejected
        }));

        let requested_paths: BTreeSet<_> = options.requested_option_paths().into_iter().collect();
        let diagnostic_paths: BTreeSet<_> = mapping
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.option_path.clone())
            .collect();
        assert!(
            requested_paths.is_subset(&diagnostic_paths),
            "missing diagnostics for requested options: {:?}",
            requested_paths
                .difference(&diagnostic_paths)
                .collect::<Vec<_>>()
        );
    }
}

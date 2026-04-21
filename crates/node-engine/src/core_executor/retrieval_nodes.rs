use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use inference::InferenceGateway;

use crate::error::{NodeEngineError, Result};

use super::{
    build_extra_settings, canonical_backend_key, read_optional_input_bool_aliases,
    read_optional_input_string_aliases, require_gateway, resolve_gguf_path,
};

pub(crate) fn parse_reranker_documents(value: &serde_json::Value) -> Result<Vec<String>> {
    let items = if let Some(items) = value.as_array() {
        items
    } else {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents input must be a JSON array".to_string(),
        ));
    };

    let mut documents = Vec::with_capacity(items.len());
    for item in items {
        if let Some(text) = item.as_str() {
            if !text.trim().is_empty() {
                documents.push(text.to_string());
            }
            continue;
        }
        if let Some(text) = item
            .get("text")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("content").and_then(|v| v.as_str()))
            .or_else(|| item.get("document").and_then(|v| v.as_str()))
        {
            if !text.trim().is_empty() {
                documents.push(text.to_string());
            }
            continue;
        }
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents must be strings or objects with text/content/document fields"
                .to_string(),
        ));
    }

    if documents.is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents input cannot be empty".to_string(),
        ));
    }

    Ok(documents)
}

pub(crate) fn parse_reranker_documents_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<Vec<String>> {
    if let Some(value) = inputs.get("documents") {
        return parse_reranker_documents(value);
    }

    if let Some(raw) = inputs
        .get("documents_json")
        .and_then(|value| value.as_str())
    {
        let parsed: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Reranker documents_json must be valid JSON: {}",
                e
            ))
        })?;
        return parse_reranker_documents(&parsed);
    }

    Err(NodeEngineError::ExecutionFailed(
        "Missing documents input".to_string(),
    ))
}

pub(crate) async fn execute_reranker(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;

    let query = inputs
        .get("query")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing query input".to_string()))?;
    if query.trim().is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker query cannot be empty".to_string(),
        ));
    }

    let documents = parse_reranker_documents_input(inputs)?;

    let model_path_raw = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?;
    let model_path = resolve_gguf_path(model_path_raw)?;

    let top_k = inputs
        .get("top_k")
        .and_then(|value| value.as_u64().map(|v| v as usize))
        .or_else(|| {
            inputs
                .get("top_k")
                .and_then(|value| value.as_i64())
                .filter(|v| *v > 0)
                .map(|v| v as usize)
        });
    let return_documents =
        read_optional_input_bool_aliases(inputs, &["return_documents", "returnDocuments"])
            .unwrap_or(true);

    let mut extra_settings = build_extra_settings(inputs);
    let mut config = inference::BackendConfig {
        model_path: Some(PathBuf::from(&model_path)),
        device: Some("auto".to_string()),
        gpu_layers: Some(-1),
        reranking_mode: true,
        ..Default::default()
    };

    if let Some(v) = extra_settings.get("gpu_layers").and_then(|v| v.as_i64()) {
        config.gpu_layers = Some(v as i32);
    }
    if let Some(v) = extra_settings
        .get("context_length")
        .and_then(|v| v.as_i64())
    {
        config.context_size = Some(v as u32);
    }
    extra_settings.remove("gpu_layers");
    extra_settings.remove("context_length");

    if !gw.is_ready().await || !gw.is_reranking_mode().await {
        if gw.is_ready().await {
            gw.stop().await;
        }

        gw.start(&config).await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to start reranking server: {}", e))
        })?;

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !gw.is_ready().await {
            if std::time::Instant::now() > deadline {
                return Err(NodeEngineError::ExecutionFailed(
                    "Timeout waiting for reranking server to start".to_string(),
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    let response = gw
        .rerank(inference::RerankRequest {
            model: model_path.clone(),
            query: query.to_string(),
            documents,
            top_n: top_k,
            return_documents,
            extra_options: serde_json::Value::Object(extra_settings.into_iter().collect()),
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Reranker request failed: {}", e)))?;

    let scores = response
        .results
        .iter()
        .map(|result| serde_json::json!(result.score))
        .collect::<Vec<_>>();
    let top_document = response
        .results
        .first()
        .and_then(|result| result.document.clone());
    let top_score = response.results.first().map(|result| result.score);

    let mut outputs = HashMap::new();
    outputs.insert(
        "results".to_string(),
        serde_json::to_value(&response.results).unwrap_or(serde_json::Value::Null),
    );
    outputs.insert("scores".to_string(), serde_json::json!(scores));
    outputs.insert(
        "model_path".to_string(),
        serde_json::json!(model_path.clone()),
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::json!({
            "contractVersion": 2,
            "engine": "llamacpp",
            "modelId": model_path,
            "modelPath": model_path,
            "taskTypePrimary": "reranking"
        }),
    );
    outputs.insert(
        "top_document".to_string(),
        top_document
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    outputs.insert(
        "top_score".to_string(),
        top_score
            .map(|value| serde_json::json!(value))
            .unwrap_or(serde_json::Value::Null),
    );
    Ok(outputs)
}

pub(crate) async fn execute_embedding(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;

    let text = inputs
        .get("text")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing text input".to_string()))?;
    if text.trim().is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Embedding input text cannot be empty".to_string(),
        ));
    }

    let backend_name = gw.current_backend_name().await;
    if !is_llamacpp_backend_name(&backend_name) {
        return Err(NodeEngineError::ExecutionFailed(format!(
            "LlamaCpp Embedding blocked execution: active backend '{}' is not supported",
            backend_name
        )));
    }

    let model = read_optional_input_string_aliases(
        inputs,
        &["model", "model_name", "modelName", "model_id", "modelId"],
    )
    .filter(|s| !s.trim().is_empty())
    .unwrap_or_else(|| "default".to_string());

    if !gw.is_ready().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: backend is not ready. Start llama.cpp in embedding mode (`--embeddings`) first".to_string(),
        ));
    }
    if !gw.is_embedding_mode().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: backend is running in inference mode. Restart with `--embeddings`".to_string(),
        ));
    }
    let capabilities = gw.capabilities().await;
    if !capabilities.embeddings {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: active backend does not support embeddings"
                .to_string(),
        ));
    }

    let emit_metadata =
        read_optional_input_bool_aliases(inputs, &["emit_metadata", "emitMetadata"])
            .unwrap_or(false);

    let start = std::time::Instant::now();
    let results = gw
        .embeddings(vec![text.to_string()], &model)
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("LlamaCpp Embedding request failed: {}", e))
        })?;
    let embedding = results.first().ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned no vectors for input text".to_string(),
        )
    })?;
    if embedding.vector.is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned an empty vector".to_string(),
        ));
    }
    if embedding.vector.iter().any(|v| !v.is_finite()) {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned invalid vector values".to_string(),
        ));
    }

    let mut outputs = HashMap::new();
    outputs.insert("embedding".to_string(), serde_json::json!(embedding.vector));
    if emit_metadata {
        outputs.insert(
            "metadata".to_string(),
            serde_json::json!({
                "backend": "llamacpp",
                "model": model,
                "vector_length": embedding.vector.len(),
                "duration_ms": start.elapsed().as_millis(),
            }),
        );
    }

    Ok(outputs)
}

pub(crate) fn is_llamacpp_backend_name(backend_name: &str) -> bool {
    canonical_backend_key(Some(backend_name)).as_deref() == Some("llamacpp")
}

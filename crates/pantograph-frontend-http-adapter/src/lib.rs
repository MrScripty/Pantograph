//! Frontend-only HTTP adapter for workflow service execution.
//!
//! This crate is intentionally separate from headless API bindings so URL-based
//! HTTP integration remains an explicit opt-in for modular GUI embedding.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pantograph_workflow_service::{
    capabilities, RuntimeSignature, WorkflowHost, WorkflowHostModelDescriptor, WorkflowServiceError,
};

pub const DEFAULT_BACKEND_NAME: &str = "openai-compatible";
pub const DEFAULT_MAX_BATCH_SIZE: usize = capabilities::DEFAULT_MAX_BATCH_SIZE;
pub const DEFAULT_MAX_TEXT_LENGTH: usize = capabilities::DEFAULT_MAX_TEXT_LENGTH;

#[derive(Debug, thiserror::Error)]
pub enum FrontendHttpWorkflowHostError {
    #[error("invalid base_url '{base_url}': {reason}")]
    InvalidUrl {
        base_url: String,
        reason: String,
    },
    #[error("unsupported URL scheme '{scheme}' in base_url '{base_url}'")]
    UnsupportedScheme {
        base_url: String,
        scheme: String,
    },
    #[error("base_url '{base_url}' is missing a host")]
    MissingHost { base_url: String },
}

/// Workflow host that executes embeddings through an OpenAI-compatible HTTP API.
///
/// This adapter is for frontend/modular GUI transport integration, not for
/// framework headless embedding consumers.
pub struct FrontendHttpWorkflowHost {
    base_url: String,
    workflow_roots: Vec<PathBuf>,
    max_batch_size: usize,
    max_text_length: usize,
    backend_name: String,
    pumas_api: Option<Arc<pumas_library::PumasApi>>,
    http_client: reqwest::Client,
    resolved_model_id: Mutex<Option<String>>,
}

impl FrontendHttpWorkflowHost {
    pub fn with_defaults(
        base_url: String,
        pumas_api: Option<Arc<pumas_library::PumasApi>>,
        manifest_dir: &Path,
    ) -> Result<Self, FrontendHttpWorkflowHostError> {
        Self::new(
            base_url,
            pumas_api,
            capabilities::default_workflow_roots(manifest_dir),
            DEFAULT_MAX_BATCH_SIZE,
            DEFAULT_MAX_TEXT_LENGTH,
            DEFAULT_BACKEND_NAME.to_string(),
        )
    }

    pub fn new(
        base_url: String,
        pumas_api: Option<Arc<pumas_library::PumasApi>>,
        workflow_roots: Vec<PathBuf>,
        max_batch_size: usize,
        max_text_length: usize,
        backend_name: String,
    ) -> Result<Self, FrontendHttpWorkflowHostError> {
        let base_url = normalize_base_url(base_url)?;
        Ok(Self {
            base_url,
            workflow_roots,
            max_batch_size,
            max_text_length,
            backend_name: backend_name.trim().to_string(),
            pumas_api,
            http_client: reqwest::Client::new(),
            resolved_model_id: Mutex::new(None),
        })
    }

    async fn resolve_model_revision_or_hash(
        &self,
        model_id: &str,
    ) -> Result<Option<String>, WorkflowServiceError> {
        let Some(api) = &self.pumas_api else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?
            .ok_or_else(|| {
                WorkflowServiceError::RuntimeSignatureUnavailable(format!(
                    "model '{}' not found in model library",
                    model_id
                ))
            })?;

        capabilities::select_preferred_hash(&model.hashes)
            .map(Some)
            .ok_or_else(|| {
                WorkflowServiceError::RuntimeSignatureUnavailable(format!(
                    "model '{}' is missing sha256/blake3 hash metadata",
                    model_id
                ))
            })
    }
}

#[async_trait]
impl WorkflowHost for FrontendHttpWorkflowHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        self.workflow_roots.clone()
    }

    fn max_batch_size(&self) -> usize {
        self.max_batch_size
    }

    fn max_text_length(&self) -> usize {
        self.max_text_length
    }

    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok(self.backend_name.clone())
    }

    async fn model_metadata(
        &self,
        model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        let Some(api) = &self.pumas_api else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| m.metadata))
    }

    async fn model_descriptor(
        &self,
        model_id: &str,
    ) -> Result<Option<WorkflowHostModelDescriptor>, WorkflowServiceError> {
        let Some(api) = &self.pumas_api else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| WorkflowHostModelDescriptor {
            model_type: Some(m.model_type.trim().to_string()).filter(|v| !v.is_empty()),
            hashes: m.hashes,
        }))
    }

    async fn run_object(
        &self,
        _workflow_id: &str,
        text: &str,
        model_id: Option<&str>,
    ) -> Result<(Vec<f32>, Option<usize>), WorkflowServiceError> {
        let model = model_id
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("default");

        let url = format!("{}/v1/embeddings", self.base_url);
        let body = serde_json::json!({
            "input": [text],
            "model": model,
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        if !response.status().is_success() {
            return Err(WorkflowServiceError::Internal(format!(
                "embedding api error {}",
                response.status()
            )));
        }

        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|e| WorkflowServiceError::Internal(e.to_string()))?;

        let (embedding, token_count, response_model_id) = parse_embedding_payload(&payload)?;
        if let Some(model_id) = response_model_id {
            if let Ok(mut guard) = self.resolved_model_id.lock() {
                *guard = Some(model_id);
            }
        }

        Ok((embedding, token_count))
    }

    async fn resolve_runtime_signature(
        &self,
        _workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<RuntimeSignature, WorkflowServiceError> {
        let resolved_model_id = model_id
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                self.resolved_model_id
                    .lock()
                    .ok()
                    .and_then(|guard| guard.clone())
            })
            .unwrap_or_else(|| "default".to_string());
        let model_revision_or_hash = self.resolve_model_revision_or_hash(&resolved_model_id).await?;

        Ok(RuntimeSignature {
            model_id: resolved_model_id,
            model_revision_or_hash,
            backend: self.backend_name.clone(),
            vector_dimensions,
        })
    }
}

pub fn parse_embedding_payload(
    payload: &serde_json::Value,
) -> Result<(Vec<f32>, Option<usize>, Option<String>), WorkflowServiceError> {
    let embedding_values = payload
        .get("data")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("embedding"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| WorkflowServiceError::Internal("missing embedding vector".to_string()))?;

    let mut embedding = Vec::with_capacity(embedding_values.len());
    for (index, value) in embedding_values.iter().enumerate() {
        let number = value.as_f64().ok_or_else(|| {
            WorkflowServiceError::Internal(format!("invalid embedding value at index {}", index))
        })?;
        embedding.push(number as f32);
    }

    let token_count = payload
        .get("usage")
        .and_then(|v| v.get("prompt_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let response_model_id = payload
        .get("model")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);

    Ok((embedding, token_count, response_model_id))
}

fn normalize_base_url(raw_base_url: String) -> Result<String, FrontendHttpWorkflowHostError> {
    let trimmed = raw_base_url.trim().to_string();
    let parsed = reqwest::Url::parse(&trimmed).map_err(|e| {
        FrontendHttpWorkflowHostError::InvalidUrl {
            base_url: trimmed.clone(),
            reason: e.to_string(),
        }
    })?;

    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(FrontendHttpWorkflowHostError::UnsupportedScheme {
                base_url: trimmed,
                scheme: other.to_string(),
            });
        }
    }

    if parsed.host_str().is_none() {
        return Err(FrontendHttpWorkflowHostError::MissingHost { base_url: trimmed });
    }

    Ok(parsed.as_str().trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_embedding_payload_rejects_non_numeric() {
        let payload = serde_json::json!({
            "data": [{ "embedding": [0.1, "oops", 0.3] }]
        });
        let err = parse_embedding_payload(&payload).expect_err("must reject malformed vector");
        assert!(err.to_string().contains("invalid embedding value"));
    }

    #[test]
    fn normalize_base_url_rejects_non_http_schemes() {
        let err = normalize_base_url("file:///tmp/server".to_string())
            .expect_err("must reject unsupported scheme");
        assert!(matches!(
            err,
            FrontendHttpWorkflowHostError::UnsupportedScheme { .. }
        ));
    }

    #[test]
    fn normalize_base_url_trims_trailing_slash() {
        let normalized = normalize_base_url("http://127.0.0.1:8080/".to_string())
            .expect("normalize");
        assert_eq!(normalized, "http://127.0.0.1:8080");
    }
}

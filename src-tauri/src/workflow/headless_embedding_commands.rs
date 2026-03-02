//! Headless embedding API adapter for Tauri transport.
//!
//! This module maps Tauri command invocations to host-agnostic service logic in
//! `pantograph-workflow-service`.

use std::sync::Arc;

use async_trait::async_trait;
use pantograph_workflow_service::{
    EmbedObjectsV1Request, EmbedObjectsV1Response, EmbeddingHost, EmbeddingHostCapabilities,
    EmbeddingService, EmbeddingServiceError, GetEmbeddingWorkflowCapabilitiesV1Request,
    GetEmbeddingWorkflowCapabilitiesV1Response, ModelSignature,
};
use tauri::State;

use crate::llm::SharedGateway;

use super::commands::SharedExtensions;

const DEFAULT_MAX_BATCH_SIZE: usize = 128;
const DEFAULT_MAX_TEXT_LENGTH: usize = 32_768;

pub async fn embed_objects_v1(
    request: EmbedObjectsV1Request,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
) -> Result<EmbedObjectsV1Response, String> {
    let host = TauriEmbeddingHost::new(gateway.inner().clone(), extensions.inner().clone());
    EmbeddingService::new()
        .embed_objects_v1(&host, request)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_embedding_workflow_capabilities_v1(
    request: GetEmbeddingWorkflowCapabilitiesV1Request,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
) -> Result<GetEmbeddingWorkflowCapabilitiesV1Response, String> {
    let host = TauriEmbeddingHost::new(gateway.inner().clone(), extensions.inner().clone());
    EmbeddingService::new()
        .get_embedding_workflow_capabilities_v1(&host, request)
        .await
        .map_err(|e| e.to_string())
}

struct TauriEmbeddingHost {
    gateway: SharedGateway,
    extensions: SharedExtensions,
}

impl TauriEmbeddingHost {
    fn new(gateway: SharedGateway, extensions: SharedExtensions) -> Self {
        Self {
            gateway,
            extensions,
        }
    }

    async fn list_supported_embedding_models(&self) -> Vec<String> {
        let pumas_api = {
            let ext = self.extensions.read().await;
            ext.get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
                .cloned()
        };

        let Some(api) = pumas_api else {
            return vec!["default".to_string()];
        };

        match api.list_models().await {
            Ok(models) => {
                let mut out = models
                    .into_iter()
                    .filter(|m| m.model_type.eq_ignore_ascii_case("embedding"))
                    .map(|m| m.id)
                    .collect::<Vec<_>>();
                out.sort();
                out.dedup();
                if out.is_empty() {
                    out.push("default".to_string());
                }
                out
            }
            Err(err) => {
                log::warn!("Failed to list embedding models from PumasApi: {err}");
                vec!["default".to_string()]
            }
        }
    }
}

#[async_trait]
impl EmbeddingHost for TauriEmbeddingHost {
    async fn validate_embedding_workflow(
        &self,
        workflow_id: &str,
    ) -> Result<(), EmbeddingServiceError> {
        if workflow_id.trim().is_empty() {
            return Err(EmbeddingServiceError::WorkflowNotFound(
                "workflow_id is empty".to_string(),
            ));
        }
        Ok(())
    }

    async fn embedding_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<EmbeddingHostCapabilities, EmbeddingServiceError> {
        Ok(EmbeddingHostCapabilities {
            supported_models: self.list_supported_embedding_models().await,
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
            max_text_length: DEFAULT_MAX_TEXT_LENGTH,
        })
    }

    async fn embed_one(
        &self,
        _workflow_id: &str,
        text: &str,
        model_id: Option<&str>,
    ) -> Result<(Vec<f32>, Option<usize>), EmbeddingServiceError> {
        let inner = self.gateway.inner_arc();

        if !inner.is_ready().await {
            return Err(EmbeddingServiceError::RuntimeNotReady(
                "inference gateway is not ready".to_string(),
            ));
        }

        let capabilities = inner.capabilities().await;
        if !capabilities.embeddings {
            return Err(EmbeddingServiceError::CapabilityViolation(
                "active backend does not support embeddings".to_string(),
            ));
        }

        let selected_model = model_id
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("default");

        let results = inner
            .embeddings(vec![text.to_string()], selected_model)
            .await
            .map_err(|err| match err {
                inference::GatewayError::Backend(inference::backend::BackendError::NotReady) => {
                    EmbeddingServiceError::RuntimeNotReady("backend is not ready".to_string())
                }
                other => EmbeddingServiceError::Internal(other.to_string()),
            })?;

        let first = results
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingServiceError::Internal("no embedding vector returned".to_string()))?;

        Ok((first.vector, Some(first.token_count)))
    }

    async fn resolve_model_signature(
        &self,
        _workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<ModelSignature, EmbeddingServiceError> {
        let backend = self.gateway.current_backend_name().await;
        let model_id = model_id
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("default")
            .to_string();

        Ok(ModelSignature {
            model_id,
            model_revision_or_hash: None,
            backend,
            vector_dimensions,
        })
    }
}

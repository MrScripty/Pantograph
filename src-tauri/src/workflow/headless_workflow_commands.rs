//! Headless workflow API adapter for Tauri transport.
//!
//! This module maps Tauri command invocations to host-agnostic service logic in
//! `pantograph-workflow-service`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_workflow_service::{
    capabilities, RuntimeSignature, WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse,
    WorkflowHost, WorkflowRunRequest, WorkflowRunResponse, WorkflowService, WorkflowServiceError,
};
use tauri::State;

use crate::llm::SharedGateway;

use super::commands::SharedExtensions;

const DEFAULT_MAX_BATCH_SIZE: usize = 128;
const DEFAULT_MAX_TEXT_LENGTH: usize = 32_768;

pub async fn workflow_run(
    request: WorkflowRunRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
) -> Result<WorkflowRunResponse, String> {
    let host = TauriWorkflowHost::new(gateway.inner().clone(), extensions.inner().clone());
    WorkflowService::new()
        .workflow_run(&host, request)
        .await
        .map_err(|e| e.to_string())
}

pub async fn workflow_get_capabilities(
    request: WorkflowCapabilitiesRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
) -> Result<WorkflowCapabilitiesResponse, String> {
    let host = TauriWorkflowHost::new(gateway.inner().clone(), extensions.inner().clone());
    WorkflowService::new()
        .workflow_get_capabilities(&host, request)
        .await
        .map_err(|e| e.to_string())
}

struct TauriWorkflowHost {
    gateway: SharedGateway,
    extensions: SharedExtensions,
}

impl TauriWorkflowHost {
    fn new(gateway: SharedGateway, extensions: SharedExtensions) -> Self {
        Self {
            gateway,
            extensions,
        }
    }

    async fn pumas_api(&self) -> Option<Arc<pumas_library::PumasApi>> {
        let ext = self.extensions.read().await;
        ext.get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
            .cloned()
    }
}

#[async_trait]
impl WorkflowHost for TauriWorkflowHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        capabilities::default_workflow_roots(Path::new(env!("CARGO_MANIFEST_DIR")))
    }

    fn max_batch_size(&self) -> usize {
        DEFAULT_MAX_BATCH_SIZE
    }

    fn max_text_length(&self) -> usize {
        DEFAULT_MAX_TEXT_LENGTH
    }

    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok(self.gateway.current_backend_name().await)
    }

    async fn model_metadata(
        &self,
        model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        let Some(api) = self.pumas_api().await else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| m.metadata))
    }

    async fn run_object(
        &self,
        _workflow_id: &str,
        text: &str,
        model_id: Option<&str>,
    ) -> Result<(Vec<f32>, Option<usize>), WorkflowServiceError> {
        let inner = self.gateway.inner_arc();

        if !inner.is_ready().await {
            return Err(WorkflowServiceError::RuntimeNotReady(
                "inference gateway is not ready".to_string(),
            ));
        }

        let capabilities = inner.capabilities().await;
        if !capabilities.embeddings {
            return Err(WorkflowServiceError::CapabilityViolation(
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
                    WorkflowServiceError::RuntimeNotReady("backend is not ready".to_string())
                }
                other => WorkflowServiceError::Internal(other.to_string()),
            })?;

        let first = results.into_iter().next().ok_or_else(|| {
            WorkflowServiceError::Internal("no embedding vector returned".to_string())
        })?;

        Ok((first.vector, Some(first.token_count)))
    }

    async fn resolve_runtime_signature(
        &self,
        _workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<RuntimeSignature, WorkflowServiceError> {
        let backend = self.gateway.current_backend_name().await;
        let model_id = model_id
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("default")
            .to_string();

        Ok(RuntimeSignature {
            model_id,
            model_revision_or_hash: None,
            backend,
            vector_dimensions,
        })
    }
}

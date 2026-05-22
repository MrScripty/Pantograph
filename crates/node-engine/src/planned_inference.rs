//! Planned inference contracts consumed by inference nodes.
//!
//! Node-engine forwards validated graph intent through host-owned execution
//! traits. Hosts own scheduler plan lookup, Pumas artifact resolution, runtime
//! launch facts, and backend execution.

use async_trait::async_trait;
use thiserror::Error;

/// Host-owned planned execution boundary for inference nodes.
///
/// Node-engine passes run/node correlation and user intent through this trait.
/// Hosts own scheduler-plan lookup, Pumas load-target resolution, runtime
/// launch facts, and backend execution.
#[async_trait]
pub trait PlannedInferenceExecutionHost: Send + Sync {
    async fn generate_image(
        &self,
        request: PlannedImageGenerationRequest,
    ) -> Result<inference::ImageGenerationResult, PlannedInferenceExecutionError>;
}

/// Path-free image-generation request sent from node-engine to the host.
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct PlannedImageGenerationRequest {
    workflow_run_id: String,
    node_id: String,
    request_id: String,
    image_request: inference::ImageGenerationRequest,
}

impl PlannedImageGenerationRequest {
    pub fn new(
        workflow_run_id: impl Into<String>,
        node_id: impl Into<String>,
        request_id: impl Into<String>,
        image_request: inference::ImageGenerationRequest,
    ) -> Result<Self, PlannedImageGenerationRequestError> {
        Ok(Self {
            workflow_run_id: validate_planned_request_text(
                "workflow_run_id",
                workflow_run_id.into(),
            )?,
            node_id: validate_planned_request_text("node_id", node_id.into())?,
            request_id: validate_planned_request_text("request_id", request_id.into())?,
            image_request,
        })
    }

    #[must_use]
    pub fn workflow_run_id(&self) -> &str {
        &self.workflow_run_id
    }

    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    #[must_use]
    pub fn image_request(&self) -> &inference::ImageGenerationRequest {
        &self.image_request
    }

    #[must_use]
    pub fn into_image_request(self) -> inference::ImageGenerationRequest {
        self.image_request
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlannedImageGenerationRequestError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PlannedInferenceExecutionError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error(
        "planned inference execution failed for workflow run '{workflow_run_id}', node '{node_id}', request '{request_id}': {message}"
    )]
    ExecutionFailed {
        workflow_run_id: String,
        node_id: String,
        request_id: String,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl PlannedInferenceExecutionError {
    pub fn execution_failed(
        request: &PlannedImageGenerationRequest,
        message: impl Into<String>,
    ) -> Self {
        Self::ExecutionFailed {
            workflow_run_id: request.workflow_run_id().to_string(),
            node_id: request.node_id().to_string(),
            request_id: request.request_id().to_string(),
            message: message.into(),
            source: None,
        }
    }

    pub fn execution_failed_with_source(
        request: &PlannedImageGenerationRequest,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::ExecutionFailed {
            workflow_run_id: request.workflow_run_id().to_string(),
            node_id: request.node_id().to_string(),
            request_id: request.request_id().to_string(),
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

fn validate_planned_request_text(
    field: &'static str,
    value: String,
) -> Result<String, PlannedImageGenerationRequestError> {
    let value = value.trim();
    if value.is_empty() {
        Err(PlannedImageGenerationRequestError::MissingField { field })
    } else {
        Ok(value.to_string())
    }
}

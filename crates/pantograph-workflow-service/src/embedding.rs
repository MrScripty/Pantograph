use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Single object input for embedding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct EmbedInputObject {
    pub object_id: String,
    pub text: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// v1 request contract for object-in/object-out embeddings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct EmbedObjectsV1Request {
    pub api_version: String,
    pub workflow_id: String,
    pub objects: Vec<EmbedInputObject>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub batch_id: Option<String>,
}

/// Canonical model signature used by consumers for compatibility checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ModelSignature {
    pub model_id: String,
    #[serde(default)]
    pub model_revision_or_hash: Option<String>,
    pub backend: String,
    pub vector_dimensions: usize,
}

/// Per-object execution status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingStatus {
    Success,
    Failed,
}

/// Structured per-object error payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct EmbedObjectError {
    pub code: String,
    pub message: String,
}

/// Per-object embedding result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct EmbedObjectResult {
    pub object_id: String,
    pub embedding: Option<Vec<f32>>,
    #[serde(default)]
    pub token_count: Option<usize>,
    pub status: EmbeddingStatus,
    #[serde(default)]
    pub error: Option<EmbedObjectError>,
}

/// v1 embedding response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct EmbedObjectsV1Response {
    pub api_version: String,
    pub run_id: String,
    pub model_signature: ModelSignature,
    pub results: Vec<EmbedObjectResult>,
    pub timing_ms: u128,
}

/// v1 capabilities request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GetEmbeddingWorkflowCapabilitiesV1Request {
    pub api_version: String,
    pub workflow_id: String,
}

/// Host capability payload consumed by the service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct EmbeddingHostCapabilities {
    pub supported_models: Vec<String>,
    pub max_batch_size: usize,
    pub max_text_length: usize,
}

/// v1 capabilities response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GetEmbeddingWorkflowCapabilitiesV1Response {
    pub api_version: String,
    pub supported_models: Vec<String>,
    pub max_batch_size: usize,
    pub max_text_length: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingServiceError {
    #[error("unsupported_api_version")]
    UnsupportedApiVersion,

    #[error("invalid_request: {0}")]
    InvalidRequest(String),

    #[error("workflow_not_found: {0}")]
    WorkflowNotFound(String),

    #[error("capability_violation: {0}")]
    CapabilityViolation(String),

    #[error("runtime_not_ready: {0}")]
    RuntimeNotReady(String),

    #[error("model_signature_unavailable: {0}")]
    ModelSignatureUnavailable(String),

    #[error("internal_error: {0}")]
    Internal(String),
}

/// Trait boundary for host/runtime concerns needed by embedding service.
#[async_trait]
pub trait EmbeddingHost: Send + Sync {
    /// Resolve workflow identity and fail if it is unknown to the host.
    async fn validate_embedding_workflow(&self, workflow_id: &str) -> Result<(), EmbeddingServiceError>;

    /// Return capability limits and model support metadata.
    async fn embedding_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<EmbeddingHostCapabilities, EmbeddingServiceError>;

    /// Generate one embedding for the given text/model.
    async fn embed_one(
        &self,
        workflow_id: &str,
        text: &str,
        model_id: Option<&str>,
    ) -> Result<(Vec<f32>, Option<usize>), EmbeddingServiceError>;

    /// Resolve model signature fields after successful generation.
    async fn resolve_model_signature(
        &self,
        workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<ModelSignature, EmbeddingServiceError>;
}

/// Service entrypoint for embedding API operations.
#[derive(Default)]
pub struct EmbeddingService;

impl EmbeddingService {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip_uses_snake_case() {
        let req = EmbedObjectsV1Request {
            api_version: "v1".to_string(),
            workflow_id: "wf-1".to_string(),
            objects: vec![EmbedInputObject {
                object_id: "obj-1".to_string(),
                text: "hello".to_string(),
                metadata: None,
            }],
            model_id: Some("model-1".to_string()),
            batch_id: Some("batch-1".to_string()),
        };

        let json = serde_json::to_value(&req).expect("serialize request");
        assert_eq!(json["api_version"], "v1");
        assert_eq!(json["workflow_id"], "wf-1");
        assert_eq!(json["objects"][0]["object_id"], "obj-1");
    }

    #[test]
    fn response_roundtrip_preserves_signature_fields() {
        let res = EmbedObjectsV1Response {
            api_version: "v1".to_string(),
            run_id: "run-1".to_string(),
            model_signature: ModelSignature {
                model_id: "model-1".to_string(),
                model_revision_or_hash: Some("abc123".to_string()),
                backend: "llamacpp".to_string(),
                vector_dimensions: 1024,
            },
            results: vec![],
            timing_ms: 5,
        };

        let json = serde_json::to_string(&res).expect("serialize response");
        let parsed: EmbedObjectsV1Response = serde_json::from_str(&json).expect("parse response");
        assert_eq!(parsed.model_signature.model_id, "model-1");
        assert_eq!(parsed.model_signature.vector_dimensions, 1024);
    }
}

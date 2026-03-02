use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use uuid::Uuid;

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

    pub async fn embed_objects_v1<H: EmbeddingHost>(
        &self,
        host: &H,
        request: EmbedObjectsV1Request,
    ) -> Result<EmbedObjectsV1Response, EmbeddingServiceError> {
        validate_version(&request.api_version)?;
        validate_workflow_id(&request.workflow_id)?;
        if request.objects.is_empty() {
            return Err(EmbeddingServiceError::InvalidRequest(
                "objects must contain at least one item".to_string(),
            ));
        }

        host.validate_embedding_workflow(&request.workflow_id).await?;
        let capabilities = host.embedding_capabilities(&request.workflow_id).await?;
        if request.objects.len() > capabilities.max_batch_size {
            return Err(EmbeddingServiceError::CapabilityViolation(format!(
                "batch size {} exceeds max_batch_size {}",
                request.objects.len(),
                capabilities.max_batch_size
            )));
        }

        let started = Instant::now();
        let model_id = request
            .model_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let mut results = Vec::with_capacity(request.objects.len());
        let mut first_success_dims: Option<usize> = None;

        for object in request.objects {
            let object_id = object.object_id.trim().to_string();
            if object_id.is_empty() {
                results.push(EmbedObjectResult {
                    object_id: object.object_id,
                    embedding: None,
                    token_count: None,
                    status: EmbeddingStatus::Failed,
                    error: Some(EmbedObjectError {
                        code: "object_validation_failed".to_string(),
                        message: "object_id must be non-empty".to_string(),
                    }),
                });
                continue;
            }

            let text = object.text.trim().to_string();
            if text.is_empty() {
                results.push(EmbedObjectResult {
                    object_id,
                    embedding: None,
                    token_count: None,
                    status: EmbeddingStatus::Failed,
                    error: Some(EmbedObjectError {
                        code: "object_validation_failed".to_string(),
                        message: "text must be non-empty".to_string(),
                    }),
                });
                continue;
            }
            if text.len() > capabilities.max_text_length {
                results.push(EmbedObjectResult {
                    object_id,
                    embedding: None,
                    token_count: None,
                    status: EmbeddingStatus::Failed,
                    error: Some(EmbedObjectError {
                        code: "object_validation_failed".to_string(),
                        message: format!(
                            "text length {} exceeds max_text_length {}",
                            text.len(),
                            capabilities.max_text_length
                        ),
                    }),
                });
                continue;
            }

            match host
                .embed_one(&request.workflow_id, &text, model_id)
                .await
            {
                Ok((embedding, token_count)) => {
                    if embedding.is_empty() {
                        results.push(EmbedObjectResult {
                            object_id,
                            embedding: None,
                            token_count: None,
                            status: EmbeddingStatus::Failed,
                            error: Some(EmbedObjectError {
                                code: "embedding_failed".to_string(),
                                message: "embedding vector is empty".to_string(),
                            }),
                        });
                        continue;
                    }

                    first_success_dims.get_or_insert(embedding.len());
                    results.push(EmbedObjectResult {
                        object_id,
                        embedding: Some(embedding),
                        token_count,
                        status: EmbeddingStatus::Success,
                        error: None,
                    });
                }
                Err(err) => {
                    let mapped = map_object_error(err);
                    results.push(EmbedObjectResult {
                        object_id,
                        embedding: None,
                        token_count: None,
                        status: EmbeddingStatus::Failed,
                        error: Some(mapped),
                    });
                }
            }
        }

        let vector_dimensions = first_success_dims.ok_or_else(|| {
            EmbeddingServiceError::ModelSignatureUnavailable(
                "no successful object results; model signature cannot be resolved".to_string(),
            )
        })?;
        let model_signature = host
            .resolve_model_signature(&request.workflow_id, model_id, vector_dimensions)
            .await?;
        validate_model_signature(&model_signature)?;

        Ok(EmbedObjectsV1Response {
            api_version: "v1".to_string(),
            run_id: Uuid::new_v4().to_string(),
            model_signature,
            results,
            timing_ms: started.elapsed().as_millis(),
        })
    }

    pub async fn get_embedding_workflow_capabilities_v1<H: EmbeddingHost>(
        &self,
        host: &H,
        request: GetEmbeddingWorkflowCapabilitiesV1Request,
    ) -> Result<GetEmbeddingWorkflowCapabilitiesV1Response, EmbeddingServiceError> {
        validate_version(&request.api_version)?;
        validate_workflow_id(&request.workflow_id)?;
        host.validate_embedding_workflow(&request.workflow_id).await?;
        let capabilities = host.embedding_capabilities(&request.workflow_id).await?;
        Ok(GetEmbeddingWorkflowCapabilitiesV1Response {
            api_version: "v1".to_string(),
            supported_models: capabilities.supported_models,
            max_batch_size: capabilities.max_batch_size,
            max_text_length: capabilities.max_text_length,
        })
    }
}

fn validate_version(version: &str) -> Result<(), EmbeddingServiceError> {
    if version == "v1" {
        return Ok(());
    }
    Err(EmbeddingServiceError::UnsupportedApiVersion)
}

fn validate_workflow_id(workflow_id: &str) -> Result<(), EmbeddingServiceError> {
    if workflow_id.trim().is_empty() {
        return Err(EmbeddingServiceError::InvalidRequest(
            "workflow_id must be non-empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_model_signature(signature: &ModelSignature) -> Result<(), EmbeddingServiceError> {
    if signature.model_id.trim().is_empty() {
        return Err(EmbeddingServiceError::ModelSignatureUnavailable(
            "model_signature.model_id is empty".to_string(),
        ));
    }
    if signature.backend.trim().is_empty() {
        return Err(EmbeddingServiceError::ModelSignatureUnavailable(
            "model_signature.backend is empty".to_string(),
        ));
    }
    if signature.vector_dimensions == 0 {
        return Err(EmbeddingServiceError::ModelSignatureUnavailable(
            "model_signature.vector_dimensions is zero".to_string(),
        ));
    }
    Ok(())
}

fn map_object_error(err: EmbeddingServiceError) -> EmbedObjectError {
    let (code, message) = match err {
        EmbeddingServiceError::RuntimeNotReady(msg) => ("runtime_not_ready".to_string(), msg),
        EmbeddingServiceError::CapabilityViolation(msg) => {
            ("capability_violation".to_string(), msg)
        }
        EmbeddingServiceError::WorkflowNotFound(msg) => ("workflow_not_found".to_string(), msg),
        EmbeddingServiceError::InvalidRequest(msg) => ("invalid_request".to_string(), msg),
        EmbeddingServiceError::UnsupportedApiVersion => (
            "unsupported_api_version".to_string(),
            "unsupported api version".to_string(),
        ),
        EmbeddingServiceError::ModelSignatureUnavailable(msg) => {
            ("model_signature_unavailable".to_string(), msg)
        }
        EmbeddingServiceError::Internal(msg) => ("embedding_failed".to_string(), msg),
    };
    EmbedObjectError { code, message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockEmbeddingHost {
        capabilities: EmbeddingHostCapabilities,
        signatures: Mutex<HashMap<String, ModelSignature>>,
    }

    impl MockEmbeddingHost {
        fn new(max_batch_size: usize, max_text_length: usize) -> Self {
            let mut signatures = HashMap::new();
            signatures.insert(
                "default".to_string(),
                ModelSignature {
                    model_id: "default".to_string(),
                    model_revision_or_hash: Some("abc123".to_string()),
                    backend: "llamacpp".to_string(),
                    vector_dimensions: 3,
                },
            );
            signatures.insert(
                "model-a".to_string(),
                ModelSignature {
                    model_id: "model-a".to_string(),
                    model_revision_or_hash: Some("hash-model-a".to_string()),
                    backend: "llamacpp".to_string(),
                    vector_dimensions: 3,
                },
            );

            Self {
                capabilities: EmbeddingHostCapabilities {
                    supported_models: vec!["default".to_string(), "model-a".to_string()],
                    max_batch_size,
                    max_text_length,
                },
                signatures: Mutex::new(signatures),
            }
        }
    }

    #[async_trait]
    impl EmbeddingHost for MockEmbeddingHost {
        async fn validate_embedding_workflow(
            &self,
            workflow_id: &str,
        ) -> Result<(), EmbeddingServiceError> {
            if workflow_id == "wf-missing" {
                return Err(EmbeddingServiceError::WorkflowNotFound(workflow_id.to_string()));
            }
            Ok(())
        }

        async fn embedding_capabilities(
            &self,
            _workflow_id: &str,
        ) -> Result<EmbeddingHostCapabilities, EmbeddingServiceError> {
            Ok(self.capabilities.clone())
        }

        async fn embed_one(
            &self,
            _workflow_id: &str,
            text: &str,
            _model_id: Option<&str>,
        ) -> Result<(Vec<f32>, Option<usize>), EmbeddingServiceError> {
            if text.contains("runtime-error") {
                return Err(EmbeddingServiceError::RuntimeNotReady(
                    "backend not ready".to_string(),
                ));
            }
            if text.contains("internal-error") {
                return Err(EmbeddingServiceError::Internal(
                    "embedding failed".to_string(),
                ));
            }
            let token_count = text.split_whitespace().count();
            Ok((vec![0.1, 0.2, 0.3], Some(token_count)))
        }

        async fn resolve_model_signature(
            &self,
            _workflow_id: &str,
            model_id: Option<&str>,
            vector_dimensions: usize,
        ) -> Result<ModelSignature, EmbeddingServiceError> {
            let key = model_id.unwrap_or("default").to_string();
            let signatures = self.signatures.lock().expect("lock signatures");
            let mut signature = signatures
                .get(&key)
                .cloned()
                .ok_or_else(|| EmbeddingServiceError::ModelSignatureUnavailable(key.clone()))?;
            signature.vector_dimensions = vector_dimensions;
            Ok(signature)
        }
    }

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

    #[tokio::test]
    async fn embed_objects_v1_preserves_order_and_partial_failures() {
        let host = MockEmbeddingHost::new(10, 64);
        let service = EmbeddingService::new();
        let response = service
            .embed_objects_v1(
                &host,
                EmbedObjectsV1Request {
                    api_version: "v1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    objects: vec![
                        EmbedInputObject {
                            object_id: "1".to_string(),
                            text: "hello world".to_string(),
                            metadata: None,
                        },
                        EmbedInputObject {
                            object_id: "2".to_string(),
                            text: "runtime-error object".to_string(),
                            metadata: None,
                        },
                        EmbedInputObject {
                            object_id: "3".to_string(),
                            text: "third object".to_string(),
                            metadata: None,
                        },
                    ],
                    model_id: Some("model-a".to_string()),
                    batch_id: Some("batch-1".to_string()),
                },
            )
            .await
            .expect("embed_objects_v1");

        assert_eq!(response.api_version, "v1");
        assert_eq!(response.results.len(), 3);
        assert_eq!(response.results[0].object_id, "1");
        assert_eq!(response.results[1].object_id, "2");
        assert_eq!(response.results[2].object_id, "3");
        assert_eq!(response.results[0].status, EmbeddingStatus::Success);
        assert_eq!(response.results[1].status, EmbeddingStatus::Failed);
        assert_eq!(response.results[2].status, EmbeddingStatus::Success);
        assert_eq!(
            response.results[1].error.as_ref().map(|e| e.code.as_str()),
            Some("runtime_not_ready")
        );
        assert_eq!(response.model_signature.model_id, "model-a");
        assert_eq!(response.model_signature.vector_dimensions, 3);
    }

    #[tokio::test]
    async fn embed_objects_v1_fails_when_all_objects_fail() {
        let host = MockEmbeddingHost::new(10, 32);
        let service = EmbeddingService::new();
        let err = service
            .embed_objects_v1(
                &host,
                EmbedObjectsV1Request {
                    api_version: "v1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    objects: vec![EmbedInputObject {
                        object_id: "1".to_string(),
                        text: "".to_string(),
                        metadata: None,
                    }],
                    model_id: None,
                    batch_id: None,
                },
            )
            .await
            .expect_err("expected no successful objects error");

        match err {
            EmbeddingServiceError::ModelSignatureUnavailable(_) => {}
            other => panic!("unexpected error: {other}"),
        }
    }

    #[tokio::test]
    async fn capabilities_v1_returns_host_capabilities() {
        let host = MockEmbeddingHost::new(8, 4096);
        let service = EmbeddingService::new();
        let response = service
            .get_embedding_workflow_capabilities_v1(
                &host,
                GetEmbeddingWorkflowCapabilitiesV1Request {
                    api_version: "v1".to_string(),
                    workflow_id: "wf-1".to_string(),
                },
            )
            .await
            .expect("capabilities");

        assert_eq!(response.api_version, "v1");
        assert_eq!(response.max_batch_size, 8);
        assert_eq!(response.max_text_length, 4096);
        assert_eq!(response.supported_models.len(), 2);
    }
}

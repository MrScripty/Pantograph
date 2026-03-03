use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use uuid::Uuid;

use crate::capabilities;

/// Single object input for workflow processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowInputObject {
    pub object_id: String,
    pub text: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Request contract for object-in/object-out workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunRequest {
    pub workflow_id: String,
    pub objects: Vec<WorkflowInputObject>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub batch_id: Option<String>,
}

/// Canonical model signature used by consumers for compatibility checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeSignature {
    pub model_id: String,
    #[serde(default)]
    pub model_revision_or_hash: Option<String>,
    pub backend: String,
    pub vector_dimensions: usize,
}

/// Per-object execution status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Success,
    Failed,
}

/// Structured per-object error payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowObjectError {
    pub code: String,
    pub message: String,
}

/// Per-object workflow result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowObjectResult {
    pub object_id: String,
    pub embedding: Option<Vec<f32>>,
    #[serde(default)]
    pub token_count: Option<usize>,
    pub status: WorkflowStatus,
    #[serde(default)]
    pub error: Option<WorkflowObjectError>,
}

/// Workflow run response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunResponse {
    pub run_id: String,
    pub model_signature: RuntimeSignature,
    pub results: Vec<WorkflowObjectResult>,
    pub timing_ms: u128,
}

/// Workflow capabilities request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowCapabilitiesRequest {
    pub workflow_id: String,
}

/// Host capability payload consumed by the service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRuntimeRequirements {
    #[serde(default)]
    pub estimated_peak_vram_mb: Option<u64>,
    #[serde(default)]
    pub estimated_peak_ram_mb: Option<u64>,
    #[serde(default)]
    pub estimated_min_vram_mb: Option<u64>,
    #[serde(default)]
    pub estimated_min_ram_mb: Option<u64>,
    pub estimation_confidence: String,
    pub required_models: Vec<String>,
    pub required_backends: Vec<String>,
    pub required_extensions: Vec<String>,
}

impl Default for WorkflowRuntimeRequirements {
    fn default() -> Self {
        Self {
            estimated_peak_vram_mb: None,
            estimated_peak_ram_mb: None,
            estimated_min_vram_mb: None,
            estimated_min_ram_mb: None,
            estimation_confidence: "unknown".to_string(),
            required_models: Vec::new(),
            required_backends: Vec::new(),
            required_extensions: Vec::new(),
        }
    }
}

/// Host capability payload consumed by the service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowHostCapabilities {
    pub max_batch_size: usize,
    pub max_text_length: usize,
    pub runtime_requirements: WorkflowRuntimeRequirements,
}

/// Workflow capabilities response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowCapabilitiesResponse {
    pub max_batch_size: usize,
    pub max_text_length: usize,
    pub runtime_requirements: WorkflowRuntimeRequirements,
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowServiceError {
    #[error("invalid_request: {0}")]
    InvalidRequest(String),

    #[error("workflow_not_found: {0}")]
    WorkflowNotFound(String),

    #[error("capability_violation: {0}")]
    CapabilityViolation(String),

    #[error("runtime_not_ready: {0}")]
    RuntimeNotReady(String),

    #[error("model_signature_unavailable: {0}")]
    RuntimeSignatureUnavailable(String),

    #[error("internal_error: {0}")]
    Internal(String),
}

/// Trait boundary for host/runtime concerns needed by workflow service.
#[async_trait]
pub trait WorkflowHost: Send + Sync {
    /// Candidate roots that may contain `.pantograph/workflows/<workflow_id>.json`.
    fn workflow_roots(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    /// Upper bound for request object count.
    fn max_batch_size(&self) -> usize {
        capabilities::DEFAULT_MAX_BATCH_SIZE
    }

    /// Upper bound for input text length.
    fn max_text_length(&self) -> usize {
        capabilities::DEFAULT_MAX_TEXT_LENGTH
    }

    /// Backend identifier used when workflow data does not declare one.
    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok("unknown".to_string())
    }

    /// Optional model metadata for runtime requirement estimation.
    async fn model_metadata(
        &self,
        _model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        Ok(None)
    }

    /// Resolve workflow identity and fail if it is unknown to the host.
    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots()).map(|_| ())
    }

    /// Return capability limits and model support metadata.
    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        let stored =
            capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots())?;
        let required_models = capabilities::extract_required_models(stored.nodes());
        let mut required_backends = capabilities::extract_required_backends(stored.nodes());
        if required_backends.is_empty() {
            required_backends.push(self.default_backend_name().await?);
        }
        required_backends.sort();
        required_backends.dedup();

        let required_extensions =
            capabilities::extract_required_extensions(stored.nodes(), !required_models.is_empty());
        let mut model_metadata = HashMap::new();
        for model_id in &required_models {
            if let Some(metadata) = self.model_metadata(model_id).await? {
                model_metadata.insert(model_id.clone(), metadata);
            }
        }

        let (
            estimated_peak_vram_mb,
            estimated_peak_ram_mb,
            estimated_min_vram_mb,
            estimated_min_ram_mb,
            estimation_confidence,
        ) = capabilities::estimate_memory_requirements(&required_models, &model_metadata);

        Ok(WorkflowHostCapabilities {
            max_batch_size: self.max_batch_size(),
            max_text_length: self.max_text_length(),
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb,
                estimated_peak_ram_mb,
                estimated_min_vram_mb,
                estimated_min_ram_mb,
                estimation_confidence,
                required_models,
                required_backends,
                required_extensions,
            },
        })
    }

    /// Run one input object for the given workflow and optional model.
    async fn run_object(
        &self,
        workflow_id: &str,
        text: &str,
        model_id: Option<&str>,
    ) -> Result<(Vec<f32>, Option<usize>), WorkflowServiceError>;

    /// Resolve model signature fields after successful generation.
    async fn resolve_runtime_signature(
        &self,
        workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<RuntimeSignature, WorkflowServiceError>;
}

/// Service entrypoint for workflow API operations.
#[derive(Default)]
pub struct WorkflowService;

impl WorkflowService {
    pub fn new() -> Self {
        Self
    }

    pub async fn workflow_run<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        if request.objects.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "objects must contain at least one item".to_string(),
            ));
        }

        host.validate_workflow(&request.workflow_id).await?;
        let capabilities = host.workflow_capabilities(&request.workflow_id).await?;
        if request.objects.len() > capabilities.max_batch_size {
            return Err(WorkflowServiceError::CapabilityViolation(format!(
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
                results.push(WorkflowObjectResult {
                    object_id: object.object_id,
                    embedding: None,
                    token_count: None,
                    status: WorkflowStatus::Failed,
                    error: Some(WorkflowObjectError {
                        code: "object_validation_failed".to_string(),
                        message: "object_id must be non-empty".to_string(),
                    }),
                });
                continue;
            }

            let text = object.text.trim().to_string();
            if text.is_empty() {
                results.push(WorkflowObjectResult {
                    object_id,
                    embedding: None,
                    token_count: None,
                    status: WorkflowStatus::Failed,
                    error: Some(WorkflowObjectError {
                        code: "object_validation_failed".to_string(),
                        message: "text must be non-empty".to_string(),
                    }),
                });
                continue;
            }
            if text.len() > capabilities.max_text_length {
                results.push(WorkflowObjectResult {
                    object_id,
                    embedding: None,
                    token_count: None,
                    status: WorkflowStatus::Failed,
                    error: Some(WorkflowObjectError {
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
                .run_object(&request.workflow_id, &text, model_id)
                .await
            {
                Ok((embedding, token_count)) => {
                    if embedding.is_empty() {
                        results.push(WorkflowObjectResult {
                            object_id,
                            embedding: None,
                            token_count: None,
                            status: WorkflowStatus::Failed,
                            error: Some(WorkflowObjectError {
                                code: "embedding_failed".to_string(),
                                message: "embedding vector is empty".to_string(),
                            }),
                        });
                        continue;
                    }

                    first_success_dims.get_or_insert(embedding.len());
                    results.push(WorkflowObjectResult {
                        object_id,
                        embedding: Some(embedding),
                        token_count,
                        status: WorkflowStatus::Success,
                        error: None,
                    });
                }
                Err(err) => {
                    let mapped = map_object_error(err);
                    results.push(WorkflowObjectResult {
                        object_id,
                        embedding: None,
                        token_count: None,
                        status: WorkflowStatus::Failed,
                        error: Some(mapped),
                    });
                }
            }
        }

        let vector_dimensions = first_success_dims.ok_or_else(|| {
            WorkflowServiceError::RuntimeSignatureUnavailable(
                "no successful object results; model signature cannot be resolved".to_string(),
            )
        })?;
        let model_signature = host
            .resolve_runtime_signature(&request.workflow_id, model_id, vector_dimensions)
            .await?;
        validate_model_signature(&model_signature)?;

        let run_id = request
            .batch_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        Ok(WorkflowRunResponse {
            run_id,
            model_signature,
            results,
            timing_ms: started.elapsed().as_millis(),
        })
    }

    pub async fn workflow_get_capabilities<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowCapabilitiesRequest,
    ) -> Result<WorkflowCapabilitiesResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        host.validate_workflow(&request.workflow_id).await?;
        let capabilities = host.workflow_capabilities(&request.workflow_id).await?;
        Ok(WorkflowCapabilitiesResponse {
            max_batch_size: capabilities.max_batch_size,
            max_text_length: capabilities.max_text_length,
            runtime_requirements: capabilities.runtime_requirements,
        })
    }
}

fn validate_workflow_id(workflow_id: &str) -> Result<(), WorkflowServiceError> {
    if workflow_id.trim().is_empty() {
        return Err(WorkflowServiceError::InvalidRequest(
            "workflow_id must be non-empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_model_signature(signature: &RuntimeSignature) -> Result<(), WorkflowServiceError> {
    if signature.model_id.trim().is_empty() {
        return Err(WorkflowServiceError::RuntimeSignatureUnavailable(
            "model_signature.model_id is empty".to_string(),
        ));
    }
    if signature.backend.trim().is_empty() {
        return Err(WorkflowServiceError::RuntimeSignatureUnavailable(
            "model_signature.backend is empty".to_string(),
        ));
    }
    if signature.vector_dimensions == 0 {
        return Err(WorkflowServiceError::RuntimeSignatureUnavailable(
            "model_signature.vector_dimensions is zero".to_string(),
        ));
    }
    Ok(())
}

fn map_object_error(err: WorkflowServiceError) -> WorkflowObjectError {
    let (code, message) = match err {
        WorkflowServiceError::RuntimeNotReady(msg) => ("runtime_not_ready".to_string(), msg),
        WorkflowServiceError::CapabilityViolation(msg) => {
            ("capability_violation".to_string(), msg)
        }
        WorkflowServiceError::WorkflowNotFound(msg) => ("workflow_not_found".to_string(), msg),
        WorkflowServiceError::InvalidRequest(msg) => ("invalid_request".to_string(), msg),
        WorkflowServiceError::RuntimeSignatureUnavailable(msg) => {
            ("model_signature_unavailable".to_string(), msg)
        }
        WorkflowServiceError::Internal(msg) => ("embedding_failed".to_string(), msg),
    };
    WorkflowObjectError { code, message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockWorkflowHost {
        capabilities: WorkflowHostCapabilities,
        signatures: Mutex<HashMap<String, RuntimeSignature>>,
    }

    impl MockWorkflowHost {
        fn new(max_batch_size: usize, max_text_length: usize) -> Self {
            let mut signatures = HashMap::new();
            signatures.insert(
                "default".to_string(),
                RuntimeSignature {
                    model_id: "default".to_string(),
                    model_revision_or_hash: Some("abc123".to_string()),
                    backend: "llamacpp".to_string(),
                    vector_dimensions: 3,
                },
            );
            signatures.insert(
                "model-a".to_string(),
                RuntimeSignature {
                    model_id: "model-a".to_string(),
                    model_revision_or_hash: Some("hash-model-a".to_string()),
                    backend: "llamacpp".to_string(),
                    vector_dimensions: 3,
                },
            );

            Self {
                capabilities: WorkflowHostCapabilities {
                    max_batch_size,
                    max_text_length,
                    runtime_requirements: WorkflowRuntimeRequirements {
                        estimated_peak_vram_mb: Some(1024),
                        estimated_peak_ram_mb: Some(2048),
                        estimated_min_vram_mb: Some(512),
                        estimated_min_ram_mb: Some(1024),
                        estimation_confidence: "estimated".to_string(),
                        required_models: vec!["model-a".to_string()],
                        required_backends: vec!["llamacpp".to_string()],
                        required_extensions: vec!["inference_gateway".to_string()],
                    },
                },
                signatures: Mutex::new(signatures),
            }
        }
    }

    struct DefaultCapabilitiesHost {
        workflow_root: PathBuf,
    }

    #[async_trait]
    impl WorkflowHost for DefaultCapabilitiesHost {
        fn workflow_roots(&self) -> Vec<PathBuf> {
            vec![self.workflow_root.clone()]
        }

        async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
            Ok("fallback-backend".to_string())
        }

        async fn model_metadata(
            &self,
            model_id: &str,
        ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
            if model_id == "model-a" {
                Ok(Some(serde_json::json!({
                    "size_bytes": 2_u64 * 1024_u64 * 1024_u64
                })))
            } else {
                Ok(None)
            }
        }

        async fn run_object(
            &self,
            _workflow_id: &str,
            _text: &str,
            _model_id: Option<&str>,
        ) -> Result<(Vec<f32>, Option<usize>), WorkflowServiceError> {
            Ok((vec![0.1, 0.2], Some(2)))
        }

        async fn resolve_runtime_signature(
            &self,
            _workflow_id: &str,
            _model_id: Option<&str>,
            vector_dimensions: usize,
        ) -> Result<RuntimeSignature, WorkflowServiceError> {
            Ok(RuntimeSignature {
                model_id: "model-a".to_string(),
                model_revision_or_hash: Some("sha256:abc123".to_string()),
                backend: "llamacpp".to_string(),
                vector_dimensions,
            })
        }
    }

    #[async_trait]
    impl WorkflowHost for MockWorkflowHost {
        async fn validate_workflow(
            &self,
            workflow_id: &str,
        ) -> Result<(), WorkflowServiceError> {
            if workflow_id == "wf-missing" {
                return Err(WorkflowServiceError::WorkflowNotFound(workflow_id.to_string()));
            }
            Ok(())
        }

        async fn workflow_capabilities(
            &self,
            _workflow_id: &str,
        ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
            Ok(self.capabilities.clone())
        }

        async fn run_object(
            &self,
            _workflow_id: &str,
            text: &str,
            _model_id: Option<&str>,
        ) -> Result<(Vec<f32>, Option<usize>), WorkflowServiceError> {
            if text.contains("runtime-error") {
                return Err(WorkflowServiceError::RuntimeNotReady(
                    "backend not ready".to_string(),
                ));
            }
            if text.contains("internal-error") {
                return Err(WorkflowServiceError::Internal(
                    "embedding failed".to_string(),
                ));
            }
            let token_count = text.split_whitespace().count();
            Ok((vec![0.1, 0.2, 0.3], Some(token_count)))
        }

        async fn resolve_runtime_signature(
            &self,
            _workflow_id: &str,
            model_id: Option<&str>,
            vector_dimensions: usize,
        ) -> Result<RuntimeSignature, WorkflowServiceError> {
            let key = model_id.unwrap_or("default").to_string();
            let signatures = self.signatures.lock().expect("lock signatures");
            let mut signature = signatures
                .get(&key)
                .cloned()
                .ok_or_else(|| WorkflowServiceError::RuntimeSignatureUnavailable(key.clone()))?;
            signature.vector_dimensions = vector_dimensions;
            Ok(signature)
        }
    }

    #[test]
    fn request_roundtrip_uses_snake_case() {
        let req = WorkflowRunRequest {
            workflow_id: "wf-1".to_string(),
            objects: vec![WorkflowInputObject {
                object_id: "obj-1".to_string(),
                text: "hello".to_string(),
                metadata: None,
            }],
            model_id: Some("model-1".to_string()),
            batch_id: Some("batch-1".to_string()),
        };

        let json = serde_json::to_value(&req).expect("serialize request");
        assert_eq!(json["workflow_id"], "wf-1");
        assert_eq!(json["objects"][0]["object_id"], "obj-1");
    }

    #[test]
    fn response_roundtrip_preserves_signature_fields() {
        let res = WorkflowRunResponse {
            run_id: "run-1".to_string(),
            model_signature: RuntimeSignature {
                model_id: "model-1".to_string(),
                model_revision_or_hash: Some("abc123".to_string()),
                backend: "llamacpp".to_string(),
                vector_dimensions: 1024,
            },
            results: vec![],
            timing_ms: 5,
        };

        let json = serde_json::to_string(&res).expect("serialize response");
        let parsed: WorkflowRunResponse = serde_json::from_str(&json).expect("parse response");
        assert_eq!(parsed.model_signature.model_id, "model-1");
        assert_eq!(parsed.model_signature.vector_dimensions, 1024);
    }

    #[tokio::test]
    async fn workflow_run_preserves_order_and_partial_failures() {
        let host = MockWorkflowHost::new(10, 64);
        let service = WorkflowService::new();
        let response = service
            .workflow_run(
                &host,
                WorkflowRunRequest {
                    workflow_id: "wf-1".to_string(),
                    objects: vec![
                        WorkflowInputObject {
                            object_id: "1".to_string(),
                            text: "hello world".to_string(),
                            metadata: None,
                        },
                        WorkflowInputObject {
                            object_id: "2".to_string(),
                            text: "runtime-error object".to_string(),
                            metadata: None,
                        },
                        WorkflowInputObject {
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
            .expect("workflow_run");

        assert_eq!(response.results.len(), 3);
        assert_eq!(response.results[0].object_id, "1");
        assert_eq!(response.results[1].object_id, "2");
        assert_eq!(response.results[2].object_id, "3");
        assert_eq!(response.results[0].status, WorkflowStatus::Success);
        assert_eq!(response.results[1].status, WorkflowStatus::Failed);
        assert_eq!(response.results[2].status, WorkflowStatus::Success);
        assert_eq!(
            response.results[1].error.as_ref().map(|e| e.code.as_str()),
            Some("runtime_not_ready")
        );
        assert_eq!(response.model_signature.model_id, "model-a");
        assert_eq!(response.model_signature.vector_dimensions, 3);
    }

    #[tokio::test]
    async fn workflow_run_fails_when_all_objects_fail() {
        let host = MockWorkflowHost::new(10, 32);
        let service = WorkflowService::new();
        let err = service
            .workflow_run(
                &host,
                WorkflowRunRequest {
                    workflow_id: "wf-1".to_string(),
                    objects: vec![WorkflowInputObject {
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
            WorkflowServiceError::RuntimeSignatureUnavailable(_) => {}
            other => panic!("unexpected error: {other}"),
        }
    }

    #[tokio::test]
    async fn capabilities_returns_host_capabilities() {
        let host = MockWorkflowHost::new(8, 4096);
        let service = WorkflowService::new();
        let response = service
            .workflow_get_capabilities(
                &host,
                WorkflowCapabilitiesRequest { workflow_id: "wf-1".to_string() },
            )
            .await
            .expect("capabilities");

        assert_eq!(response.max_batch_size, 8);
        assert_eq!(response.max_text_length, 4096);
        assert_eq!(response.runtime_requirements.estimated_peak_ram_mb, Some(2048));
        assert_eq!(response.runtime_requirements.required_models.len(), 1);
    }

    #[tokio::test]
    async fn workflow_run_uses_batch_id_as_run_id_when_present() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::new();
        let response = service
            .workflow_run(
                &host,
                WorkflowRunRequest {
                    workflow_id: "wf-1".to_string(),
                    objects: vec![WorkflowInputObject {
                        object_id: "a".to_string(),
                        text: "ok".to_string(),
                        metadata: None,
                    }],
                    model_id: None,
                    batch_id: Some("batch-xyz".to_string()),
                },
            )
            .await
            .expect("embed with batch id");

        assert_eq!(response.run_id, "batch-xyz");
    }

    #[tokio::test]
    async fn default_capabilities_derive_runtime_requirements_from_workflow() {
        let temp_root = std::env::temp_dir()
            .join("pantograph-workflow-service-tests")
            .join(uuid::Uuid::new_v4().to_string());
        let workflow_root = temp_root.join(".pantograph").join("workflows");
        fs::create_dir_all(&workflow_root).expect("create workflow root");
        let workflow_path = workflow_root.join("wf-default.json");
        fs::write(
            &workflow_path,
            serde_json::json!({
                "metadata": {
                    "name": "Default Capability Test"
                },
                "graph": {
                    "nodes": [
                        {
                            "id": "node-1",
                            "node_type": "text-input",
                            "data": {
                                "model_id": "model-a",
                                "backend_key": "llamacpp"
                            },
                            "position": { "x": 0.0, "y": 0.0 }
                        }
                    ],
                    "edges": []
                }
            })
            .to_string(),
        )
        .expect("write workflow");

        let host = DefaultCapabilitiesHost { workflow_root };
        let response = WorkflowService::new()
            .workflow_get_capabilities(
                &host,
                WorkflowCapabilitiesRequest {
                    workflow_id: "wf-default".to_string(),
                },
            )
            .await
            .expect("capabilities response");

        assert_eq!(response.max_batch_size, capabilities::DEFAULT_MAX_BATCH_SIZE);
        assert_eq!(response.max_text_length, capabilities::DEFAULT_MAX_TEXT_LENGTH);
        assert_eq!(response.runtime_requirements.required_models, vec!["model-a"]);
        assert_eq!(response.runtime_requirements.required_backends, vec!["llamacpp"]);
        assert_eq!(
            response.runtime_requirements.required_extensions,
            vec!["inference_gateway".to_string(), "pumas_api".to_string()]
        );
        assert_eq!(response.runtime_requirements.estimated_peak_ram_mb, Some(2));
        assert_eq!(response.runtime_requirements.estimated_min_ram_mb, Some(2));
        assert_eq!(
            response.runtime_requirements.estimation_confidence,
            "estimated_from_model_sizes"
        );

        let _ = fs::remove_dir_all(temp_root);
    }
}

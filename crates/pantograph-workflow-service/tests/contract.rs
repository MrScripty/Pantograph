use async_trait::async_trait;
use pantograph_workflow_service::{
    RuntimeSignature, WorkflowCapabilitiesRequest, WorkflowHost, WorkflowHostCapabilities,
    WorkflowInputObject, WorkflowRunRequest, WorkflowRuntimeRequirements, WorkflowService,
    WorkflowServiceError,
};

struct ContractHost;

#[async_trait]
impl WorkflowHost for ContractHost {
    async fn validate_workflow(
        &self,
        _workflow_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(WorkflowHostCapabilities {
            max_batch_size: 32,
            max_text_length: 4096,
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: Some(1536),
                estimated_peak_ram_mb: Some(3072),
                estimated_min_vram_mb: Some(1024),
                estimated_min_ram_mb: Some(2048),
                estimation_confidence: "estimated".to_string(),
                required_models: vec!["model-a".to_string()],
                required_backends: vec!["llamacpp".to_string()],
                required_extensions: vec!["inference_gateway".to_string()],
            },
        })
    }

    async fn run_object(
        &self,
        _workflow_id: &str,
        _text: &str,
        _model_id: Option<&str>,
    ) -> Result<(Vec<f32>, Option<usize>), WorkflowServiceError> {
        Ok((vec![0.1, 0.2, 0.3], Some(2)))
    }

    async fn resolve_runtime_signature(
        &self,
        _workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<RuntimeSignature, WorkflowServiceError> {
        Ok(RuntimeSignature {
            model_id: model_id.unwrap_or("model-a").to_string(),
            model_revision_or_hash: Some("rev-1".to_string()),
            backend: "llamacpp".to_string(),
            vector_dimensions,
        })
    }
}

#[tokio::test]
async fn workflow_run_contract_snapshot() {
    let service = WorkflowService::new();
    let host = ContractHost;

    let response = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                objects: vec![WorkflowInputObject {
                    object_id: "obj-1".to_string(),
                    text: "hello world".to_string(),
                    metadata: None,
                }],
                model_id: Some("model-a".to_string()),
                batch_id: Some("batch-123".to_string()),
            },
        )
        .await
        .expect("workflow_run response");

    let value = serde_json::to_value(response).expect("serialize response");
    let expected_without_embedding = serde_json::json!({
        "run_id": "batch-123",
        "model_signature": {
            "model_id": "model-a",
            "model_revision_or_hash": "rev-1",
            "backend": "llamacpp",
            "vector_dimensions": 3
        },
        "results": [
            {
                "object_id": "obj-1",
                "embedding": value["results"][0]["embedding"],
                "token_count": 2,
                "status": "success",
                "error": null
            }
        ],
        "timing_ms": value["timing_ms"]
    });

    assert_eq!(value, expected_without_embedding);

    let embedding = value["results"][0]["embedding"]
        .as_array()
        .expect("embedding array");
    assert_eq!(embedding.len(), 3);
    let first = embedding[0].as_f64().expect("first value");
    let second = embedding[1].as_f64().expect("second value");
    let third = embedding[2].as_f64().expect("third value");
    assert!((first - 0.1).abs() < 0.000_001);
    assert!((second - 0.2).abs() < 0.000_001);
    assert!((third - 0.3).abs() < 0.000_001);
}

#[tokio::test]
async fn workflow_capabilities_contract_snapshot() {
    let service = WorkflowService::new();
    let host = ContractHost;

    let response = service
        .workflow_get_capabilities(
            &host,
            WorkflowCapabilitiesRequest { workflow_id: "wf-1".to_string() },
        )
        .await
        .expect("capabilities response");

    let value = serde_json::to_value(response).expect("serialize capabilities");
    let expected = serde_json::json!({
        "max_batch_size": 32,
        "max_text_length": 4096,
        "runtime_requirements": {
            "estimated_peak_vram_mb": 1536,
            "estimated_peak_ram_mb": 3072,
            "estimated_min_vram_mb": 1024,
            "estimated_min_ram_mb": 2048,
            "estimation_confidence": "estimated",
            "required_models": ["model-a"],
            "required_backends": ["llamacpp"],
            "required_extensions": ["inference_gateway"]
        }
    });

    assert_eq!(value, expected);
}

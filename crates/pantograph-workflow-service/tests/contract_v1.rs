use async_trait::async_trait;
use pantograph_workflow_service::{
    EmbedInputObject, EmbedObjectsV1Request, EmbeddingHost, EmbeddingHostCapabilities,
    EmbeddingService, EmbeddingServiceError, GetEmbeddingWorkflowCapabilitiesV1Request,
    ModelSignature,
};

struct ContractHost;

#[async_trait]
impl EmbeddingHost for ContractHost {
    async fn validate_embedding_workflow(
        &self,
        _workflow_id: &str,
    ) -> Result<(), EmbeddingServiceError> {
        Ok(())
    }

    async fn embedding_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<EmbeddingHostCapabilities, EmbeddingServiceError> {
        Ok(EmbeddingHostCapabilities {
            supported_models: vec!["model-a".to_string()],
            max_batch_size: 32,
            max_text_length: 4096,
        })
    }

    async fn embed_one(
        &self,
        _workflow_id: &str,
        _text: &str,
        _model_id: Option<&str>,
    ) -> Result<(Vec<f32>, Option<usize>), EmbeddingServiceError> {
        Ok((vec![0.1, 0.2, 0.3], Some(2)))
    }

    async fn resolve_model_signature(
        &self,
        _workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<ModelSignature, EmbeddingServiceError> {
        Ok(ModelSignature {
            model_id: model_id.unwrap_or("model-a").to_string(),
            model_revision_or_hash: Some("rev-1".to_string()),
            backend: "llamacpp".to_string(),
            vector_dimensions,
        })
    }
}

#[tokio::test]
async fn embed_objects_v1_contract_snapshot() {
    let service = EmbeddingService::new();
    let host = ContractHost;

    let response = service
        .embed_objects_v1(
            &host,
            EmbedObjectsV1Request {
                api_version: "v1".to_string(),
                workflow_id: "wf-1".to_string(),
                objects: vec![EmbedInputObject {
                    object_id: "obj-1".to_string(),
                    text: "hello world".to_string(),
                    metadata: None,
                }],
                model_id: Some("model-a".to_string()),
                batch_id: Some("batch-123".to_string()),
            },
        )
        .await
        .expect("embed_objects_v1 response");

    let value = serde_json::to_value(response).expect("serialize response");
    let expected_without_embedding = serde_json::json!({
        "api_version": "v1",
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
async fn capabilities_v1_contract_snapshot() {
    let service = EmbeddingService::new();
    let host = ContractHost;

    let response = service
        .get_embedding_workflow_capabilities_v1(
            &host,
            GetEmbeddingWorkflowCapabilitiesV1Request {
                api_version: "v1".to_string(),
                workflow_id: "wf-1".to_string(),
            },
        )
        .await
        .expect("capabilities response");

    let value = serde_json::to_value(response).expect("serialize capabilities");
    let expected = serde_json::json!({
        "api_version": "v1",
        "supported_models": ["model-a"],
        "max_batch_size": 32,
        "max_text_length": 4096
    });

    assert_eq!(value, expected);
}

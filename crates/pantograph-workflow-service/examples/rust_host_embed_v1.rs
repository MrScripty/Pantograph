use async_trait::async_trait;
use pantograph_workflow_service::{
    EmbedInputObject, EmbedObjectsV1Request, EmbeddingHost, EmbeddingHostCapabilities,
    EmbeddingService, EmbeddingServiceError, ModelSignature,
};

struct ExampleHost;

#[async_trait]
impl EmbeddingHost for ExampleHost {
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
            supported_models: vec!["example-embed-model".to_string()],
            max_batch_size: 16,
            max_text_length: 2048,
        })
    }

    async fn embed_one(
        &self,
        _workflow_id: &str,
        text: &str,
        _model_id: Option<&str>,
    ) -> Result<(Vec<f32>, Option<usize>), EmbeddingServiceError> {
        let token_count = text.split_whitespace().count();
        Ok((vec![0.01, 0.02, 0.03, 0.04], Some(token_count)))
    }

    async fn resolve_model_signature(
        &self,
        _workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<ModelSignature, EmbeddingServiceError> {
        Ok(ModelSignature {
            model_id: model_id.unwrap_or("example-embed-model").to_string(),
            model_revision_or_hash: Some("example-revision-1".to_string()),
            backend: "example-backend".to_string(),
            vector_dimensions,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = EmbeddingService::new();
    let host = ExampleHost;

    let response = service
        .embed_objects_v1(
            &host,
            EmbedObjectsV1Request {
                api_version: "v1".to_string(),
                workflow_id: "embedding-default".to_string(),
                objects: vec![
                    EmbedInputObject {
                        object_id: "doc-1".to_string(),
                        text: "Pantograph headless embedding example".to_string(),
                        metadata: None,
                    },
                    EmbedInputObject {
                        object_id: "doc-2".to_string(),
                        text: "Second object".to_string(),
                        metadata: None,
                    },
                ],
                model_id: Some("example-embed-model".to_string()),
                batch_id: Some("example-batch-1".to_string()),
            },
        )
        .await?;

    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

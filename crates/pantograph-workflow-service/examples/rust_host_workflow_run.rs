use async_trait::async_trait;
use pantograph_workflow_service::{
    RuntimeSignature, WorkflowHost, WorkflowHostCapabilities, WorkflowInputObject,
    WorkflowRunRequest, WorkflowService, WorkflowServiceError,
};

struct ExampleHost;

#[async_trait]
impl WorkflowHost for ExampleHost {
    async fn validate_workflow(
        &self,
        workflow_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        if workflow_id.trim().is_empty() {
            return Err(WorkflowServiceError::WorkflowNotFound(
                "workflow_id is empty".to_string(),
            ));
        }
        Ok(())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(WorkflowHostCapabilities {
            supported_models: vec!["example-embed-model".to_string()],
            max_batch_size: 16,
            max_text_length: 2048,
        })
    }

    async fn run_object(
        &self,
        _workflow_id: &str,
        text: &str,
        _model_id: Option<&str>,
    ) -> Result<(Vec<f32>, Option<usize>), WorkflowServiceError> {
        let token_count = text.split_whitespace().count();
        Ok((vec![0.01, 0.02, 0.03, 0.04], Some(token_count)))
    }

    async fn resolve_runtime_signature(
        &self,
        _workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<RuntimeSignature, WorkflowServiceError> {
        Ok(RuntimeSignature {
            model_id: model_id.unwrap_or("example-embed-model").to_string(),
            model_revision_or_hash: Some("example-revision-1".to_string()),
            backend: "example-backend".to_string(),
            vector_dimensions,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = WorkflowService::new();
    let host = ExampleHost;

    let response = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "embedding-default".to_string(),
                objects: vec![
                    WorkflowInputObject {
                        object_id: "doc-1".to_string(),
                        text: "Pantograph headless embedding example".to_string(),
                        metadata: None,
                    },
                    WorkflowInputObject {
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

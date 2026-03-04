use async_trait::async_trait;
use pantograph_workflow_service::{
    WorkflowCapabilityModel, WorkflowHost, WorkflowHostCapabilities, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowRunRequest, WorkflowRuntimeRequirements, WorkflowService,
    WorkflowServiceError,
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
            max_input_bindings: 16,
            max_output_targets: 16,
            max_value_bytes: 4096,
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: Some(512),
                estimated_peak_ram_mb: Some(1024),
                estimated_min_vram_mb: Some(256),
                estimated_min_ram_mb: Some(512),
                estimation_confidence: "estimated".to_string(),
                required_models: vec!["example-embed-model".to_string()],
                required_backends: vec!["example-backend".to_string()],
                required_extensions: vec!["inference_gateway".to_string()],
            },
            models: vec![WorkflowCapabilityModel {
                model_id: "example-embed-model".to_string(),
                model_revision_or_hash: Some("sha256:examplehash".to_string()),
                model_type: Some("embedding".to_string()),
                node_ids: vec!["embedding-node".to_string()],
                roles: vec!["embedding".to_string()],
            }],
        })
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if let Some(targets) = output_targets {
            return Ok(targets
                .iter()
                .map(|target| WorkflowPortBinding {
                    node_id: target.node_id.clone(),
                    port_id: target.port_id.clone(),
                    value: serde_json::json!([0.01, 0.02, 0.03, 0.04]),
                })
                .collect());
        }

        Ok(vec![WorkflowPortBinding {
            node_id: "vector-output-1".to_string(),
            port_id: "vector".to_string(),
            value: serde_json::json!([0.01, 0.02, 0.03, 0.04]),
        }])
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
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("Pantograph headless workflow example"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "vector-output-1".to_string(),
                    port_id: "vector".to_string(),
                }]),
                run_id: Some("example-run-1".to_string()),
            },
        )
        .await?;

    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

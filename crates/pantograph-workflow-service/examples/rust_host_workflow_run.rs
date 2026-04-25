use async_trait::async_trait;
use pantograph_workflow_service::{
    WorkflowCapabilityModel, WorkflowExecutionSessionCreateRequest,
    WorkflowExecutionSessionRunRequest, WorkflowHost, WorkflowHostCapabilities,
    WorkflowOutputTarget, WorkflowPortBinding, WorkflowRuntimeCapability,
    WorkflowRuntimeInstallState, WorkflowRuntimeRequirements, WorkflowRuntimeSourceKind,
    WorkflowService, WorkflowServiceError,
};

struct ExampleHost;

#[async_trait]
impl WorkflowHost for ExampleHost {
    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
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
            runtime_capabilities: vec![WorkflowRuntimeCapability {
                runtime_id: "example-backend".to_string(),
                display_name: "Example Backend".to_string(),
                install_state: WorkflowRuntimeInstallState::Installed,
                available: true,
                configured: true,
                can_install: false,
                can_remove: false,
                source_kind: WorkflowRuntimeSourceKind::Host,
                selected: true,
                readiness_state: Some(
                    pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready,
                ),
                selected_version: None,
                supports_external_connection: false,
                backend_keys: vec!["example-backend".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        })
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: pantograph_workflow_service::WorkflowRunOptions,
        _run_handle: pantograph_workflow_service::WorkflowRunHandle,
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
    let session = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "embedding-default".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await?;

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: session.session_id,
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("Pantograph headless workflow example"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "vector-output-1".to_string(),
                    port_id: "vector".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                run_id: Some("example-run-1".to_string()),
                priority: None,
            },
        )
        .await?;

    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

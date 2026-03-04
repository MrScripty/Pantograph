use async_trait::async_trait;
use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowCapabilityModel, WorkflowHost, WorkflowHostCapabilities,
    WorkflowOutputTarget, WorkflowPortBinding, WorkflowRunRequest, WorkflowRuntimeRequirements,
    WorkflowService, WorkflowServiceError,
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
            max_input_bindings: 32,
            max_output_targets: 8,
            max_value_bytes: 4096,
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
            models: vec![WorkflowCapabilityModel {
                model_id: "model-a".to_string(),
                model_revision_or_hash: Some("sha256:model-a-hash".to_string()),
                model_type: Some("embedding".to_string()),
                node_ids: vec!["node-embed".to_string()],
                roles: vec!["embedding".to_string(), "inference".to_string()],
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
                    value: serde_json::json!([0.1, 0.2, 0.3]),
                })
                .collect());
        }

        Ok(vec![WorkflowPortBinding {
            node_id: "vector-output-1".to_string(),
            port_id: "vector".to_string(),
            value: serde_json::json!([0.1, 0.2, 0.3]),
        }])
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
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello world"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "vector-output-1".to_string(),
                    port_id: "vector".to_string(),
                }]),
                run_id: Some("run-123".to_string()),
            },
        )
        .await
        .expect("workflow_run response");

    let value = serde_json::to_value(response).expect("serialize response");
    let expected = serde_json::json!({
        "run_id": "run-123",
        "outputs": [
            {
                "node_id": "vector-output-1",
                "port_id": "vector",
                "value": [0.1, 0.2, 0.3]
            }
        ],
        "timing_ms": value["timing_ms"]
    });

    assert_eq!(value, expected);
}

#[tokio::test]
async fn workflow_capabilities_contract_snapshot() {
    let service = WorkflowService::new();
    let host = ContractHost;

    let response = service
        .workflow_get_capabilities(
            &host,
            WorkflowCapabilitiesRequest {
                workflow_id: "wf-1".to_string(),
            },
        )
        .await
        .expect("capabilities response");

    let value = serde_json::to_value(response).expect("serialize capabilities");
    let expected = serde_json::json!({
        "max_input_bindings": 32,
        "max_output_targets": 8,
        "max_value_bytes": 4096,
        "runtime_requirements": {
            "estimated_peak_vram_mb": 1536,
            "estimated_peak_ram_mb": 3072,
            "estimated_min_vram_mb": 1024,
            "estimated_min_ram_mb": 2048,
            "estimation_confidence": "estimated",
            "required_models": ["model-a"],
            "required_backends": ["llamacpp"],
            "required_extensions": ["inference_gateway"]
        },
        "models": [{
            "model_id": "model-a",
            "model_revision_or_hash": "sha256:model-a-hash",
            "model_type": "embedding",
            "node_ids": ["node-embed"],
            "roles": ["embedding", "inference"]
        }]
    });

    assert_eq!(value, expected);
}

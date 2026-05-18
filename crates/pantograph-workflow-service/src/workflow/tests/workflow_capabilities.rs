use super::*;

fn peak_estimate_bytes(
    requirements: &WorkflowRuntimeRequirements,
    kind: WorkflowTechnicalFitResourceEstimateKind,
) -> Option<u64> {
    requirements
        .resource_estimates
        .iter()
        .find(|estimate| {
            estimate.kind() == kind
                && estimate.state() == WorkflowTechnicalFitResourceEstimateState::Available
        })
        .and_then(|estimate| estimate.value_bytes())
}

#[tokio::test]
async fn capabilities_returns_host_capabilities() {
    let host = MockWorkflowHost::new(8, 4096);
    let service = WorkflowService::new();
    let response = service
        .workflow_get_capabilities(
            &host,
            WorkflowCapabilitiesRequest {
                workflow_id: "wf-1".to_string(),
            },
        )
        .await
        .expect("capabilities");

    assert_eq!(response.max_input_bindings, 8);
    assert_eq!(response.max_output_targets, 16);
    assert_eq!(response.max_value_bytes, 4096);
    assert_eq!(
        peak_estimate_bytes(
            &response.runtime_requirements,
            WorkflowTechnicalFitResourceEstimateKind::PeakRamBytes
        ),
        Some(2048_u64 * 1024 * 1024)
    );
    assert_eq!(response.runtime_requirements.required_models.len(), 1);
    assert_eq!(response.models.len(), 1);
    assert_eq!(response.models[0].model_id, "model-a");
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
                            "backend_key": "llamacpp",
                            "embedding": true
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

    assert_eq!(
        response.max_input_bindings,
        capabilities::DEFAULT_MAX_INPUT_BINDINGS
    );
    assert_eq!(
        response.max_output_targets,
        capabilities::DEFAULT_MAX_OUTPUT_TARGETS
    );
    assert_eq!(
        response.max_value_bytes,
        capabilities::DEFAULT_MAX_VALUE_BYTES
    );
    assert_eq!(
        response.runtime_requirements.required_models,
        vec!["model-a"]
    );
    assert_eq!(
        response.runtime_requirements.required_backends,
        vec!["llama_cpp"]
    );
    assert_eq!(
        response.runtime_requirements.required_extensions,
        vec!["inference_gateway".to_string(), "pumas_api".to_string()]
    );
    assert_eq!(response.models.len(), 1);
    assert_eq!(response.models[0].model_id, "model-a");
    assert_eq!(response.models[0].model_type.as_deref(), Some("embedding"));
    assert_eq!(
        response.models[0].model_revision_or_hash.as_deref(),
        Some("sha256:abc123")
    );
    assert_eq!(response.models[0].node_ids, vec!["node-1".to_string()]);
    assert_eq!(response.models[0].roles, vec!["embedding".to_string()]);
    assert_eq!(
        peak_estimate_bytes(
            &response.runtime_requirements,
            WorkflowTechnicalFitResourceEstimateKind::PeakRamBytes
        ),
        Some(2_u64 * 1024 * 1024)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn default_capabilities_project_inference_runtime_as_scheduler_requirement() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-service-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    fs::write(
        workflow_root.join("wf-runtime.json"),
        serde_json::json!({
            "metadata": {
                "name": "Runtime Intent Test"
            },
            "graph": {
                "nodes": [
                    {
                        "id": "image-1",
                        "node_type": "llm-inference",
                        "data": {
                            "runtime": "PyTorch",
                            "backend_key": "llama_cpp",
                            "resolved_model_package_facts": {
                                "recommended_backend": "diffusers"
                            }
                        },
                        "position": { "x": 0.0, "y": 0.0 }
                    },
                    {
                        "id": "image-2",
                        "node_type": "llm-inference",
                        "data": {
                            "backend_key": "candle",
                            "resolved_model_package_facts": {
                                "recommended_backend": "diffusers"
                            }
                        },
                        "position": { "x": 100.0, "y": 0.0 }
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
                workflow_id: "wf-runtime".to_string(),
            },
        )
        .await
        .expect("capabilities response");

    assert_eq!(
        response.runtime_requirements.required_backends,
        vec!["pytorch"]
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn default_capabilities_accept_dot_workflow_identity() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-service-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    fs::write(
        workflow_root.join("Maid-Qwen2.6-27b-heritic.json"),
        serde_json::json!({
            "metadata": {
                "name": "Maid Qwen2.6 27b heritic"
            },
            "graph": {
                "nodes": [],
                "edges": []
            }
        })
        .to_string(),
    )
    .expect("write workflow");

    let host = DefaultCapabilitiesHost { workflow_root };
    WorkflowService::new()
        .workflow_get_capabilities(
            &host,
            WorkflowCapabilitiesRequest {
                workflow_id: "Maid-Qwen2.6-27b-heritic".to_string(),
            },
        )
        .await
        .expect("capabilities response");

    let _ = fs::remove_dir_all(temp_root);
}

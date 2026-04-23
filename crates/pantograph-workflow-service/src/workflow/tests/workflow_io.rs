use super::*;

#[tokio::test]
async fn workflow_get_io_derives_inputs_and_outputs_from_workflow() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O" },
            "graph": {
                "nodes": [
                    {
                        "id": "text-input-1",
                        "node_type": "text-input",
                        "data": {
                            "name": "Prompt",
                            "description": "Prompt supplied by the caller",
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "client_session",
                                "label": "Text Input",
                                "description": "Provides text input",
                                "inputs": [
                                    {
                                        "id": "text",
                                        "label": "Text",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ],
                                "outputs": [
                                    {
                                        "id": "legacy-out",
                                        "label": "Legacy Out",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ]
                            }
                        },
                        "position": { "x": 0.0, "y": 0.0 }
                    },
                    {
                        "id": "text-output-1",
                        "node_type": "text-output",
                        "data": {
                            "definition": {
                                "category": "output",
                                "io_binding_origin": "client_session",
                                "label": "Text Output",
                                "description": "Displays text output",
                                "inputs": [
                                    {
                                        "id": "text",
                                        "label": "Text",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    },
                                    {
                                        "id": "stream",
                                        "label": "Stream",
                                        "data_type": "stream",
                                        "required": false,
                                        "multiple": false
                                    }
                                ],
                                "outputs": [
                                    {
                                        "id": "text",
                                        "label": "Text",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ]
                            }
                        },
                        "position": { "x": 120.0, "y": 0.0 }
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
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io".to_string(),
            },
        )
        .await
        .expect("workflow io response");

    assert_eq!(response.inputs.len(), 1);
    assert_eq!(response.inputs[0].node_id, "text-input-1");
    assert_eq!(response.inputs[0].name.as_deref(), Some("Prompt"));
    assert_eq!(
        response.inputs[0].description.as_deref(),
        Some("Prompt supplied by the caller")
    );
    assert_eq!(response.inputs[0].ports.len(), 1);
    assert_eq!(response.inputs[0].ports[0].port_id, "text");
    assert_eq!(
        response.inputs[0].ports[0].data_type.as_deref(),
        Some("string")
    );
    assert!(response.inputs[0]
        .ports
        .iter()
        .all(|port| port.port_id != "legacy-out"));

    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, "text-output-1");
    assert_eq!(response.outputs[0].ports.len(), 1);
    assert_eq!(response.outputs[0].ports[0].port_id, "text");
    assert!(response.outputs[0]
        .ports
        .iter()
        .all(|port| port.port_id != "stream"));

    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_get_io_rejects_missing_directional_ports() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io-invalid.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O Invalid" },
            "graph": {
                "nodes": [
                    {
                        "id": "text-input-1",
                        "node_type": "text-input",
                        "data": {
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "client_session",
                                "outputs": [
                                    { "id": "text", "label": "Text", "data_type": "string" }
                                ]
                            }
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
    let err = WorkflowService::new()
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io-invalid".to_string(),
            },
        )
        .await
        .expect_err("workflow io should reject missing directional ports");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("text-input-1"));
    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_get_io_skips_integrated_io_nodes() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io-integrated.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O Integrated" },
            "graph": {
                "nodes": [
                    {
                        "id": "puma-lib-1",
                        "node_type": "puma-lib",
                        "data": {
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "integrated",
                                "inputs": [],
                                "outputs": [
                                    { "id": "model_path", "label": "Model Path", "data_type": "string" }
                                ]
                            }
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
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io-integrated".to_string(),
            },
        )
        .await
        .expect("workflow io should skip integrated io nodes");

    assert!(response.inputs.is_empty());
    assert!(response.outputs.is_empty());
    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_get_io_rejects_missing_io_binding_origin() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io-missing-origin.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O Missing Origin" },
            "graph": {
                "nodes": [
                    {
                        "id": "text-input-1",
                        "node_type": "text-input",
                        "data": {
                            "definition": {
                                "category": "input",
                                "inputs": [
                                    { "id": "text", "label": "Text", "data_type": "string" }
                                ]
                            }
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
    let err = WorkflowService::new()
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io-missing-origin".to_string(),
            },
        )
        .await
        .expect_err("workflow io should reject missing io_binding_origin");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err
        .to_string()
        .contains("missing definition.io_binding_origin"));
    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_get_io_rejects_invalid_or_duplicate_port_ids() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io-dup.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O Duplicates" },
            "graph": {
                "nodes": [
                    {
                        "id": "text-output-1",
                        "node_type": "text-output",
                        "data": {
                            "definition": {
                                "category": "output",
                                "io_binding_origin": "client_session",
                                "outputs": [
                                    { "id": "text", "label": "Text", "data_type": "string" },
                                    { "id": "text", "label": "Text 2", "data_type": "string" }
                                ]
                            }
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
    let err = WorkflowService::new()
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io-dup".to_string(),
            },
        )
        .await
        .expect_err("workflow io should reject duplicate port ids");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("duplicate port id 'text'"));
    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_get_io_rejects_whitespace_port_ids() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io-whitespace.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O Whitespace" },
            "graph": {
                "nodes": [
                    {
                        "id": "text-input-1",
                        "node_type": "text-input",
                        "data": {
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "client_session",
                                "inputs": [
                                    { "id": "   ", "label": "Text", "data_type": "string" }
                                ]
                            }
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
    let err = WorkflowService::new()
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io-whitespace".to_string(),
            },
        )
        .await
        .expect_err("workflow io should reject whitespace port ids");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("text-input-1"));
    let _ = fs::remove_dir_all(temp_root);
}

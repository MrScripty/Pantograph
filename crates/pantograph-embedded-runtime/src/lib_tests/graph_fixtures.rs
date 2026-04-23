pub(super) fn runtime_diffusion_data_graph() -> node_engine::WorkflowGraph {
    node_engine::WorkflowGraph {
        id: "runtime-diffusion-data-graph".to_string(),
        name: "Runtime Diffusion Data Graph".to_string(),
        nodes: vec![
            node_engine::GraphNode {
                id: "text-input-1".to_string(),
                node_type: "text-input".to_string(),
                data: serde_json::json!({ "text": "a tiny painted robot" }),
                position: (0.0, 0.0),
            },
            node_engine::GraphNode {
                id: "diffusion-inference-1".to_string(),
                node_type: "diffusion-inference".to_string(),
                data: serde_json::json!({
                    "model_path": "/tmp/mock-diffusion-model",
                    "model_type": "diffusion",
                    "environment_ref": {
                        "state": "ready",
                        "env_ids": ["mock-python-env"]
                    }
                }),
                position: (240.0, 0.0),
            },
            node_engine::GraphNode {
                id: "image-output-1".to_string(),
                node_type: "image-output".to_string(),
                data: serde_json::json!({}),
                position: (520.0, 0.0),
            },
        ],
        edges: vec![
            node_engine::GraphEdge {
                id: "e-prompt".to_string(),
                source: "text-input-1".to_string(),
                source_handle: "text".to_string(),
                target: "diffusion-inference-1".to_string(),
                target_handle: "prompt".to_string(),
            },
            node_engine::GraphEdge {
                id: "e-image".to_string(),
                source: "diffusion-inference-1".to_string(),
                source_handle: "image".to_string(),
                target: "image-output-1".to_string(),
                target_handle: "image".to_string(),
            },
        ],
        groups: Vec::new(),
    }
}

pub(super) fn multi_python_runtime_data_graph() -> node_engine::WorkflowGraph {
    node_engine::WorkflowGraph {
        id: "multi-python-runtime-data-graph".to_string(),
        name: "Multi Python Runtime Data Graph".to_string(),
        nodes: vec![
            node_engine::GraphNode {
                id: "text-input-1".to_string(),
                node_type: "text-input".to_string(),
                data: serde_json::json!({ "text": "painted robot" }),
                position: (0.0, 0.0),
            },
            node_engine::GraphNode {
                id: "text-input-2".to_string(),
                node_type: "text-input".to_string(),
                data: serde_json::json!({ "text": "tiny waveform" }),
                position: (0.0, 180.0),
            },
            node_engine::GraphNode {
                id: "diffusion-inference-1".to_string(),
                node_type: "diffusion-inference".to_string(),
                data: serde_json::json!({
                    "model_path": "/tmp/mock-diffusion-model",
                    "backend_key": "diffusers",
                    "model_type": "diffusion",
                    "environment_ref": {
                        "state": "ready",
                        "env_ids": ["mock-python-env"]
                    }
                }),
                position: (240.0, 0.0),
            },
            node_engine::GraphNode {
                id: "onnx-inference-1".to_string(),
                node_type: "onnx-inference".to_string(),
                data: serde_json::json!({
                    "model_path": "/tmp/mock-onnx-model",
                    "backend_key": "onnxruntime",
                    "model_type": "audio",
                    "environment_ref": {
                        "state": "ready",
                        "env_ids": ["mock-onnx-env"]
                    }
                }),
                position: (240.0, 180.0),
            },
        ],
        edges: vec![
            node_engine::GraphEdge {
                id: "e-prompt".to_string(),
                source: "text-input-1".to_string(),
                source_handle: "text".to_string(),
                target: "diffusion-inference-1".to_string(),
                target_handle: "prompt".to_string(),
            },
            node_engine::GraphEdge {
                id: "e-audio".to_string(),
                source: "text-input-2".to_string(),
                source_handle: "text".to_string(),
                target: "onnx-inference-1".to_string(),
                target_handle: "prompt".to_string(),
            },
        ],
        groups: Vec::new(),
    }
}

pub(super) fn synthetic_kv_node_memory_snapshot(
    session_id: &str,
    node_id: &str,
    cache_id: &str,
) -> node_engine::NodeMemorySnapshot {
    node_engine::NodeMemorySnapshot {
        identity: node_engine::NodeMemoryIdentity {
            session_id: session_id.to_string(),
            node_id: node_id.to_string(),
            node_type: "llamacpp-inference".to_string(),
            schema_version: Some("v1".to_string()),
        },
        status: node_engine::NodeMemoryStatus::Ready,
        input_fingerprint: Some(format!("fp-{cache_id}")),
        output_snapshot: Some(serde_json::json!({
            "kv_cache_out": {
                "cache_id": cache_id,
            }
        })),
        private_state: None,
        indirect_state_reference: Some(node_engine::engine::NodeMemoryIndirectStateReference {
            reference_kind: "kv_cache_handle".to_string(),
            reference_id: cache_id.to_string(),
            restore_strategy: node_engine::engine::NodeMemoryRestoreStrategy::RehydrateBeforeResume,
            inspection_metadata: Some(serde_json::json!({
                "source_port": "kv_cache_out",
                "backend_key": "llamacpp",
                "model_fingerprint": {
                    "model_id": "model-1",
                    "config_hash": "cfg-1",
                },
                "runtime_fingerprint": {
                    "runtime_id": "runtime-1",
                    "backend_key": "llamacpp",
                    "tokenizer_fingerprint": "tok-1",
                    "prompt_format_fingerprint": "prompt-1",
                    "runtime_build_fingerprint": "build-1",
                }
            })),
        }),
        inspection_metadata: Some(serde_json::json!({
            "projection_source": "test",
        })),
    }
}

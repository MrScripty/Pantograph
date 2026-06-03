use super::*;

#[tokio::test]
async fn onnx_nodes_fail_fast_when_environment_ref_is_not_ready() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: HashMap::new(),
    });
    let (executor, extensions) = test_executor(adapter);

    let mut inputs = HashMap::new();
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({"model_id": "pumas://models/model-ready"}),
    );
    inputs.insert("model_type".to_string(), serde_json::json!("audio"));
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "environment_ref".to_string(),
        serde_json::json!({
            "state": "missing",
            "env_id": "env:test"
        }),
    );

    let err = executor
        .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect_err("preflight should block when environment_ref state is not ready");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("environment_ref_gate"));
            assert!(message.contains("missing"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert_eq!(requests.lock().expect("recording lock").len(), 0);
}

#[tokio::test]
async fn python_nodes_block_when_no_dependency_bindings_are_available() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: HashMap::new(),
    });

    let (executor, extensions) = test_executor(adapter);

    let mut inputs = HashMap::new();
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({"model_id": "pumas://models/tiny-tts"}),
    );
    inputs.insert("model_type".to_string(), serde_json::json!("audio"));
    inputs.insert(
        "prompt".to_string(),
        serde_json::json!("paper lantern in the rain"),
    );

    let err = executor
        .execute_task("onnx-inference-2", inputs, &Context::new(), &extensions)
        .await
        .expect_err("python nodes should block without dependency bindings");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Dependency preflight blocked execution"));
            assert!(message.contains("dependency_preflight_retired"));
            assert!(message.contains("diagnostic-only"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert_eq!(requests.lock().expect("recording lock").len(), 0);
}

#[tokio::test]
async fn python_nodes_block_when_bindings_are_missing_runtime_packages() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: HashMap::new(),
    });

    let (executor, extensions) = test_executor(adapter);

    let mut inputs = HashMap::new();
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({"model_id": "pumas://models/tiny-tts"}),
    );
    inputs.insert("model_type".to_string(), serde_json::json!("audio"));
    inputs.insert(
        "prompt".to_string(),
        serde_json::json!("paper lantern in the rain"),
    );

    let err = executor
        .execute_task("onnx-inference-3", inputs, &Context::new(), &extensions)
        .await
        .expect_err("python nodes should block when runtime packages are missing");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Dependency preflight blocked execution"));
            assert!(message.contains("dependency_preflight_retired"));
            assert!(message.contains("legacy_dependency_resolver"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert_eq!(requests.lock().expect("recording lock").len(), 0);
}

use super::*;

#[tokio::test]
async fn onnx_nodes_block_when_dependency_preflight_is_not_ready() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: HashMap::new(),
    });
    let resolver = Arc::new(CountingDependencyResolver::new());
    let (executor, extensions) = test_executor(adapter, resolver.clone());

    let mut inputs = HashMap::new();
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({"model_id": "pumas://models/model-not-ready"}),
    );
    inputs.insert("model_type".to_string(), serde_json::json!("audio"));
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));

    let err = executor
        .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect_err("preflight should block retired embedded-runtime path");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Dependency preflight blocked execution"));
            assert!(message.contains("dependency_preflight_retired"));
            assert!(message.contains("diagnostic-only"));
            assert!(message.contains("ModelDependencyResolver"));
            assert!(message.contains("ModelDependencyRequest"));
            assert!(message.contains("ModelRefV2"));
            assert!(message.contains("python_runtime_adapter"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert_eq!(resolver.call_count(), 0);
    assert_eq!(requests.lock().expect("recording lock").len(), 0);
}

#[tokio::test]
async fn onnx_nodes_fail_closed_before_resolved_model_ref_preflight() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("response".to_string(), serde_json::json!("ok"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolver = Arc::new(CountingDependencyResolver::new());
    let (executor, extensions) = test_executor(adapter, resolver.clone());

    let mut inputs = HashMap::new();
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({"model_id": "pumas://models/model-ready"}),
    );
    inputs.insert("model_type".to_string(), serde_json::json!("audio"));
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));

    let error = executor
        .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect_err("retired preflight must fail before model_ref resolution");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("dependency_preflight_retired"));
            assert!(message.contains("pumas://models/model-ready"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert_eq!(resolver.call_count(), 0);
    assert_eq!(requests.lock().expect("recording lock").len(), 0);
}

#[tokio::test]
async fn onnx_nodes_with_ready_environment_ref_fail_closed_before_adapter() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolver = Arc::new(CountingDependencyResolver::new());
    let (executor, extensions) = test_executor(adapter, resolver.clone());

    let mut inputs = HashMap::new();
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({"model_id": "pumas://models/model-onnx"}),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "environment_ref".to_string(),
        serde_json::json!({
            "state": "ready",
            "env_id": "venv:onnx"
        }),
    );

    let error = executor
        .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect_err("ready environment_ref must not preserve legacy adapter launch");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("dependency_preflight_retired"));
            assert!(message.contains("\"environment_ref_gate\":true"));
            assert!(message.contains("pumas://models/model-onnx"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert_eq!(resolver.call_count(), 0);
    assert_eq!(requests.lock().expect("recording lock").len(), 0);
}

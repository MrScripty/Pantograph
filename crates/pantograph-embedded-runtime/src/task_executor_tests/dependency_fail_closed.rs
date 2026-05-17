use super::*;

#[tokio::test]
async fn onnx_nodes_fail_fast_when_environment_ref_is_not_ready() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: HashMap::new(),
    });
    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: make_requirements(DependencyValidationState::Resolved),
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let (executor, extensions) = test_executor(adapter, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model-ready"),
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

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: make_requirements(DependencyValidationState::Resolved),
        status: make_status(DependencyState::Unresolved, Some("no_dependency_bindings")),
        model_ref: None,
    });
    let (executor, extensions) = test_executor(adapter, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/external/tiny-tts.onnx"),
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
            assert!(message.contains("no_dependency_bindings"));
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

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: make_requirements(DependencyValidationState::Resolved),
        status: make_missing_binding_status("requirements_missing"),
        model_ref: None,
    });
    let (executor, extensions) = test_executor(adapter, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/external/tiny-tts.onnx"),
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
            assert!(message.contains("requirements_missing"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert_eq!(requests.lock().expect("recording lock").len(), 0);
}

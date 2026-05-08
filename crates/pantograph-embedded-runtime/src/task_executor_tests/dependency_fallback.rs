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
async fn python_nodes_allow_execution_when_no_dependency_bindings_are_available() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolved_model_ref = ModelRefV2 {
        contract_version: 2,
        engine: "onnx-runtime".to_string(),
        model_id: "audio/imported/tiny-tts".to_string(),
        model_path: "/tmp/external/tiny-tts.onnx".to_string(),
        task_type_primary: "text-to-audio".to_string(),
        dependency_bindings: Vec::new(),
        dependency_requirements_id: Some("requirements-onnx".to_string()),
    };

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: make_requirements(DependencyValidationState::Resolved),
        status: make_status(DependencyState::Unresolved, Some("no_dependency_bindings")),
        model_ref: Some(resolved_model_ref),
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

    let outputs = executor
        .execute_task("onnx-inference-2", inputs, &Context::new(), &extensions)
        .await
        .expect("python nodes should execute without dependency bindings");
    assert_eq!(
        outputs.get("audio"),
        Some(&serde_json::json!("base64-audio"))
    );

    let recorded = requests.lock().expect("recording lock");
    assert_eq!(recorded.len(), 1);
    let request = &recorded[0];
    assert_eq!(request.node_type, "onnx-inference");
    assert!(request.env_ids.is_empty());
    assert_eq!(
        request
            .inputs
            .get("model_ref")
            .and_then(|value| value.get("modelPath"))
            .and_then(|value| value.as_str()),
        Some("/tmp/external/tiny-tts.onnx")
    );
}

#[tokio::test]
async fn python_nodes_allow_execution_when_bindings_are_missing_only_runtime_packages() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolved_model_ref = ModelRefV2 {
        contract_version: 2,
        engine: "onnx-runtime".to_string(),
        model_id: "audio/imported/tiny-tts".to_string(),
        model_path: "/tmp/external/tiny-tts.onnx".to_string(),
        task_type_primary: "text-to-audio".to_string(),
        dependency_bindings: Vec::new(),
        dependency_requirements_id: Some("requirements-onnx".to_string()),
    };

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: make_requirements(DependencyValidationState::Resolved),
        status: make_missing_binding_status("requirements_missing"),
        model_ref: Some(resolved_model_ref),
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

    let outputs = executor
        .execute_task("onnx-inference-3", inputs, &Context::new(), &extensions)
        .await
        .expect("python nodes should execute when only runtime packages are missing");
    assert_eq!(
        outputs.get("audio"),
        Some(&serde_json::json!("base64-audio"))
    );

    let recorded = requests.lock().expect("recording lock");
    assert_eq!(recorded.len(), 1);
    let request = &recorded[0];
    assert_eq!(request.node_type, "onnx-inference");
    assert!(request.env_ids.is_empty());
    assert_eq!(
        request
            .inputs
            .get("model_ref")
            .and_then(|value| value.get("engine"))
            .and_then(|value| value.as_str()),
        Some("onnx-runtime")
    );
}

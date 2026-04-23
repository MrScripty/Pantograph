use super::*;

#[tokio::test]
async fn python_nodes_fail_fast_when_environment_ref_is_not_ready() {
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
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "environment_ref".to_string(),
        serde_json::json!({
            "state": "missing",
            "env_id": "env:test"
        }),
    );

    let err = executor
        .execute_task("pytorch-inference-1", inputs, &Context::new(), &extensions)
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
    adapter_response.insert("image".to_string(), serde_json::json!("base64-image"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolved_model_ref = ModelRefV2 {
        contract_version: 2,
        engine: "pytorch".to_string(),
        model_id: "diffusion/imported/tiny-sd-turbo".to_string(),
        model_path: "/tmp/external/tiny-sd-turbo".to_string(),
        task_type_primary: "text-to-image".to_string(),
        dependency_bindings: Vec::new(),
        dependency_requirements_id: Some("requirements-diffusion".to_string()),
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
        serde_json::json!("/tmp/external/tiny-sd-turbo"),
    );
    inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));
    inputs.insert(
        "prompt".to_string(),
        serde_json::json!("paper lantern in the rain"),
    );

    let outputs = executor
        .execute_task(
            "diffusion-inference-2",
            inputs,
            &Context::new(),
            &extensions,
        )
        .await
        .expect("python nodes should execute without dependency bindings");
    assert_eq!(
        outputs.get("image"),
        Some(&serde_json::json!("base64-image"))
    );

    let recorded = requests.lock().expect("recording lock");
    assert_eq!(recorded.len(), 1);
    let request = &recorded[0];
    assert_eq!(request.node_type, "diffusion-inference");
    assert!(request.env_ids.is_empty());
    assert_eq!(
        request
            .inputs
            .get("model_ref")
            .and_then(|value| value.get("modelPath"))
            .and_then(|value| value.as_str()),
        Some("/tmp/external/tiny-sd-turbo")
    );
}

#[tokio::test]
async fn python_nodes_allow_execution_when_bindings_are_missing_only_runtime_packages() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("image".to_string(), serde_json::json!("base64-image"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolved_model_ref = ModelRefV2 {
        contract_version: 2,
        engine: "diffusers".to_string(),
        model_id: "diffusion/cc-nms/tiny-sd-turbo".to_string(),
        model_path: "/tmp/external/tiny-sd-turbo".to_string(),
        task_type_primary: "text-to-image".to_string(),
        dependency_bindings: Vec::new(),
        dependency_requirements_id: Some("requirements-diffusion".to_string()),
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
        serde_json::json!("/tmp/external/tiny-sd-turbo"),
    );
    inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));
    inputs.insert(
        "prompt".to_string(),
        serde_json::json!("paper lantern in the rain"),
    );

    let outputs = executor
        .execute_task(
            "diffusion-inference-3",
            inputs,
            &Context::new(),
            &extensions,
        )
        .await
        .expect("python nodes should execute when only runtime packages are missing");
    assert_eq!(
        outputs.get("image"),
        Some(&serde_json::json!("base64-image"))
    );

    let recorded = requests.lock().expect("recording lock");
    assert_eq!(recorded.len(), 1);
    let request = &recorded[0];
    assert_eq!(request.node_type, "diffusion-inference");
    assert!(request.env_ids.is_empty());
    assert_eq!(
        request
            .inputs
            .get("model_ref")
            .and_then(|value| value.get("engine"))
            .and_then(|value| value.as_str()),
        Some("diffusers")
    );
}

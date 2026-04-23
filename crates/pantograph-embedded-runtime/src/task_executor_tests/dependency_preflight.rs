use super::*;

#[tokio::test]
async fn python_nodes_block_when_dependency_preflight_is_not_ready() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: HashMap::new(),
    });
    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: make_requirements(DependencyValidationState::InvalidProfile),
        status: make_status(DependencyState::Invalid, Some("invalid_profile")),
        model_ref: None,
    });
    let (executor, extensions) = test_executor(adapter, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model-not-ready"),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));

    let err = executor
        .execute_task("pytorch-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect_err("preflight should block non-ready dependency state");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Dependency preflight blocked execution"));
            assert!(message.contains("invalid_profile"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert_eq!(requests.lock().expect("recording lock").len(), 0);
}

#[tokio::test]
async fn python_nodes_receive_resolved_model_ref_and_env_ids_after_preflight() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("response".to_string(), serde_json::json!("ok"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolved_model_ref = ModelRefV2 {
        contract_version: 2,
        engine: "pytorch".to_string(),
        model_id: "model-a".to_string(),
        model_path: "/tmp/model-ready".to_string(),
        task_type_primary: "text-generation".to_string(),
        dependency_bindings: vec![ModelDependencyBinding {
            binding_id: "binding-a".to_string(),
            profile_id: "profile-a".to_string(),
            profile_version: 1,
            profile_hash: Some("hash".to_string()),
            backend_key: Some("pytorch".to_string()),
            platform_selector: Some("linux-x86_64".to_string()),
            environment_kind: Some("python".to_string()),
            env_id: Some("venv:test".to_string()),
            python_executable_override: None,
            validation_state: DependencyValidationState::Resolved,
            validation_errors: Vec::new(),
            requirements: Vec::new(),
        }],
        dependency_requirements_id: Some("requirements-test".to_string()),
    };

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: make_requirements(DependencyValidationState::Resolved),
        status: make_status(DependencyState::Ready, None),
        model_ref: Some(resolved_model_ref),
    });
    let (executor, extensions) = test_executor(adapter, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model-ready"),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));

    let outputs = executor
        .execute_task("pytorch-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect("ready preflight should allow adapter execution");
    assert_eq!(outputs.get("response"), Some(&serde_json::json!("ok")));

    let recorded = requests.lock().expect("recording lock");
    assert_eq!(recorded.len(), 1);
    let request = &recorded[0];
    assert_eq!(request.node_type, "pytorch-inference");
    assert_eq!(request.env_ids, vec!["venv:test".to_string()]);
    assert!(request.inputs.contains_key("model_ref"));
}

#[tokio::test]
async fn diffusion_nodes_route_through_python_adapter_with_preflight() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("image".to_string(), serde_json::json!("base64-image"));
    adapter_response.insert("seed_used".to_string(), serde_json::json!(1234));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolved_model_ref = ModelRefV2 {
        contract_version: 2,
        engine: "pytorch".to_string(),
        model_id: "qwen-image".to_string(),
        model_path: "/tmp/qwen-image".to_string(),
        task_type_primary: "text-to-image".to_string(),
        dependency_bindings: vec![ModelDependencyBinding {
            binding_id: "binding-diffusion".to_string(),
            profile_id: "profile-diffusion".to_string(),
            profile_version: 1,
            profile_hash: Some("hash".to_string()),
            backend_key: Some("pytorch".to_string()),
            platform_selector: Some("linux-x86_64".to_string()),
            environment_kind: Some("python".to_string()),
            env_id: Some("venv:diffusion".to_string()),
            python_executable_override: None,
            validation_state: DependencyValidationState::Resolved,
            validation_errors: Vec::new(),
            requirements: Vec::new(),
        }],
        dependency_requirements_id: Some("requirements-diffusion".to_string()),
    };

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: make_requirements(DependencyValidationState::Resolved),
        status: make_status(DependencyState::Ready, None),
        model_ref: Some(resolved_model_ref),
    });
    let (executor, extensions) = test_executor(adapter, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/qwen-image"),
    );
    inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));
    inputs.insert(
        "prompt".to_string(),
        serde_json::json!("paper lantern in the rain"),
    );

    let outputs = executor
        .execute_task(
            "diffusion-inference-1",
            inputs,
            &Context::new(),
            &extensions,
        )
        .await
        .expect("diffusion preflight should allow adapter execution");
    assert_eq!(
        outputs.get("image"),
        Some(&serde_json::json!("base64-image"))
    );
    assert_eq!(outputs.get("seed_used"), Some(&serde_json::json!(1234)));

    let recorded = requests.lock().expect("recording lock");
    assert_eq!(recorded.len(), 1);
    let request = &recorded[0];
    assert_eq!(request.node_type, "diffusion-inference");
    assert_eq!(request.env_ids, vec!["venv:diffusion".to_string()]);
    assert_eq!(
        request
            .inputs
            .get("model_ref")
            .and_then(|value| value.get("taskTypePrimary"))
            .and_then(|value| value.as_str()),
        Some("text-to-image")
    );
    assert!(request.inputs.contains_key("model_ref"));
}

#[tokio::test]
async fn onnx_nodes_route_through_python_adapter_with_preflight() {
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
        model_id: "kitten-tts".to_string(),
        model_path: "/tmp/model.onnx".to_string(),
        task_type_primary: "text-to-audio".to_string(),
        dependency_bindings: vec![ModelDependencyBinding {
            binding_id: "binding-onnx".to_string(),
            profile_id: "profile-onnx".to_string(),
            profile_version: 1,
            profile_hash: Some("hash".to_string()),
            backend_key: Some("onnx-runtime".to_string()),
            platform_selector: Some("linux-x86_64".to_string()),
            environment_kind: Some("python".to_string()),
            env_id: Some("venv:onnx".to_string()),
            python_executable_override: None,
            validation_state: DependencyValidationState::Resolved,
            validation_errors: Vec::new(),
            requirements: Vec::new(),
        }],
        dependency_requirements_id: Some("requirements-onnx".to_string()),
    };

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("onnx-runtime".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: Some(resolved_model_ref),
    });
    let (executor, extensions) = test_executor(adapter, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.onnx"),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));

    let outputs = executor
        .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect("onnx preflight should allow adapter execution");
    assert_eq!(
        outputs.get("audio"),
        Some(&serde_json::json!("base64-audio"))
    );

    let recorded = requests.lock().expect("recording lock");
    assert_eq!(recorded.len(), 1);
    let request = &recorded[0];
    assert_eq!(request.node_type, "onnx-inference");
    assert_eq!(request.env_ids, vec!["venv:onnx".to_string()]);
    assert!(request.inputs.contains_key("model_ref"));
}

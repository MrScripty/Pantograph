use super::*;
use node_engine::{
    DependencyState, DependencyValidationState, ExecutorExtensions, ModelDependencyBinding,
    ModelDependencyBindingStatus, ModelDependencyInstallResult, ModelDependencyRequest,
    ModelDependencyRequirements, ModelDependencyResolver, ModelDependencyStatus, ModelRefV2,
    VecEventSink, WorkflowEvent, extension_keys,
};
use std::sync::Mutex;

#[test]
fn canonical_backend_key_accepts_llama_cpp_alias() {
    assert_eq!(
        TauriTaskExecutor::canonical_backend_key(Some("llama_cpp")),
        Some("llamacpp".to_string())
    );
}

#[derive(Clone)]
struct StubDependencyResolver {
    requirements: ModelDependencyRequirements,
    status: ModelDependencyStatus,
    model_ref: Option<ModelRefV2>,
}

#[async_trait]
impl ModelDependencyResolver for StubDependencyResolver {
    async fn resolve_model_dependency_requirements(
        &self,
        _request: ModelDependencyRequest,
    ) -> std::result::Result<ModelDependencyRequirements, String> {
        Ok(self.requirements.clone())
    }

    async fn check_dependencies(
        &self,
        _request: ModelDependencyRequest,
    ) -> std::result::Result<ModelDependencyStatus, String> {
        Ok(self.status.clone())
    }

    async fn install_dependencies(
        &self,
        _request: ModelDependencyRequest,
    ) -> std::result::Result<ModelDependencyInstallResult, String> {
        Err("install not used in task-executor tests".to_string())
    }

    async fn resolve_model_ref(
        &self,
        _request: ModelDependencyRequest,
        _requirements: Option<ModelDependencyRequirements>,
    ) -> std::result::Result<Option<ModelRefV2>, String> {
        Ok(self.model_ref.clone())
    }
}

struct RecordingPythonAdapter {
    requests: Arc<Mutex<Vec<PythonNodeExecutionRequest>>>,
    response: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl PythonRuntimeAdapter for RecordingPythonAdapter {
    async fn execute_node(
        &self,
        request: PythonNodeExecutionRequest,
    ) -> std::result::Result<HashMap<String, serde_json::Value>, String> {
        self.requests.lock().expect("recording lock").push(request);
        Ok(self.response.clone())
    }
}

fn test_executor(
    adapter: Arc<dyn PythonRuntimeAdapter>,
    resolver: Arc<dyn ModelDependencyResolver>,
) -> (TauriTaskExecutor, ExecutorExtensions) {
    let executor = TauriTaskExecutor::with_python_runtime(None, adapter);

    let mut extensions = ExecutorExtensions::new();
    extensions.set(extension_keys::MODEL_DEPENDENCY_RESOLVER, resolver);
    (executor, extensions)
}

fn install_python_runtime_recorder(
    extensions: &mut ExecutorExtensions,
) -> Arc<PythonRuntimeExecutionRecorder> {
    let recorder = Arc::new(PythonRuntimeExecutionRecorder::default());
    extensions.set(
        runtime_extension_keys::PYTHON_RUNTIME_EXECUTION_RECORDER,
        recorder.clone(),
    );
    recorder
}

fn make_requirements(state: DependencyValidationState) -> ModelDependencyRequirements {
    ModelDependencyRequirements {
        model_id: "model-a".to_string(),
        platform_key: "linux-x86_64".to_string(),
        backend_key: Some("pytorch".to_string()),
        dependency_contract_version: 1,
        validation_state: state,
        validation_errors: Vec::new(),
        bindings: Vec::new(),
        selected_binding_ids: Vec::new(),
    }
}

fn make_status(state: DependencyState, code: Option<&str>) -> ModelDependencyStatus {
    ModelDependencyStatus {
        state,
        code: code.map(|s| s.to_string()),
        message: code.map(|s| format!("status={}", s)),
        requirements: make_requirements(DependencyValidationState::Resolved),
        bindings: Vec::new(),
        checked_at: None,
    }
}

fn make_missing_binding_status(binding_code: &str) -> ModelDependencyStatus {
    ModelDependencyStatus {
        state: DependencyState::Missing,
        code: None,
        message: None,
        requirements: make_requirements(DependencyValidationState::Resolved),
        bindings: vec![ModelDependencyBindingStatus {
            binding_id: "binding-a".to_string(),
            env_id: Some("python-venv:test".to_string()),
            state: DependencyState::Missing,
            code: Some(binding_code.to_string()),
            message: None,
            missing_requirements: vec!["diffusers".to_string()],
            installed_requirements: Vec::new(),
            failed_requirements: Vec::new(),
        }],
        checked_at: None,
    }
}

fn create_test_env() -> tempfile::TempDir {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    std::fs::create_dir_all(temp_dir.path().join("launcher-data/metadata")).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("launcher-data/cache")).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("launcher-data/logs")).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("shared-resources/models")).unwrap();
    temp_dir
}

fn write_test_diffusers_bundle(root: &std::path::Path) {
    std::fs::create_dir_all(root.join("scheduler")).unwrap();
    std::fs::create_dir_all(root.join("text_encoder")).unwrap();
    std::fs::create_dir_all(root.join("tokenizer")).unwrap();
    std::fs::create_dir_all(root.join("unet")).unwrap();
    std::fs::create_dir_all(root.join("vae")).unwrap();
    std::fs::write(
        root.join("model_index.json"),
        serde_json::json!({
            "_class_name": "StableDiffusionPipeline",
            "scheduler": ["diffusers", "EulerDiscreteScheduler"],
            "text_encoder": ["transformers", "CLIPTextModel"],
            "tokenizer": ["transformers", "CLIPTokenizer"],
            "unet": ["diffusers", "UNet2DConditionModel"],
            "vae": ["diffusers", "AutoencoderKL"]
        })
        .to_string(),
    )
    .unwrap();
}

fn write_imported_diffusion_metadata(
    model_dir: &std::path::Path,
    model_id: &str,
    entry_path: &std::path::Path,
) {
    std::fs::create_dir_all(model_dir).unwrap();
    std::fs::write(
        model_dir.join("metadata.json"),
        serde_json::json!({
            "schema_version": 2,
            "model_id": model_id,
            "family": "imported",
            "model_type": "diffusion",
            "official_name": "test-bundle",
            "cleaned_name": "test-bundle",
            "source_path": entry_path.display().to_string(),
            "entry_path": entry_path.display().to_string(),
            "storage_kind": "external_reference",
            "bundle_format": "diffusers_directory",
            "pipeline_class": "StableDiffusionPipeline",
            "import_state": "ready",
            "validation_state": "valid",
            "pipeline_tag": "text-to-image",
            "task_type_primary": "text-to-image",
            "input_modalities": ["text"],
            "output_modalities": ["image"],
            "task_classification_source": "external-diffusers-import",
            "task_classification_confidence": 1.0,
            "model_type_resolution_source": "external-diffusers-import",
            "model_type_resolution_confidence": 1.0,
            "recommended_backend": "diffusers",
            "runtime_engine_hints": ["diffusers", "pytorch"]
        })
        .to_string(),
    )
    .unwrap();
}

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

#[tokio::test]
async fn python_runtime_recorder_tracks_backend_and_environment_identity() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests,
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
    let (executor, mut extensions) = test_executor(adapter, resolver);
    let recorder = install_python_runtime_recorder(&mut extensions);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.onnx"),
    );
    inputs.insert("backend_key".to_string(), serde_json::json!("onnxruntime"));

    executor
        .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect("onnx execution should succeed");

    let metadata = recorder.snapshot().expect("python runtime metadata");
    assert_eq!(
        metadata.snapshot.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
    assert_eq!(
        metadata.snapshot.runtime_instance_id.as_deref(),
        Some("python-runtime:onnx-runtime:venv_onnx")
    );
    assert_eq!(metadata.snapshot.runtime_reused, Some(false));
    assert_eq!(
        metadata.snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
    assert!(!metadata.snapshot.active);
    assert_eq!(metadata.model_target.as_deref(), Some("/tmp/model.onnx"));
    assert_eq!(metadata.health_assessment, None);
}

#[tokio::test]
async fn python_runtime_recorder_keeps_process_runtime_non_reused_across_runs() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests,
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
    let (executor, mut extensions) = test_executor(adapter, resolver);
    let recorder = install_python_runtime_recorder(&mut extensions);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.onnx"),
    );
    inputs.insert("backend_key".to_string(), serde_json::json!("onnxruntime"));

    executor
        .execute_task(
            "onnx-inference-1",
            inputs.clone(),
            &Context::new(),
            &extensions,
        )
        .await
        .expect("first onnx execution should succeed");
    executor
        .execute_task("onnx-inference-2", inputs, &Context::new(), &extensions)
        .await
        .expect("second onnx execution should succeed");

    let metadata = recorder.snapshot().expect("python runtime metadata");
    assert_eq!(metadata.snapshot.runtime_reused, Some(false));
    assert_eq!(
        metadata.snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
    assert!(!metadata.snapshot.active);
    assert_eq!(metadata.health_assessment, None);
}

struct FailingPythonAdapter;

#[async_trait]
impl PythonRuntimeAdapter for FailingPythonAdapter {
    async fn execute_node(
        &self,
        _request: PythonNodeExecutionRequest,
    ) -> std::result::Result<HashMap<String, serde_json::Value>, String> {
        Err("python sidecar crashed".to_string())
    }
}

#[tokio::test]
async fn python_runtime_recorder_progresses_failed_execution_health_state() {
    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("pytorch".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let executor = TauriTaskExecutor::with_python_runtime(None, Arc::new(FailingPythonAdapter));
    let mut extensions = ExecutorExtensions::new();
    extensions.set(extension_keys::MODEL_DEPENDENCY_RESOLVER, resolver);
    let recorder = install_python_runtime_recorder(&mut extensions);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.safetensors"),
    );
    inputs.insert("backend_key".to_string(), serde_json::json!("pytorch"));
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));

    for _ in 0..3 {
        let error = executor
            .execute_task(
                "pytorch-inference-1",
                inputs.clone(),
                &Context::new(),
                &extensions,
            )
            .await
            .expect_err("python execution should fail");

        match error {
            NodeEngineError::ExecutionFailed(message) => {
                assert!(message.contains("python sidecar crashed"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    let snapshots = recorder.snapshots();
    assert_eq!(snapshots.len(), 3);

    let first_assessment = snapshots[0]
        .health_assessment
        .clone()
        .expect("first failed execution health assessment");
    assert!(first_assessment.healthy);
    assert_eq!(first_assessment.consecutive_failures, 1);
    assert_eq!(
        first_assessment.state,
        crate::runtime_health::RuntimeHealthState::Degraded {
            reason: "python sidecar crashed".to_string(),
        }
    );

    let second_assessment = snapshots[1]
        .health_assessment
        .clone()
        .expect("second failed execution health assessment");
    assert!(second_assessment.healthy);
    assert_eq!(second_assessment.consecutive_failures, 2);
    assert_eq!(
        second_assessment.state,
        crate::runtime_health::RuntimeHealthState::Degraded {
            reason: "python sidecar crashed".to_string(),
        }
    );

    let third = snapshots.last().expect("third runtime metadata");
    assert!(!third.snapshot.active);
    assert_eq!(
        third.snapshot.last_error.as_deref(),
        Some("python sidecar crashed")
    );
    let third_assessment = third
        .health_assessment
        .clone()
        .expect("third failed execution health assessment");
    assert!(!third_assessment.healthy);
    assert_eq!(
        third_assessment.error.as_deref(),
        Some("python sidecar crashed")
    );
    assert_eq!(third_assessment.consecutive_failures, 3);
    assert_eq!(
        third_assessment.state,
        crate::runtime_health::RuntimeHealthState::Unhealthy {
            reason: "python sidecar crashed".to_string(),
        }
    );
}

#[tokio::test]
async fn onnx_nodes_apply_inference_setting_defaults_before_python_execution() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("onnx-runtime".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let (executor, extensions) = test_executor(adapter, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.onnx"),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "voice", "default": "expr-voice-5-m"},
            {"key": "speed", "default": 0.9},
            {"key": "clean_text", "default": true},
            {"key": "sample_rate", "default": 24000}
        ]),
    );

    let _ = executor
        .execute_task(
            "onnx-inference-defaults",
            inputs,
            &Context::new(),
            &extensions,
        )
        .await
        .expect("onnx execution with inference defaults should succeed");

    let recorded = requests.lock().expect("recording lock");
    assert_eq!(recorded.len(), 1);
    let request = &recorded[0];
    assert_eq!(
        request.inputs.get("voice"),
        Some(&serde_json::json!("expr-voice-5-m"))
    );
    assert_eq!(request.inputs.get("speed"), Some(&serde_json::json!(0.9)));
    assert_eq!(
        request.inputs.get("clean_text"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        request.inputs.get("sample_rate"),
        Some(&serde_json::json!(24000))
    );
}

#[tokio::test]
async fn python_nodes_emit_stream_events_when_event_sink_extension_exists() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("final-audio"));
    adapter_response.insert(
        "stream".to_string(),
        serde_json::json!([
            {
                "type": "audio_chunk",
                "mode": "append",
                "audio_base64": "chunk-1",
                "mime_type": "audio/wav",
                "sequence": 0,
                "is_final": false
            },
            {
                "type": "audio_chunk",
                "mode": "append",
                "audio_base64": "chunk-2",
                "mime_type": "audio/wav",
                "sequence": 1,
                "is_final": true
            }
        ]),
    );
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("onnx-runtime".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let (executor, mut extensions) = test_executor(adapter, resolver);
    let sink = Arc::new(VecEventSink::new());
    extensions.set(
        runtime_extension_keys::EVENT_SINK,
        sink.clone() as Arc<dyn node_engine::EventSink>,
    );
    extensions.set(
        runtime_extension_keys::EXECUTION_ID,
        "exec-stream-test".to_string(),
    );

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.onnx"),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("stream this"));

    let _ = executor
        .execute_task(
            "onnx-inference-stream",
            inputs,
            &Context::new(),
            &extensions,
        )
        .await
        .expect("onnx stream execution should succeed");

    let events = sink.events();
    let stream_events: Vec<_> = events
        .into_iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskStream {
                task_id,
                execution_id,
                port,
                data,
                ..
            } => Some((task_id, execution_id, port, data)),
            _ => None,
        })
        .collect();

    assert_eq!(stream_events.len(), 2);
    assert_eq!(stream_events[0].0, "onnx-inference-stream");
    assert_eq!(stream_events[0].1, "exec-stream-test");
    assert_eq!(stream_events[0].2, "stream");
    assert_eq!(stream_events[0].3["audio_base64"], "chunk-1");
    assert_eq!(stream_events[0].3["sequence"], 0);
    assert_eq!(stream_events[0].3["is_final"], false);
    assert_eq!(stream_events[1].3["audio_base64"], "chunk-2");
    assert_eq!(stream_events[1].3["sequence"], 1);
    assert_eq!(stream_events[1].3["is_final"], true);
}

#[tokio::test]
async fn audio_generation_nodes_do_not_emit_buffered_stream_events_after_completion() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("final-audio"));
    adapter_response.insert(
        "stream".to_string(),
        serde_json::json!([
            {
                "type": "audio_chunk",
                "mode": "append",
                "audio_base64": "chunk-1",
                "mime_type": "audio/wav",
                "sequence": 0,
                "is_final": false
            }
        ]),
    );
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests,
        response: adapter_response,
    });

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("stable_audio".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let (executor, mut extensions) = test_executor(adapter, resolver);
    let sink = Arc::new(VecEventSink::new());
    extensions.set(
        runtime_extension_keys::EVENT_SINK,
        sink.clone() as Arc<dyn node_engine::EventSink>,
    );
    extensions.set(
        runtime_extension_keys::EXECUTION_ID,
        "exec-audio-batch-test".to_string(),
    );

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/stable-audio"),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("pad ambience"));

    let outputs = executor
        .execute_task(
            "audio-generation-batch",
            inputs,
            &Context::new(),
            &extensions,
        )
        .await
        .expect("audio-generation execution should succeed");

    assert_eq!(
        outputs.get("audio"),
        Some(&serde_json::json!("final-audio"))
    );
    let stream_events: Vec<_> = sink
        .events()
        .into_iter()
        .filter(|event| matches!(event, WorkflowEvent::TaskStream { .. }))
        .collect();
    assert!(stream_events.is_empty());
}

#[test]
fn apply_inference_setting_defaults_preserves_explicit_values() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "voice", "default": "expr-voice-5-m"},
            {"key": "speed", "default": 1.0}
        ]),
    );
    inputs.insert("voice".to_string(), serde_json::json!("custom-voice"));
    inputs.insert("speed".to_string(), serde_json::Value::Null);

    TauriTaskExecutor::apply_inference_setting_defaults(&mut inputs);

    assert_eq!(
        inputs.get("voice"),
        Some(&serde_json::json!("custom-voice"))
    );
    assert_eq!(inputs.get("speed"), Some(&serde_json::json!(1.0)));
}

#[test]
fn apply_inference_setting_defaults_resolves_option_object_values() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "voice", "default": {"label": "Leo", "value": "expr-voice-5-m"}},
            {"key": "speed", "default": 1.0}
        ]),
    );
    inputs.insert(
        "speed".to_string(),
        serde_json::json!({"label": "Fast", "value": 1.2}),
    );

    TauriTaskExecutor::apply_inference_setting_defaults(&mut inputs);

    assert_eq!(
        inputs.get("voice"),
        Some(&serde_json::json!("expr-voice-5-m"))
    );
    assert_eq!(inputs.get("speed"), Some(&serde_json::json!(1.2)));
}

#[test]
fn apply_inference_setting_defaults_resolves_allowed_value_labels() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {
                "key": "voice",
                "default": "Leo",
                "constraints": {
                    "allowed_values": [
                        {"label": "Leo", "value": "expr-voice-5-m"}
                    ]
                }
            },
            {"key": "speed", "default": 1.0}
        ]),
    );
    inputs.insert("speed".to_string(), serde_json::json!(1.2));

    TauriTaskExecutor::apply_inference_setting_defaults(&mut inputs);

    assert_eq!(
        inputs.get("voice"),
        Some(&serde_json::json!("expr-voice-5-m"))
    );
    assert_eq!(inputs.get("speed"), Some(&serde_json::json!(1.2)));
}

#[test]
fn collect_runtime_env_ids_includes_environment_ref() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "environment_ref".to_string(),
        serde_json::json!({
            "state": "ready",
            "env_id": "env:primary",
            "env_ids": ["env:extra"]
        }),
    );
    inputs.insert(
        "model_ref".to_string(),
        serde_json::json!({
            "dependencyBindings": [
                {"envId": "env:primary"},
                {"envId": "env:secondary"}
            ]
        }),
    );

    let env_ids = TauriTaskExecutor::collect_runtime_env_ids(&inputs);
    assert_eq!(
        env_ids,
        vec![
            "env:extra".to_string(),
            "env:primary".to_string(),
            "env:secondary".to_string(),
        ]
    );
}

#[test]
fn stable_hash_hex_is_deterministic() {
    let one = TauriTaskExecutor::stable_hash_hex("abc|123");
    let two = TauriTaskExecutor::stable_hash_hex("abc|123");
    let three = TauriTaskExecutor::stable_hash_hex("abc|124");
    assert_eq!(one, two);
    assert_ne!(one, three);
    assert_eq!(one.len(), 16);
}

#[test]
fn build_model_dependency_request_normalizes_backend_aliases() {
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("onnx-runtime"));

    let request = TauriTaskExecutor::build_model_dependency_request(
        "pytorch-inference",
        "/tmp/model",
        &inputs,
    );
    assert_eq!(request.backend_key.as_deref(), Some("onnx-runtime"));
}

#[test]
fn build_model_dependency_request_prefers_requirements_backend_when_input_missing() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "dependency_requirements".to_string(),
        serde_json::json!({
            "model_id": "model-a",
            "platform_key": "linux-x86_64",
            "backend_key": "torch",
            "dependency_contract_version": 1,
            "validation_state": "resolved",
            "validation_errors": [],
            "bindings": [],
            "selected_binding_ids": []
        }),
    );

    let request = TauriTaskExecutor::build_model_dependency_request(
        "pytorch-inference",
        "/tmp/model",
        &inputs,
    );
    assert_eq!(request.backend_key.as_deref(), Some("pytorch"));
}

#[test]
fn build_model_dependency_request_prefers_recommended_backend_for_diffusion() {
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("pytorch"));
    inputs.insert(
        "recommended_backend".to_string(),
        serde_json::json!("diffusers"),
    );

    let request = TauriTaskExecutor::build_model_dependency_request(
        "diffusion-inference",
        "/tmp/model",
        &inputs,
    );
    assert_eq!(request.backend_key.as_deref(), Some("diffusers"));
}

#[test]
fn build_model_dependency_request_leaves_diffusion_backend_unspecified_by_default() {
    let mut inputs = HashMap::new();
    inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));

    let request = TauriTaskExecutor::build_model_dependency_request(
        "diffusion-inference",
        "/tmp/model",
        &inputs,
    );
    assert_eq!(request.backend_key, None);
}

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

#[tokio::test]
async fn puma_lib_execution_rebinds_stale_model_path_from_model_id() {
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: Arc::new(Mutex::new(Vec::new())),
        response: HashMap::new(),
    });
    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: make_requirements(DependencyValidationState::Resolved),
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let (executor, mut extensions) = test_executor(adapter, resolver);

    let temp_dir = create_test_env();
    let bundle_root = temp_dir.path().join("external/tiny-sd-turbo");
    write_test_diffusers_bundle(&bundle_root);
    let model_dir = temp_dir
        .path()
        .join("shared-resources/models/diffusion/imported/test-bundle");
    write_imported_diffusion_metadata(&model_dir, "diffusion/imported/test-bundle", &bundle_root);

    let api = Arc::new(
        pumas_library::PumasApi::builder(temp_dir.path())
            .build()
            .await
            .expect("pumas api should initialize"),
    );
    extensions.set(extension_keys::PUMAS_API, api);

    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({
            "modelPath": "/stale/location/tiny-sd-turbo",
            "model_id": "diffusion/imported/test-bundle",
            "model_type": "diffusion",
            "task_type_primary": "text-to-image",
            "recommended_backend": "diffusers",
            "inference_settings": []
        }),
    );

    let outputs = executor
        .execute_task("puma-lib-1", inputs, &Context::new(), &extensions)
        .await
        .expect("puma-lib should resolve runtime path");

    assert_eq!(
        outputs.get("model_path"),
        Some(&serde_json::json!(bundle_root.display().to_string()))
    );
    assert_eq!(
        outputs.get("model_id"),
        Some(&serde_json::json!("diffusion/imported/test-bundle"))
    );
    assert_eq!(
        outputs.get("recommended_backend"),
        Some(&serde_json::json!("diffusers"))
    );
}

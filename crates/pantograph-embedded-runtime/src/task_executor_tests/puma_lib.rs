use super::*;
use workflow_nodes::setup::{PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};

#[tokio::test]
async fn puma_lib_execution_hydrates_model_ref_from_model_id_without_path_outputs() {
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
    api.rebuild_model_index()
        .await
        .expect("model index should rebuild");
    extensions.set(
        PUMAS_SELECTOR_ACCESS,
        Arc::new(PumasSelectorAccess::Owner(api)),
    );

    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({
            "modelPath": "/stale/location/tiny-sd-turbo",
            "model_id": "diffusion/imported/test-bundle",
            "model_type": "diffusion",
            "task_type_primary": "text-to-image",
            "recommended_backend": "llamacpp",
            "inference_settings": []
        }),
    );

    let outputs = executor
        .execute_task("puma-lib-1", inputs, &Context::new(), &extensions)
        .await
        .expect("puma-lib should resolve selector metadata");

    assert!(
        outputs.get("model_path").is_none(),
        "puma-lib must not emit executable path outputs"
    );
    assert_eq!(
        outputs.get("model_id"),
        Some(&serde_json::json!("diffusion/imported/test-bundle"))
    );
    assert_eq!(
        outputs.get("recommended_backend"),
        Some(&serde_json::json!("diffusers"))
    );
    assert_eq!(
        outputs
            .get("pumas_model_ref")
            .and_then(|value| value.get("model_id")),
        Some(&serde_json::json!("diffusion/imported/test-bundle"))
    );
    assert!(
        outputs
            .get("pumas_model_ref")
            .and_then(|value| value.get("model_path"))
            .is_none(),
        "puma-lib must not hide executable paths inside pumas_model_ref"
    );
    assert!(
        outputs.get("backend_key").is_none(),
        "puma-lib must not emit graph-visible backend-key aliases"
    );
    assert!(
        outputs.get("resolved_model_package_facts").is_none(),
        "puma-lib must not emit hidden package facts"
    );
    assert!(
        outputs.get("resolved_model_artifact_load_target").is_none(),
        "puma-lib must not emit hidden artifact load targets"
    );
}

#[tokio::test]
async fn puma_lib_execution_preserves_explicit_model_ref_without_model_path_output() {
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: Arc::new(Mutex::new(Vec::new())),
        response: HashMap::new(),
    });
    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: make_requirements(DependencyValidationState::Resolved),
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let (executor, extensions) = test_executor(adapter, resolver);

    let artifact_path = "/opt/Pumas-Library/shared-resources/models/vlm/qwen35/qwen3_6-27b-heretic-ara-gguf/Qwen3.6-27B-heretic-ara-Q4_K_M.gguf";
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({
            "modelName": "Qwen3.6-27B-heretic-ara-GGUF",
            "model_id": "vlm/qwen35/qwen3_6-27b-heretic-ara-gguf",
            "pumas_model_ref": {
                "model_ref_contract_version": 1,
                "model_id": "vlm/qwen35/qwen3_6-27b-heretic-ara-gguf",
                "selected_artifact_path": artifact_path
            },
            "recommended_backend": "llamacpp",
            "inference_settings": []
        }),
    );

    let outputs = executor
        .execute_task("puma-lib-1", inputs, &Context::new(), &extensions)
        .await
        .expect("puma-lib should preserve explicit model ref");

    assert!(
        outputs.get("model_path").is_none(),
        "puma-lib must not promote selected artifact paths to executable outputs"
    );
    assert_eq!(
        outputs
            .get("pumas_model_ref")
            .and_then(|value| value.get("selected_artifact_path")),
        Some(&serde_json::json!(artifact_path))
    );
    assert_eq!(
        outputs.get("model_id"),
        Some(&serde_json::json!(
            "vlm/qwen35/qwen3_6-27b-heretic-ara-gguf"
        ))
    );
    assert!(
        outputs.get("backend_key").is_none(),
        "puma-lib must not emit graph-visible backend-key aliases"
    );
}

#[tokio::test]
async fn puma_lib_execution_does_not_rebind_model_id_from_raw_pumas_api() {
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
    let model_id = "diffusion/imported/test-bundle";
    let model_dir = temp_dir
        .path()
        .join("shared-resources/models")
        .join(model_id);
    write_imported_diffusion_metadata(&model_dir, model_id, &bundle_root);

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
            "model_id": model_id,
            "model_type": "diffusion",
            "task_type_primary": "text-to-image",
            "recommended_backend": "llamacpp",
            "inference_settings": []
        }),
    );

    let outputs = executor
        .execute_task("puma-lib-1", inputs, &Context::new(), &extensions)
        .await
        .expect("puma-lib should preserve saved data without selector access");

    assert!(
        outputs.get("model_path").is_none(),
        "raw saved paths must not be emitted as executable outputs"
    );
    assert_eq!(
        outputs.get("recommended_backend"),
        Some(&serde_json::json!("llamacpp"))
    );
    assert_eq!(
        outputs
            .get("pumas_model_ref")
            .and_then(|value| value.get("model_id")),
        Some(&serde_json::json!(model_id))
    );
    assert!(
        outputs.get("backend_key").is_none(),
        "puma-lib must not emit graph-visible backend-key aliases"
    );
    assert!(
        outputs.get("resolved_model_package_facts").is_none(),
        "raw PUMAS_API alone must not rehydrate selected model facts"
    );
    assert!(
        outputs.get("resolved_model_artifact_load_target").is_none(),
        "raw PUMAS_API alone must not rehydrate selected artifact load targets"
    );
}

#[tokio::test]
async fn puma_lib_execution_hydrates_model_ref_from_selector_access_without_pumas_api() {
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
    let model_id = "diffusion/imported/test-bundle";
    let model_dir = temp_dir
        .path()
        .join("shared-resources/models")
        .join(model_id);
    write_imported_diffusion_metadata(&model_dir, model_id, &bundle_root);

    let api = pumas_library::PumasApi::builder(temp_dir.path())
        .build()
        .await
        .expect("pumas api should initialize");
    api.rebuild_model_index()
        .await
        .expect("model index should rebuild");
    let read_only =
        pumas_library::PumasReadOnlyLibrary::open(temp_dir.path().join("shared-resources/models"))
            .expect("read-only selector should open");
    extensions.set(
        PUMAS_SELECTOR_ACCESS,
        Arc::new(PumasSelectorAccess::ReadOnly(Arc::new(read_only))),
    );

    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({
            "modelPath": "/stale/location/tiny-sd-turbo",
            "model_id": model_id,
            "model_type": "diffusion",
            "task_type_primary": "text-to-image",
            "recommended_backend": "llamacpp",
            "inference_settings": []
        }),
    );

    let outputs = executor
        .execute_task("puma-lib-1", inputs, &Context::new(), &extensions)
        .await
        .expect("puma-lib should resolve selector metadata from selector access");

    assert!(
        outputs.get("model_path").is_none(),
        "puma-lib must not emit executable path outputs"
    );
    assert_eq!(outputs.get("model_id"), Some(&serde_json::json!(model_id)));
    assert_eq!(
        outputs.get("recommended_backend"),
        Some(&serde_json::json!("diffusers"))
    );
    assert_eq!(
        outputs
            .get("pumas_model_ref")
            .and_then(|value| value.get("model_id")),
        Some(&serde_json::json!(model_id))
    );
    assert!(
        outputs
            .get("pumas_model_ref")
            .and_then(|value| value.get("model_path"))
            .is_none(),
        "puma-lib must not hide executable paths inside pumas_model_ref"
    );
    assert!(
        outputs.get("resolved_model_package_facts").is_none(),
        "read-only selector rows must not be promoted to full package facts"
    );
    assert!(
        outputs.get("resolved_model_artifact_load_target").is_none(),
        "read-only selector rows must not be promoted to artifact load targets"
    );
}

#[tokio::test]
async fn puma_lib_execution_prefers_selected_detail_inference_settings() {
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
    let model_id = "diffusion/imported/test-bundle";
    let model_dir = temp_dir
        .path()
        .join("shared-resources/models")
        .join(model_id);
    write_imported_diffusion_metadata(&model_dir, model_id, &bundle_root);

    let api = Arc::new(
        pumas_library::PumasApi::builder(temp_dir.path())
            .build()
            .await
            .expect("pumas api should initialize"),
    );
    api.rebuild_model_index()
        .await
        .expect("model index should rebuild");
    extensions.set(
        PUMAS_SELECTOR_ACCESS,
        Arc::new(PumasSelectorAccess::Owner(api)),
    );

    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({
            "modelPath": "/stale/location/tiny-sd-turbo",
            "model_id": model_id,
            "model_type": "diffusion",
            "task_type_primary": "text-to-image",
            "recommended_backend": "llamacpp",
            "inference_settings": [
                {
                    "key": "stale_setting",
                    "label": "Stale Setting",
                    "param_type": "Number",
                    "default": 99
                }
            ]
        }),
    );

    let outputs = executor
        .execute_task("puma-lib-1", inputs, &Context::new(), &extensions)
        .await
        .expect("puma-lib should refresh inference settings from selected detail");

    let settings = outputs
        .get("inference_settings")
        .and_then(|value| value.as_array())
        .expect("inference settings should be an array");
    assert!(
        settings.iter().any(
            |setting| setting.get("key").and_then(|value| value.as_str())
                == Some("num_inference_steps")
        ),
        "selected-detail diffusion defaults should replace stale saved settings"
    );
    assert!(
        settings.iter().all(
            |setting| setting.get("key").and_then(|value| value.as_str()) != Some("stale_setting")
        ),
        "stale saved inference settings should not survive selected-detail hydration"
    );
}

#[tokio::test]
async fn puma_lib_execution_does_not_resolve_saved_model_name_without_model_id() {
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
    let model_id = "diffusion/imported/test-bundle";
    let model_dir = temp_dir
        .path()
        .join("shared-resources/models")
        .join(model_id);
    write_imported_diffusion_metadata(&model_dir, model_id, &bundle_root);

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
            "modelPath": "",
            "modelName": "Test Bundle",
            "recommended_backend": "diffusers",
            "inference_settings": []
        }),
    );

    let outputs = executor
        .execute_task("puma-lib-1", inputs, &Context::new(), &extensions)
        .await
        .expect("puma-lib should execute with saved data only");

    assert!(
        outputs.get("model_path").is_none(),
        "puma-lib must not emit empty path outputs"
    );
    assert!(outputs.get("model_id").is_none());
    assert!(outputs.get("model_type").is_none());
    assert!(outputs.get("task_type_primary").is_none());
    assert!(outputs.get("resolved_model_package_facts").is_none());
    assert!(outputs.get("resolved_model_artifact_load_target").is_none());
    assert!(outputs.get("pumas_model_ref").is_none());
}

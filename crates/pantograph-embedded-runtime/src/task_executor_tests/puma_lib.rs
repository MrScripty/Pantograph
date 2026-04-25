use super::*;

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

#[tokio::test]
async fn puma_lib_execution_resolves_saved_model_name_without_path_or_id() {
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
            "model_type": "diffusion",
            "task_type_primary": "text-to-image",
            "recommended_backend": "diffusers",
            "inference_settings": []
        }),
    );

    let outputs = executor
        .execute_task("puma-lib-1", inputs, &Context::new(), &extensions)
        .await
        .expect("puma-lib should resolve model name");

    assert_eq!(
        outputs.get("model_path"),
        Some(&serde_json::json!(bundle_root.display().to_string()))
    );
    assert_eq!(outputs.get("model_id"), Some(&serde_json::json!(model_id)));
    assert_eq!(
        outputs.get("task_type_primary"),
        Some(&serde_json::json!("text-to-image"))
    );
}

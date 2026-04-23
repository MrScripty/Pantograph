use super::*;
use node_engine::{DependencyOverrideScope, extension_keys};
use pumas_library::PumasApi;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

fn test_resolver() -> TauriModelDependencyResolver {
    TauriModelDependencyResolver::new(
        Arc::new(RwLock::new(node_engine::ExecutorExtensions::default())),
        PathBuf::from("."),
    )
}

fn create_test_env() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    std::fs::create_dir_all(temp_dir.path().join("launcher-data")).unwrap();
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

fn write_imported_diffusion_metadata(model_dir: &std::path::Path, entry_path: &std::path::Path) {
    std::fs::create_dir_all(model_dir).unwrap();
    std::fs::write(
        model_dir.join("metadata.json"),
        serde_json::json!({
            "schema_version": 2,
            "model_id": "diffusion/imported/test-bundle",
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

fn write_library_owned_file_model(
    model_dir: &std::path::Path,
    file_name: &str,
    file_size_bytes: usize,
) -> std::path::PathBuf {
    std::fs::create_dir_all(model_dir).unwrap();
    let model_file = model_dir.join(file_name);
    std::fs::write(&model_file, vec![0_u8; file_size_bytes]).unwrap();
    std::fs::write(
        model_dir.join("metadata.json"),
        serde_json::json!({
            "schema_version": 2,
            "model_id": "llm/imported/test-gguf",
            "family": "imported",
            "model_type": "llm",
            "official_name": "test-gguf",
            "cleaned_name": "test-gguf",
            "source_path": model_dir.display().to_string(),
            "storage_kind": "library_owned",
            "import_state": "ready",
            "validation_state": "valid",
            "task_type_primary": "text-generation",
            "recommended_backend": "llamacpp",
            "runtime_engine_hints": ["llamacpp"]
        })
        .to_string(),
    )
    .unwrap();
    model_file
}

async fn test_resolver_with_pumas(
    temp_dir: &TempDir,
) -> (TauriModelDependencyResolver, Arc<PumasApi>) {
    let api = Arc::new(PumasApi::builder(temp_dir.path()).build().await.unwrap());
    api.rebuild_model_index().await.unwrap();

    let shared_extensions = Arc::new(RwLock::new(node_engine::ExecutorExtensions::default()));
    shared_extensions
        .write()
        .await
        .set(extension_keys::PUMAS_API, api.clone());

    (
        TauriModelDependencyResolver::new(shared_extensions, PathBuf::from(".")),
        api,
    )
}

fn sample_request() -> ModelDependencyRequest {
    ModelDependencyRequest {
        node_type: "pytorch-inference".to_string(),
        model_path: "/tmp/model".to_string(),
        model_id: Some("model-id".to_string()),
        model_type: Some("diffusion".to_string()),
        task_type_primary: Some("text-to-image".to_string()),
        backend_key: Some("pytorch".to_string()),
        platform_context: Some(serde_json::json!({
            "os": "linux",
            "arch": "x86_64"
        })),
        selected_binding_ids: vec!["binding-b".to_string(), "binding-a".to_string()],
        dependency_override_patches: Vec::new(),
    }
}

#[test]
fn cache_key_is_deterministic_for_binding_order() {
    let mut left = sample_request();
    left.selected_binding_ids = vec!["binding-b".to_string(), "binding-a".to_string()];
    let mut right = sample_request();
    right.selected_binding_ids = vec!["binding-a".to_string(), "binding-b".to_string()];

    assert_eq!(
        TauriModelDependencyResolver::cache_key(&left),
        TauriModelDependencyResolver::cache_key(&right)
    );
}

#[test]
fn aggregate_state_for_empty_bindings_is_unresolved() {
    let rows: Vec<ModelDependencyBindingStatus> = Vec::new();
    assert_eq!(
        TauriModelDependencyResolver::aggregate_binding_runtime_state(&rows),
        DependencyState::Unresolved
    );
}

#[test]
fn normalized_backend_key_canonicalizes_aliases() {
    assert_eq!(
        descriptors::normalized_backend_key(&Some("onnx-runtime".to_string())),
        Some("onnx-runtime".to_string())
    );
    assert_eq!(
        descriptors::normalized_backend_key(&Some("llama_cpp".to_string())),
        Some("llamacpp".to_string())
    );
    assert_eq!(
        descriptors::normalized_backend_key(&Some("torch".to_string())),
        Some("pytorch".to_string())
    );
}

#[test]
fn infer_engine_defaults_diffusion_node_to_pytorch() {
    let engine = descriptors::infer_engine(None, "diffusion-inference", Some("diffusion"));
    assert_eq!(engine, "pytorch");
}

#[test]
fn infer_engine_uses_llamacpp_for_reranker_node() {
    let engine = descriptors::infer_engine(None, "reranker", Some("reranker"));
    assert_eq!(engine, "llamacpp");
}

#[test]
fn map_pipeline_tag_recognizes_reranking() {
    let task = descriptors::map_pipeline_tag_to_task("reranking");
    assert_eq!(task, "reranking");
}

#[tokio::test]
async fn resolve_without_api_returns_unknown_profile_requirements() {
    let resolver = test_resolver();
    let requirements = resolver
        .resolve_requirements_request(sample_request())
        .await
        .unwrap();

    assert_eq!(
        requirements.validation_state,
        DependencyValidationState::UnknownProfile
    );
    assert_eq!(
        requirements
            .validation_errors
            .first()
            .map(|e| e.code.as_str()),
        Some("pumas_api_unavailable")
    );
}

#[tokio::test]
async fn check_without_api_returns_unresolved_and_caches_status() {
    let resolver = test_resolver();
    let request = sample_request();
    let status = resolver.check_request(request.clone()).await.unwrap();

    assert_eq!(status.state, DependencyState::Unresolved);
    assert_eq!(status.code.as_deref(), Some("pumas_api_unavailable"));

    let cached = resolver.cached_status(&request).await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().state, DependencyState::Unresolved);
}

#[tokio::test]
async fn resolve_model_ref_filters_to_selected_bindings() {
    let resolver = test_resolver();
    let request = sample_request();
    let requirements = ModelDependencyRequirements {
        model_id: "model-id".to_string(),
        platform_key: "linux-x86_64".to_string(),
        backend_key: Some("pytorch".to_string()),
        dependency_contract_version: 1,
        validation_state: DependencyValidationState::Resolved,
        validation_errors: Vec::new(),
        bindings: vec![
            ModelDependencyBinding {
                binding_id: "binding-a".to_string(),
                profile_id: "profile-a".to_string(),
                profile_version: 1,
                profile_hash: None,
                backend_key: Some("pytorch".to_string()),
                platform_selector: Some("linux-x86_64".to_string()),
                environment_kind: Some("python".to_string()),
                env_id: Some("env-a".to_string()),
                python_executable_override: None,
                validation_state: DependencyValidationState::Resolved,
                validation_errors: Vec::new(),
                requirements: Vec::new(),
            },
            ModelDependencyBinding {
                binding_id: "binding-b".to_string(),
                profile_id: "profile-b".to_string(),
                profile_version: 1,
                profile_hash: None,
                backend_key: Some("pytorch".to_string()),
                platform_selector: Some("linux-x86_64".to_string()),
                environment_kind: Some("python".to_string()),
                env_id: Some("env-b".to_string()),
                python_executable_override: None,
                validation_state: DependencyValidationState::Resolved,
                validation_errors: Vec::new(),
                requirements: Vec::new(),
            },
        ],
        selected_binding_ids: vec!["binding-a".to_string()],
    };

    let model_ref = resolver
        .resolve_model_ref_request(request, Some(requirements))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(model_ref.contract_version, 2);
    assert_eq!(model_ref.dependency_bindings.len(), 1);
    assert_eq!(model_ref.dependency_bindings[0].binding_id, "binding-a");
}

#[tokio::test]
async fn resolve_descriptor_uses_entry_path_for_external_diffusers_bundle() {
    let temp_dir = create_test_env();
    let bundle_root = temp_dir.path().join("external/tiny-sd-turbo");
    write_test_diffusers_bundle(&bundle_root);

    let model_dir = temp_dir
        .path()
        .join("shared-resources/models/diffusion/imported/test-bundle");
    write_imported_diffusion_metadata(&model_dir, &bundle_root);

    let (resolver, api) = test_resolver_with_pumas(&temp_dir).await;
    let request = ModelDependencyRequest {
        node_type: "diffusion-inference".to_string(),
        model_path: bundle_root.display().to_string(),
        model_id: Some("diffusion/imported/test-bundle".to_string()),
        model_type: Some("diffusion".to_string()),
        task_type_primary: Some("text-to-image".to_string()),
        backend_key: Some("diffusers".to_string()),
        platform_context: Some(serde_json::json!({
            "os": "linux",
            "arch": "x86_64"
        })),
        selected_binding_ids: Vec::new(),
        dependency_override_patches: Vec::new(),
    };

    let descriptor = resolver
        .resolve_descriptor(&request, Some(&api))
        .await
        .expect("descriptor should resolve");

    assert_eq!(descriptor.model_id, "diffusion/imported/test-bundle");
    assert_eq!(descriptor.model_path, bundle_root.display().to_string());
    assert_eq!(descriptor.task_type_primary, "text-to-image");
    assert_eq!(descriptor.model_type.as_deref(), Some("diffusion"));
    assert!(descriptor.model_id_resolved);
}

#[tokio::test]
async fn resolve_descriptor_uses_primary_file_for_library_owned_file_model() {
    let temp_dir = create_test_env();
    let model_dir = temp_dir
        .path()
        .join("shared-resources/models/llm/imported/test-gguf");
    let model_file = write_library_owned_file_model(&model_dir, "model.gguf", 256);

    let (resolver, api) = test_resolver_with_pumas(&temp_dir).await;
    let request = ModelDependencyRequest {
        node_type: "pytorch-inference".to_string(),
        model_path: model_dir.display().to_string(),
        model_id: Some("llm/imported/test-gguf".to_string()),
        model_type: Some("llm".to_string()),
        task_type_primary: Some("text-generation".to_string()),
        backend_key: Some("llamacpp".to_string()),
        platform_context: Some(serde_json::json!({
            "os": "linux",
            "arch": "x86_64"
        })),
        selected_binding_ids: Vec::new(),
        dependency_override_patches: Vec::new(),
    };

    let descriptor = resolver
        .resolve_descriptor(&request, Some(&api))
        .await
        .expect("descriptor should resolve");

    assert_eq!(descriptor.model_id, "llm/imported/test-gguf");
    assert_eq!(descriptor.model_path, model_file.display().to_string());
    assert_eq!(descriptor.task_type_primary, "text-generation");
    assert_eq!(descriptor.model_type.as_deref(), Some("llm"));
    assert!(descriptor.model_id_resolved);
}

#[tokio::test]
async fn puma_lib_option_and_dependency_resolver_agree_on_primary_file_path() {
    let temp_dir = create_test_env();
    let model_dir = temp_dir
        .path()
        .join("shared-resources/models/llm/imported/test-gguf");
    let model_file = write_library_owned_file_model(&model_dir, "model.gguf", 256);

    let (resolver, api) = test_resolver_with_pumas(&temp_dir).await;

    let registry = node_engine::NodeRegistry::with_builtins();
    let mut extensions = node_engine::ExecutorExtensions::default();
    extensions.set(extension_keys::PUMAS_API, api.clone());
    let query = node_engine::PortOptionsQuery {
        search: Some("test-gguf".to_string()),
        limit: Some(10),
        offset: Some(0),
    };

    let options = registry
        .query_port_options("puma-lib", "model_path", &query, &extensions)
        .await
        .expect("puma-lib options should resolve");
    let option = options
        .options
        .into_iter()
        .find(|candidate| {
            candidate
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("id"))
                .and_then(|value| value.as_str())
                == Some("llm/imported/test-gguf")
        })
        .expect("test option should be present");
    let option_model_path = option
        .value
        .as_str()
        .expect("option value should be a string path")
        .to_string();

    assert_eq!(option_model_path, model_file.display().to_string());

    let request = ModelDependencyRequest {
        node_type: "pytorch-inference".to_string(),
        model_path: option_model_path.clone(),
        model_id: option
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("id"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        model_type: option
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("model_type"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        task_type_primary: option
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("task_type_primary"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        backend_key: Some("llamacpp".to_string()),
        platform_context: Some(serde_json::json!({
            "os": "linux",
            "arch": "x86_64"
        })),
        selected_binding_ids: Vec::new(),
        dependency_override_patches: Vec::new(),
    };

    let descriptor = resolver
        .resolve_descriptor(&request, Some(&api))
        .await
        .expect("descriptor should resolve");

    assert_eq!(descriptor.model_id, "llm/imported/test-gguf");
    assert_eq!(descriptor.model_path, option_model_path);
    assert_eq!(descriptor.model_path, model_file.display().to_string());
}

#[test]
fn descriptor_lookup_fallback_is_allowed_only_for_missing_descriptor_cases() {
    assert!(descriptors::descriptor_lookup_fallback_allowed(
        &pumas_library::PumasError::ModelNotFound {
            model_id: "missing".to_string()
        }
    ));
    assert!(descriptors::descriptor_lookup_fallback_allowed(
        &pumas_library::PumasError::NotFound {
            resource: "resolve_model_execution_descriptor".to_string()
        }
    ));
    assert!(!descriptors::descriptor_lookup_fallback_allowed(
        &pumas_library::PumasError::Validation {
            field: "validation_state".to_string(),
            message: "invalid".to_string()
        }
    ));
}

#[test]
fn override_patches_apply_binding_level_python_and_indexes() {
    let python_path = std::env::current_exe()
        .expect("current exe should exist")
        .to_string_lossy()
        .to_string();
    let requirements = ModelDependencyRequirements {
        model_id: "model-id".to_string(),
        platform_key: "linux-x86_64".to_string(),
        backend_key: Some("pytorch".to_string()),
        dependency_contract_version: 1,
        validation_state: DependencyValidationState::Resolved,
        validation_errors: Vec::new(),
        bindings: vec![ModelDependencyBinding {
            binding_id: "binding-a".to_string(),
            profile_id: "profile-a".to_string(),
            profile_version: 1,
            profile_hash: None,
            backend_key: Some("pytorch".to_string()),
            platform_selector: Some("linux-x86_64".to_string()),
            environment_kind: Some("python".to_string()),
            env_id: Some("env-a".to_string()),
            python_executable_override: None,
            validation_state: DependencyValidationState::Resolved,
            validation_errors: Vec::new(),
            requirements: vec![ModelDependencyRequirement {
                kind: "python_package".to_string(),
                name: "torch".to_string(),
                exact_pin: "==2.1.0".to_string(),
                index_url: None,
                extra_index_urls: Vec::new(),
                markers: None,
                python_requires: None,
                platform_constraints: Vec::new(),
                hashes: Vec::new(),
                source: None,
            }],
        }],
        selected_binding_ids: vec!["binding-a".to_string()],
    };

    let patch = DependencyOverridePatchV1 {
        contract_version: 1,
        binding_id: "binding-a".to_string(),
        scope: DependencyOverrideScope::Binding,
        requirement_name: None,
        fields: node_engine::DependencyOverrideFieldsV1 {
            python_executable: Some(python_path),
            index_url: Some("https://download.pytorch.org/whl/cu124".to_string()),
            extra_index_urls: Some(vec!["https://pypi.org/simple".to_string()]),
            wheel_source_path: None,
            package_source_override: None,
        },
        source: Some("user".to_string()),
        updated_at: Some("2026-02-28T00:00:00Z".to_string()),
    };

    let patched =
        TauriModelDependencyResolver::apply_dependency_override_patches(requirements, &[patch])
            .expect("patch should apply");
    let binding = &patched.bindings[0];
    assert!(binding.python_executable_override.is_some());
    assert_eq!(
        binding.requirements[0].index_url.as_deref(),
        Some("https://download.pytorch.org/whl/cu124")
    );
    assert_eq!(
        binding.requirements[0].extra_index_urls,
        vec!["https://pypi.org/simple".to_string()]
    );
}

#[test]
fn override_patches_reject_unknown_binding() {
    let requirements = ModelDependencyRequirements {
        model_id: "model-id".to_string(),
        platform_key: "linux-x86_64".to_string(),
        backend_key: Some("pytorch".to_string()),
        dependency_contract_version: 1,
        validation_state: DependencyValidationState::Resolved,
        validation_errors: Vec::new(),
        bindings: Vec::new(),
        selected_binding_ids: Vec::new(),
    };
    let patch = DependencyOverridePatchV1 {
        contract_version: 1,
        binding_id: "binding-missing".to_string(),
        scope: DependencyOverrideScope::Binding,
        requirement_name: None,
        fields: node_engine::DependencyOverrideFieldsV1 {
            python_executable: Some("python3".to_string()),
            index_url: None,
            extra_index_urls: None,
            wheel_source_path: None,
            package_source_override: None,
        },
        source: Some("user".to_string()),
        updated_at: None,
    };

    let err =
        TauriModelDependencyResolver::apply_dependency_override_patches(requirements, &[patch])
            .expect_err("unknown binding should fail");
    assert!(err.contains("unknown binding_id"));
}

#[test]
fn requirement_install_target_prefers_source_url_when_present() {
    let requirement = ModelDependencyRequirement {
        kind: "python_package".to_string(),
        name: "kittentts".to_string(),
        exact_pin: "0.8.1".to_string(),
        index_url: None,
        extra_index_urls: Vec::new(),
        markers: None,
        python_requires: None,
        platform_constraints: Vec::new(),
        hashes: Vec::new(),
        source: Some("https://example.invalid/kittentts-0.8.1.whl".to_string()),
    };

    assert_eq!(
        TauriModelDependencyResolver::requirement_install_target(&requirement),
        "https://example.invalid/kittentts-0.8.1.whl".to_string()
    );
}

#[test]
fn requirement_install_target_supports_wheel_source_path_override() {
    let requirement = ModelDependencyRequirement {
        kind: "python_package".to_string(),
        name: "kittentts".to_string(),
        exact_pin: "0.8.1".to_string(),
        index_url: None,
        extra_index_urls: Vec::new(),
        markers: None,
        python_requires: None,
        platform_constraints: Vec::new(),
        hashes: Vec::new(),
        source: Some("wheel_source_path=/tmp/kittentts-0.8.1.whl".to_string()),
    };

    assert_eq!(
        TauriModelDependencyResolver::requirement_install_target(&requirement),
        "/tmp/kittentts-0.8.1.whl".to_string()
    );
}

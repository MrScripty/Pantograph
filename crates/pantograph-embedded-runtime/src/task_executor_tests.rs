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

#[path = "task_executor_tests/dependency_fallback.rs"]
mod dependency_fallback;
#[path = "task_executor_tests/dependency_preflight.rs"]
mod dependency_preflight;
#[path = "task_executor_tests/input_helpers.rs"]
mod input_helpers;
#[path = "task_executor_tests/puma_lib.rs"]
mod puma_lib;
#[path = "task_executor_tests/recorder_stream.rs"]
mod recorder_stream;

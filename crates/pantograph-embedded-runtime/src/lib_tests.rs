use super::*;
use futures_util::stream;
use inference::backend::{
    BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use inference::{RerankRequest, RerankResponse};
use node_engine::ExecutorExtensions;
use pantograph_runtime_registry::{
    RuntimeRegistration, RuntimeRegistry, RuntimeRegistryError, RuntimeRegistrySnapshot,
    RuntimeRegistryStatus, RuntimeReservationRequest, RuntimeRetentionHint, RuntimeTransition,
};
use pantograph_workflow_service::{
    GraphEdge, GraphNode, Position, WorkflowCapabilitiesRequest,
    WorkflowExecutionSessionCloseRequest, WorkflowExecutionSessionCreateRequest,
    WorkflowExecutionSessionKeepAliveRequest, WorkflowExecutionSessionRetentionHint,
    WorkflowExecutionSessionRunRequest, WorkflowExecutionSessionRuntimeSelectionTarget,
    WorkflowExecutionSessionRuntimeUnloadCandidate, WorkflowExecutionSessionState,
    WorkflowExecutionSessionStatusRequest, WorkflowGraph, WorkflowGraphEditSessionCreateRequest,
    WorkflowHost, WorkflowOutputTarget, WorkflowPortBinding, WorkflowPreflightRequest,
    WorkflowRunOptions, WorkflowRunResponse, WorkflowRuntimeInstallState,
    WorkflowRuntimeRequirements, WorkflowRuntimeSourceKind, WorkflowSchedulerRuntimeWarmupDecision,
    WorkflowSchedulerRuntimeWarmupReason, WorkflowServiceError,
};
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::{RwLock, mpsc};

#[path = "lib_tests/data_graph_execution_tests.rs"]
mod data_graph_execution_tests;
#[path = "lib_tests/edit_session_execution_tests.rs"]
mod edit_session_execution_tests;
#[path = "lib_tests/graph_fixtures.rs"]
mod graph_fixtures;
#[path = "lib_tests/host_helper_tests.rs"]
mod host_helper_tests;
#[path = "lib_tests/runtime_lifecycle_capability_tests.rs"]
mod runtime_lifecycle_capability_tests;
#[path = "lib_tests/runtime_preflight_tests.rs"]
mod runtime_preflight_tests;
#[path = "lib_tests/session_checkpoint_capacity_tests.rs"]
mod session_checkpoint_capacity_tests;
#[path = "lib_tests/session_checkpoint_recovery_tests.rs"]
mod session_checkpoint_recovery_tests;
#[path = "lib_tests/session_execution_state_tests.rs"]
mod session_execution_state_tests;
#[path = "lib_tests/session_runtime_lifecycle_tests.rs"]
mod session_runtime_lifecycle_tests;
#[path = "lib_tests/workflow_run_execution_tests.rs"]
mod workflow_run_execution_tests;

use graph_fixtures::{
    multi_python_runtime_data_graph, runtime_diffusion_data_graph,
    synthetic_kv_node_memory_snapshot,
};

struct MockImagePythonRuntime {
    requests: Mutex<Vec<PythonNodeExecutionRequest>>,
}

struct MockReadyBackend {
    ready: bool,
}

struct MockRestoreFailureBackend {
    ready: bool,
    inference_model_path: PathBuf,
    embedding_model_path: PathBuf,
    embedding_started: bool,
}

struct MockProcessHandle;

struct MockProcessSpawner;

#[async_trait::async_trait]
impl PythonRuntimeAdapter for MockImagePythonRuntime {
    async fn execute_node(
        &self,
        request: PythonNodeExecutionRequest,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        self.requests.lock().expect("requests lock").push(request);
        Ok(HashMap::from([(
            "image".to_string(),
            serde_json::json!("data:image/png;base64,bW9jay1pbWFnZQ=="),
        )]))
    }
}

impl ProcessHandle for MockProcessHandle {
    fn pid(&self) -> u32 {
        1
    }

    fn kill(&self) -> Result<(), String> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl ProcessSpawner for MockProcessSpawner {
    async fn spawn_sidecar(
        &self,
        _sidecar_name: &str,
        _args: &[&str],
    ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
        let (_tx, rx) = mpsc::channel(1);
        Ok((rx, Box::new(MockProcessHandle)))
    }

    fn app_data_dir(&self) -> Result<PathBuf, String> {
        Ok(PathBuf::from("/tmp"))
    }

    fn binaries_dir(&self) -> Result<PathBuf, String> {
        Ok(PathBuf::from("/tmp"))
    }
}

#[async_trait::async_trait]
impl InferenceBackend for MockReadyBackend {
    fn name(&self) -> &'static str {
        "Mock"
    }

    fn description(&self) -> &'static str {
        "Mock ready backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        self.ready = true;
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("started_mock_runtime".to_string()),
        })
    }

    fn stop(&mut self) {
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    async fn health_check(&self) -> bool {
        self.ready
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<
        Pin<Box<dyn futures_util::Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
        BackendError,
    > {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait::async_trait]
impl InferenceBackend for MockRestoreFailureBackend {
    fn name(&self) -> &'static str {
        "Mock"
    }

    fn description(&self) -> &'static str {
        "Mock backend that fails inference restore after embedding mode"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        let model_path = config.model_path.clone().unwrap_or_default();
        if model_path == self.embedding_model_path {
            self.embedding_started = true;
            self.ready = true;
            return Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("started_mock_embedding_runtime".to_string()),
            });
        }

        if model_path == self.inference_model_path && self.embedding_started {
            self.ready = false;
            return Err(BackendError::StartupFailed(
                "mock restore failure after embedding mode".to_string(),
            ));
        }

        self.ready = true;
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("started_mock_runtime".to_string()),
        })
    }

    fn stop(&mut self) {
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    async fn health_check(&self) -> bool {
        self.ready
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<
        Pin<Box<dyn futures_util::Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
        BackendError,
    > {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

fn install_fake_default_runtime(app_data_dir: &Path) {
    let runtime_dir = app_data_dir.join("runtimes").join("llama-cpp");
    std::fs::create_dir_all(&runtime_dir).expect("create fake runtime dir");

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        for file_name in [
            "llama-server-x86_64-unknown-linux-gnu",
            "libllama.so",
            "libggml.so",
        ] {
            std::fs::write(runtime_dir.join(file_name), [])
                .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
        }
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        for file_name in ["llama-server-aarch64-apple-darwin", "libllama.dylib"] {
            std::fs::write(runtime_dir.join(file_name), [])
                .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
        }
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        for file_name in ["llama-server-x86_64-apple-darwin", "libllama.dylib"] {
            std::fs::write(runtime_dir.join(file_name), [])
                .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
        }
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        for file_name in [
            "llama-server-x86_64-pc-windows-msvc.exe",
            "llama-runtime.dll",
        ] {
            std::fs::write(runtime_dir.join(file_name), [])
                .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
        }
    }
}

fn persist_failed_selected_runtime_version(app_data_dir: &Path, version: &str, error: &str) {
    let install_root = app_data_dir.join("runtimes").join("llama-cpp");
    let state = inference::ManagedRuntimePersistedState {
        schema_version: 1,
        runtimes: vec![inference::ManagedRuntimePersistedRuntime {
            id: inference::ManagedBinaryId::LlamaCpp,
            catalog_versions: Vec::new(),
            catalog_refreshed_at_ms: None,
            versions: vec![inference::ManagedRuntimePersistedVersion {
                version: version.to_string(),
                runtime_key: Some(inference::ManagedBinaryId::LlamaCpp.key().to_string()),
                platform_key: Some("linux-x86_64".to_string()),
                readiness_state: inference::ManagedRuntimeReadinessState::Failed,
                install_root: Some(install_root.display().to_string()),
                last_ready_at_ms: None,
                last_error: Some(error.to_string()),
            }],
            selection: inference::ManagedRuntimeSelectionState {
                selected_version: Some(version.to_string()),
                active_version: None,
                default_version: Some(version.to_string()),
            },
            active_job: None,
            active_job_artifact: None,
            install_history: Vec::new(),
        }],
    };
    inference::save_managed_runtime_state(app_data_dir, &state)
        .expect("persist failed selected runtime state");
}

fn persist_interrupted_runtime_job(app_data_dir: &Path) {
    let state = inference::ManagedRuntimePersistedState {
        schema_version: 1,
        runtimes: vec![inference::ManagedRuntimePersistedRuntime {
            id: inference::ManagedBinaryId::LlamaCpp,
            catalog_versions: Vec::new(),
            catalog_refreshed_at_ms: None,
            versions: Vec::new(),
            selection: inference::ManagedRuntimeSelectionState::default(),
            active_job: Some(inference::ManagedRuntimeJobStatus {
                state: inference::ManagedRuntimeJobState::Downloading,
                status: "Downloading".to_string(),
                current: 5,
                total: 10,
                resumable: true,
                cancellable: true,
                error: None,
            }),
            active_job_artifact: None,
            install_history: Vec::new(),
        }],
    };
    inference::save_managed_runtime_state(app_data_dir, &state)
        .expect("persist interrupted runtime job");
}

fn write_test_workflow(root: &Path, workflow_id: &str) {
    let workflows_dir = root.join(".pantograph").join("workflows");
    std::fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    let workflow_json = serde_json::json!({
        "version": "1.0",
        "metadata": {
            "name": "Test Workflow",
            "created": "2026-01-01T00:00:00Z",
            "modified": "2026-01-01T00:00:00Z"
        },
        "graph": {
            "nodes": [
                {
                    "id": "text-input-1",
                    "node_type": "text-input",
                    "data": {
                        "name": "Prompt",
                        "description": "Prompt supplied by the caller",
                        "definition": {
                            "category": "input",
                            "io_binding_origin": "client_session",
                            "label": "Text Input",
                            "description": "Provides text input",
                            "inputs": [
                                {
                                    "id": "text",
                                    "label": "Text",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                }
                            ],
                            "outputs": [
                                {
                                    "id": "legacy-out",
                                    "label": "Legacy Out",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                }
                            ]
                        },
                        "text": "hello"
                    },
                    "position": { "x": 0.0, "y": 0.0 }
                },
                {
                    "id": "text-output-1",
                    "node_type": "text-output",
                    "data": {
                        "definition": {
                            "category": "output",
                            "io_binding_origin": "client_session",
                            "label": "Text Output",
                            "description": "Displays text output",
                            "inputs": [
                                {
                                    "id": "text",
                                    "label": "Text",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                },
                                {
                                    "id": "stream",
                                    "label": "Stream",
                                    "data_type": "stream",
                                    "required": false,
                                    "multiple": false
                                }
                            ],
                            "outputs": [
                                {
                                    "id": "text",
                                    "label": "Text",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                }
                            ]
                        }
                    },
                    "position": { "x": 200.0, "y": 0.0 }
                }
            ],
            "edges": [
                {
                    "id": "e-text",
                    "source": "text-input-1",
                    "source_handle": "text",
                    "target": "text-output-1",
                    "target_handle": "text"
                }
            ]
        }
    });
    std::fs::write(
        workflows_dir.join(format!("{workflow_id}.json")),
        serde_json::to_vec(&workflow_json).expect("serialize workflow"),
    )
    .expect("write workflow");
}

fn rewrite_test_workflow_input_description(root: &Path, workflow_id: &str, description: &str) {
    let workflow_path = root
        .join(".pantograph")
        .join("workflows")
        .join(format!("{workflow_id}.json"));
    let mut workflow_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&workflow_path).expect("read workflow"))
            .expect("parse workflow");
    workflow_json["graph"]["nodes"][0]["data"]["description"] = serde_json::json!(description);
    std::fs::write(
        workflow_path,
        serde_json::to_vec(&workflow_json).expect("serialize workflow"),
    )
    .expect("rewrite workflow");
}

fn rewrite_test_workflow_required_backend(root: &Path, workflow_id: &str, backend_key: &str) {
    let workflow_path = root
        .join(".pantograph")
        .join("workflows")
        .join(format!("{workflow_id}.json"));
    let mut workflow_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&workflow_path).expect("read workflow"))
            .expect("parse workflow");
    workflow_json["graph"]["nodes"][0]["data"]["backend_key"] = serde_json::json!(backend_key);
    std::fs::write(
        workflow_path,
        serde_json::to_vec(&workflow_json).expect("serialize workflow"),
    )
    .expect("rewrite workflow");
}

fn rewrite_test_workflow_output_node_to_human_input(root: &Path, workflow_id: &str) {
    let workflow_path = root
        .join(".pantograph")
        .join("workflows")
        .join(format!("{workflow_id}.json"));
    let mut workflow_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&workflow_path).expect("read workflow"))
            .expect("parse workflow");
    workflow_json["graph"]["nodes"][1] = serde_json::json!({
        "id": "text-output-1",
        "node_type": "human-input",
        "data": {
            "prompt": "Approve the resumed output?",
            "definition": {
                "category": "input",
                "io_binding_origin": "client_session",
                "label": "Human Input",
                "description": "Forces resumed execution to wait for interactive input",
                "inputs": [
                    workflow_port_definition("prompt", "Prompt", "string"),
                    workflow_port_definition("default", "Default Value", "string"),
                    workflow_port_definition("auto_accept", "Auto Accept", "boolean"),
                    workflow_port_definition("user_response", "User Response", "string")
                ],
                "outputs": [workflow_port_definition("value", "Value", "string")]
            }
        },
        "position": { "x": 200.0, "y": 0.0 }
    });
    workflow_json["graph"]["edges"] = serde_json::json!([]);
    std::fs::write(
        workflow_path,
        serde_json::to_vec(&workflow_json).expect("serialize workflow"),
    )
    .expect("rewrite workflow");
}

fn write_human_input_workflow(root: &Path, workflow_id: &str) {
    let workflows_dir = root.join(".pantograph").join("workflows");
    std::fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    let workflow_json = serde_json::json!({
        "version": "1.0",
        "metadata": {
            "name": "Interactive Workflow",
            "created": "2026-01-01T00:00:00Z",
            "modified": "2026-01-01T00:00:00Z"
        },
        "graph": {
            "nodes": [
                {
                    "id": "human-input-1",
                    "node_type": "human-input",
                    "data": {
                        "prompt": "Approve deployment?",
                        "definition": {
                            "category": "input",
                            "io_binding_origin": "client_session",
                            "label": "Human Input",
                            "description": "Pauses workflow to wait for interactive input",
                            "inputs": [
                                {
                                    "id": "prompt",
                                    "label": "Prompt",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                },
                                {
                                    "id": "default",
                                    "label": "Default Value",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                },
                                {
                                    "id": "auto_accept",
                                    "label": "Auto Accept",
                                    "data_type": "boolean",
                                    "required": false,
                                    "multiple": false
                                },
                                {
                                    "id": "user_response",
                                    "label": "User Response",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                }
                            ],
                            "outputs": [
                                {
                                    "id": "value",
                                    "label": "Value",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                }
                            ]
                        }
                    },
                    "position": { "x": 0.0, "y": 0.0 }
                }
            ],
            "edges": []
        }
    });
    std::fs::write(
        workflows_dir.join(format!("{workflow_id}.json")),
        serde_json::to_vec(&workflow_json).expect("serialize workflow"),
    )
    .expect("write workflow");
}

fn workflow_port_definition(id: &str, label: &str, data_type: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "label": label,
        "data_type": data_type,
        "required": false,
        "multiple": false
    })
}

fn write_mock_diffusion_workflow(root: &Path, workflow_id: &str) {
    let workflows_dir = root.join(".pantograph").join("workflows");
    std::fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    let workflow_json = serde_json::json!({
        "version": "1.0",
        "metadata": {
            "name": "Mock Diffusion Workflow",
            "created": "2026-01-01T00:00:00Z",
            "modified": "2026-01-01T00:00:00Z"
        },
        "graph": {
            "nodes": [
                {
                    "id": "text-input-1",
                    "node_type": "text-input",
                    "data": {
                        "definition": {
                            "category": "input",
                            "io_binding_origin": "client_session",
                            "label": "Prompt",
                            "description": "Prompt supplied by the caller",
                            "inputs": [workflow_port_definition("text", "Text", "string")],
                            "outputs": [workflow_port_definition("text", "Text", "string")]
                        },
                        "text": "a tiny painted robot"
                    },
                    "position": { "x": 0.0, "y": 0.0 }
                },
                {
                    "id": "diffusion-inference-1",
                    "node_type": "diffusion-inference",
                    "data": {
                        "model_path": "/tmp/mock-diffusion-model",
                        "model_type": "diffusion",
                        "environment_ref": {
                            "state": "ready",
                            "env_ids": ["mock-python-env"]
                        }
                    },
                    "position": { "x": 240.0, "y": 0.0 }
                },
                {
                    "id": "image-output-1",
                    "node_type": "image-output",
                    "data": {
                        "definition": {
                            "category": "output",
                            "io_binding_origin": "client_session",
                            "label": "Generated Image",
                            "description": "Generated image output",
                            "inputs": [workflow_port_definition("image", "Image", "image")],
                            "outputs": [workflow_port_definition("image", "Image", "image")]
                        }
                    },
                    "position": { "x": 520.0, "y": 0.0 }
                }
            ],
            "edges": [
                {
                    "id": "e-prompt",
                    "source": "text-input-1",
                    "source_handle": "text",
                    "target": "diffusion-inference-1",
                    "target_handle": "prompt"
                },
                {
                    "id": "e-image",
                    "source": "diffusion-inference-1",
                    "source_handle": "image",
                    "target": "image-output-1",
                    "target_handle": "image"
                }
            ]
        }
    });
    std::fs::write(
        workflows_dir.join(format!("{workflow_id}.json")),
        serde_json::to_vec(&workflow_json).expect("serialize workflow"),
    )
    .expect("write workflow");
}

fn write_imported_embedding_model(root: &Path) -> (String, PathBuf) {
    let model_dir = root
        .join("shared-resources")
        .join("models")
        .join("embedding")
        .join("imported")
        .join("test-embed");
    std::fs::create_dir_all(&model_dir).expect("create embedding model dir");

    let model_file = model_dir.join("embed.gguf");
    std::fs::write(&model_file, b"gguf").expect("write embedding model");
    std::fs::write(
        model_dir.join("metadata.json"),
        serde_json::json!({
            "schema_version": 2,
            "model_id": "embedding/imported/test-embed",
            "family": "imported",
            "model_type": "embedding",
            "official_name": "test-embed",
            "cleaned_name": "test-embed",
            "source_path": model_dir.display().to_string(),
            "storage_kind": "library_owned",
            "import_state": "ready",
            "validation_state": "valid",
            "task_type_primary": "feature-extraction",
            "recommended_backend": "llamacpp",
            "runtime_engine_hints": ["llamacpp"]
        })
        .to_string(),
    )
    .expect("write embedding metadata");

    ("embedding/imported/test-embed".to_string(), model_file)
}

fn edit_session_embedding_graph(model_id: &str) -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "puma-lib-1".to_string(),
                node_type: "puma-lib".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({ "model_id": model_id }),
            },
            GraphNode {
                id: "embedding-1".to_string(),
                node_type: "embedding".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: serde_json::json!({}),
            },
        ],
        edges: vec![GraphEdge {
            id: "edge-model".to_string(),
            source: "puma-lib-1".to_string(),
            source_handle: "model_path".to_string(),
            target: "embedding-1".to_string(),
            target_handle: "model".to_string(),
        }],
        ..WorkflowGraph::default()
    }
}

fn multi_python_edit_session_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "text-input-1".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({ "text": "painted robot" }),
            },
            GraphNode {
                id: "text-input-2".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 180.0 },
                data: serde_json::json!({ "text": "tiny waveform" }),
            },
            GraphNode {
                id: "diffusion-inference-1".to_string(),
                node_type: "diffusion-inference".to_string(),
                position: Position { x: 240.0, y: 0.0 },
                data: serde_json::json!({
                    "model_path": "/tmp/mock-diffusion-model",
                    "backend_key": "diffusers",
                    "model_type": "diffusion",
                    "environment_ref": {
                        "state": "ready",
                        "env_ids": ["mock-python-env"]
                    }
                }),
            },
            GraphNode {
                id: "onnx-inference-1".to_string(),
                node_type: "onnx-inference".to_string(),
                position: Position { x: 240.0, y: 180.0 },
                data: serde_json::json!({
                    "model_path": "/tmp/mock-onnx-model",
                    "backend_key": "onnxruntime",
                    "model_type": "audio",
                    "environment_ref": {
                        "state": "ready",
                        "env_ids": ["mock-onnx-env"]
                    }
                }),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "e-prompt".to_string(),
                source: "text-input-1".to_string(),
                source_handle: "text".to_string(),
                target: "diffusion-inference-1".to_string(),
                target_handle: "prompt".to_string(),
            },
            GraphEdge {
                id: "e-audio".to_string(),
                source: "text-input-2".to_string(),
                source_handle: "text".to_string(),
                target: "onnx-inference-1".to_string(),
                target_handle: "prompt".to_string(),
            },
        ],
        ..WorkflowGraph::default()
    }
}

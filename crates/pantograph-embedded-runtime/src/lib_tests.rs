use super::*;
use futures_util::stream;
use inference::backend::{
    BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use inference::{RerankRequest, RerankResponse};
use pantograph_runtime_registry::{
    RuntimeRegistration, RuntimeRegistry, RuntimeRegistrySnapshot, RuntimeRegistryStatus,
    RuntimeReservationRequest, RuntimeTransition,
};
use pantograph_workflow_service::{
    GraphEdge, GraphNode, Position, WorkflowGraph, WorkflowRuntimeInstallState,
    WorkflowRuntimeSourceKind, WorkflowSchedulerRuntimeWarmupDecision,
    WorkflowSchedulerRuntimeWarmupReason,
};
use std::path::Path;
use std::pin::Pin;
use tempfile::TempDir;
use tokio::sync::mpsc;

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

fn runtime_diffusion_data_graph() -> node_engine::WorkflowGraph {
    node_engine::WorkflowGraph {
        id: "runtime-diffusion-data-graph".to_string(),
        name: "Runtime Diffusion Data Graph".to_string(),
        nodes: vec![
            node_engine::GraphNode {
                id: "text-input-1".to_string(),
                node_type: "text-input".to_string(),
                data: serde_json::json!({ "text": "a tiny painted robot" }),
                position: (0.0, 0.0),
            },
            node_engine::GraphNode {
                id: "diffusion-inference-1".to_string(),
                node_type: "diffusion-inference".to_string(),
                data: serde_json::json!({
                    "model_path": "/tmp/mock-diffusion-model",
                    "model_type": "diffusion",
                    "environment_ref": {
                        "state": "ready",
                        "env_ids": ["mock-python-env"]
                    }
                }),
                position: (240.0, 0.0),
            },
            node_engine::GraphNode {
                id: "image-output-1".to_string(),
                node_type: "image-output".to_string(),
                data: serde_json::json!({}),
                position: (520.0, 0.0),
            },
        ],
        edges: vec![
            node_engine::GraphEdge {
                id: "e-prompt".to_string(),
                source: "text-input-1".to_string(),
                source_handle: "text".to_string(),
                target: "diffusion-inference-1".to_string(),
                target_handle: "prompt".to_string(),
            },
            node_engine::GraphEdge {
                id: "e-image".to_string(),
                source: "diffusion-inference-1".to_string(),
                source_handle: "image".to_string(),
                target: "image-output-1".to_string(),
                target_handle: "image".to_string(),
            },
        ],
        groups: Vec::new(),
    }
}

fn multi_python_runtime_data_graph() -> node_engine::WorkflowGraph {
    node_engine::WorkflowGraph {
        id: "multi-python-runtime-data-graph".to_string(),
        name: "Multi Python Runtime Data Graph".to_string(),
        nodes: vec![
            node_engine::GraphNode {
                id: "text-input-1".to_string(),
                node_type: "text-input".to_string(),
                data: serde_json::json!({ "text": "painted robot" }),
                position: (0.0, 0.0),
            },
            node_engine::GraphNode {
                id: "text-input-2".to_string(),
                node_type: "text-input".to_string(),
                data: serde_json::json!({ "text": "tiny waveform" }),
                position: (0.0, 180.0),
            },
            node_engine::GraphNode {
                id: "diffusion-inference-1".to_string(),
                node_type: "diffusion-inference".to_string(),
                data: serde_json::json!({
                    "model_path": "/tmp/mock-diffusion-model",
                    "backend_key": "diffusers",
                    "model_type": "diffusion",
                    "environment_ref": {
                        "state": "ready",
                        "env_ids": ["mock-python-env"]
                    }
                }),
                position: (240.0, 0.0),
            },
            node_engine::GraphNode {
                id: "onnx-inference-1".to_string(),
                node_type: "onnx-inference".to_string(),
                data: serde_json::json!({
                    "model_path": "/tmp/mock-onnx-model",
                    "backend_key": "onnxruntime",
                    "model_type": "audio",
                    "environment_ref": {
                        "state": "ready",
                        "env_ids": ["mock-onnx-env"]
                    }
                }),
                position: (240.0, 180.0),
            },
        ],
        edges: vec![
            node_engine::GraphEdge {
                id: "e-prompt".to_string(),
                source: "text-input-1".to_string(),
                source_handle: "text".to_string(),
                target: "diffusion-inference-1".to_string(),
                target_handle: "prompt".to_string(),
            },
            node_engine::GraphEdge {
                id: "e-audio".to_string(),
                source: "text-input-2".to_string(),
                source_handle: "text".to_string(),
                target: "onnx-inference-1".to_string(),
                target_handle: "prompt".to_string(),
            },
        ],
        groups: Vec::new(),
    }
}

fn synthetic_kv_node_memory_snapshot(
    session_id: &str,
    node_id: &str,
    cache_id: &str,
) -> node_engine::NodeMemorySnapshot {
    node_engine::NodeMemorySnapshot {
        identity: node_engine::NodeMemoryIdentity {
            session_id: session_id.to_string(),
            node_id: node_id.to_string(),
            node_type: "llamacpp-inference".to_string(),
            schema_version: Some("v1".to_string()),
        },
        status: node_engine::NodeMemoryStatus::Ready,
        input_fingerprint: Some(format!("fp-{cache_id}")),
        output_snapshot: Some(serde_json::json!({
            "kv_cache_out": {
                "cache_id": cache_id,
            }
        })),
        private_state: None,
        indirect_state_reference: Some(node_engine::engine::NodeMemoryIndirectStateReference {
            reference_kind: "kv_cache_handle".to_string(),
            reference_id: cache_id.to_string(),
            restore_strategy: node_engine::engine::NodeMemoryRestoreStrategy::RehydrateBeforeResume,
            inspection_metadata: Some(serde_json::json!({
                "source_port": "kv_cache_out",
                "backend_key": "llamacpp",
                "model_fingerprint": {
                    "model_id": "model-1",
                    "config_hash": "cfg-1",
                },
                "runtime_fingerprint": {
                    "runtime_id": "runtime-1",
                    "backend_key": "llamacpp",
                    "tokenizer_fingerprint": "tok-1",
                    "prompt_format_fingerprint": "prompt-1",
                    "runtime_build_fingerprint": "build-1",
                }
            })),
        }),
        inspection_metadata: Some(serde_json::json!({
            "projection_source": "test",
        })),
    }
}

#[tokio::test]
async fn test_runtime_run_and_session_execution() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let run_response = runtime
        .workflow_run(WorkflowRunRequest {
            workflow_id: "runtime-text".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("hello"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            run_id: Some("run-1".to_string()),
        })
        .await
        .expect("workflow run");
    assert_eq!(run_response.outputs.len(), 1);
    assert_eq!(run_response.outputs[0].value, serde_json::json!("hello"));

    let created = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await
        .expect("create session");

    let session_response = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: created.session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("world"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-2".to_string()),
        })
        .await
        .expect("run session");
    assert_eq!(session_response.outputs.len(), 1);
    assert_eq!(
        session_response.outputs[0].value,
        serde_json::json!("world")
    );

    runtime
        .close_workflow_session(WorkflowSessionCloseRequest {
            session_id: created.session_id,
        })
        .await
        .expect("close session");
}

#[tokio::test]
async fn workflow_run_returns_invalid_request_for_human_input_workflow() {
    let temp = TempDir::new().expect("temp dir");
    write_human_input_workflow(temp.path(), "interactive-human-input");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let error = runtime
        .workflow_run(WorkflowRunRequest {
            workflow_id: "interactive-human-input".to_string(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "human-input-1".to_string(),
                port_id: "value".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            run_id: Some("run-human-input".to_string()),
        })
        .await
        .expect_err("interactive workflow run should fail for non-streaming callers");

    match error {
        WorkflowServiceError::InvalidRequest(message) => {
            assert!(
                message.contains("interactive") || message.contains("input"),
                "unexpected invalid-request message: {message}"
            );
        }
        other => panic!("expected invalid request error, got {other:?}"),
    }
}

#[tokio::test]
async fn embedded_workflow_host_run_workflow_returns_cancelled_for_precancelled_run_handle() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let run_handle = pantograph_workflow_service::WorkflowRunHandle::new();
    run_handle.cancel();

    let error = runtime
        .host()
        .run_workflow(
            "runtime-text",
            &[WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("hello"),
            }],
            Some(&[WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            WorkflowRunOptions {
                timeout_ms: None,
                workflow_session_id: None,
            },
            run_handle,
        )
        .await
        .expect_err("pre-cancelled host run should return cancelled");

    match error {
        WorkflowServiceError::Cancelled(message) => {
            assert!(
                message.contains("cancelled before execution started"),
                "unexpected cancelled message: {message}"
            );
        }
        other => panic!("expected cancelled error, got {other:?}"),
    }
}

#[tokio::test]
async fn workflow_run_session_returns_invalid_request_for_human_input_workflow() {
    let temp = TempDir::new().expect("temp dir");
    write_human_input_workflow(temp.path(), "interactive-human-input");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let created = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "interactive-human-input".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: false,
        })
        .await
        .expect("create interactive session");

    let error = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: created.session_id,
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "human-input-1".to_string(),
                port_id: "value".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-human-input-session".to_string()),
        })
        .await
        .expect_err("interactive workflow session run should fail for non-streaming callers");

    match error {
        WorkflowServiceError::InvalidRequest(message) => {
            assert!(
                message.contains("interactive") || message.contains("input"),
                "unexpected invalid-request message: {message}"
            );
        }
        other => panic!("expected invalid request error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_runtime_routes_diffusion_workflow_through_python_adapter() {
    let temp = TempDir::new().expect("temp dir");
    write_mock_diffusion_workflow(temp.path(), "runtime-diffusion");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let python_runtime = Arc::new(MockImagePythonRuntime {
        requests: Mutex::new(Vec::new()),
    });
    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
        python_runtime.clone(),
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let response = runtime
        .workflow_run(WorkflowRunRequest {
            workflow_id: "runtime-diffusion".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("a tiny painted robot"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "image-output-1".to_string(),
                port_id: "image".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            run_id: Some("diffusion-run-1".to_string()),
        })
        .await
        .expect("workflow run");

    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, "image-output-1");
    assert_eq!(response.outputs[0].port_id, "image");
    assert_eq!(
        response.outputs[0].value,
        serde_json::json!("data:image/png;base64,bW9jay1pbWFnZQ==")
    );

    let requests = python_runtime.requests.lock().expect("requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].node_type, "diffusion-inference");
    assert_eq!(
        requests[0].inputs.get("prompt"),
        Some(&serde_json::json!("a tiny painted robot"))
    );
}

#[tokio::test]
async fn test_runtime_run_reconciles_python_sidecar_runtime_into_registry() {
    let temp = TempDir::new().expect("temp dir");
    write_mock_diffusion_workflow(temp.path(), "runtime-diffusion");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
        Arc::new(MockImagePythonRuntime {
            requests: Mutex::new(Vec::new()),
        }),
    )
    .with_runtime_registry(runtime_registry.clone());

    runtime
        .workflow_run(WorkflowRunRequest {
            workflow_id: "runtime-diffusion".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("a tiny painted robot"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "image-output-1".to_string(),
                port_id: "image".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            run_id: Some("diffusion-run-2".to_string()),
        })
        .await
        .expect("workflow run");

    let snapshot = runtime_registry.snapshot();
    let pytorch = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("python runtime should be observed");
    assert_eq!(pytorch.display_name, "PyTorch (Python sidecar)");
    assert_eq!(pytorch.status, RuntimeRegistryStatus::Stopped);
    assert!(pytorch.runtime_instance_id.is_none());
    assert!(pytorch.models.is_empty());
}

#[tokio::test]
async fn execute_data_graph_reconciles_python_sidecar_runtime_into_registry() {
    let temp = TempDir::new().expect("temp dir");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
        Arc::new(MockImagePythonRuntime {
            requests: Mutex::new(Vec::new()),
        }),
    )
    .with_runtime_registry(runtime_registry.clone());

    let outputs = runtime
        .execute_data_graph(
            "runtime-diffusion-data-graph",
            &runtime_diffusion_data_graph(),
            &HashMap::from([(
                "text".to_string(),
                serde_json::json!("a tiny painted robot"),
            )]),
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("data graph execution");

    assert_eq!(
        outputs.get("image"),
        Some(&serde_json::json!("data:image/png;base64,bW9jay1pbWFnZQ=="))
    );
    assert_eq!(
        outputs.get("_graph_id"),
        Some(&serde_json::json!("runtime-diffusion-data-graph"))
    );

    let snapshot = runtime_registry.snapshot();
    let pytorch = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("python runtime should be observed");
    assert_eq!(pytorch.display_name, "PyTorch (Python sidecar)");
    assert_eq!(pytorch.status, RuntimeRegistryStatus::Stopped);
    assert!(pytorch.runtime_instance_id.is_none());
    assert!(pytorch.models.is_empty());
}

#[tokio::test]
async fn execute_data_graph_reconciles_multiple_python_sidecar_runtimes_into_registry() {
    let temp = TempDir::new().expect("temp dir");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
        Arc::new(MockImagePythonRuntime {
            requests: Mutex::new(Vec::new()),
        }),
    )
    .with_runtime_registry(runtime_registry.clone());

    runtime
        .execute_data_graph(
            "multi-python-runtime-data-graph",
            &multi_python_runtime_data_graph(),
            &HashMap::new(),
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("data graph execution");

    let snapshot = runtime_registry.snapshot();
    let diffusers = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "diffusers")
        .expect("diffusers runtime should be observed");
    assert_eq!(diffusers.status, RuntimeRegistryStatus::Stopped);
    assert!(diffusers.runtime_instance_id.is_none());
    assert!(diffusers.models.is_empty());

    let onnx = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "onnx-runtime")
        .expect("onnx runtime should be observed");
    assert_eq!(onnx.status, RuntimeRegistryStatus::Stopped);
    assert!(onnx.runtime_instance_id.is_none());
    assert!(onnx.models.is_empty());
}

#[tokio::test]
async fn execute_data_graph_propagates_waiting_for_input_without_synthetic_error_output() {
    let temp = TempDir::new().expect("temp dir");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
        Arc::new(ProcessPythonRuntimeAdapter),
    );
    let event_sink = Arc::new(node_engine::VecEventSink::new());
    let graph = node_engine::WorkflowGraph {
        id: "interactive-data-graph".to_string(),
        name: "Interactive Data Graph".to_string(),
        nodes: vec![node_engine::GraphNode {
            id: "approval".to_string(),
            node_type: "human-input".to_string(),
            data: serde_json::json!({ "prompt": "Approve deployment?" }),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };

    let result = runtime
        .execute_data_graph(
            "interactive-data-graph",
            &graph,
            &HashMap::new(),
            event_sink.clone(),
        )
        .await;

    assert!(matches!(
        result,
        Err(node_engine::NodeEngineError::WaitingForInput { task_id, prompt })
            if task_id == "approval"
                && prompt.as_deref() == Some("Approve deployment?")
    ));
    let events = event_sink.events();
    assert!(events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::WaitingForInput {
            task_id,
            prompt: Some(prompt),
            ..
        } if task_id == "approval" && prompt == "Approve deployment?"
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, node_engine::WorkflowEvent::WorkflowFailed { .. }))
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::WorkflowCompleted { .. }
            | node_engine::WorkflowEvent::WorkflowCancelled { .. }
    )));
}

#[tokio::test]
async fn test_keep_alive_session_load_tracks_registry_reservation_lifecycle() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    let created = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");

    let reserved_snapshot = runtime_registry.snapshot();
    assert_eq!(reserved_snapshot.reservations.len(), 1);
    assert_eq!(
        reserved_snapshot.reservations[0].workflow_id,
        "runtime-text"
    );
    assert_eq!(
        reserved_snapshot.reservations[0].usage_profile.as_deref(),
        Some("interactive")
    );
    assert_eq!(
        reserved_snapshot.reservations[0].retention_hint,
        RuntimeRetentionHint::KeepAlive
    );
    assert_eq!(
        reserved_snapshot.runtimes[0].active_reservation_ids.len(),
        1
    );
    assert_eq!(
        reserved_snapshot.runtimes[0].status,
        RuntimeRegistryStatus::Warming
    );

    runtime
        .workflow_set_session_keep_alive(WorkflowSessionKeepAliveRequest {
            session_id: created.session_id.clone(),
            keep_alive: false,
        })
        .await
        .expect("disable keep alive");

    let released_snapshot = runtime_registry.snapshot();
    assert!(released_snapshot.reservations.is_empty());
    assert!(
        released_snapshot.runtimes[0]
            .active_reservation_ids
            .is_empty()
    );
    assert_eq!(
        released_snapshot.runtimes[0].status,
        RuntimeRegistryStatus::Stopped
    );
}

#[tokio::test]
async fn keep_alive_disable_reclaim_flips_scheduler_runtime_registry_diagnostics_to_start_runtime()
{
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "llama.cpp",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");

    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        gateway.clone(),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::with_capacity_limits(4, 1)),
        None,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await;

    let created = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");

    runtime
        .workflow_set_session_keep_alive(WorkflowSessionKeepAliveRequest {
            session_id: created.session_id,
            keep_alive: false,
        })
        .await
        .expect("disable keep alive");

    let snapshot = runtime_registry.snapshot();
    let runtime_record = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("runtime should remain observable after reclaim");
    assert_eq!(runtime_record.status, RuntimeRegistryStatus::Stopped);

    let diagnostics_provider = EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
        gateway.clone(),
        runtime_registry.clone(),
    );
    let diagnostics = diagnostics_provider
        .scheduler_runtime_registry_diagnostics(&WorkflowSchedulerRuntimeDiagnosticsRequest {
            session_id: "queued-after-reclaim".to_string(),
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: false,
            runtime_loaded: false,
            next_admission_queue_id: Some("queue-after-reclaim".to_string()),
            reclaim_candidates: Vec::new(),
        })
        .await
        .expect("scheduler diagnostics provider should succeed")
        .expect("runtime registry diagnostics should be present");

    assert_eq!(
        diagnostics,
        WorkflowSchedulerRuntimeRegistryDiagnostics {
            target_runtime_id: Some("llama_cpp".to_string()),
            reclaim_candidate_session_id: None,
            reclaim_candidate_runtime_id: None,
            next_warmup_decision: Some(WorkflowSchedulerRuntimeWarmupDecision::StartRuntime,),
            next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::NoLoadedInstance),
        }
    );
}

#[tokio::test]
async fn test_sync_loaded_session_runtime_retention_hint_updates_running_session() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    runtime_registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
    runtime_registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("llama-runtime-1".to_string()),
            },
        )
        .expect("ready transition");

    let lease = runtime_registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "llama.cpp".to_string(),
            workflow_id: "runtime-text".to_string(),
            reservation_owner_id: Some("session-running".to_string()),
            usage_profile: Some("interactive".to_string()),
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation should be created");
    let host = runtime.host();
    host.record_session_runtime_reservation("session-running", lease.reservation_id)
        .expect("reservation id should be recorded");

    host.sync_loaded_session_runtime_retention_hint(
        "session-running",
        true,
        WorkflowSessionState::Running,
    )
    .expect("running session retention hint should update");

    let snapshot = runtime_registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(
        snapshot.reservations[0].retention_hint,
        RuntimeRetentionHint::KeepAlive
    );
}

#[tokio::test]
async fn test_session_runtime_load_reuses_ready_gateway_runtime_in_registry() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "mock",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        gateway,
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    runtime
        .host()
        .load_session_runtime(
            "session-ready",
            "runtime-text",
            Some("interactive"),
            WorkflowSessionRetentionHint::KeepAlive,
        )
        .await
        .expect("ready runtime should be reused");

    let snapshot = runtime_registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(snapshot.runtimes.len(), 1);
    assert_eq!(snapshot.runtimes[0].runtime_id, "mock");
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Ready);
    assert!(snapshot.runtimes[0].runtime_instance_id.is_some());
}

#[tokio::test]
async fn test_session_runtime_load_waits_for_existing_warmup_transition() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    runtime_registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
    runtime_registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::WarmupStarted {
                runtime_instance_id: Some("llama-1".to_string()),
            },
        )
        .expect("runtime should enter warming");

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    let ready_registry = runtime_registry.clone();
    let ready_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        ready_registry
            .transition_runtime(
                "llama.cpp",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("llama-1".to_string()),
                },
            )
            .expect("runtime should become ready");
    });

    runtime
        .host()
        .load_session_runtime(
            "session-wait",
            "runtime-text",
            None,
            WorkflowSessionRetentionHint::KeepAlive,
        )
        .await
        .expect("load should wait for warmup completion");
    ready_task.await.expect("ready transition task");

    let snapshot = runtime_registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Ready);
    assert_eq!(
        snapshot.runtimes[0].runtime_instance_id.as_deref(),
        Some("llama-1")
    );
}

#[tokio::test]
async fn test_session_runtime_load_blocks_when_runtime_preflight_reports_not_ready() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    let error = runtime
        .host()
        .load_session_runtime(
            "session-not-ready",
            "runtime-text",
            None,
            WorkflowSessionRetentionHint::KeepAlive,
        )
        .await
        .expect_err("load should fail when required runtime is not ready");

    assert!(matches!(error, WorkflowServiceError::RuntimeNotReady(_)));
    assert!(
        error.to_string().contains("llama.cpp"),
        "expected readiness error to mention llama.cpp, got: {error}"
    );

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot.reservations.is_empty());
}

#[tokio::test]
async fn test_session_runtime_unload_stops_active_gateway_runtime_when_evictable() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "mock",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        gateway,
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    runtime
        .host()
        .load_session_runtime(
            "session-stop",
            "runtime-text",
            None,
            WorkflowSessionRetentionHint::KeepAlive,
        )
        .await
        .expect("ready runtime should load");
    runtime
        .host()
        .unload_session_runtime(
            "session-stop",
            "runtime-text",
            pantograph_workflow_service::WorkflowSessionUnloadReason::SessionClosed,
        )
        .await
        .expect("runtime should unload");

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot.reservations.is_empty());
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Stopped);
    assert!(!runtime.gateway().is_ready().await);
}

#[tokio::test]
async fn test_session_runtime_load_releases_reservation_after_warmup_timeout() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    runtime_registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
    runtime_registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::WarmupStarted {
                runtime_instance_id: Some("llama-timeout".to_string()),
            },
        )
        .expect("runtime should enter warming");

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    let error = runtime
        .host()
        .load_session_runtime(
            "session-timeout",
            "runtime-text",
            None,
            WorkflowSessionRetentionHint::KeepAlive,
        )
        .await
        .expect_err("warming timeout should fail");
    assert!(matches!(error, WorkflowServiceError::RuntimeTimeout(_)));

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot.reservations.is_empty());
    assert!(
        snapshot
            .runtimes
            .iter()
            .all(|runtime| runtime.active_reservation_ids.is_empty())
    );
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Stopped);
}

#[tokio::test]
async fn test_session_run_without_keep_alive_releases_runtime_reservation_after_run() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    let created = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let run_response = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id,
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("session-world"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-queued".to_string()),
        })
        .await
        .expect("run queued session");
    assert_eq!(run_response.outputs.len(), 1);
    assert_eq!(
        run_response.outputs[0].value,
        serde_json::json!("session-world")
    );

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot.reservations.is_empty());
    assert!(
        snapshot
            .runtimes
            .iter()
            .all(|runtime| runtime.active_reservation_ids.is_empty())
    );
    assert!(
        runtime
            .session_executions
            .handle(&created.session_id)
            .expect("session execution lookup should succeed")
            .is_none()
    );
}

#[tokio::test]
async fn keep_alive_session_reuses_backend_executor_and_carries_forward_inputs() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let created = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");
    let session_id = created.session_id.clone();

    let first_run = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-alpha".to_string()),
        })
        .await
        .expect("run keep-alive session first time");
    assert_eq!(first_run.outputs[0].value, serde_json::json!("alpha"));

    let first_executor = runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive session executor should exist");
    let first_snapshots = {
        let executor = first_executor.lock().await;
        executor
            .workflow_session_node_memory_snapshots(&session_id)
            .await
    };
    assert_eq!(first_snapshots.len(), 2);
    assert!(
        first_snapshots
            .iter()
            .any(|snapshot| snapshot.identity.node_id == "text-input-1")
    );
    assert!(
        first_snapshots
            .iter()
            .any(|snapshot| snapshot.identity.node_id == "text-output-1")
    );

    let second_run = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-carry-forward".to_string()),
        })
        .await
        .expect("run keep-alive session with carried-forward inputs");
    assert_eq!(second_run.outputs[0].value, serde_json::json!("alpha"));

    let second_executor = runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive session executor should still exist");
    assert!(Arc::ptr_eq(&first_executor, &second_executor));

    let third_run = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("beta"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-beta".to_string()),
        })
        .await
        .expect("run keep-alive session after updating one input");
    assert_eq!(third_run.outputs[0].value, serde_json::json!("beta"));

    let third_executor = runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive session executor should still exist");
    assert!(Arc::ptr_eq(&first_executor, &third_executor));

    runtime
        .close_workflow_session(WorkflowSessionCloseRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("close keep-alive session");
    assert!(
        runtime
            .session_executions
            .handle(&session_id)
            .expect("session execution lookup should succeed")
            .is_none()
    );
}

#[tokio::test]
async fn keep_alive_session_reconciles_graph_change_and_replays_carried_inputs() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let created = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");
    let session_id = created.session_id.clone();

    let first_run = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-before-edit".to_string()),
        })
        .await
        .expect("run before workflow edit");
    assert_eq!(first_run.outputs[0].value, serde_json::json!("alpha"));

    let first_executor = runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive session executor should exist");

    rewrite_test_workflow_input_description(
        temp.path(),
        "runtime-text",
        "Prompt updated after session creation",
    );

    let second_run = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-after-edit".to_string()),
        })
        .await
        .expect("run after workflow edit");
    assert_eq!(second_run.outputs[0].value, serde_json::json!("alpha"));

    let second_executor = runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive session executor should still exist");
    assert!(Arc::ptr_eq(&first_executor, &second_executor));

    let snapshots = {
        let executor = second_executor.lock().await;
        executor
            .workflow_session_node_memory_snapshots(&session_id)
            .await
    };
    assert_eq!(snapshots.len(), 2);
    assert!(
        snapshots
            .iter()
            .all(|snapshot| snapshot.status == node_engine::NodeMemoryStatus::Ready)
    );
}

#[tokio::test]
async fn keep_alive_session_retains_checkpoint_across_capacity_rebalance() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::with_capacity_limits(4, 1)),
        None,
    );

    let first = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create first keep-alive session");

    let first_output = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: first.session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-first".to_string()),
        })
        .await
        .expect("run first keep-alive session");
    assert_eq!(first_output.outputs[0].value, serde_json::json!("alpha"));

    let first_executor = runtime
        .session_executions
        .handle(&first.session_id)
        .expect("first session execution lookup should succeed")
        .expect("first keep-alive executor should exist");
    {
        let executor = first_executor.lock().await;
        executor
            .record_workflow_session_node_memory(synthetic_kv_node_memory_snapshot(
                &first.session_id,
                "kv-memory",
                "cache-session-1",
            ))
            .await;
    }

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &first.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("checkpoint keep-alive session for capacity rebalance");

    let checkpointed_summary = {
        let executor = first_executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&first.session_id)
            .await
    };
    assert!(checkpointed_summary.checkpoint_available);
    assert_eq!(
        checkpointed_summary.residency,
        node_engine::WorkflowSessionResidencyState::CheckpointedButUnloaded
    );
    assert!(
        checkpointed_summary.preserved_node_count >= 2,
        "checkpoint should preserve node memory for the keep-alive session"
    );
    let checkpointed_snapshots = {
        let executor = first_executor.lock().await;
        executor
            .workflow_session_node_memory_snapshots(&first.session_id)
            .await
    };
    assert!(
        checkpointed_snapshots.iter().any(|snapshot| {
            snapshot.identity.node_id == "kv-memory"
                && snapshot
                    .indirect_state_reference
                    .as_ref()
                    .map(|reference| reference.reference_id.as_str())
                    == Some("cache-session-1")
        }),
        "checkpoint should preserve the synthetic KV node-memory reference"
    );

    let resumed_output = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: first.session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-resume".to_string()),
        })
        .await
        .expect("resume first keep-alive session from checkpoint");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("alpha"));

    let resumed_executor = runtime
        .session_executions
        .handle(&first.session_id)
        .expect("resumed session execution lookup should succeed")
        .expect("resumed keep-alive executor should exist");
    assert!(Arc::ptr_eq(&first_executor, &resumed_executor));

    let resumed_summary = {
        let executor = resumed_executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&first.session_id)
            .await
    };
    assert!(!resumed_summary.checkpoint_available);
    assert_eq!(
        resumed_summary.residency,
        node_engine::WorkflowSessionResidencyState::Warm
    );
    let resumed_snapshots = {
        let executor = resumed_executor.lock().await;
        executor
            .workflow_session_node_memory_snapshots(&first.session_id)
            .await
    };
    assert!(
        resumed_snapshots.iter().any(|snapshot| {
            snapshot.identity.node_id == "kv-memory"
                && snapshot
                    .indirect_state_reference
                    .as_ref()
                    .map(|reference| reference.reference_id.as_str())
                    == Some("cache-session-1")
        }),
        "restored keep-alive session should retain its KV node-memory reference"
    );

    runtime
        .close_workflow_session(WorkflowSessionCloseRequest {
            session_id: first.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
}

#[tokio::test]
async fn scheduler_driven_rebalance_checkpoints_keep_alive_session() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let keep_alive = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");

    let first_output = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: keep_alive.session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-keep-alive".to_string()),
        })
        .await
        .expect("run keep-alive session");
    assert_eq!(first_output.outputs[0].value, serde_json::json!("alpha"));

    let keep_alive_executor = runtime
        .session_executions
        .handle(&keep_alive.session_id)
        .expect("keep-alive session lookup should succeed")
        .expect("keep-alive executor should exist");

    let one_shot = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("batch".to_string()),
            keep_alive: false,
        })
        .await
        .expect("create one-shot session");

    let second_output = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: one_shot.session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("beta"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-one-shot".to_string()),
        })
        .await
        .expect("run one-shot session under capacity pressure");
    assert_eq!(second_output.outputs[0].value, serde_json::json!("beta"));

    let checkpointed_summary = {
        let executor = keep_alive_executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&keep_alive.session_id)
            .await
    };
    assert!(checkpointed_summary.checkpoint_available);
    assert_eq!(
        checkpointed_summary.residency,
        node_engine::WorkflowSessionResidencyState::CheckpointedButUnloaded
    );
    assert!(
        checkpointed_summary.preserved_node_count >= 2,
        "scheduler-driven rebalance should preserve node memory for keep-alive sessions"
    );

    let resumed_output = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: keep_alive.session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-keep-alive-resume".to_string()),
        })
        .await
        .expect("resume keep-alive session after scheduler rebalance");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("alpha"));

    let resumed_executor = runtime
        .session_executions
        .handle(&keep_alive.session_id)
        .expect("resumed keep-alive session lookup should succeed")
        .expect("resumed keep-alive executor should exist");
    assert!(Arc::ptr_eq(&keep_alive_executor, &resumed_executor));

    let resumed_summary = {
        let executor = resumed_executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&keep_alive.session_id)
            .await
    };
    assert!(!resumed_summary.checkpoint_available);
    assert_eq!(
        resumed_summary.residency,
        node_engine::WorkflowSessionResidencyState::Warm
    );

    runtime
        .close_workflow_session(WorkflowSessionCloseRequest {
            session_id: keep_alive.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
    runtime
        .close_workflow_session(WorkflowSessionCloseRequest {
            session_id: one_shot.session_id.clone(),
        })
        .await
        .expect("close one-shot session");
}

#[tokio::test]
async fn repeated_capacity_unload_keeps_checkpoint_identity_and_keep_alive_disable_clears_it() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let session = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");

    runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session.session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-first".to_string()),
        })
        .await
        .expect("run keep-alive session");

    let executor = runtime
        .session_executions
        .handle(&session.session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive executor should exist");

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &session.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("first capacity unload");
    let first_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session.session_id)
            .await
    };

    tokio::time::sleep(std::time::Duration::from_millis(2)).await;

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &session.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("second capacity unload should be idempotent");
    let second_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session.session_id)
            .await
    };

    assert!(first_summary.checkpoint_available);
    assert_eq!(
        first_summary.checkpointed_at_ms,
        second_summary.checkpointed_at_ms
    );
    assert_eq!(
        second_summary.residency,
        node_engine::WorkflowSessionResidencyState::CheckpointedButUnloaded
    );

    runtime
        .workflow_set_session_keep_alive(WorkflowSessionKeepAliveRequest {
            session_id: session.session_id.clone(),
            keep_alive: false,
        })
        .await
        .expect("disable keep-alive after checkpoint");

    assert!(
        runtime
            .session_executions
            .handle(&session.session_id)
            .expect("session execution lookup should succeed")
            .is_none()
    );
}

#[tokio::test]
async fn failed_restore_keeps_checkpoint_until_resume_succeeds() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let session = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");

    runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session.session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-first".to_string()),
        })
        .await
        .expect("run keep-alive session");

    let executor = runtime
        .session_executions
        .handle(&session.session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive executor should exist");

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &session.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("checkpoint keep-alive session");

    let checkpointed_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(checkpointed_summary.checkpoint_available);

    rewrite_test_workflow_output_node_to_human_input(temp.path(), "runtime-text");

    let error = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session.session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "value".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-resume-fail".to_string()),
        })
        .await
        .expect_err("resume should fail when the output node now requires interactive input");
    match error {
        WorkflowServiceError::InvalidRequest(message) => {
            assert!(
                message.contains("text-output-1"),
                "unexpected invalid-request message: {message}"
            );
        }
        other => panic!("expected invalid request error, got {other:?}"),
    }

    let failed_restore_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(failed_restore_summary.checkpoint_available);
    assert_eq!(
        failed_restore_summary.checkpointed_at_ms,
        checkpointed_summary.checkpointed_at_ms
    );
    assert_eq!(
        failed_restore_summary.residency,
        node_engine::WorkflowSessionResidencyState::CheckpointedButUnloaded
    );

    write_test_workflow(temp.path(), "runtime-text");

    let resumed_output = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session.session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-resume-success".to_string()),
        })
        .await
        .expect("resume should succeed after restoring a runnable graph");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("alpha"));

    let resumed_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(!resumed_summary.checkpoint_available);
    assert_eq!(
        resumed_summary.residency,
        node_engine::WorkflowSessionResidencyState::Warm
    );

    runtime
        .close_workflow_session(WorkflowSessionCloseRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
}

#[tokio::test]
async fn runtime_not_ready_resume_keeps_checkpoint_until_runtime_returns() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir: app_data_dir.clone(),
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let session = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");

    runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session.session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-first".to_string()),
        })
        .await
        .expect("run keep-alive session");

    let executor = runtime
        .session_executions
        .handle(&session.session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive executor should exist");

    let one_shot = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("batch".to_string()),
            keep_alive: false,
        })
        .await
        .expect("create one-shot session");

    let one_shot_output = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: one_shot.session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("beta"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-one-shot".to_string()),
        })
        .await
        .expect("run one-shot session to force keep-alive rebalance");
    assert_eq!(one_shot_output.outputs[0].value, serde_json::json!("beta"));

    let checkpointed_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(checkpointed_summary.checkpoint_available);

    std::fs::remove_dir_all(app_data_dir.join("runtimes").join("llama-cpp"))
        .expect("remove fake runtime before resume");

    let error = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session.session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-resume-missing-runtime".to_string()),
        })
        .await
        .expect_err("resume should fail when the selected runtime is no longer ready");
    match error {
        WorkflowServiceError::RuntimeNotReady(message) => {
            assert!(
                message.contains("llama.cpp"),
                "unexpected runtime-not-ready message: {message}"
            );
        }
        other => panic!("expected runtime-not-ready error, got {other:?}"),
    }

    let failed_resume_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(failed_resume_summary.checkpoint_available);
    assert_eq!(
        failed_resume_summary.checkpointed_at_ms,
        checkpointed_summary.checkpointed_at_ms
    );
    assert_eq!(
        failed_resume_summary.residency,
        node_engine::WorkflowSessionResidencyState::CheckpointedButUnloaded
    );

    install_fake_default_runtime(&app_data_dir);

    let resumed_output = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session.session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-resume-runtime-restored".to_string()),
        })
        .await
        .expect("resume should succeed after the runtime becomes ready again");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("alpha"));

    let resumed_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(!resumed_summary.checkpoint_available);
    assert_eq!(
        resumed_summary.residency,
        node_engine::WorkflowSessionResidencyState::Warm
    );

    runtime
        .close_workflow_session(WorkflowSessionCloseRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
}

#[tokio::test]
async fn scheduler_reclaim_keeps_checkpointed_sessions_isolated_across_resumes() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let session_a = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create first keep-alive session");
    let session_b = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create second keep-alive session");

    let first_output = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_a.session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-a-initial".to_string()),
        })
        .await
        .expect("run first keep-alive session");
    assert_eq!(first_output.outputs[0].value, serde_json::json!("alpha"));

    let executor_a = runtime
        .session_executions
        .handle(&session_a.session_id)
        .expect("first session execution lookup should succeed")
        .expect("first keep-alive executor should exist");
    {
        let executor = executor_a.lock().await;
        executor
            .record_workflow_session_node_memory(synthetic_kv_node_memory_snapshot(
                &session_a.session_id,
                "kv-memory-a",
                "cache-session-a",
            ))
            .await;
    }

    let second_output = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_b.session_id.clone(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("beta"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-b-initial".to_string()),
        })
        .await
        .expect("run second keep-alive session under reclaim pressure");
    assert_eq!(second_output.outputs[0].value, serde_json::json!("beta"));

    let executor_b = runtime
        .session_executions
        .handle(&session_b.session_id)
        .expect("second session execution lookup should succeed")
        .expect("second keep-alive executor should exist");
    {
        let executor = executor_b.lock().await;
        executor
            .record_workflow_session_node_memory(synthetic_kv_node_memory_snapshot(
                &session_b.session_id,
                "kv-memory-b",
                "cache-session-b",
            ))
            .await;
    }
    assert!(
        !Arc::ptr_eq(&executor_a, &executor_b),
        "distinct workflow sessions must not share the same executor"
    );

    let first_checkpoint_summary = {
        let executor = executor_a.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session_a.session_id)
            .await
    };
    assert!(first_checkpoint_summary.checkpoint_available);
    assert_eq!(
        first_checkpoint_summary.residency,
        node_engine::WorkflowSessionResidencyState::CheckpointedButUnloaded
    );

    let resumed_a = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_a.session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-a-resume".to_string()),
        })
        .await
        .expect("resume first session after scheduler reclaim");
    assert_eq!(resumed_a.outputs[0].value, serde_json::json!("alpha"));

    let resumed_a_summary = {
        let executor = executor_a.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session_a.session_id)
            .await
    };
    assert!(!resumed_a_summary.checkpoint_available);
    assert_eq!(
        resumed_a_summary.residency,
        node_engine::WorkflowSessionResidencyState::Warm
    );
    let resumed_a_snapshots = {
        let executor = executor_a.lock().await;
        executor
            .workflow_session_node_memory_snapshots(&session_a.session_id)
            .await
    };
    assert!(
        resumed_a_snapshots.iter().any(|snapshot| {
            snapshot.identity.node_id == "kv-memory-a"
                && snapshot
                    .indirect_state_reference
                    .as_ref()
                    .map(|reference| reference.reference_id.as_str())
                    == Some("cache-session-a")
        }),
        "session A should retain only its own KV node-memory reference after resume"
    );
    assert!(
        resumed_a_snapshots.iter().all(|snapshot| {
            snapshot
                .indirect_state_reference
                .as_ref()
                .map(|reference| reference.reference_id.as_str())
                != Some("cache-session-b")
        }),
        "session A should not observe session B KV references"
    );

    let second_checkpoint_summary = {
        let executor = executor_b.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session_b.session_id)
            .await
    };
    assert!(second_checkpoint_summary.checkpoint_available);
    assert_eq!(
        second_checkpoint_summary.residency,
        node_engine::WorkflowSessionResidencyState::CheckpointedButUnloaded
    );

    let resumed_b = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_b.session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-b-resume".to_string()),
        })
        .await
        .expect("resume second session after reclaiming the first");
    assert_eq!(resumed_b.outputs[0].value, serde_json::json!("beta"));

    let resumed_b_summary = {
        let executor = executor_b.lock().await;
        executor
            .workflow_session_checkpoint_summary(&session_b.session_id)
            .await
    };
    assert!(!resumed_b_summary.checkpoint_available);
    assert_eq!(
        resumed_b_summary.residency,
        node_engine::WorkflowSessionResidencyState::Warm
    );
    let resumed_b_snapshots = {
        let executor = executor_b.lock().await;
        executor
            .workflow_session_node_memory_snapshots(&session_b.session_id)
            .await
    };
    assert!(
        resumed_b_snapshots.iter().any(|snapshot| {
            snapshot.identity.node_id == "kv-memory-b"
                && snapshot
                    .indirect_state_reference
                    .as_ref()
                    .map(|reference| reference.reference_id.as_str())
                    == Some("cache-session-b")
        }),
        "session B should retain only its own KV node-memory reference after resume"
    );
    assert!(
        resumed_b_snapshots.iter().all(|snapshot| {
            snapshot
                .indirect_state_reference
                .as_ref()
                .map(|reference| reference.reference_id.as_str())
                != Some("cache-session-a")
        }),
        "session B should not observe session A KV references"
    );

    runtime
        .close_workflow_session(WorkflowSessionCloseRequest {
            session_id: session_a.session_id.clone(),
        })
        .await
        .expect("close first resumed keep-alive session");
    runtime
        .close_workflow_session(WorkflowSessionCloseRequest {
            session_id: session_b.session_id.clone(),
        })
        .await
        .expect("close second resumed keep-alive session");
}

#[tokio::test]
async fn test_runtime_unload_candidate_selection_uses_registry_eviction_order() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    runtime_registry.observe_runtimes(vec![pantograph_runtime_registry::RuntimeObservation {
        runtime_id: "shared-runtime".to_string(),
        display_name: "shared-runtime".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: pantograph_runtime_registry::RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("shared-runtime-1".to_string()),
        last_error: None,
    }]);
    runtime_registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "shared-runtime".to_string(),
            workflow_id: "wf-a".to_string(),
            reservation_owner_id: Some("session-a".to_string()),
            usage_profile: Some("interactive".to_string()),
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("keep-alive reservation");
    runtime_registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "shared-runtime".to_string(),
            workflow_id: "wf-b".to_string(),
            reservation_owner_id: Some("session-b".to_string()),
            usage_profile: Some("batch".to_string()),
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("ephemeral reservation");

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry);

    let selected = runtime
        .host()
        .select_runtime_unload_candidate(
            &WorkflowSessionRuntimeSelectionTarget {
                session_id: "session-target".to_string(),
                workflow_id: "wf-a".to_string(),
                usage_profile: Some("interactive".to_string()),
                required_backends: Vec::new(),
                required_models: Vec::new(),
            },
            &[
                WorkflowSessionRuntimeUnloadCandidate {
                    session_id: "session-a".to_string(),
                    workflow_id: "wf-a".to_string(),
                    usage_profile: Some("interactive".to_string()),
                    required_backends: Vec::new(),
                    required_models: Vec::new(),
                    keep_alive: true,
                    access_tick: 1,
                    run_count: 0,
                },
                WorkflowSessionRuntimeUnloadCandidate {
                    session_id: "session-b".to_string(),
                    workflow_id: "wf-b".to_string(),
                    usage_profile: Some("batch".to_string()),
                    required_backends: Vec::new(),
                    required_models: Vec::new(),
                    keep_alive: false,
                    access_tick: 99,
                    run_count: 5,
                },
            ],
        )
        .await
        .expect("select unload candidate")
        .expect("candidate should exist");

    assert_eq!(selected.session_id, "session-b");
}

#[tokio::test]
async fn workflow_preflight_reports_candle_runtime_as_available() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::with_backend(
            Box::new(inference::CandleBackend::new()),
            "Candle",
        )),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let response = runtime
        .workflow_preflight(WorkflowPreflightRequest {
            workflow_id: "runtime-text".to_string(),
            inputs: Vec::new(),
            output_targets: None,
            override_selection: None,
        })
        .await
        .expect("workflow preflight");

    assert!(response.blocking_runtime_issues.is_empty());
    assert!(response.can_run);

    let capabilities = runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: "runtime-text".to_string(),
        })
        .await
        .expect("workflow capabilities");
    assert_eq!(
        capabilities.runtime_requirements.required_backends,
        vec!["candle".to_string()]
    );
    let candle = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.runtime_id == "candle")
        .expect("candle capability");
    assert_eq!(candle.source_kind, WorkflowRuntimeSourceKind::Host);
    assert!(candle.selected);
}

#[tokio::test]
async fn workflow_preflight_blocks_selected_runtime_failed_after_restart() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");
    rewrite_test_workflow_required_backend(temp.path(), "runtime-text", "llama_cpp");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);
    persist_failed_selected_runtime_version(&app_data_dir, "b8248", "validation failed");

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let capabilities = runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: "runtime-text".to_string(),
        })
        .await
        .expect("workflow capabilities");
    assert_eq!(
        capabilities.runtime_requirements.required_backends,
        vec!["llama_cpp".to_string()]
    );
    let runtime_capability = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.runtime_id == "llama_cpp")
        .expect("llama.cpp capability");
    assert_eq!(
        runtime_capability.readiness_state,
        Some(pantograph_workflow_service::WorkflowRuntimeReadinessState::Failed)
    );
    assert!(!runtime_capability.configured);
    assert_eq!(
        runtime_capability.unavailable_reason.as_deref(),
        Some("validation failed")
    );

    let preflight = runtime
        .workflow_preflight(WorkflowPreflightRequest {
            workflow_id: "runtime-text".to_string(),
            inputs: Vec::new(),
            output_targets: None,
            override_selection: None,
        })
        .await
        .expect("workflow preflight");
    assert!(!preflight.can_run);
    assert!(
        preflight
            .blocking_runtime_issues
            .iter()
            .any(|issue| issue.message.contains("validation failed"))
    );

    let error = runtime
        .workflow_run(WorkflowRunRequest {
            workflow_id: "runtime-text".to_string(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            run_id: Some("failed-selected-runtime".to_string()),
        })
        .await
        .expect_err("workflow run should fail when selected runtime failed validation");
    assert!(matches!(error, WorkflowServiceError::RuntimeNotReady(_)));
}

#[tokio::test]
async fn workflow_preflight_blocks_interrupted_runtime_job_after_restart() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");
    rewrite_test_workflow_required_backend(temp.path(), "runtime-text", "llama_cpp");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);
    persist_interrupted_runtime_job(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let capabilities = runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: "runtime-text".to_string(),
        })
        .await
        .expect("workflow capabilities");
    assert_eq!(
        capabilities.runtime_requirements.required_backends,
        vec!["llama_cpp".to_string()]
    );
    let runtime_capability = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.runtime_id == "llama_cpp")
        .expect("llama.cpp capability");
    assert_eq!(
        runtime_capability.readiness_state,
        Some(pantograph_workflow_service::WorkflowRuntimeReadinessState::Failed)
    );
    assert!(!runtime_capability.configured);
    assert!(
        runtime_capability
            .unavailable_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("reconciled during startup"))
    );

    let preflight = runtime
        .workflow_preflight(WorkflowPreflightRequest {
            workflow_id: "runtime-text".to_string(),
            inputs: Vec::new(),
            output_targets: None,
            override_selection: None,
        })
        .await
        .expect("workflow preflight");
    assert!(!preflight.can_run);
    assert!(
        preflight
            .blocking_runtime_issues
            .iter()
            .any(|issue| issue.message.contains("reconciled during startup"))
    );

    let error = runtime
        .workflow_run(WorkflowRunRequest {
            workflow_id: "runtime-text".to_string(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            run_id: Some("interrupted-runtime-job".to_string()),
        })
        .await
        .expect_err("workflow run should fail when restart reconciles an interrupted runtime job");
    assert!(matches!(error, WorkflowServiceError::RuntimeNotReady(_)));
}

#[tokio::test]
async fn hosted_runtime_constructor_syncs_registry_and_derives_capabilities_from_mode_info() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let mode_info = HostRuntimeModeSnapshot::from_mode_info(&inference::ServerModeInfo {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        mode: "sidecar_inference".to_string(),
        ready: true,
        url: Some("http://127.0.0.1:11434".to_string()),
        model_path: None,
        is_embedding_mode: false,
        active_model_target: Some("/models/qwen.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-2".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-8".to_string()),
            warmup_started_at_ms: Some(11),
            warmup_completed_at_ms: Some(19),
            warmup_duration_ms: Some(8),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
    });
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
        Some(runtime_registry.clone()),
        Some(mode_info),
    )
    .await;

    let snapshot = runtime_registry.snapshot();
    assert_eq!(snapshot.runtimes.len(), 2);
    let active = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime");
    assert_eq!(active.status, RuntimeRegistryStatus::Ready);
    let embedding = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
        .expect("embedding runtime");
    assert_eq!(embedding.status, RuntimeRegistryStatus::Ready);

    let capabilities = runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: "runtime-text".to_string(),
        })
        .await
        .expect("workflow capabilities");
    assert!(
        capabilities
            .runtime_capabilities
            .iter()
            .any(|capability| capability.runtime_id == "llama.cpp.embedding")
    );
}

#[tokio::test]
async fn embedded_runtime_shutdown_reconciles_registry_to_stopped() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let mode_info = HostRuntimeModeSnapshot::from_mode_info(&inference::ServerModeInfo {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        mode: "sidecar_inference".to_string(),
        ready: true,
        url: Some("http://127.0.0.1:11434".to_string()),
        model_path: None,
        is_embedding_mode: false,
        active_model_target: Some("/models/qwen.gguf".to_string()),
        embedding_model_target: None,
        active_runtime: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-9".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime: None,
    });
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
        Some(runtime_registry.clone()),
        Some(mode_info),
    )
    .await;

    let ready_snapshot = runtime_registry.snapshot();
    let ready_runtime = ready_snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime should be registered before shutdown");
    assert_eq!(ready_runtime.status, RuntimeRegistryStatus::Ready);

    runtime.shutdown().await;

    let stopped_snapshot = runtime_registry.snapshot();
    let stopped_runtime = stopped_snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime should remain observable after shutdown");
    assert_eq!(stopped_runtime.status, RuntimeRegistryStatus::Stopped);
}

#[tokio::test]
async fn embedded_runtime_shutdown_marks_loaded_sessions_unloaded() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
        Some(runtime_registry),
        None,
    )
    .await;

    let session = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create workflow session");

    let status = runtime
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("session status before shutdown");
    assert_eq!(status.session.state, WorkflowSessionState::IdleLoaded);

    runtime.shutdown().await;

    let status = runtime
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: session.session_id,
        })
        .await
        .expect("session status after shutdown");
    assert_eq!(status.session.state, WorkflowSessionState::IdleUnloaded);
}

#[tokio::test]
async fn execute_edit_session_graph_reconciles_registry_after_restore() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");
    let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

    let pumas_api = Arc::new(
        pumas_library::PumasApi::builder(temp.path())
            .build()
            .await
            .expect("build pumas api"),
    );
    pumas_api
        .rebuild_model_index()
        .await
        .expect("rebuild model index");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let inference_model_path = temp.path().join("main.gguf");
    std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
    let mmproj_path = temp.path().join("main.mmproj");
    std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "llama.cpp",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&inference::BackendConfig {
            model_path: Some(inference_model_path.clone()),
            mmproj_path: Some(mmproj_path),
            ..inference::BackendConfig::default()
        })
        .await
        .expect("gateway should start in inference mode");

    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let initial_runtime_instance_id = host_runtime_mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_instance_id.clone())
        .expect("initial runtime instance id");

    let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
    extensions
        .write()
        .await
        .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        gateway.clone(),
        extensions,
        Arc::new(WorkflowService::new()),
        None,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await;

    let graph = edit_session_embedding_graph(&model_id);
    let session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");

    let outcome = runtime
        .execute_edit_session_graph(
            &session.session_id,
            &graph,
            inference::EmbeddingStartRequest {
                gguf_model_path: Some(embedding_model_path),
                ..inference::EmbeddingStartRequest::default()
            },
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("edit-session execution should restore runtime even when node demand fails");
    assert!(outcome.error.is_some());

    let restored_mode_info = gateway.mode_info().await;
    let restored_runtime_instance_id = restored_mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_instance_id.clone())
        .expect("restored runtime instance id");
    assert_ne!(
        restored_runtime_instance_id, initial_runtime_instance_id,
        "restore path should produce a fresh runtime instance for this regression check"
    );

    let snapshot = runtime_registry.snapshot();
    let registry_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime should remain registered after restore");
    assert_eq!(
        registry_runtime.runtime_instance_id.as_deref(),
        Some(restored_runtime_instance_id.as_str())
    );
    assert_eq!(registry_runtime.status, RuntimeRegistryStatus::Ready);
}

#[tokio::test]
async fn execute_edit_session_graph_restore_keeps_scheduler_runtime_registry_diagnostics_ready() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");
    let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

    let pumas_api = Arc::new(
        pumas_library::PumasApi::builder(temp.path())
            .build()
            .await
            .expect("build pumas api"),
    );
    pumas_api
        .rebuild_model_index()
        .await
        .expect("rebuild model index");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let inference_model_path = temp.path().join("main.gguf");
    std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
    let mmproj_path = temp.path().join("main.mmproj");
    std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "llama.cpp",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&inference::BackendConfig {
            model_path: Some(inference_model_path),
            mmproj_path: Some(mmproj_path),
            ..inference::BackendConfig::default()
        })
        .await
        .expect("gateway should start in inference mode");

    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
    extensions
        .write()
        .await
        .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        gateway.clone(),
        extensions,
        Arc::new(WorkflowService::with_capacity_limits(4, 1)),
        None,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await;

    let graph = edit_session_embedding_graph(&model_id);
    let edit_session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");

    let outcome = runtime
        .execute_edit_session_graph(
            &edit_session.session_id,
            &graph,
            inference::EmbeddingStartRequest {
                gguf_model_path: Some(embedding_model_path),
                ..inference::EmbeddingStartRequest::default()
            },
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("edit-session execution should restore runtime even when node demand fails");
    assert!(outcome.error.is_some());

    let restored_runtime_instance_id = gateway
        .mode_info()
        .await
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_instance_id.clone())
        .expect("restored runtime instance id");
    let restored_runtime = runtime_registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("restored runtime should remain registered");
    assert_eq!(restored_runtime.status, RuntimeRegistryStatus::Ready);
    assert_eq!(
        restored_runtime.runtime_instance_id.as_deref(),
        Some(restored_runtime_instance_id.as_str())
    );

    let loaded = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create loaded session");

    let diagnostics_provider = EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
        gateway.clone(),
        runtime_registry.clone(),
    );
    let diagnostics = diagnostics_provider
        .scheduler_runtime_registry_diagnostics(&WorkflowSchedulerRuntimeDiagnosticsRequest {
            session_id: "queued-session".to_string(),
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: false,
            runtime_loaded: false,
            next_admission_queue_id: Some("queue-after-restore".to_string()),
            reclaim_candidates: vec![WorkflowSessionRuntimeUnloadCandidate {
                session_id: loaded.session_id.clone(),
                workflow_id: "runtime-text".to_string(),
                usage_profile: Some("interactive".to_string()),
                required_backends: Vec::new(),
                required_models: Vec::new(),
                keep_alive: true,
                access_tick: 1,
                run_count: 0,
            }],
        })
        .await
        .expect("scheduler diagnostics provider should succeed")
        .expect("runtime registry diagnostics should be present");

    assert_eq!(
        diagnostics,
        WorkflowSchedulerRuntimeRegistryDiagnostics {
            target_runtime_id: Some("llama_cpp".to_string()),
            reclaim_candidate_session_id: Some(loaded.session_id),
            reclaim_candidate_runtime_id: Some("llama_cpp".to_string()),
            next_warmup_decision: Some(WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime,),
            next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady),
        }
    );
}

#[tokio::test]
async fn execute_edit_session_graph_reconciles_registry_after_embedding_prepare() {
    let temp = TempDir::new().expect("temp dir");
    let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

    let pumas_api = Arc::new(
        pumas_library::PumasApi::builder(temp.path())
            .build()
            .await
            .expect("build pumas api"),
    );
    pumas_api
        .rebuild_model_index()
        .await
        .expect("rebuild model index");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let inference_model_path = temp.path().join("main.gguf");
    std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
    let mmproj_path = temp.path().join("main.mmproj");
    std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "llama.cpp",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&inference::BackendConfig {
            model_path: Some(inference_model_path.clone()),
            mmproj_path: Some(mmproj_path),
            ..inference::BackendConfig::default()
        })
        .await
        .expect("gateway should start in inference mode");

    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let initial_runtime_instance_id = host_runtime_mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_instance_id.clone())
        .expect("initial runtime instance id");

    let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
    extensions
        .write()
        .await
        .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        gateway.clone(),
        extensions,
        Arc::new(WorkflowService::new()),
        None,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await;

    let started_snapshot = Arc::new(Mutex::new(None::<RuntimeRegistrySnapshot>));
    let started_snapshot_sink = started_snapshot.clone();
    let runtime_registry_for_sink = runtime_registry.clone();
    let event_sink = Arc::new(node_engine::CallbackEventSink::new(move |event| {
        if matches!(event, node_engine::WorkflowEvent::WorkflowStarted { .. }) {
            let mut guard = started_snapshot_sink
                .lock()
                .expect("started snapshot lock poisoned");
            *guard = Some(runtime_registry_for_sink.snapshot());
        }
    }));

    let graph = edit_session_embedding_graph(&model_id);
    let session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");

    let outcome = runtime
        .execute_edit_session_graph(
            &session.session_id,
            &graph,
            inference::EmbeddingStartRequest {
                gguf_model_path: Some(embedding_model_path),
                ..inference::EmbeddingStartRequest::default()
            },
            event_sink,
        )
        .await
        .expect("edit-session execution should still finish");
    assert!(outcome.error.is_some());

    let started_snapshot = started_snapshot
        .lock()
        .expect("started snapshot lock poisoned")
        .clone()
        .expect("workflow started snapshot");
    let started_runtime = started_snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime snapshot at workflow start");
    assert_eq!(started_runtime.status, RuntimeRegistryStatus::Ready);
    assert_ne!(
        started_runtime.runtime_instance_id.as_deref(),
        Some(initial_runtime_instance_id.as_str()),
        "registry should be refreshed to the prepared embedding runtime before execution starts"
    );
}

#[tokio::test]
async fn execute_edit_session_graph_reconciles_registry_after_failed_restore() {
    let temp = TempDir::new().expect("temp dir");
    let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

    let pumas_api = Arc::new(
        pumas_library::PumasApi::builder(temp.path())
            .build()
            .await
            .expect("build pumas api"),
    );
    pumas_api
        .rebuild_model_index()
        .await
        .expect("rebuild model index");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let inference_model_path = temp.path().join("main.gguf");
    std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
    let mmproj_path = temp.path().join("main.mmproj");
    std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockRestoreFailureBackend {
            ready: false,
            inference_model_path: inference_model_path.clone(),
            embedding_model_path: embedding_model_path.clone(),
            embedding_started: false,
        }),
        "llama.cpp",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&inference::BackendConfig {
            model_path: Some(inference_model_path.clone()),
            mmproj_path: Some(mmproj_path),
            ..inference::BackendConfig::default()
        })
        .await
        .expect("gateway should start in inference mode");

    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
    extensions
        .write()
        .await
        .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        gateway.clone(),
        extensions,
        Arc::new(WorkflowService::new()),
        None,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await;

    let graph = edit_session_embedding_graph(&model_id);
    let session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");

    let outcome = runtime
        .execute_edit_session_graph(
            &session.session_id,
            &graph,
            inference::EmbeddingStartRequest {
                gguf_model_path: Some(embedding_model_path),
                ..inference::EmbeddingStartRequest::default()
            },
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("edit-session execution should still complete when restore fails");
    assert!(outcome.error.is_some());

    let mode_info = gateway.mode_info().await;
    let expected_observation = runtime_registry::active_runtime_observation(
        &HostRuntimeModeSnapshot::from_mode_info(&mode_info),
        true,
    )
    .expect("active runtime observation after failed restore");

    let snapshot = runtime_registry.snapshot();
    let registry_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == expected_observation.runtime_id)
        .expect("active runtime should remain observable after failed restore");
    assert_eq!(registry_runtime.status, expected_observation.status);
    assert_eq!(
        registry_runtime.runtime_instance_id,
        expected_observation.runtime_instance_id
    );
}

#[tokio::test]
async fn execute_edit_session_graph_reports_all_python_runtime_ids_in_trace_metrics() {
    let temp = TempDir::new().expect("temp dir");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
        Arc::new(MockImagePythonRuntime {
            requests: Mutex::new(Vec::new()),
        }),
    );

    let graph = multi_python_edit_session_graph();
    let session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");

    let outcome = runtime
        .execute_edit_session_graph(
            &session.session_id,
            &graph,
            inference::EmbeddingStartRequest::default(),
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("edit-session execution");

    assert_eq!(
        outcome.trace_runtime_metrics.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
    assert_eq!(
        outcome.trace_runtime_metrics.observed_runtime_ids,
        vec!["onnx-runtime".to_string(), "diffusers".to_string()]
    );
    assert_eq!(
        outcome.trace_runtime_metrics.model_target.as_deref(),
        Some("/tmp/mock-onnx-model")
    );
    assert_eq!(
        outcome.runtime_snapshot.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
    assert_eq!(
        outcome.runtime_model_target.as_deref(),
        Some("/tmp/mock-onnx-model")
    );
    assert!(!outcome.waiting_for_input);
}

#[tokio::test]
async fn execute_edit_session_graph_waiting_for_input_does_not_emit_workflow_failed() {
    let temp = TempDir::new().expect("temp dir");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
        Arc::new(ProcessPythonRuntimeAdapter),
    );

    let graph = WorkflowGraph {
        nodes: vec![GraphNode {
            id: "approval".to_string(),
            node_type: "human-input".to_string(),
            data: serde_json::json!({ "prompt": "Approve deployment?" }),
            position: Position::default(),
        }],
        edges: Vec::new(),
        derived_graph: None,
    };
    let session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");
    let event_sink = Arc::new(node_engine::VecEventSink::new());

    let outcome = runtime
        .execute_edit_session_graph(
            &session.session_id,
            &graph,
            inference::EmbeddingStartRequest::default(),
            event_sink.clone(),
        )
        .await
        .expect("edit-session execution should pause instead of failing");

    assert!(outcome.waiting_for_input);
    assert!(outcome.error.is_none());

    let events = event_sink.events();
    assert!(events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::WaitingForInput {
            task_id,
            prompt: Some(prompt),
            ..
        } if task_id == "approval" && prompt == "Approve deployment?"
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, node_engine::WorkflowEvent::WorkflowFailed { .. }))
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::WorkflowCompleted { .. }
            | node_engine::WorkflowEvent::WorkflowCancelled { .. }
    )));
}

#[tokio::test]
async fn workflow_capabilities_include_injected_runtime_capabilities() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_additional_runtime_capabilities(vec![WorkflowRuntimeCapability {
        runtime_id: "llama.cpp.embedding".to_string(),
        display_name: "Dedicated embedding runtime".to_string(),
        install_state: WorkflowRuntimeInstallState::Installed,
        available: true,
        configured: true,
        can_install: false,
        can_remove: false,
        source_kind: WorkflowRuntimeSourceKind::Host,
        selected: false,
        readiness_state: Some(pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready),
        selected_version: None,
        supports_external_connection: false,
        backend_keys: vec!["llama_cpp".to_string(), "llamacpp".to_string()],
        missing_files: Vec::new(),
        unavailable_reason: None,
    }]);

    let capabilities = runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: "runtime-text".to_string(),
        })
        .await
        .expect("workflow capabilities");

    let embedding_runtime = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.runtime_id == "llama.cpp.embedding")
        .expect("dedicated embedding capability");
    assert_eq!(
        embedding_runtime.source_kind,
        WorkflowRuntimeSourceKind::Host
    );
    assert!(!embedding_runtime.selected);
    assert!(embedding_runtime.available);
}

#[test]
fn reservation_requirements_returns_none_when_workflow_estimate_is_unknown() {
    assert_eq!(
        EmbeddedWorkflowHost::reservation_requirements(&WorkflowRuntimeRequirements::default()),
        None
    );
}

#[test]
fn reservation_requirements_maps_workflow_memory_estimates() {
    let requirements =
        EmbeddedWorkflowHost::reservation_requirements(&WorkflowRuntimeRequirements {
            estimated_peak_vram_mb: Some(2048),
            estimated_peak_ram_mb: Some(1024),
            estimated_min_vram_mb: Some(1536),
            estimated_min_ram_mb: Some(768),
            estimation_confidence: "estimated_from_model_sizes".to_string(),
            required_models: vec!["model-a".to_string()],
            required_backends: vec!["llama_cpp".to_string()],
            required_extensions: Vec::new(),
        })
        .expect("requirements should be forwarded when estimates exist");

    assert_eq!(requirements.estimated_peak_vram_mb, Some(2048));
    assert_eq!(requirements.estimated_peak_ram_mb, Some(1024));
    assert_eq!(requirements.estimated_min_vram_mb, Some(1536));
    assert_eq!(requirements.estimated_min_ram_mb, Some(768));
}

#[test]
fn runtime_registry_admission_errors_map_to_runtime_not_ready() {
    let error = runtime_registry_errors::workflow_service_error_from_runtime_registry(
        RuntimeRegistryError::AdmissionRejected {
            runtime_id: "pytorch".to_string(),
            failure: pantograph_runtime_registry::RuntimeAdmissionFailure::InsufficientRam {
                requested_mb: 1024,
                available_mb: 0,
                reserved_mb: 2048,
                total_mb: 2048,
                safety_margin_mb: 0,
            },
        },
    );

    assert!(matches!(error, WorkflowServiceError::RuntimeNotReady(_)));
    assert_eq!(
        error.code(),
        pantograph_workflow_service::WorkflowErrorCode::RuntimeNotReady
    );
}

#[test]
fn runtime_registry_owner_conflicts_map_to_invalid_request() {
    let error = runtime_registry_errors::workflow_service_error_from_runtime_registry(
        RuntimeRegistryError::ReservationOwnerConflict {
            owner_id: "session-a".to_string(),
            existing_runtime_id: "llama_cpp".to_string(),
            requested_runtime_id: "pytorch".to_string(),
        },
    );

    assert!(matches!(error, WorkflowServiceError::InvalidRequest(_)));
    assert_eq!(
        error.code(),
        pantograph_workflow_service::WorkflowErrorCode::InvalidRequest
    );
}

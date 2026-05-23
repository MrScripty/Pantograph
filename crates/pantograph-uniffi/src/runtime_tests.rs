use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use pantograph_managed_dependencies::{
    managed_redistributable_catalog_entry, ManagedRedistributableId,
};
use pantograph_workflow_service::{WorkflowErrorCode, WorkflowErrorEnvelope};

use super::{FfiEmbeddedRuntimeConfig, FfiPantographRuntime};
use crate::FfiError;

fn create_temp_root(workflow_id: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("pantograph-uniffi-runtime-tests-{suffix}"));
    write_test_workflow(&root, workflow_id);
    install_fake_default_runtime(&root.join("app-data"));
    root
}

fn install_fake_default_runtime(app_data_dir: &Path) {
    let runtime_dir = app_data_dir.join("runtimes").join("llama-cpp");
    std::fs::create_dir_all(&runtime_dir).expect("create fake runtime dir");

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let file_names = [
        "llama-server-x86_64-unknown-linux-gnu",
        "libllama.so",
        "libggml.so",
    ];
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let file_names = ["llama-server-aarch64-apple-darwin", "libllama.dylib"];
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let file_names = ["llama-server-x86_64-apple-darwin", "libllama.dylib"];
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    let file_names = [
        "llama-server-x86_64-pc-windows-msvc.exe",
        "llama-runtime.dll",
    ];

    for file_name in file_names {
        std::fs::write(runtime_dir.join(file_name), [])
            .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
    }
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
                        "definition": {
                            "category": "input",
                            "io_binding_origin": "client_session",
                            "label": "Text Input",
                            "description": "Provides text input",
                            "inputs": [{
                                "id": "text",
                                "label": "Text",
                                "data_type": "string",
                                "required": false,
                                "multiple": false
                            }],
                            "outputs": [{
                                "id": "legacy-out",
                                "label": "Legacy Out",
                                "data_type": "string",
                                "required": false,
                                "multiple": false
                            }]
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
                            "inputs": [{
                                "id": "text",
                                "label": "Text",
                                "data_type": "string",
                                "required": false,
                                "multiple": false
                            }],
                            "outputs": [{
                                "id": "text",
                                "label": "Text",
                                "data_type": "string",
                                "required": false,
                                "multiple": false
                            }]
                        }
                    },
                    "position": { "x": 200.0, "y": 0.0 }
                }
            ],
            "edges": [{
                "id": "e-text",
                "source": "text-input-1",
                "source_handle": "text",
                "target": "text-output-1",
                "target_handle": "text"
            }]
        }
    });
    std::fs::write(
        workflows_dir.join(format!("{workflow_id}.json")),
        serde_json::to_vec(&workflow_json).expect("serialize workflow"),
    )
    .expect("write workflow");
}

fn write_selector_ready_gguf_model(root: &Path) -> PathBuf {
    let model_dir = root.join("shared-resources/models/llm/imported/uniffi-test-gguf");
    std::fs::create_dir_all(&model_dir).expect("create pumas model dir");
    let model_file = model_dir.join("model.gguf");
    std::fs::write(&model_file, vec![0_u8; 256]).expect("write pumas model file");
    std::fs::write(
        model_dir.join("metadata.json"),
        serde_json::json!({
            "schema_version": 2,
            "model_id": "llm/imported/uniffi-test-gguf",
            "family": "imported",
            "model_type": "llm",
            "official_name": "uniffi-test-gguf",
            "cleaned_name": "uniffi-test-gguf",
            "source_path": model_dir.display().to_string(),
            "entry_path": model_file.display().to_string(),
            "storage_kind": "library_owned",
            "import_state": "ready",
            "validation_state": "valid",
            "task_type_primary": "text-generation",
            "recommended_backend": "llamacpp",
            "runtime_engine_hints": ["llamacpp"]
        })
        .to_string(),
    )
    .expect("write pumas model metadata");
    model_file
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
                            "inputs": [{
                                "id": "text",
                                "label": "Text",
                                "data_type": "string",
                                "required": false,
                                "multiple": false
                            }],
                            "outputs": [{
                                "id": "text",
                                "label": "Text",
                                "data_type": "string",
                                "required": false,
                                "multiple": false
                            }]
                        }
                    },
                    "position": { "x": 240.0, "y": 0.0 }
                }
            ],
            "edges": [{
                "id": "e-human-output",
                "source": "human-input-1",
                "source_handle": "value",
                "target": "text-output-1",
                "target_handle": "text"
            }]
        }
    });
    std::fs::write(
        workflows_dir.join(format!("{workflow_id}.json")),
        serde_json::to_vec(&workflow_json).expect("serialize workflow"),
    )
    .expect("write workflow");
}

fn workflow_error_envelope(err: FfiError) -> WorkflowErrorEnvelope {
    let message = match err {
        FfiError::Other { message } => message,
        other => panic!("expected FfiError::Other with envelope JSON, got {other:?}"),
    };
    serde_json::from_str(&message).expect("parse workflow error envelope")
}

#[tokio::test]
async fn direct_runtime_runs_workflow_session_from_json() {
    let workflow_id = "uniffi-runtime-text";
    let root = create_temp_root(workflow_id);

    let runtime = FfiPantographRuntime::new(
        FfiEmbeddedRuntimeConfig {
            app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
            project_root: root.to_string_lossy().into_owned(),
            workflow_roots: Vec::new(),
            max_loaded_sessions: None,
        },
        None,
    )
    .await
    .expect("runtime");

    let create_session_json = runtime
        .workflow_create_session(
            serde_json::json!({
                "workflow_id": workflow_id,
                "keep_alive": true
            })
            .to_string(),
        )
        .await
        .expect("create execution session");
    let create_session: serde_json::Value =
        serde_json::from_str(&create_session_json).expect("parse create session");
    let session_id = create_session["session_id"]
        .as_str()
        .expect("execution session id");

    let session_run_json = runtime
        .workflow_run_session(
            serde_json::json!({
                "session_id": session_id,
                "workflow_semantic_version": "0.1.0",
                "inputs": [{
                    "node_id": "text-input-1",
                    "port_id": "text",
                    "value": "session run"
                }],
                "output_targets": [{
                    "node_id": "text-output-1",
                    "port_id": "text"
                }]
            })
            .to_string(),
        )
        .await
        .expect("run execution session");
    let session_run: serde_json::Value =
        serde_json::from_str(&session_run_json).expect("parse session run");
    assert_eq!(session_run["outputs"][0]["value"], "session run");

    let status_json = runtime
        .workflow_get_session_status(serde_json::json!({ "session_id": session_id }).to_string())
        .await
        .expect("session status");
    let status: serde_json::Value = serde_json::from_str(&status_json).expect("parse status");
    assert_eq!(status["session"]["workflow_id"], workflow_id);

    let queue_json = runtime
        .workflow_list_session_queue(serde_json::json!({ "session_id": session_id }).to_string())
        .await
        .expect("session queue");
    let queue: serde_json::Value = serde_json::from_str(&queue_json).expect("parse queue");
    assert!(queue["items"].as_array().expect("queue items").is_empty());

    let keep_alive_json = runtime
        .workflow_set_session_keep_alive(
            serde_json::json!({
                "session_id": session_id,
                "keep_alive": false
            })
            .to_string(),
        )
        .await
        .expect("set keep alive");
    let keep_alive: serde_json::Value =
        serde_json::from_str(&keep_alive_json).expect("parse keep alive");
    assert_eq!(keep_alive["keep_alive"], false);

    let close_json = runtime
        .workflow_close_session(serde_json::json!({ "session_id": session_id }).to_string())
        .await
        .expect("close execution session");
    let close: serde_json::Value = serde_json::from_str(&close_json).expect("parse close");
    assert_eq!(close["ok"], true);

    runtime.shutdown().await;

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn direct_runtime_workflow_session_run_preserves_invalid_request_envelope() {
    let workflow_id = "uniffi-runtime-interactive-run";
    let root = create_temp_root(workflow_id);
    write_human_input_workflow(&root, workflow_id);

    let runtime = FfiPantographRuntime::new(
        FfiEmbeddedRuntimeConfig {
            app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
            project_root: root.to_string_lossy().into_owned(),
            workflow_roots: Vec::new(),
            max_loaded_sessions: None,
        },
        None,
    )
    .await
    .expect("runtime");

    let create_session_json = runtime
        .workflow_create_session(
            serde_json::json!({
                "workflow_id": workflow_id
            })
            .to_string(),
        )
        .await
        .expect("create execution session");
    let create_session: serde_json::Value =
        serde_json::from_str(&create_session_json).expect("parse create session");
    let session_id = create_session["session_id"]
        .as_str()
        .expect("execution session id");

    let err = runtime
        .workflow_run_session(
            serde_json::json!({
                "session_id": session_id,
                "workflow_semantic_version": "0.1.0",
                "inputs": [],
                "output_targets": [{
                    "node_id": "text-output-1",
                    "port_id": "text"
                }]
            })
            .to_string(),
        )
        .await
        .expect_err("interactive workflow session run should preserve invalid-request envelope");

    let envelope = workflow_error_envelope(err);
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert_eq!(
        envelope.message,
        "workflow 'uniffi-runtime-interactive-run' requires interactive input at node 'human-input-1'"
    );

    runtime.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn direct_runtime_exposes_attribution_client_session_json() {
    let workflow_id = "uniffi-runtime-attributed-text";
    let root = create_temp_root(workflow_id);

    let runtime = FfiPantographRuntime::new(
        FfiEmbeddedRuntimeConfig {
            app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
            project_root: root.to_string_lossy().into_owned(),
            workflow_roots: Vec::new(),
            max_loaded_sessions: None,
        },
        None,
    )
    .await
    .expect("runtime");

    let registration_json = runtime
        .workflow_register_attribution_client(
            serde_json::json!({
                "display_name": "UniFFI attributed client",
                "metadata_json": null
            })
            .to_string(),
        )
        .expect("register attribution client");
    let registration: serde_json::Value =
        serde_json::from_str(&registration_json).expect("parse registration");
    let credential_id = registration["credential"]["client_credential_id"]
        .as_str()
        .expect("credential id");
    let credential_secret = registration["credential_secret"]
        .as_str()
        .expect("credential secret");

    let open_session_json = runtime
        .workflow_open_client_session(
            serde_json::json!({
                "credential": {
                    "credential_id": credential_id,
                    "secret": credential_secret
                },
                "takeover": false,
                "reason": "test launch"
            })
            .to_string(),
        )
        .expect("open client session");
    let opened: serde_json::Value =
        serde_json::from_str(&open_session_json).expect("parse open response");
    assert!(opened["session"]["client_session_id"].as_str().is_some());

    runtime.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn direct_runtime_exposes_workflow_graph_persistence_and_edit_session() {
    let root = create_temp_root("uniffi-runtime-unused");
    let runtime = FfiPantographRuntime::new(
        FfiEmbeddedRuntimeConfig {
            app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
            project_root: root.to_string_lossy().into_owned(),
            workflow_roots: Vec::new(),
            max_loaded_sessions: None,
        },
        None,
    )
    .await
    .expect("runtime");

    let graph = serde_json::json!({
        "nodes": [{
            "id": "text-input-1",
            "node_type": "text-input",
            "position": { "x": 0.0, "y": 0.0 },
            "data": { "text": "draft" }
        }],
        "edges": []
    });
    let save_response_json = runtime
        .workflow_graph_save(
            serde_json::json!({
                "name": "native-edited-workflow",
                "graph": graph
            })
            .to_string(),
        )
        .expect("save workflow graph");
    let save_response: serde_json::Value =
        serde_json::from_str(&save_response_json).expect("parse save response");
    let path = save_response["path"].as_str().expect("saved path");

    let list_response_json = runtime.workflow_graph_list().expect("list workflow graphs");
    let list_response: serde_json::Value =
        serde_json::from_str(&list_response_json).expect("parse list response");
    assert!(list_response["workflows"]
        .as_array()
        .expect("workflows")
        .iter()
        .any(|metadata| metadata["id"] == "native-edited-workflow"));

    let load_response_json = runtime
        .workflow_graph_load(serde_json::json!({ "path": path }).to_string())
        .expect("load workflow graph");
    let load_response: serde_json::Value =
        serde_json::from_str(&load_response_json).expect("parse load response");
    assert_eq!(load_response["metadata"]["name"], "native-edited-workflow");

    let create_response_json = runtime
        .workflow_graph_create_edit_session(
            serde_json::json!({
                "graph": load_response["graph"]
            })
            .to_string(),
        )
        .await
        .expect("create graph edit session");
    let create_response: serde_json::Value =
        serde_json::from_str(&create_response_json).expect("parse create response");
    let edit_session_id = create_response["session_id"]
        .as_str()
        .expect("edit session id");

    let update_response_json = runtime
        .workflow_graph_update_node_data(
            serde_json::json!({
                "session_id": edit_session_id,
                "node_id": "text-input-1",
                "data": { "text": "native edit" }
            })
            .to_string(),
        )
        .await
        .expect("update node data");
    let update_response: serde_json::Value =
        serde_json::from_str(&update_response_json).expect("parse update response");
    assert_eq!(
        update_response["graph"]["nodes"][0]["data"]["text"],
        "native edit"
    );
    assert_eq!(update_response["workflow_event"]["type"], "graphModified");
    assert_eq!(
        update_response["workflow_event"]["dirtyTasks"],
        serde_json::json!(["text-input-1"])
    );

    let undo_state_json = runtime
        .workflow_graph_get_undo_redo_state(
            serde_json::json!({ "session_id": edit_session_id }).to_string(),
        )
        .await
        .expect("undo-redo state");
    let undo_state: serde_json::Value =
        serde_json::from_str(&undo_state_json).expect("parse undo-redo state");
    assert_eq!(undo_state["can_undo"], true);

    let undo_response_json = runtime
        .workflow_graph_undo(serde_json::json!({ "session_id": edit_session_id }).to_string())
        .await
        .expect("undo graph edit");
    let undo_response: serde_json::Value =
        serde_json::from_str(&undo_response_json).expect("parse undo response");
    assert_eq!(undo_response["graph"]["nodes"][0]["data"]["text"], "draft");
    assert_eq!(undo_response["workflow_event"]["type"], "graphModified");

    runtime
        .workflow_graph_close_edit_session(
            serde_json::json!({ "session_id": edit_session_id }).to_string(),
        )
        .await
        .expect("close graph edit session");
    runtime.shutdown().await;

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn direct_runtime_exposes_backend_owned_graph_authoring_discovery() {
    let root = create_temp_root("uniffi-runtime-discovery");
    let runtime = FfiPantographRuntime::new(
        FfiEmbeddedRuntimeConfig {
            app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
            project_root: root.to_string_lossy().into_owned(),
            workflow_roots: Vec::new(),
            max_loaded_sessions: None,
        },
        None,
    )
    .await
    .expect("runtime");

    let definitions_json = runtime
        .workflow_graph_list_node_definitions()
        .expect("list node definitions");
    let definitions: serde_json::Value =
        serde_json::from_str(&definitions_json).expect("parse definitions");
    assert!(definitions
        .as_array()
        .expect("definitions")
        .iter()
        .any(|definition| definition["node_type"] == "text-input"));

    let text_input_json = runtime
        .workflow_graph_get_node_definition("text-input".to_string())
        .expect("get text-input definition");
    let text_input: serde_json::Value =
        serde_json::from_str(&text_input_json).expect("parse text-input definition");
    assert_eq!(text_input["category"], "input");
    assert!(text_input["outputs"]
        .as_array()
        .expect("outputs")
        .iter()
        .any(|port| port["id"] == "text"));

    let grouped_json = runtime
        .workflow_graph_get_node_definitions_by_category()
        .expect("group node definitions");
    let grouped: serde_json::Value =
        serde_json::from_str(&grouped_json).expect("parse grouped definitions");
    assert!(grouped["input"]
        .as_array()
        .expect("input category")
        .iter()
        .any(|definition| definition["node_type"] == "text-input"));

    let queryable_json = runtime
        .workflow_graph_get_queryable_ports()
        .expect("queryable ports");
    let queryable: serde_json::Value =
        serde_json::from_str(&queryable_json).expect("parse queryable ports");
    assert!(queryable
        .as_array()
        .expect("queryable ports")
        .iter()
        .any(|port| port["node_type"] == "puma-lib" && port["port_id"] == "model_path"));

    let missing = runtime
        .workflow_graph_get_node_definition("missing-node".to_string())
        .expect_err("unknown node type should be rejected");
    let envelope = workflow_error_envelope(missing);
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert_eq!(envelope.message, "unknown node_type 'missing-node'");

    let missing_options = runtime
        .workflow_graph_query_port_options(
            "text-input".to_string(),
            "text".to_string(),
            "{}".to_string(),
        )
        .await
        .expect_err("non-queryable port should be rejected");
    let envelope = workflow_error_envelope(missing_options);
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert!(envelope
        .message
        .contains("No options provider for text-input:text"));

    runtime.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn direct_runtime_puma_lib_options_use_selector_access_from_pumas_api() {
    let workflow_id = "uniffi-runtime-puma-options";
    let root = create_temp_root(workflow_id);
    let model_file = write_selector_ready_gguf_model(&root);
    let api = Arc::new(
        pumas_library::PumasApi::builder(&root)
            .auto_create_dirs(true)
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .expect("pumas api should build"),
    );
    api.rebuild_model_index()
        .await
        .expect("pumas index should rebuild");

    let runtime = FfiPantographRuntime::new(
        FfiEmbeddedRuntimeConfig {
            app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
            project_root: root.to_string_lossy().into_owned(),
            workflow_roots: Vec::new(),
            max_loaded_sessions: None,
        },
        Some(Arc::new(crate::FfiPumasApi {
            api: api.clone(),
            selector_access: Arc::new(workflow_nodes::setup::PumasSelectorAccess::Owner(api)),
        })),
    )
    .await
    .expect("runtime should initialize");

    let options_json = runtime
        .workflow_graph_query_port_options(
            "puma-lib".to_string(),
            "model_path".to_string(),
            serde_json::json!({
                "limit": 10,
                "context": {
                    "targetNodeId": "puma-lib-1",
                    "taskKind": "image_generation",
                    "selectedModelRef": "pumas://models/llm/imported/uniffi-test-gguf",
                    "packageFactsSummaryCursor": "model-library-updates:1",
                    "requestedRuntimeId": "pytorch",
                    "requestedDeviceId": "cpu"
                }
            })
            .to_string(),
        )
        .await
        .expect("puma-lib options should use selector access");
    let result: serde_json::Value =
        serde_json::from_str(&options_json).expect("parse puma-lib options");
    let option = result["options"]
        .as_array()
        .expect("options should be an array")
        .iter()
        .find(|option| option["metadata"]["id"] == "llm/imported/uniffi-test-gguf")
        .expect("selector option should be present");

    assert_eq!(
        option["value"],
        serde_json::json!(model_file.display().to_string())
    );
    assert!(result["metadata"]["package_facts_summary_cursor"]
        .as_str()
        .is_some_and(|cursor| cursor.starts_with("model-library-updates:")));

    runtime.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn direct_runtime_exposes_artifact_store_contract_surface() {
    let root = create_temp_root("uniffi-runtime-artifacts");
    let runtime = FfiPantographRuntime::new(
        FfiEmbeddedRuntimeConfig {
            app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
            project_root: root.to_string_lossy().into_owned(),
            workflow_roots: Vec::new(),
            max_loaded_sessions: None,
        },
        None,
    )
    .await
    .expect("runtime");

    let policy_json = runtime
        .workflow_artifact_policy()
        .expect("artifact policy is available");
    let policy: serde_json::Value = serde_json::from_str(&policy_json).expect("parse policy");
    assert_eq!(policy["policy_id"], "artifact-global-default");
    assert_eq!(policy["delete_on_consume"], false);

    let updated_policy = serde_json::json!({
        "policy_id": "artifact-global-test",
        "policy_version": 2,
        "ttl_seconds": 60,
        "max_disk_bytes": 4096,
        "max_memory_bytes": 2048,
        "max_single_artifact_bytes": 1024,
        "spill_threshold_bytes": 512,
        "delete_on_consume": true
    });
    let updated_json = runtime
        .workflow_update_artifact_policy(updated_policy.to_string())
        .expect("update artifact policy");
    let updated: serde_json::Value =
        serde_json::from_str(&updated_json).expect("parse updated policy");
    assert_eq!(updated["policy_id"], "artifact-global-test");
    assert_eq!(updated["delete_on_consume"], true);

    let stats_json = runtime
        .workflow_artifact_store_stats()
        .expect("artifact store stats");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).expect("parse stats");
    assert_eq!(stats["artifact_count"], 0);
    assert_eq!(stats["retained_body_bytes"], 0);

    let missing = runtime
        .workflow_artifact_descriptor(
            serde_json::json!({ "artifact_id": "missing-artifact" }).to_string(),
        )
        .expect_err("missing artifact should be rejected");
    let envelope = workflow_error_envelope(missing);
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert!(envelope.message.contains("artifact not found"));

    runtime.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn direct_runtime_exposes_artifact_format_settings_and_capabilities_json() {
    let root = create_temp_root("uniffi-runtime-artifact-format-settings");
    let runtime = FfiPantographRuntime::new(
        FfiEmbeddedRuntimeConfig {
            app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
            project_root: root.to_string_lossy().into_owned(),
            workflow_roots: Vec::new(),
            max_loaded_sessions: None,
        },
        None,
    )
    .await
    .expect("runtime");

    let defaults_json = runtime
        .workflow_artifact_format_settings("{}".to_string())
        .expect("artifact format settings defaults");
    let defaults: serde_json::Value = serde_json::from_str(&defaults_json).expect("parse defaults");
    assert_eq!(defaults["settings"]["image"]["format_id"], "jpg");
    assert_eq!(defaults["settings"]["image"]["quality_percent"], 75);
    assert_eq!(defaults["settings"]["audio"]["container_id"], "ogg");
    assert_eq!(defaults["settings"]["audio"]["codec_id"], "opus");
    assert_eq!(defaults["settings"]["video"]["codec_id"], "svt_av1");
    assert_eq!(defaults["settings"]["video"]["crf"], 32);
    assert_eq!(defaults["settings"]["three_d"]["format_id"], "glb");

    let updated_settings = serde_json::json!({
        "image": {
            "format_id": "png",
            "quality_percent": 75,
            "color_profile_id": "srgb"
        },
        "audio": {
            "container_id": "ogg",
            "codec_id": "vorbis",
            "bitrate_kbps": 128
        },
        "video": {
            "container_id": "ivf",
            "codec_id": "svt_av1",
            "crf": 28,
            "bit_depth": "10bit"
        },
        "three_d": {
            "format_id": "obj"
        }
    });
    let update_json = runtime
        .workflow_update_artifact_format_settings(
            serde_json::json!({
                "settings": updated_settings,
                "reason": "uniffi runtime test"
            })
            .to_string(),
        )
        .expect("update artifact format settings");
    let update: serde_json::Value = serde_json::from_str(&update_json).expect("parse update");
    assert_eq!(update["settings"]["image"]["format_id"], "png");
    assert_eq!(update["settings"]["audio"]["codec_id"], "vorbis");
    assert_eq!(update["settings"]["video"]["bit_depth"], "10bit");
    assert_eq!(update["settings"]["three_d"]["format_id"], "obj");

    let persisted_json = runtime
        .workflow_artifact_format_settings("{}".to_string())
        .expect("artifact format settings after update");
    let persisted: serde_json::Value =
        serde_json::from_str(&persisted_json).expect("parse persisted settings");
    assert_eq!(persisted["settings"], update["settings"]);

    let capabilities_json = runtime
        .workflow_artifact_format_capabilities()
        .expect("artifact format capabilities");
    let capabilities: serde_json::Value =
        serde_json::from_str(&capabilities_json).expect("parse capabilities");
    assert!(capabilities["image_formats"]
        .as_array()
        .expect("image formats")
        .iter()
        .any(|option| option["format_id"] == "jpg"
            && option["provided_by_dependency_id"] == "oiiotool"));
    assert!(capabilities["audio_formats"]
        .as_array()
        .expect("audio formats")
        .iter()
        .any(|option| option["format_id"] == "ogg"
            && option["codec_ids"]
                .as_array()
                .expect("audio codecs")
                .iter()
                .any(|codec| codec == "opus")));
    assert!(capabilities["video_formats"]
        .as_array()
        .expect("video formats")
        .iter()
        .any(|option| option["codec_ids"]
            .as_array()
            .expect("video codecs")
            .iter()
            .any(|codec| codec == "svt_av1")));
    assert!(capabilities["three_d_formats"]
        .as_array()
        .expect("3d formats")
        .iter()
        .any(|option| option["format_id"] == "glb"));

    let invalid = runtime
        .workflow_update_artifact_format_settings(
            serde_json::json!({
                "settings": {
                    "image": {
                        "format_id": "jpg",
                        "quality_percent": 0,
                        "color_profile_id": "srgb"
                    },
                    "audio": {
                        "container_id": "ogg",
                        "codec_id": "opus",
                        "bitrate_kbps": 96
                    },
                    "video": {
                        "container_id": "ivf",
                        "codec_id": "svt_av1",
                        "crf": 32,
                        "bit_depth": "8bit"
                    },
                    "three_d": {
                        "format_id": "glb"
                    }
                },
                "reason": "invalid setting test"
            })
            .to_string(),
        )
        .expect_err("invalid format setting should preserve workflow error envelope");
    let envelope = workflow_error_envelope(invalid);
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert_eq!(
        envelope.message,
        "image quality_percent 0 is outside allowed range"
    );

    runtime.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn direct_runtime_exposes_managed_media_dependency_statuses_and_actions_json() {
    let root = create_temp_root("uniffi-runtime-managed-media");
    let app_data_dir = root.join("app-data");
    let runtime = FfiPantographRuntime::new(
        FfiEmbeddedRuntimeConfig {
            app_data_dir: app_data_dir.to_string_lossy().into_owned(),
            project_root: root.to_string_lossy().into_owned(),
            workflow_roots: Vec::new(),
            max_loaded_sessions: None,
        },
        None,
    )
    .await
    .expect("create runtime");

    let statuses_json = runtime
        .managed_media_dependency_statuses()
        .expect("list managed media dependency statuses");
    let statuses: serde_json::Value =
        serde_json::from_str(&statuses_json).expect("parse managed media dependency statuses");
    let statuses = statuses.as_array().expect("statuses array");
    for dependency_id in ["ffmpeg", "ocioconvert", "oiiotool", "open_color_io"] {
        assert!(
            statuses
                .iter()
                .any(|status| status["id"] == serde_json::json!(dependency_id)),
            "missing managed media dependency status for {dependency_id}"
        );
    }

    let neutral_statuses_json = runtime
        .managed_dependency_statuses()
        .expect("list neutral managed dependency statuses");
    let neutral_statuses: serde_json::Value = serde_json::from_str(&neutral_statuses_json)
        .expect("parse neutral managed dependency statuses");
    let neutral_statuses = neutral_statuses.as_array().expect("neutral statuses array");
    assert!(neutral_statuses.iter().any(|status| {
        status["key"]["runtime_sidecar"] == serde_json::json!("llama_cpp")
            && status["category"] == serde_json::json!("runtime_sidecar")
            && status["readiness_state"] == serde_json::json!("ready")
    }));
    assert!(neutral_statuses.iter().any(|status| {
        status["key"]["media_tool"] == serde_json::json!("ffmpeg")
            && status["category"] == serde_json::json!("media_tool")
    }));
    assert!(neutral_statuses.iter().any(|status| {
        status["key"]["native_artifact"] == serde_json::json!("open_color_io")
            && status["category"] == serde_json::json!("native_artifact")
    }));

    for id in [
        ManagedRedistributableId::Ffmpeg,
        ManagedRedistributableId::Ocioconvert,
        ManagedRedistributableId::Oiiotool,
        ManagedRedistributableId::OpenColorIo,
    ] {
        let catalog = managed_redistributable_catalog_entry(id);
        if catalog.platform_key == "unsupported" {
            continue;
        }

        let dependency_id = serde_json::to_value(id).expect("serialize dependency id");
        let staging_dir = root
            .join("staging")
            .join(catalog.id.key())
            .join(&catalog.version);
        write_managed_media_expected_files(&staging_dir, &catalog.expected_files);

        let install_json = runtime
            .managed_media_dependency_install_from_staging(
                serde_json::json!({
                    "dependency_id": dependency_id,
                    "version": catalog.version,
                    "staging_dir": staging_dir.to_string_lossy()
                })
                .to_string(),
            )
            .expect("install managed media dependency from staging");
        let install: serde_json::Value =
            serde_json::from_str(&install_json).expect("parse install response");
        assert_eq!(install["status"]["id"], dependency_id);
        assert_eq!(install["status"]["readiness"], "ready");
        assert_eq!(install["status"]["missing_files"], serde_json::json!([]));

        let selected_json = runtime
            .managed_media_dependency_select_version(
                serde_json::json!({
                    "dependency_id": dependency_id,
                    "version": catalog.version
                })
                .to_string(),
            )
            .expect("select managed media dependency version");
        let selected: serde_json::Value =
            serde_json::from_str(&selected_json).expect("parse selected response");
        assert_eq!(
            selected["selection"]["selected_version"],
            serde_json::json!(catalog.version)
        );
        assert_eq!(selected["versions"][0]["selected"], true);

        let default_json = runtime
            .managed_media_dependency_set_default_version(
                serde_json::json!({
                    "dependency_id": dependency_id,
                    "version": catalog.version
                })
                .to_string(),
            )
            .expect("set default managed media dependency version");
        let default_status: serde_json::Value =
            serde_json::from_str(&default_json).expect("parse default response");
        assert_eq!(
            default_status["selection"]["default_version"],
            serde_json::json!(catalog.version)
        );

        let active_json = runtime
            .managed_media_dependency_activate_version(
                serde_json::json!({
                    "dependency_id": dependency_id,
                    "version": catalog.version
                })
                .to_string(),
            )
            .expect("activate managed media dependency version");
        let active: serde_json::Value =
            serde_json::from_str(&active_json).expect("parse active response");
        assert_eq!(
            active["selection"]["active_version"],
            serde_json::json!(catalog.version)
        );
        assert_eq!(active["versions"][0]["active"], true);

        let single_json = runtime
            .managed_media_dependency_status(
                serde_json::json!({ "dependency_id": dependency_id }).to_string(),
            )
            .expect("get managed media dependency status");
        let single: serde_json::Value =
            serde_json::from_str(&single_json).expect("parse single status");
        assert_eq!(single["id"], dependency_id);
        assert_eq!(single["readiness"], "ready");

        let removed_json = runtime
            .managed_media_dependency_remove_version(
                serde_json::json!({
                    "dependency_id": dependency_id,
                    "version": catalog.version
                })
                .to_string(),
            )
            .expect("remove managed media dependency version");
        let removed: serde_json::Value =
            serde_json::from_str(&removed_json).expect("parse removed response");
        assert_eq!(
            removed["selection"]["active_version"],
            serde_json::Value::Null
        );
        assert_eq!(removed["readiness"], "missing");
    }

    let missing_staging = root.join("missing-staging");
    std::fs::create_dir_all(&missing_staging).expect("create missing staging dir");
    let ffmpeg = managed_redistributable_catalog_entry(ManagedRedistributableId::Ffmpeg);
    if ffmpeg.platform_key != "unsupported" {
        let invalid = runtime
            .managed_media_dependency_install_from_staging(
                serde_json::json!({
                    "dependency_id": "ffmpeg",
                    "version": ffmpeg.version,
                    "staging_dir": missing_staging.to_string_lossy()
                })
                .to_string(),
            )
            .expect_err("missing expected files should preserve workflow error envelope");
        let envelope = workflow_error_envelope(invalid);
        assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
        assert!(envelope.message.contains("missing expected file"));
    }

    runtime.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

fn write_managed_media_expected_files(root: &Path, expected_files: &[String]) {
    for expected_file in expected_files {
        let file_path = root.join(expected_file);
        std::fs::create_dir_all(file_path.parent().expect("expected file parent"))
            .expect("create expected file parent");
        std::fs::write(file_path, []).expect("write expected file");
    }
}

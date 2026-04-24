use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
async fn direct_runtime_runs_workflow_and_session_from_json() {
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

    let run_response_json = runtime
        .workflow_run(
            serde_json::json!({
                "workflow_id": workflow_id,
                "inputs": [{
                    "node_id": "text-input-1",
                    "port_id": "text",
                    "value": "direct run"
                }],
                "output_targets": [{
                    "node_id": "text-output-1",
                    "port_id": "text"
                }]
            })
            .to_string(),
        )
        .await
        .expect("workflow run");
    let run_response: serde_json::Value =
        serde_json::from_str(&run_response_json).expect("parse run response");
    assert_eq!(run_response["outputs"][0]["value"], "direct run");

    let create_response_json = runtime
        .workflow_create_session(
            serde_json::json!({
                "workflow_id": workflow_id,
                "keep_alive": false
            })
            .to_string(),
        )
        .await
        .expect("create session");
    let session_id = serde_json::from_str::<serde_json::Value>(&create_response_json)
        .expect("parse create response")["session_id"]
        .as_str()
        .expect("session_id")
        .to_string();

    let session_response_json = runtime
        .workflow_run_session(
            serde_json::json!({
                "session_id": session_id,
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
        .expect("run session");
    let session_response: serde_json::Value =
        serde_json::from_str(&session_response_json).expect("parse session response");
    assert_eq!(session_response["outputs"][0]["value"], "session run");

    runtime
        .workflow_close_session(serde_json::json!({ "session_id": session_id }).to_string())
        .await
        .expect("close session");
    runtime.shutdown().await;

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn direct_runtime_workflow_run_preserves_invalid_request_envelope() {
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

    let err = runtime
        .workflow_run(
            serde_json::json!({
                "workflow_id": workflow_id,
                "inputs": [],
                "output_targets": [{
                    "node_id": "text-output-1",
                    "port_id": "text"
                }]
            })
            .to_string(),
        )
        .await
        .expect_err("interactive workflow run should preserve invalid-request envelope");

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
async fn direct_runtime_runs_attributed_workflow_from_json() {
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
    let client_session_id = opened["session"]["client_session_id"]
        .as_str()
        .expect("client session id");

    let response_json = runtime
        .workflow_run_attributed(
            serde_json::json!({
                "credential": {
                    "credential_id": credential_id,
                    "secret": credential_secret
                },
                "client_session_id": client_session_id,
                "bucket_selection": { "type": "default" },
                "run": {
                    "workflow_id": workflow_id,
                    "inputs": [{
                        "node_id": "text-input-1",
                        "port_id": "text",
                        "value": "attributed run"
                    }],
                    "output_targets": [{
                        "node_id": "text-output-1",
                        "port_id": "text"
                    }]
                }
            })
            .to_string(),
        )
        .await
        .expect("attributed workflow run");
    let response: serde_json::Value =
        serde_json::from_str(&response_json).expect("parse attributed response");

    assert_eq!(response["run"]["outputs"][0]["value"], "attributed run");
    assert_eq!(
        response["run"]["run_id"],
        response["workflow_run"]["workflow_run_id"]
    );
    assert_eq!(
        response["attribution"]["client_session_id"],
        serde_json::Value::String(client_session_id.to_string())
    );

    runtime.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn direct_runtime_workflow_run_session_preserves_invalid_request_envelope() {
    let workflow_id = "uniffi-runtime-interactive-session";
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

    let create_response_json = runtime
        .workflow_create_session(
            serde_json::json!({
                "workflow_id": workflow_id,
                "usage_profile": "interactive",
                "keep_alive": false
            })
            .to_string(),
        )
        .await
        .expect("create session");
    let session_id = serde_json::from_str::<serde_json::Value>(&create_response_json)
        .expect("parse create response")["session_id"]
        .as_str()
        .expect("session_id")
        .to_string();

    let err = runtime
        .workflow_run_session(
            serde_json::json!({
                "session_id": session_id,
                "inputs": [],
                "output_targets": [{
                    "node_id": "text-output-1",
                    "port_id": "text"
                }],
                "run_id": "run-human-input-session"
            })
            .to_string(),
        )
        .await
        .expect_err("interactive session run should preserve invalid-request envelope");

    let envelope = workflow_error_envelope(err);
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert_eq!(
        envelope.message,
        "workflow 'uniffi-runtime-interactive-session' requires interactive input at node 'human-input-1'"
    );

    runtime
        .workflow_close_session(serde_json::json!({ "session_id": session_id }).to_string())
        .await
        .expect("close session");
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
                "name": "Native Edited Workflow",
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
        .any(|metadata| metadata["id"] == "Native Edited Workflow"));

    let load_response_json = runtime
        .workflow_graph_load(serde_json::json!({ "path": path }).to_string())
        .expect("load workflow graph");
    let load_response: serde_json::Value =
        serde_json::from_str(&load_response_json).expect("parse load response");
    assert_eq!(load_response["metadata"]["name"], "Native Edited Workflow");

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

use std::sync::Arc;

use node_engine::{EventSink, WorkflowEvent, WorkflowGraph};
use tokio::sync::RwLock;

use crate::{
    validate_workflow_json, version, workflow_event_bridge::BufferedEventSink, FfiError,
    FfiOrchestrationStore, FfiWorkflowEngine, FfiWorkflowGraph,
};

#[cfg(feature = "frontend-http")]
use crate::{
    frontend_http_workflow_create_session, frontend_http_workflow_get_capabilities,
    frontend_http_workflow_run, frontend_http_workflow_run_session,
};
#[cfg(feature = "frontend-http")]
use pantograph_frontend_http_adapter::{
    parse_workflow_outputs_payload, DEFAULT_MAX_INPUT_BINDINGS, DEFAULT_MAX_VALUE_BYTES,
};
#[cfg(feature = "frontend-http")]
use pantograph_workflow_service::{WorkflowErrorCode, WorkflowErrorEnvelope};
#[cfg(feature = "frontend-http")]
use std::io::{Read, Write};
#[cfg(feature = "frontend-http")]
use std::net::TcpListener;
#[cfg(feature = "frontend-http")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(feature = "frontend-http")]
static CWD_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

#[test]
fn test_version() {
    assert!(!version().is_empty());
}

#[test]
fn test_ffi_error_conversion() {
    let err = node_engine::NodeEngineError::ExecutionFailed("test".to_string());
    let ffi_err: FfiError = err.into();
    assert!(matches!(ffi_err, FfiError::ExecutionFailed { .. }));
}

#[test]
fn test_ffi_error_cancelled() {
    let err = node_engine::NodeEngineError::Cancelled;
    let ffi_err: FfiError = err.into();
    assert!(matches!(ffi_err, FfiError::Cancelled));
}

#[test]
fn test_ffi_error_waiting_for_input() {
    let err = node_engine::NodeEngineError::WaitingForInput {
        task_id: "human-input-1".to_string(),
        prompt: Some("Approve deployment?".to_string()),
    };
    let ffi_err: FfiError = err.into();
    assert!(matches!(
        ffi_err,
        FfiError::WaitingForInput { task_id, prompt }
            if task_id == "human-input-1" && prompt.as_deref() == Some("Approve deployment?")
    ));
}

#[test]
fn test_ffi_graph_conversion() {
    let graph = WorkflowGraph::new("test", "Test Graph");
    let ffi = FfiWorkflowGraph::from(graph);
    assert_eq!(ffi.id, "test");
    assert_eq!(ffi.name, "Test Graph");
    assert!(ffi.nodes.is_empty());
    assert!(ffi.edges.is_empty());
}

#[test]
fn test_validate_empty_workflow() {
    let graph = WorkflowGraph::new("test", "Test");
    let json = serde_json::to_string(&graph).unwrap();
    let errors = validate_workflow_json(json).unwrap();
    assert!(errors.is_empty());
}

#[tokio::test]
async fn test_workflow_engine_new() {
    let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
    let graph = engine.get_graph().await;
    assert_eq!(graph.id, "wf-1");
    assert_eq!(graph.name, "Test");
}

#[tokio::test]
async fn test_workflow_engine_add_node() {
    let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
    engine
        .add_node(
            "n1".to_string(),
            "text-input".to_string(),
            0.0,
            0.0,
            "{}".to_string(),
        )
        .await
        .unwrap();

    let graph = engine.get_graph().await;
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].id, "n1");
}

#[tokio::test]
async fn test_workflow_engine_export_json() {
    let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
    let json = engine.export_graph_json().await.unwrap();
    assert!(json.contains("wf-1"));
}

#[tokio::test]
async fn test_orchestration_store() {
    let store = FfiOrchestrationStore::new();
    let list = store.list_graphs().await;
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_drain_events_empty() {
    let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
    let events = engine.drain_events().await;
    assert!(events.is_empty());
}

#[tokio::test]
async fn test_buffered_event_sink_uses_canonical_event_type_names() {
    let buffer = Arc::new(RwLock::new(Vec::new()));
    let sink = BufferedEventSink::new(buffer.clone());

    sink.send(WorkflowEvent::WaitingForInput {
        workflow_id: "wf-1".to_string(),
        execution_id: "exec-1".to_string(),
        task_id: "human-input-1".to_string(),
        prompt: Some("Approve deployment?".to_string()),
        occurred_at_ms: None,
    })
    .expect("send waiting event");
    sink.send(WorkflowEvent::GraphModified {
        workflow_id: "wf-1".to_string(),
        execution_id: "exec-1".to_string(),
        dirty_tasks: vec!["node-a".to_string(), "node-b".to_string()],
        memory_impact: None,
        occurred_at_ms: None,
    })
    .expect("send graph modified event");
    sink.send(WorkflowEvent::WorkflowCancelled {
        workflow_id: "wf-1".to_string(),
        execution_id: "exec-1".to_string(),
        error: "workflow run cancelled during execution".to_string(),
        occurred_at_ms: None,
    })
    .expect("send cancelled event");
    sink.send(WorkflowEvent::IncrementalExecutionStarted {
        workflow_id: "wf-1".to_string(),
        execution_id: "exec-1".to_string(),
        tasks: vec!["node-c".to_string()],
        occurred_at_ms: None,
    })
    .expect("send incremental event");

    let events = {
        let guard = buffer.read().await;
        guard.clone()
    };
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].event_type, "WaitingForInput");
    assert_eq!(events[1].event_type, "GraphModified");
    assert_eq!(events[2].event_type, "WorkflowCancelled");
    assert_eq!(events[3].event_type, "IncrementalExecutionStarted");
    let waiting_json: serde_json::Value =
        serde_json::from_str(&events[0].event_json).expect("parse waiting json");
    let graph_modified_json: serde_json::Value =
        serde_json::from_str(&events[1].event_json).expect("parse graph modified json");
    let cancelled_json: serde_json::Value =
        serde_json::from_str(&events[2].event_json).expect("parse cancelled json");
    let incremental_json: serde_json::Value =
        serde_json::from_str(&events[3].event_json).expect("parse incremental json");

    assert_eq!(waiting_json["type"], "waitingForInput");
    assert_eq!(waiting_json["taskId"], "human-input-1");
    assert_eq!(waiting_json["prompt"], "Approve deployment?");

    assert_eq!(graph_modified_json["type"], "graphModified");
    assert_eq!(graph_modified_json["workflowId"], "wf-1");
    assert_eq!(graph_modified_json["executionId"], "exec-1");
    assert_eq!(
        graph_modified_json["dirtyTasks"],
        serde_json::json!(["node-a", "node-b"])
    );

    assert_eq!(cancelled_json["type"], "workflowCancelled");
    assert_eq!(
        cancelled_json["error"],
        "workflow run cancelled during execution"
    );

    assert_eq!(incremental_json["type"], "incrementalExecutionStarted");
    assert_eq!(incremental_json["workflowId"], "wf-1");
    assert_eq!(incremental_json["executionId"], "exec-1");
    assert_eq!(incremental_json["tasks"], serde_json::json!(["node-c"]));
}

#[cfg(feature = "frontend-http")]
fn create_temp_workflow_root(workflow_id: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("pantograph-uniffi-tests-{suffix}"));
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
                        "definition": {
                            "category": "input",
                            "io_binding_origin": "client_session",
                            "inputs": [
                                {
                                    "id": "text",
                                    "data_type": "string",
                                    "required": true,
                                    "multiple": false
                                }
                            ]
                        }
                    },
                    "position": { "x": 0.0, "y": 0.0 }
                },
                {
                    "id": "vector-output-1",
                    "node_type": "vector-output",
                    "data": {
                        "definition": {
                            "category": "output",
                            "io_binding_origin": "client_session",
                            "outputs": [
                                {
                                    "id": "vector",
                                    "data_type": "embedding",
                                    "required": false,
                                    "multiple": false
                                }
                            ]
                        }
                    },
                    "position": { "x": 200.0, "y": 0.0 }
                }
            ],
            "edges": []
        }
    });
    let file_path = workflows_dir.join(format!("{}.json", workflow_id));
    std::fs::write(
        file_path,
        serde_json::to_vec(&workflow_json).expect("serialize workflow"),
    )
    .expect("write workflow");
    root
}

#[cfg(feature = "frontend-http")]
fn spawn_single_workflow_server(
    status_code: u16,
    body: serde_json::Value,
) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().expect("local addr");
    let body_text = body.to_string();
    let reason = if status_code == 200 { "OK" } else { "ERROR" };

    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set timeout");
        let mut request_buf = [0_u8; 8192];
        let _ = stream.read(&mut request_buf);

        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status_code,
            reason,
            body_text.len(),
            body_text
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    (format!("http://{}", addr), handle)
}

#[test]
#[cfg(feature = "frontend-http")]
fn test_workflow_run_contract_success() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let workflow_id = "wf_contract_success";
    let root = create_temp_workflow_root(workflow_id);
    let original_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    let payload = serde_json::json!({
        "run_id": "server-run-1",
        "outputs": [{ "node_id": "vector-output-1", "port_id": "vector", "value": [0.1, 0.2, 0.3] }],
        "timing_ms": 4
    });
    let (base_url, server_thread) = spawn_single_workflow_server(200, payload);

    let request_json = serde_json::json!({
        "workflow_id": workflow_id,
        "inputs": [{ "node_id": "text-input-1", "port_id": "text", "value": "hello world" }],
        "output_targets": [{ "node_id": "vector-output-1", "port_id": "vector" }]
    })
    .to_string();

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let response_json = runtime
        .block_on(frontend_http_workflow_run(base_url, request_json, None))
        .expect("workflow_run");
    let response: pantograph_workflow_service::WorkflowRunResponse =
        serde_json::from_str(&response_json).expect("parse response");

    server_thread.join().expect("join server");
    std::env::set_current_dir(original_cwd).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);

    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, "vector-output-1");
}

#[test]
#[cfg(feature = "frontend-http")]
fn test_workflow_run_contract_preserves_cancelled_envelope() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let workflow_id = "wf_contract_cancelled";
    let root = create_temp_workflow_root(workflow_id);
    let original_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    let payload = serde_json::to_value(WorkflowErrorEnvelope {
        code: WorkflowErrorCode::Cancelled,
        message: "workflow run cancelled".to_string(),
        details: None,
    })
    .expect("serialize envelope");
    let (base_url, server_thread) = spawn_single_workflow_server(409, payload);

    let request_json = serde_json::json!({
        "workflow_id": workflow_id,
        "inputs": [{ "node_id": "text-input-1", "port_id": "text", "value": "hello world" }],
        "output_targets": [{ "node_id": "vector-output-1", "port_id": "vector" }]
    })
    .to_string();

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let err = runtime
        .block_on(frontend_http_workflow_run(base_url, request_json, None))
        .expect_err("cancelled envelope should be preserved");

    server_thread.join().expect("join server");
    std::env::set_current_dir(original_cwd).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);

    let message = match err {
        FfiError::Other { message } => message,
        other => panic!("expected FfiError::Other with envelope JSON, got {other:?}"),
    };
    let envelope: WorkflowErrorEnvelope =
        serde_json::from_str(&message).expect("parse cancelled envelope");
    assert_eq!(envelope.code, WorkflowErrorCode::Cancelled);
    assert_eq!(envelope.message, "workflow run cancelled");
}

#[test]
#[cfg(feature = "frontend-http")]
fn test_workflow_run_contract_preserves_invalid_request_envelope() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let workflow_id = "wf_contract_invalid_request";
    let root = create_temp_workflow_root(workflow_id);
    let original_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    let payload = serde_json::to_value(WorkflowErrorEnvelope {
        code: WorkflowErrorCode::InvalidRequest,
        message:
            "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'"
                .to_string(),
        details: None,
    })
    .expect("serialize envelope");
    let (base_url, server_thread) = spawn_single_workflow_server(400, payload);

    let request_json = serde_json::json!({
        "workflow_id": workflow_id,
        "inputs": [{ "node_id": "text-input-1", "port_id": "text", "value": "hello world" }],
        "output_targets": [{ "node_id": "vector-output-1", "port_id": "vector" }]
    })
    .to_string();

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let err = runtime
        .block_on(frontend_http_workflow_run(base_url, request_json, None))
        .expect_err("invalid-request envelope should be preserved");

    server_thread.join().expect("join server");
    std::env::set_current_dir(original_cwd).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);

    let message = match err {
        FfiError::Other { message } => message,
        other => panic!("expected FfiError::Other with envelope JSON, got {other:?}"),
    };
    let envelope: WorkflowErrorEnvelope =
        serde_json::from_str(&message).expect("parse invalid-request envelope");
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert_eq!(
        envelope.message,
        "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'"
    );
}

#[test]
#[cfg(feature = "frontend-http")]
fn test_workflow_run_session_contract_preserves_cancelled_envelope() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let workflow_id = "wf_session_contract_cancelled";
    let root = create_temp_workflow_root(workflow_id);
    let original_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    let payload = serde_json::to_value(WorkflowErrorEnvelope {
        code: WorkflowErrorCode::Cancelled,
        message: "workflow run cancelled".to_string(),
        details: None,
    })
    .expect("serialize envelope");
    let (base_url, server_thread) = spawn_single_workflow_server(409, payload);

    let create_session_json = serde_json::json!({
        "workflow_id": workflow_id,
        "keep_alive": false
    })
    .to_string();

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let created_json = runtime
        .block_on(frontend_http_workflow_create_session(
            base_url.clone(),
            create_session_json,
            None,
        ))
        .expect("create session");
    let created: pantograph_workflow_service::WorkflowSessionCreateResponse =
        serde_json::from_str(&created_json).expect("parse session response");

    let run_session_json = serde_json::json!({
        "session_id": created.session_id,
        "inputs": [{ "node_id": "text-input-1", "port_id": "text", "value": "hello world" }],
        "output_targets": [{ "node_id": "vector-output-1", "port_id": "vector" }]
    })
    .to_string();

    let err = runtime
        .block_on(frontend_http_workflow_run_session(
            base_url,
            run_session_json,
            None,
        ))
        .expect_err("cancelled session envelope should be preserved");

    server_thread.join().expect("join server");
    std::env::set_current_dir(original_cwd).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);

    let message = match err {
        FfiError::Other { message } => message,
        other => panic!("expected FfiError::Other with envelope JSON, got {other:?}"),
    };
    let envelope: WorkflowErrorEnvelope =
        serde_json::from_str(&message).expect("parse cancelled envelope");
    assert_eq!(envelope.code, WorkflowErrorCode::Cancelled);
    assert_eq!(envelope.message, "workflow run cancelled");
}

#[test]
#[cfg(feature = "frontend-http")]
fn test_workflow_run_session_contract_preserves_invalid_request_envelope() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let workflow_id = "wf_session_contract_invalid_request";
    let root = create_temp_workflow_root(workflow_id);
    let original_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    let payload = serde_json::to_value(WorkflowErrorEnvelope {
        code: WorkflowErrorCode::InvalidRequest,
        message:
            "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'"
                .to_string(),
        details: None,
    })
    .expect("serialize envelope");
    let (base_url, server_thread) = spawn_single_workflow_server(400, payload);

    let create_session_json = serde_json::json!({
        "workflow_id": workflow_id,
        "keep_alive": false
    })
    .to_string();

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let created_json = runtime
        .block_on(frontend_http_workflow_create_session(
            base_url.clone(),
            create_session_json,
            None,
        ))
        .expect("create session");
    let created: pantograph_workflow_service::WorkflowSessionCreateResponse =
        serde_json::from_str(&created_json).expect("parse session response");

    let run_session_json = serde_json::json!({
        "session_id": created.session_id,
        "inputs": [{ "node_id": "text-input-1", "port_id": "text", "value": "hello world" }],
        "output_targets": [{ "node_id": "vector-output-1", "port_id": "vector" }]
    })
    .to_string();

    let err = runtime
        .block_on(frontend_http_workflow_run_session(
            base_url,
            run_session_json,
            None,
        ))
        .expect_err("invalid-request session envelope should be preserved");

    server_thread.join().expect("join server");
    std::env::set_current_dir(original_cwd).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);

    let message = match err {
        FfiError::Other { message } => message,
        other => panic!("expected FfiError::Other with envelope JSON, got {other:?}"),
    };
    let envelope: WorkflowErrorEnvelope =
        serde_json::from_str(&message).expect("parse invalid-request envelope");
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert_eq!(
        envelope.message,
        "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'"
    );
}

#[test]
#[cfg(feature = "frontend-http")]
fn test_workflow_get_capabilities_contract_success() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let workflow_id = "wf_contract_caps";
    let root = create_temp_workflow_root(workflow_id);
    let original_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    let request_json = serde_json::json!({
        "workflow_id": workflow_id
    })
    .to_string();

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let response_json = runtime
        .block_on(frontend_http_workflow_get_capabilities(
            "http://127.0.0.1:9".to_string(),
            request_json,
            None,
        ))
        .expect("capabilities");
    let response: pantograph_workflow_service::WorkflowCapabilitiesResponse =
        serde_json::from_str(&response_json).expect("parse capabilities");

    std::env::set_current_dir(original_cwd).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);

    assert_eq!(response.max_input_bindings, DEFAULT_MAX_INPUT_BINDINGS);
    assert_eq!(response.max_value_bytes, DEFAULT_MAX_VALUE_BYTES);
    assert_eq!(response.runtime_requirements.required_models.len(), 0);
    assert_eq!(response.runtime_requirements.estimated_peak_ram_mb, Some(0));
}

#[test]
#[cfg(feature = "frontend-http")]
fn test_parse_workflow_outputs_payload_rejects_missing_port() {
    let payload = serde_json::json!({
        "outputs": [{ "node_id": "node-1", "value": [0.1, 0.2, 0.3] }]
    });
    let err = parse_workflow_outputs_payload(&payload).expect_err("must reject malformed output");
    assert!(err.to_string().contains("port_id"));
}

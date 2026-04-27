use node_engine::WorkflowGraph;
#[cfg(feature = "frontend-http")]
use pantograph_frontend_http_adapter::parse_workflow_outputs_payload;
#[cfg(feature = "frontend-http")]
use pantograph_workflow_service::WorkflowService;
#[cfg(feature = "frontend-http")]
use std::io::{Read, Write};
#[cfg(feature = "frontend-http")]
use std::net::TcpListener;
#[cfg(feature = "frontend-http")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn test_workflow_graph_json_roundtrip() {
    let graph = WorkflowGraph::new("wf-1", "Test");
    let json = serde_json::to_string(&graph).unwrap();
    let parsed: WorkflowGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "wf-1");
    assert_eq!(parsed.name, "Test");
}

#[test]
fn test_workflow_graph_add_node() {
    let mut graph = WorkflowGraph::new("wf-1", "Test");
    graph.nodes.push(node_engine::GraphNode {
        id: "n1".to_string(),
        node_type: "text-input".to_string(),
        position: (0.0, 0.0),
        data: serde_json::Value::Null,
    });
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].id, "n1");
}

#[test]
fn test_validation_empty_graph() {
    let graph = WorkflowGraph::new("wf-1", "Test");
    let errors = node_engine::validation::validate_workflow(&graph, None);
    assert!(errors.is_empty());
}

#[test]
fn test_callback_channel_lifecycle() {
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();
    let callback_id = "test-cb-1".to_string();

    crate::callback_bridge::insert_pending_callback_for_test(callback_id.clone(), tx);

    crate::callback_bridge::callback_respond(callback_id, r#"{"result": "ok"}"#.to_string())
        .expect("callback response");

    let result = rx.blocking_recv().unwrap();
    assert!(result.is_ok());
}

#[test]
fn test_orchestration_store_roundtrip() {
    let store = node_engine::OrchestrationStore::new();
    assert!(store.list_graphs().is_empty());
}

#[test]
fn test_context_keys_input_output() {
    let input_key = node_engine::ContextKeys::input("node-1", "prompt");
    assert_eq!(input_key, "node-1.input.prompt");

    let output_key = node_engine::ContextKeys::output("node-1", "response");
    assert_eq!(output_key, "node-1.output.response");
}

#[test]
fn test_node_registry_metadata() {
    let mut registry = node_engine::NodeRegistry::new();
    assert!(registry.all_metadata().is_empty());

    let metadata = node_engine::TaskMetadata {
        node_type: "test-node".to_string(),
        category: node_engine::NodeCategory::Processing,
        label: "Test Node".to_string(),
        description: "A test node".to_string(),
        inputs: vec![],
        outputs: vec![],
        execution_mode: node_engine::ExecutionMode::Reactive,
    };

    registry.register_metadata(metadata);
    assert_eq!(registry.all_metadata().len(), 1);
    assert!(registry.has_node_type("test-node"));

    let all = registry.all_metadata();
    let json = serde_json::to_string(&all).unwrap();
    assert!(json.contains("test-node"));
}

#[test]
fn test_task_metadata_json_roundtrip() {
    let json = r#"{
            "nodeType": "my-node",
            "category": "processing",
            "label": "My Node",
            "description": "Does things",
            "inputs": [],
            "outputs": [],
            "executionMode": "reactive"
        }"#;
    let metadata: node_engine::TaskMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(metadata.node_type, "my-node");
    assert_eq!(metadata.label, "My Node");
}

#[cfg(feature = "frontend-http")]
static CWD_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

#[cfg(feature = "frontend-http")]
fn create_temp_workflow_root(workflow_id: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("pantograph-rustler-tests-{suffix}"));
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
            "nodes": [],
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

#[tokio::test(flavor = "current_thread")]
#[cfg(feature = "frontend-http")]
#[ignore = "requires local TCP bind permissions in test environment"]
async fn test_rustler_workflow_host_contract_success() {
    let _guard = CWD_LOCK.lock().await;
    let workflow_id = "wf_rustler_contract";
    let root = create_temp_workflow_root(workflow_id);
    let original_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    let payload = serde_json::json!({
        "workflow_run_id": "server-run-1",
        "outputs": [{ "node_id": "vector-output-1", "port_id": "vector", "value": [1.0, 2.0, 3.0] }],
        "timing_ms": 2
    });
    let (base_url, server_thread) = spawn_single_workflow_server(200, payload);

    let host = crate::workflow_host_contract::build_frontend_http_host(base_url, None)
        .expect("frontend HTTP host");
    let service = WorkflowService::new();
    let session = service
        .create_workflow_execution_session(
            &host,
            pantograph_workflow_service::WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create execution session");
    let response = service
        .run_workflow_execution_session(
            &host,
            pantograph_workflow_service::WorkflowExecutionSessionRunRequest {
                session_id: session.session_id,
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![pantograph_workflow_service::WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello world"),
                }],
                output_targets: Some(vec![pantograph_workflow_service::WorkflowOutputTarget {
                    node_id: "vector-output-1".to_string(),
                    port_id: "vector".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run workflow");

    server_thread.join().expect("join server");
    std::env::set_current_dir(original_cwd).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);

    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, "vector-output-1");
}

#[tokio::test(flavor = "current_thread")]
#[cfg(feature = "frontend-http")]
#[ignore = "requires local TCP bind permissions in test environment"]
async fn test_rustler_workflow_execution_session_host_contract_preserves_cancelled_envelope() {
    let _guard = CWD_LOCK.lock().await;
    let workflow_id = "wf_rustler_session_cancelled";
    let root = create_temp_workflow_root(workflow_id);
    let original_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    let payload = serde_json::json!({
        "code": "cancelled",
        "message": "workflow run cancelled"
    });
    let (base_url, server_thread) = spawn_single_workflow_server(409, payload);

    let host = crate::workflow_host_contract::build_frontend_http_host(base_url, None)
        .expect("frontend HTTP host");
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            pantograph_workflow_service::WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let err = service
        .run_workflow_execution_session(
            &host,
            pantograph_workflow_service::WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![pantograph_workflow_service::WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello world"),
                }],
                output_targets: Some(vec![pantograph_workflow_service::WorkflowOutputTarget {
                    node_id: "vector-output-1".to_string(),
                    port_id: "vector".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("cancelled session envelope should be preserved");

    server_thread.join().expect("join server");
    std::env::set_current_dir(original_cwd).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);

    match err {
        pantograph_workflow_service::WorkflowServiceError::Cancelled(message) => {
            assert_eq!(message, "workflow run cancelled");
        }
        other => panic!("expected cancelled envelope, got {other:?}"),
    }
}

#[tokio::test(flavor = "current_thread")]
#[cfg(feature = "frontend-http")]
#[ignore = "requires local TCP bind permissions in test environment"]
async fn test_rustler_workflow_execution_session_host_contract_preserves_invalid_request_envelope()
{
    let _guard = CWD_LOCK.lock().await;
    let workflow_id = "wf_rustler_session_invalid_request";
    let root = create_temp_workflow_root(workflow_id);
    let original_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    let payload = serde_json::json!({
        "code": "invalid_request",
        "message": "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'"
    });
    let (base_url, server_thread) = spawn_single_workflow_server(400, payload);

    let host = crate::workflow_host_contract::build_frontend_http_host(base_url, None)
        .expect("frontend HTTP host");
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            pantograph_workflow_service::WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let err = service
        .run_workflow_execution_session(
            &host,
            pantograph_workflow_service::WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![pantograph_workflow_service::WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello world"),
                }],
                output_targets: Some(vec![pantograph_workflow_service::WorkflowOutputTarget {
                    node_id: "vector-output-1".to_string(),
                    port_id: "vector".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("invalid-request session envelope should be preserved");

    server_thread.join().expect("join server");
    std::env::set_current_dir(original_cwd).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);

    match err {
        pantograph_workflow_service::WorkflowServiceError::InvalidRequest(message) => {
            assert_eq!(
                message,
                "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'"
            );
        }
        other => panic!("expected invalid-request envelope, got {other:?}"),
    }
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

#[test]
#[cfg(feature = "frontend-http")]
fn test_validate_workflow_requires_existing_workflow_file() {
    let host = crate::workflow_host_contract::build_frontend_http_host(
        "http://127.0.0.1:9".to_string(),
        None,
    )
    .expect("frontend HTTP host");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let err = runtime
        .block_on(async {
            pantograph_workflow_service::WorkflowHost::validate_workflow(&host, "missing-workflow")
                .await
        })
        .expect_err("must fail");
    assert!(matches!(
        err,
        pantograph_workflow_service::WorkflowServiceError::WorkflowNotFound(_)
    ));
}

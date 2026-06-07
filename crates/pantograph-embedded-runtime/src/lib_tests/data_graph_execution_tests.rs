use super::*;

#[tokio::test]
async fn execute_data_graph_retired_onnx_audio_path_does_not_call_python_sidecar_or_reconcile_runtime(
) {
    let temp = TempDir::new().expect("temp dir");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let python_runtime = Arc::new(MockMediaPythonRuntime {
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
    .with_runtime_registry(runtime_registry.clone());

    let outputs = runtime
        .execute_data_graph(
            "runtime-onnx-audio-data-graph",
            &runtime_onnx_audio_data_graph(),
            &HashMap::from([(
                "text".to_string(),
                serde_json::json!("a tiny painted robot"),
            )]),
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("data graph execution");

    assert!(outputs.get("audio").is_none());
    assert_eq!(
        outputs.get("_graph_id"),
        Some(&serde_json::json!("runtime-onnx-audio-data-graph"))
    );

    let requests = python_runtime.requests.lock().expect("requests lock");
    assert!(requests.is_empty());

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot
        .runtimes
        .iter()
        .all(|runtime| runtime.runtime_id != "onnx-runtime"));
}

#[tokio::test]
async fn execute_data_graph_retired_python_media_nodes_do_not_reconcile_sidecar_runtimes() {
    let temp = TempDir::new().expect("temp dir");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let python_runtime = Arc::new(MockMediaPythonRuntime {
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
    .with_runtime_registry(runtime_registry.clone());

    let outputs = runtime
        .execute_data_graph(
            "multi-python-runtime-data-graph",
            &multi_python_runtime_data_graph(),
            &HashMap::new(),
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("data graph execution");

    let audio_error = outputs
        .get("audio-generation-1.error")
        .and_then(|value| value.as_str())
        .expect("retired audio-generation error");
    assert!(audio_error.contains("dependency_preflight_retired"));
    let onnx_error = outputs
        .get("onnx-inference-1.error")
        .and_then(|value| value.as_str())
        .expect("retired onnx-inference error");
    assert!(onnx_error.contains("dependency_preflight_retired"));

    let requests = python_runtime.requests.lock().expect("requests lock");
    assert!(requests.is_empty());

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot
        .runtimes
        .iter()
        .all(|runtime| runtime.runtime_id != "stable_audio"));
    assert!(snapshot
        .runtimes
        .iter()
        .all(|runtime| runtime.runtime_id != "onnx-runtime"));
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
    assert!(!events
        .iter()
        .any(|event| matches!(event, node_engine::WorkflowEvent::WorkflowFailed { .. })));
    assert!(!events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::WorkflowCompleted { .. }
            | node_engine::WorkflowEvent::WorkflowCancelled { .. }
    )));
}

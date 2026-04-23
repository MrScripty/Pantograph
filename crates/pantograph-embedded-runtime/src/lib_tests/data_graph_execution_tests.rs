use super::*;

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
    assert!(!events
        .iter()
        .any(|event| matches!(event, node_engine::WorkflowEvent::WorkflowFailed { .. })));
    assert!(!events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::WorkflowCompleted { .. }
            | node_engine::WorkflowEvent::WorkflowCancelled { .. }
    )));
}

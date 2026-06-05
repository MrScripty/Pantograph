use super::*;

#[tokio::test]
async fn python_runtime_recorder_is_not_used_after_retired_preflight_blocks() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: HashMap::from([("audio".to_string(), serde_json::json!("base64-audio"))]),
    });
    let (executor, mut extensions) = test_executor(adapter);
    let recorder = install_python_runtime_recorder(&mut extensions);

    let inputs = HashMap::from([
        (
            "model_path".to_string(),
            serde_json::json!("/tmp/model.onnx"),
        ),
        ("backend_key".to_string(), serde_json::json!("pytorch")),
    ]);

    let error = executor
        .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect_err("retired embedded-runtime preflight must fail closed");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("dependency_preflight_retired"));
            assert!(message.contains("python_runtime_adapter"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert!(requests.lock().expect("recording lock").is_empty());
    assert!(recorder.snapshots().is_empty());
}

#[test]
fn python_runtime_backend_id_ignores_retired_model_ref_engine() {
    let inputs = HashMap::from([
        ("backend_key".to_string(), serde_json::json!("pytorch")),
        (
            "model_ref".to_string(),
            serde_json::json!({
                "engine": "onnx-runtime",
                "modelId": "model-a",
                "taskTypePrimary": "text-to-audio",
                "dependencyBindings": [],
                "dependencyRequirementsId": null,
                "contractVersion": 2
            }),
        ),
    ]);

    assert_eq!(
        TauriTaskExecutor::python_runtime_backend_id("audio-generation", &inputs),
        "stable_audio"
    );
}

#[test]
fn python_runtime_metadata_does_not_project_legacy_model_path_targets() {
    let request = PythonNodeExecutionRequest {
        node_type: "onnx-inference".to_string(),
        inputs: HashMap::from([
            (
                "model_ref".to_string(),
                serde_json::json!({
                    "engine": "onnx-runtime",
                    "modelId": "model-a",
                    "modelPath": "/tmp/model.onnx",
                    "taskTypePrimary": "text-to-audio",
                    "dependencyBindings": [],
                    "dependencyRequirementsId": null,
                    "contractVersion": 2
                }),
            ),
            (
                "model_path".to_string(),
                serde_json::json!("/tmp/legacy.onnx"),
            ),
        ]),
        env_ids: Vec::new(),
    };

    let metadata =
        TauriTaskExecutor::python_runtime_execution_metadata("onnx-inference", &request, false);

    assert_eq!(
        metadata.snapshot.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
    assert_eq!(metadata.model_target, None);
}

struct FailingPythonAdapter;

#[async_trait]
impl PythonRuntimeAdapter for FailingPythonAdapter {
    async fn execute_node(
        &self,
        _request: PythonNodeExecutionRequest,
    ) -> std::result::Result<HashMap<String, serde_json::Value>, String> {
        Err("python sidecar crashed".to_string())
    }
}

#[tokio::test]
async fn failing_python_adapter_is_not_reached_after_retired_preflight_blocks() {
    let executor = TauriTaskExecutor::with_python_runtime(None, Arc::new(FailingPythonAdapter));
    let mut extensions = ExecutorExtensions::new();
    let recorder = install_python_runtime_recorder(&mut extensions);

    let inputs = HashMap::from([
        (
            "model_path".to_string(),
            serde_json::json!("/tmp/model.onnx"),
        ),
        ("backend_key".to_string(), serde_json::json!("onnx-runtime")),
        ("model_type".to_string(), serde_json::json!("audio")),
        ("prompt".to_string(), serde_json::json!("hello")),
    ]);

    let error = executor
        .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect_err("retired preflight should block before the adapter");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("dependency_preflight_retired"));
            assert!(!message.contains("python sidecar crashed"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert!(recorder.snapshots().is_empty());
}

struct StreamingPythonAdapter {
    chunks: Vec<serde_json::Value>,
    response: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl PythonRuntimeAdapter for StreamingPythonAdapter {
    async fn execute_node(
        &self,
        _request: PythonNodeExecutionRequest,
    ) -> std::result::Result<HashMap<String, serde_json::Value>, String> {
        Ok(self.response.clone())
    }

    async fn execute_node_with_stream(
        &self,
        _request: PythonNodeExecutionRequest,
        on_stream: Option<PythonStreamHandler>,
    ) -> std::result::Result<HashMap<String, serde_json::Value>, String> {
        if let Some(on_stream) = on_stream {
            for chunk in &self.chunks {
                on_stream(chunk.clone());
            }
        }
        Ok(self.response.clone())
    }
}

#[tokio::test]
async fn stream_events_are_not_emitted_after_retired_preflight_blocks() {
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(StreamingPythonAdapter {
        chunks: vec![serde_json::json!({
            "type": "audio_chunk",
            "mode": "append",
            "audio_base64": "chunk-1",
            "mime_type": "audio/wav",
            "sequence": 0,
            "is_final": false
        })],
        response: HashMap::from([("audio".to_string(), serde_json::json!("final-audio"))]),
    });
    let (executor, mut extensions) = test_executor(adapter);
    let sink = Arc::new(VecEventSink::new());
    extensions.set(
        runtime_extension_keys::EVENT_SINK,
        sink.clone() as Arc<dyn node_engine::EventSink>,
    );
    extensions.set(
        runtime_extension_keys::EXECUTION_ID,
        "exec-stream-test".to_string(),
    );

    let inputs = HashMap::from([
        (
            "model_path".to_string(),
            serde_json::json!("/tmp/model.onnx"),
        ),
        ("prompt".to_string(), serde_json::json!("stream this")),
    ]);

    let error = executor
        .execute_task(
            "onnx-inference-stream",
            inputs,
            &Context::new(),
            &extensions,
        )
        .await
        .expect_err("retired preflight should block before stream adapter");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("dependency_preflight_retired"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    assert!(sink
        .events()
        .into_iter()
        .all(|event| !matches!(event, WorkflowEvent::TaskStream { .. })));
}

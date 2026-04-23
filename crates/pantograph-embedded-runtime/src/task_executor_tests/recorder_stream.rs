use super::*;

#[tokio::test]
async fn python_runtime_recorder_tracks_backend_and_environment_identity() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests,
        response: adapter_response,
    });

    let resolved_model_ref = ModelRefV2 {
        contract_version: 2,
        engine: "onnx-runtime".to_string(),
        model_id: "kitten-tts".to_string(),
        model_path: "/tmp/model.onnx".to_string(),
        task_type_primary: "text-to-audio".to_string(),
        dependency_bindings: vec![ModelDependencyBinding {
            binding_id: "binding-onnx".to_string(),
            profile_id: "profile-onnx".to_string(),
            profile_version: 1,
            profile_hash: Some("hash".to_string()),
            backend_key: Some("onnx-runtime".to_string()),
            platform_selector: Some("linux-x86_64".to_string()),
            environment_kind: Some("python".to_string()),
            env_id: Some("venv:onnx".to_string()),
            python_executable_override: None,
            validation_state: DependencyValidationState::Resolved,
            validation_errors: Vec::new(),
            requirements: Vec::new(),
        }],
        dependency_requirements_id: Some("requirements-onnx".to_string()),
    };

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("onnx-runtime".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: Some(resolved_model_ref),
    });
    let (executor, mut extensions) = test_executor(adapter, resolver);
    let recorder = install_python_runtime_recorder(&mut extensions);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.onnx"),
    );
    inputs.insert("backend_key".to_string(), serde_json::json!("onnxruntime"));

    executor
        .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
        .await
        .expect("onnx execution should succeed");

    let metadata = recorder.snapshot().expect("python runtime metadata");
    assert_eq!(
        metadata.snapshot.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
    assert_eq!(
        metadata.snapshot.runtime_instance_id.as_deref(),
        Some("python-runtime:onnx-runtime:venv_onnx")
    );
    assert_eq!(metadata.snapshot.runtime_reused, Some(false));
    assert_eq!(
        metadata.snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
    assert!(!metadata.snapshot.active);
    assert_eq!(metadata.model_target.as_deref(), Some("/tmp/model.onnx"));
    assert_eq!(metadata.health_assessment, None);
}

#[tokio::test]
async fn python_runtime_recorder_keeps_process_runtime_non_reused_across_runs() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests,
        response: adapter_response,
    });

    let resolved_model_ref = ModelRefV2 {
        contract_version: 2,
        engine: "onnx-runtime".to_string(),
        model_id: "kitten-tts".to_string(),
        model_path: "/tmp/model.onnx".to_string(),
        task_type_primary: "text-to-audio".to_string(),
        dependency_bindings: vec![ModelDependencyBinding {
            binding_id: "binding-onnx".to_string(),
            profile_id: "profile-onnx".to_string(),
            profile_version: 1,
            profile_hash: Some("hash".to_string()),
            backend_key: Some("onnx-runtime".to_string()),
            platform_selector: Some("linux-x86_64".to_string()),
            environment_kind: Some("python".to_string()),
            env_id: Some("venv:onnx".to_string()),
            python_executable_override: None,
            validation_state: DependencyValidationState::Resolved,
            validation_errors: Vec::new(),
            requirements: Vec::new(),
        }],
        dependency_requirements_id: Some("requirements-onnx".to_string()),
    };

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("onnx-runtime".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: Some(resolved_model_ref),
    });
    let (executor, mut extensions) = test_executor(adapter, resolver);
    let recorder = install_python_runtime_recorder(&mut extensions);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.onnx"),
    );
    inputs.insert("backend_key".to_string(), serde_json::json!("onnxruntime"));

    executor
        .execute_task(
            "onnx-inference-1",
            inputs.clone(),
            &Context::new(),
            &extensions,
        )
        .await
        .expect("first onnx execution should succeed");
    executor
        .execute_task("onnx-inference-2", inputs, &Context::new(), &extensions)
        .await
        .expect("second onnx execution should succeed");

    let metadata = recorder.snapshot().expect("python runtime metadata");
    assert_eq!(metadata.snapshot.runtime_reused, Some(false));
    assert_eq!(
        metadata.snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
    assert!(!metadata.snapshot.active);
    assert_eq!(metadata.health_assessment, None);
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
async fn python_runtime_recorder_progresses_failed_execution_health_state() {
    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("pytorch".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let executor = TauriTaskExecutor::with_python_runtime(None, Arc::new(FailingPythonAdapter));
    let mut extensions = ExecutorExtensions::new();
    extensions.set(extension_keys::MODEL_DEPENDENCY_RESOLVER, resolver);
    let recorder = install_python_runtime_recorder(&mut extensions);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.safetensors"),
    );
    inputs.insert("backend_key".to_string(), serde_json::json!("pytorch"));
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));

    for _ in 0..3 {
        let error = executor
            .execute_task(
                "pytorch-inference-1",
                inputs.clone(),
                &Context::new(),
                &extensions,
            )
            .await
            .expect_err("python execution should fail");

        match error {
            NodeEngineError::ExecutionFailed(message) => {
                assert!(message.contains("python sidecar crashed"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    let snapshots = recorder.snapshots();
    assert_eq!(snapshots.len(), 3);

    let first_assessment = snapshots[0]
        .health_assessment
        .clone()
        .expect("first failed execution health assessment");
    assert!(first_assessment.healthy);
    assert_eq!(first_assessment.consecutive_failures, 1);
    assert_eq!(
        first_assessment.state,
        crate::runtime_health::RuntimeHealthState::Degraded {
            reason: "python sidecar crashed".to_string(),
        }
    );

    let second_assessment = snapshots[1]
        .health_assessment
        .clone()
        .expect("second failed execution health assessment");
    assert!(second_assessment.healthy);
    assert_eq!(second_assessment.consecutive_failures, 2);
    assert_eq!(
        second_assessment.state,
        crate::runtime_health::RuntimeHealthState::Degraded {
            reason: "python sidecar crashed".to_string(),
        }
    );

    let third = snapshots.last().expect("third runtime metadata");
    assert!(!third.snapshot.active);
    assert_eq!(
        third.snapshot.last_error.as_deref(),
        Some("python sidecar crashed")
    );
    let third_assessment = third
        .health_assessment
        .clone()
        .expect("third failed execution health assessment");
    assert!(!third_assessment.healthy);
    assert_eq!(
        third_assessment.error.as_deref(),
        Some("python sidecar crashed")
    );
    assert_eq!(third_assessment.consecutive_failures, 3);
    assert_eq!(
        third_assessment.state,
        crate::runtime_health::RuntimeHealthState::Unhealthy {
            reason: "python sidecar crashed".to_string(),
        }
    );
}

#[tokio::test]
async fn onnx_nodes_apply_inference_setting_defaults_before_python_execution() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("onnx-runtime".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let (executor, extensions) = test_executor(adapter, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.onnx"),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "voice", "default": "expr-voice-5-m"},
            {"key": "speed", "default": 0.9},
            {"key": "clean_text", "default": true},
            {"key": "sample_rate", "default": 24000}
        ]),
    );

    let _ = executor
        .execute_task(
            "onnx-inference-defaults",
            inputs,
            &Context::new(),
            &extensions,
        )
        .await
        .expect("onnx execution with inference defaults should succeed");

    let recorded = requests.lock().expect("recording lock");
    assert_eq!(recorded.len(), 1);
    let request = &recorded[0];
    assert_eq!(
        request.inputs.get("voice"),
        Some(&serde_json::json!("expr-voice-5-m"))
    );
    assert_eq!(request.inputs.get("speed"), Some(&serde_json::json!(0.9)));
    assert_eq!(
        request.inputs.get("clean_text"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        request.inputs.get("sample_rate"),
        Some(&serde_json::json!(24000))
    );
}

#[tokio::test]
async fn python_nodes_emit_stream_events_when_event_sink_extension_exists() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("final-audio"));
    adapter_response.insert(
        "stream".to_string(),
        serde_json::json!([
            {
                "type": "audio_chunk",
                "mode": "append",
                "audio_base64": "chunk-1",
                "mime_type": "audio/wav",
                "sequence": 0,
                "is_final": false
            },
            {
                "type": "audio_chunk",
                "mode": "append",
                "audio_base64": "chunk-2",
                "mime_type": "audio/wav",
                "sequence": 1,
                "is_final": true
            }
        ]),
    );
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests: requests.clone(),
        response: adapter_response,
    });

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("onnx-runtime".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let (executor, mut extensions) = test_executor(adapter, resolver);
    let sink = Arc::new(VecEventSink::new());
    extensions.set(
        runtime_extension_keys::EVENT_SINK,
        sink.clone() as Arc<dyn node_engine::EventSink>,
    );
    extensions.set(
        runtime_extension_keys::EXECUTION_ID,
        "exec-stream-test".to_string(),
    );

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.onnx"),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("stream this"));

    let _ = executor
        .execute_task(
            "onnx-inference-stream",
            inputs,
            &Context::new(),
            &extensions,
        )
        .await
        .expect("onnx stream execution should succeed");

    let events = sink.events();
    let stream_events: Vec<_> = events
        .into_iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskStream {
                task_id,
                execution_id,
                port,
                data,
                ..
            } => Some((task_id, execution_id, port, data)),
            _ => None,
        })
        .collect();

    assert_eq!(stream_events.len(), 2);
    assert_eq!(stream_events[0].0, "onnx-inference-stream");
    assert_eq!(stream_events[0].1, "exec-stream-test");
    assert_eq!(stream_events[0].2, "stream");
    assert_eq!(stream_events[0].3["audio_base64"], "chunk-1");
    assert_eq!(stream_events[0].3["sequence"], 0);
    assert_eq!(stream_events[0].3["is_final"], false);
    assert_eq!(stream_events[1].3["audio_base64"], "chunk-2");
    assert_eq!(stream_events[1].3["sequence"], 1);
    assert_eq!(stream_events[1].3["is_final"], true);
}

#[tokio::test]
async fn audio_generation_nodes_do_not_emit_buffered_stream_events_after_completion() {
    let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
    let mut adapter_response = HashMap::new();
    adapter_response.insert("audio".to_string(), serde_json::json!("final-audio"));
    adapter_response.insert(
        "stream".to_string(),
        serde_json::json!([
            {
                "type": "audio_chunk",
                "mode": "append",
                "audio_base64": "chunk-1",
                "mime_type": "audio/wav",
                "sequence": 0,
                "is_final": false
            }
        ]),
    );
    let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
        requests,
        response: adapter_response,
    });

    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
        requirements: ModelDependencyRequirements {
            backend_key: Some("stable_audio".to_string()),
            ..make_requirements(DependencyValidationState::Resolved)
        },
        status: make_status(DependencyState::Ready, None),
        model_ref: None,
    });
    let (executor, mut extensions) = test_executor(adapter, resolver);
    let sink = Arc::new(VecEventSink::new());
    extensions.set(
        runtime_extension_keys::EVENT_SINK,
        sink.clone() as Arc<dyn node_engine::EventSink>,
    );
    extensions.set(
        runtime_extension_keys::EXECUTION_ID,
        "exec-audio-batch-test".to_string(),
    );

    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/stable-audio"),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("pad ambience"));

    let outputs = executor
        .execute_task(
            "audio-generation-batch",
            inputs,
            &Context::new(),
            &extensions,
        )
        .await
        .expect("audio-generation execution should succeed");

    assert_eq!(
        outputs.get("audio"),
        Some(&serde_json::json!("final-audio"))
    );
    let stream_events: Vec<_> = sink
        .events()
        .into_iter()
        .filter(|event| matches!(event, WorkflowEvent::TaskStream { .. }))
        .collect();
    assert!(stream_events.is_empty());
}

use super::*;

#[test]
fn build_runtime_event_projection_prefers_stored_runtime_over_gateway_snapshot() {
    let stored_active_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("onnx-runtime".to_string()),
        runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
        warmup_started_at_ms: None,
        warmup_completed_at_ms: None,
        warmup_duration_ms: None,
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: false,
        last_error: None,
    };
    let stored_embedding_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp.embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-3".to_string()),
        warmup_started_at_ms: Some(10),
        warmup_completed_at_ms: Some(20),
        warmup_duration_ms: Some(10),
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    };
    let gateway_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp".to_string()),
        runtime_instance_id: Some("llama-cpp-main-1".to_string()),
        warmup_started_at_ms: Some(1),
        warmup_completed_at_ms: Some(2),
        warmup_duration_ms: Some(1),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: true,
        last_error: None,
    };
    let gateway_mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime: None,
        embedding_runtime: None,
    };

    let projection = build_runtime_event_projection(
        Some(&stored_active_runtime_snapshot),
        Some(&stored_embedding_runtime_snapshot),
        Some("/models/sidecar.onnx"),
        Some("/models/embed.gguf"),
        None,
        None,
        &gateway_snapshot,
        None,
        &gateway_mode_info,
        None,
    );

    assert_eq!(
        projection.active_runtime_snapshot.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
    assert_eq!(
        projection
            .embedding_runtime_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_id.as_deref()),
        Some("llama.cpp.embedding")
    );
    assert_eq!(
        projection.active_model_target.as_deref(),
        Some("/models/sidecar.onnx")
    );
    assert_eq!(
        projection.embedding_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    assert_eq!(
        projection.trace_runtime_metrics.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
}

#[test]
fn build_runtime_event_projection_preserves_live_embedding_snapshot_without_stored_override() {
    let gateway_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp".to_string()),
        runtime_instance_id: Some("llama-cpp-main-2".to_string()),
        warmup_started_at_ms: Some(1),
        warmup_completed_at_ms: Some(3),
        warmup_duration_ms: Some(2),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: true,
        last_error: None,
    };
    let live_embedding_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama_cpp_embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-7".to_string()),
        warmup_started_at_ms: Some(4),
        warmup_completed_at_ms: Some(7),
        warmup_duration_ms: Some(3),
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    };
    let gateway_mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime: None,
        embedding_runtime: None,
    };

    let projection = build_runtime_event_projection(
        None,
        None,
        None,
        None,
        None,
        None,
        &gateway_snapshot,
        Some(&live_embedding_runtime_snapshot),
        &gateway_mode_info,
        None,
    );

    assert_eq!(
        projection.active_model_target.as_deref(),
        Some("/models/main.gguf")
    );
    assert_eq!(
        projection.embedding_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    assert_eq!(
        projection
            .embedding_runtime_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_instance_id.as_deref()),
        Some("llama-cpp-embedding-7")
    );
}

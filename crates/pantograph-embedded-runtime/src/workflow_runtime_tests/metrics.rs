use super::*;

#[test]
fn trace_runtime_metrics_keeps_canonical_backend_lifecycle_reason() {
    let metrics = trace_runtime_metrics(
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("pytorch".to_string()),
            runtime_instance_id: Some("pytorch-1".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        },
        Some("/models/demo"),
    );

    assert_eq!(
        metrics.lifecycle_decision_reason.as_deref(),
        Some("runtime_reused")
    );
    assert_eq!(metrics.model_target.as_deref(), Some("/models/demo"));
}

#[test]
fn trace_runtime_metrics_normalizes_known_runtime_aliases() {
    let metrics = trace_runtime_metrics(
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-1".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        },
        Some("/models/main.gguf"),
    );

    assert_eq!(metrics.runtime_id.as_deref(), Some("llama_cpp"));
    assert_eq!(metrics.observed_runtime_ids, vec!["llama_cpp".to_string()]);
}

#[test]
fn normalized_runtime_lifecycle_snapshot_canonicalizes_runtime_aliases() {
    let snapshot = normalized_runtime_lifecycle_snapshot(&inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("PyTorch".to_string()),
        runtime_instance_id: Some("pytorch-1".to_string()),
        warmup_started_at_ms: None,
        warmup_completed_at_ms: None,
        warmup_duration_ms: None,
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    });

    assert_eq!(snapshot.runtime_id.as_deref(), Some("pytorch"));
    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_reused")
    );
}

#[test]
fn normalized_runtime_lifecycle_snapshot_infers_backend_owned_default_reason() {
    let snapshot = normalized_runtime_lifecycle_snapshot(&inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp".to_string()),
        runtime_instance_id: Some("llama-cpp-1".to_string()),
        warmup_started_at_ms: Some(10),
        warmup_completed_at_ms: Some(20),
        warmup_duration_ms: Some(10),
        runtime_reused: Some(false),
        lifecycle_decision_reason: None,
        active: true,
        last_error: None,
    });

    assert_eq!(snapshot.runtime_id.as_deref(), Some("llama_cpp"));
    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
}

#[test]
fn trace_runtime_metrics_with_observed_runtime_ids_preserves_all_runtime_ids() {
    let metrics = trace_runtime_metrics_with_observed_runtime_ids(
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("onnxruntime".to_string()),
            runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: false,
            last_error: None,
        },
        Some("/tmp/model.onnx"),
        &[
            "diffusers".to_string(),
            "onnx-runtime".to_string(),
            "diffusers".to_string(),
        ],
    );

    assert_eq!(metrics.runtime_id.as_deref(), Some("onnx-runtime"));
    assert_eq!(
        metrics.observed_runtime_ids,
        vec!["onnx-runtime".to_string(), "diffusers".to_string()]
    );
    assert_eq!(metrics.model_target.as_deref(), Some("/tmp/model.onnx"));
}

#[test]
fn resolve_runtime_model_target_prefers_embedding_target_for_embedding_alias() {
    let mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime: None,
        embedding_runtime: None,
    };
    let snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama_cpp_embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
        warmup_started_at_ms: None,
        warmup_completed_at_ms: None,
        warmup_duration_ms: None,
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    };

    assert_eq!(
        resolve_runtime_model_target(&mode_info, &snapshot).as_deref(),
        Some("/models/embed.gguf")
    );
}

#[test]
fn build_runtime_diagnostics_projection_prefers_execution_snapshot_override() {
    let execution_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp.embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
        warmup_started_at_ms: Some(100),
        warmup_completed_at_ms: Some(110),
        warmup_duration_ms: Some(10),
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    };
    let restored_gateway_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp".to_string()),
        runtime_instance_id: Some("llama-cpp-restore-9".to_string()),
        warmup_started_at_ms: Some(200),
        warmup_completed_at_ms: Some(240),
        warmup_duration_ms: Some(40),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: true,
        last_error: None,
    };
    let mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/restore.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime: None,
        embedding_runtime: None,
    };

    let projection = build_runtime_diagnostics_projection(
        Some(&execution_snapshot),
        &restored_gateway_snapshot,
        &mode_info,
        Some("/models/embed.gguf"),
    );

    assert_eq!(
        projection
            .active_runtime_snapshot
            .runtime_instance_id
            .as_deref(),
        Some("llama-cpp-embedding-2")
    );
    assert_eq!(
        projection.runtime_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    assert_eq!(
        projection.trace_runtime_metrics.runtime_id.as_deref(),
        Some("llama.cpp.embedding")
    );
    assert_eq!(
        projection.trace_runtime_metrics.observed_runtime_ids,
        vec!["llama.cpp.embedding".to_string()]
    );
    assert_eq!(
        projection
            .trace_runtime_metrics
            .lifecycle_decision_reason
            .as_deref(),
        Some("runtime_reused")
    );
}

use super::*;

#[test]
fn observe_runtimes_registers_active_and_embedding_runtimes() {
    let registry = RuntimeRegistry::new();

    let snapshots = registry.observe_runtimes(vec![
        RuntimeObservation {
            runtime_id: "llama.cpp".to_string(),
            display_name: "llama.cpp".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("/models/qwen.gguf".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("llama-main-1".to_string()),
            last_error: None,
        },
        RuntimeObservation {
            runtime_id: "llama.cpp.embedding".to_string(),
            display_name: "Dedicated embedding runtime".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("/models/embed.gguf".to_string()),
            status: RuntimeRegistryStatus::Warming,
            runtime_instance_id: Some("llama-embed-1".to_string()),
            last_error: None,
        },
    ]);

    assert_eq!(snapshots.len(), 2);
    let active_runtime = snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_id == "llama_cpp")
        .expect("active runtime snapshot");
    assert_eq!(active_runtime.status, RuntimeRegistryStatus::Ready);
    assert_eq!(active_runtime.models[0].model_id, "/models/qwen.gguf");

    let embedding_runtime = snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_id == "llama.cpp.embedding")
        .expect("embedding runtime snapshot");
    assert_eq!(embedding_runtime.status, RuntimeRegistryStatus::Warming);
    assert_eq!(embedding_runtime.models[0].model_id, "/models/embed.gguf");
}

#[test]
fn observe_runtimes_stops_unobserved_runtimes_without_reservations() {
    let registry = RuntimeRegistry::new();

    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "llama.cpp".to_string(),
        display_name: "llama.cpp".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("/models/qwen.gguf".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("llama-main-1".to_string()),
        last_error: None,
    }]);

    let snapshots = registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "ollama".to_string(),
        display_name: "ollama".to_string(),
        backend_keys: vec!["ollama".to_string()],
        model_id: Some("llava:13b".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("ollama-1".to_string()),
        last_error: None,
    }]);

    assert_eq!(snapshots.len(), 2);
    let llama = snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_id == "llama_cpp")
        .expect("llama snapshot");
    assert_eq!(llama.status, RuntimeRegistryStatus::Stopped);
    assert!(llama.models.is_empty());

    let ollama = snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_id == "ollama")
        .expect("ollama snapshot");
    assert_eq!(ollama.status, RuntimeRegistryStatus::Ready);
    assert_eq!(ollama.models[0].model_id, "llava:13b");
}

#[test]
fn observe_runtime_updates_single_runtime_without_stopping_others() {
    let registry = RuntimeRegistry::new();

    registry.observe_runtimes(vec![
        RuntimeObservation {
            runtime_id: "llama.cpp".to_string(),
            display_name: "llama.cpp".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("/models/qwen.gguf".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("llama-main-1".to_string()),
            last_error: None,
        },
        RuntimeObservation {
            runtime_id: "onnx-runtime".to_string(),
            display_name: "ONNX Runtime (Python sidecar)".to_string(),
            backend_keys: vec!["onnx-runtime".to_string()],
            model_id: Some("/models/voice.onnx".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
            last_error: None,
        },
    ]);

    let updated = registry.observe_runtime(RuntimeObservation {
        runtime_id: "onnx-runtime".to_string(),
        display_name: "ONNX Runtime (Python sidecar)".to_string(),
        backend_keys: vec!["onnx-runtime".to_string()],
        model_id: Some("/models/voice-v2.onnx".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
        last_error: None,
    });

    assert_eq!(updated.runtime_id, "onnx-runtime");
    assert_eq!(updated.status, RuntimeRegistryStatus::Ready);
    assert_eq!(updated.models[0].model_id, "/models/voice-v2.onnx");

    let snapshot = registry.snapshot();
    let llama = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("llama runtime should remain observed");
    assert_eq!(llama.status, RuntimeRegistryStatus::Ready);
    assert_eq!(llama.models[0].model_id, "/models/qwen.gguf");
}

use super::*;

#[test]
fn test_backend_name() {
    let backend = PyTorchBackend::new();
    assert_eq!(backend.name(), "PyTorch");
}

#[test]
fn test_capabilities() {
    let caps = PyTorchBackend::static_capabilities();
    assert!(!caps.vision);
    assert!(!caps.embeddings);
    assert!(caps.gpu);
    assert!(caps.device_selection);
    assert!(caps.streaming);
    assert!(!caps.tool_calling);
}

#[test]
fn test_not_ready_initially() {
    let backend = PyTorchBackend::new();
    assert!(!backend.is_ready());
    assert!(backend.base_url().is_none());
}

#[test]
fn test_no_loaded_model_initially() {
    let backend = PyTorchBackend::new();
    assert!(backend.loaded_model.is_none());
}

#[test]
fn test_can_reuse_loaded_model_requires_matching_request() {
    let mut backend = PyTorchBackend::new();
    backend.loaded_model = Some(LoadedModelInfo {
        model_path: "/models/demo".to_string(),
        model_type: "text-generation".to_string(),
        device: "cuda".to_string(),
    });

    assert!(backend.can_reuse_loaded_model("/models/demo", "cuda", None));
    assert!(backend.can_reuse_loaded_model("/models/demo", "cuda", Some("text-generation")));
    assert!(!backend.can_reuse_loaded_model("/models/other", "cuda", None));
    assert!(!backend.can_reuse_loaded_model("/models/demo", "cpu", None));
    assert!(!backend.can_reuse_loaded_model("/models/demo", "cuda", Some("dllm")));
}

#[test]
fn test_kv_runtime_fingerprint_for_loaded_model_is_stable() {
    let loaded = LoadedModelInfo {
        model_path: "/models/demo".to_string(),
        model_type: "dllm".to_string(),
        device: "cuda".to_string(),
    };

    let fingerprint = PyTorchBackend::kv_cache_runtime_fingerprint_for_loaded_model(&loaded);
    assert_eq!(fingerprint.backend_key, "pytorch");
    assert_eq!(fingerprint.runtime_id, "pytorch");
    assert!(fingerprint.tokenizer_fingerprint.contains("/models/demo"));
    assert_eq!(
        fingerprint.prompt_format_fingerprint.as_deref(),
        Some("pytorch_dllm")
    );
    assert_eq!(
        fingerprint.runtime_build_fingerprint.as_deref(),
        Some("cuda")
    );
}

#[test]
fn test_kv_model_fingerprint_for_loaded_model_tracks_model_identity() {
    let loaded = LoadedModelInfo {
        model_path: "/models/demo".to_string(),
        model_type: "dllm".to_string(),
        device: "cuda".to_string(),
    };

    let fingerprint = PyTorchBackend::kv_cache_model_fingerprint_for_loaded_model(&loaded);
    assert_eq!(fingerprint.model_id, "/models/demo");
    assert_eq!(fingerprint.config_hash, "pytorch:dllm");
}

#[test]
fn test_require_live_kv_slot_rejects_nonzero_slots() {
    assert!(PyTorchBackend::require_live_kv_slot(0).is_ok());
    match PyTorchBackend::require_live_kv_slot(1) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("slot_id 0"));
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn test_live_kv_fingerprint_helpers_match_loaded_model_helpers() {
    let info = PyTorchLiveKvInfo {
        token_count: 42,
        model_path: "/models/demo".to_string(),
        model_type: "dllm".to_string(),
        device: "cuda".to_string(),
    };
    let loaded = LoadedModelInfo {
        model_path: info.model_path.clone(),
        model_type: info.model_type.clone(),
        device: info.device.clone(),
    };

    assert_eq!(
        kv_cache_runtime_fingerprint_for_live_kv(&info),
        PyTorchBackend::kv_cache_runtime_fingerprint_for_loaded_model(&loaded)
    );
    assert_eq!(
        kv_cache_model_fingerprint_for_live_kv(&info),
        PyTorchBackend::kv_cache_model_fingerprint_for_loaded_model(&loaded)
    );
}

#[test]
fn test_in_process_no_base_url() {
    let backend = PyTorchBackend::new();
    assert!(backend.base_url().is_none());
}

#[test]
fn test_extract_prompt() {
    let req = serde_json::json!({
        "messages": [
            {"role": "system", "content": "You are helpful."},
            {"role": "user", "content": "Hello!"}
        ]
    });
    assert_eq!(extract_prompt_from_messages(&req).unwrap(), "Hello!");
}

#[test]
fn test_extract_system_prompt() {
    let req = serde_json::json!({
        "messages": [
            {"role": "system", "content": "Be concise."},
            {"role": "user", "content": "Hi"}
        ]
    });
    assert_eq!(extract_system_prompt(&req), Some("Be concise.".to_string()));
}

#[test]
fn test_extract_system_prompt_missing() {
    let req = serde_json::json!({
        "messages": [{"role": "user", "content": "Hi"}]
    });
    assert_eq!(extract_system_prompt(&req), None);
}

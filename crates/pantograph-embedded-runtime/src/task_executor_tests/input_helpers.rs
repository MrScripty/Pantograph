use super::*;

#[test]
fn apply_inference_setting_defaults_preserves_explicit_values() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "voice", "default": "expr-voice-5-m"},
            {"key": "speed", "default": 1.0}
        ]),
    );
    inputs.insert("voice".to_string(), serde_json::json!("custom-voice"));
    inputs.insert("speed".to_string(), serde_json::Value::Null);

    TauriTaskExecutor::apply_inference_setting_defaults(&mut inputs);

    assert_eq!(
        inputs.get("voice"),
        Some(&serde_json::json!("custom-voice"))
    );
    assert_eq!(inputs.get("speed"), Some(&serde_json::json!(1.0)));
}

#[test]
fn apply_inference_setting_defaults_resolves_option_object_values() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "voice", "default": {"label": "Leo", "value": "expr-voice-5-m"}},
            {"key": "speed", "default": 1.0}
        ]),
    );
    inputs.insert(
        "speed".to_string(),
        serde_json::json!({"label": "Fast", "value": 1.2}),
    );

    TauriTaskExecutor::apply_inference_setting_defaults(&mut inputs);

    assert_eq!(
        inputs.get("voice"),
        Some(&serde_json::json!("expr-voice-5-m"))
    );
    assert_eq!(inputs.get("speed"), Some(&serde_json::json!(1.2)));
}

#[test]
fn apply_inference_setting_defaults_resolves_allowed_value_labels() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {
                "key": "voice",
                "default": "Leo",
                "constraints": {
                    "allowed_values": [
                        {"label": "Leo", "value": "expr-voice-5-m"}
                    ]
                }
            },
            {"key": "speed", "default": 1.0}
        ]),
    );
    inputs.insert("speed".to_string(), serde_json::json!(1.2));

    TauriTaskExecutor::apply_inference_setting_defaults(&mut inputs);

    assert_eq!(
        inputs.get("voice"),
        Some(&serde_json::json!("expr-voice-5-m"))
    );
    assert_eq!(inputs.get("speed"), Some(&serde_json::json!(1.2)));
}

#[test]
fn collect_runtime_env_ids_includes_environment_ref() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "environment_ref".to_string(),
        serde_json::json!({
            "state": "ready",
            "env_id": "env:primary",
            "env_ids": ["env:extra"]
        }),
    );
    inputs.insert(
        "model_ref".to_string(),
        serde_json::json!({
            "dependencyBindings": [
                {"envId": "env:primary"},
                {"envId": "env:secondary"}
            ]
        }),
    );

    let env_ids = TauriTaskExecutor::collect_runtime_env_ids(&inputs);
    assert_eq!(
        env_ids,
        vec![
            "env:extra".to_string(),
            "env:primary".to_string(),
            "env:secondary".to_string(),
        ]
    );
}

#[test]
fn stable_hash_hex_is_deterministic() {
    let one = TauriTaskExecutor::stable_hash_hex("abc|123");
    let two = TauriTaskExecutor::stable_hash_hex("abc|123");
    let three = TauriTaskExecutor::stable_hash_hex("abc|124");
    assert_eq!(one, two);
    assert_ne!(one, three);
    assert_eq!(one.len(), 16);
}

#[test]
fn build_model_dependency_request_normalizes_backend_aliases() {
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("onnx-runtime"));

    let request = TauriTaskExecutor::build_model_dependency_request(
        "pytorch-inference",
        "/tmp/model",
        &inputs,
    );
    assert_eq!(request.backend_key.as_deref(), Some("onnx-runtime"));
}

#[test]
fn build_model_dependency_request_prefers_requirements_backend_when_input_missing() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "dependency_requirements".to_string(),
        serde_json::json!({
            "model_id": "model-a",
            "platform_key": "linux-x86_64",
            "backend_key": "torch",
            "dependency_contract_version": 1,
            "validation_state": "resolved",
            "validation_errors": [],
            "bindings": [],
            "selected_binding_ids": []
        }),
    );

    let request = TauriTaskExecutor::build_model_dependency_request(
        "pytorch-inference",
        "/tmp/model",
        &inputs,
    );
    assert_eq!(request.backend_key.as_deref(), Some("pytorch"));
}

#[test]
fn build_model_dependency_request_prefers_recommended_backend_for_diffusion() {
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("pytorch"));
    inputs.insert(
        "recommended_backend".to_string(),
        serde_json::json!("diffusers"),
    );

    let request = TauriTaskExecutor::build_model_dependency_request(
        "diffusion-inference",
        "/tmp/model",
        &inputs,
    );
    assert_eq!(request.backend_key.as_deref(), Some("diffusers"));
}

#[test]
fn build_model_dependency_request_leaves_diffusion_backend_unspecified_by_default() {
    let mut inputs = HashMap::new();
    inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));

    let request = TauriTaskExecutor::build_model_dependency_request(
        "diffusion-inference",
        "/tmp/model",
        &inputs,
    );
    assert_eq!(request.backend_key, None);
}

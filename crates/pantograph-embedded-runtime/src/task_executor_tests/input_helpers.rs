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
fn build_model_dependency_request_ignores_backend_key_for_canonical_preflight() {
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("onnx-runtime"));

    let request = TauriTaskExecutor::build_model_dependency_request("onnx-inference", &inputs);
    assert_eq!(request.backend_key.as_deref(), None);
    assert_eq!(request.model_path, "");
}

#[test]
fn build_model_dependency_request_uses_package_facts_without_selecting_backend() {
    let package_facts: serde_json::Value = serde_json::from_str(include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    ))
    .expect("package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert(
        "dependency_requirements".to_string(),
        serde_json::json!({
            "model_id": "legacy-model",
            "platform_key": "linux-x86_64",
            "backend_key": "candle",
            "dependency_contract_version": 1,
            "validation_state": "resolved",
            "validation_errors": [],
            "bindings": [],
            "selected_binding_ids": []
        }),
    );
    inputs.insert("model_type".to_string(), serde_json::json!("embedding"));
    inputs.insert("resolved_model_package_facts".to_string(), package_facts);
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({"model_id": "llm/example/tiny-transformers"}),
    );

    let request = TauriTaskExecutor::build_model_dependency_request("llm-inference", &inputs);

    assert_eq!(request.backend_key.as_deref(), None);
    assert_eq!(
        request.model_id.as_deref(),
        Some("llm/example/tiny-transformers")
    );
    assert_eq!(
        request.task_type_primary.as_deref(),
        Some("text_generation")
    );
}

#[test]
fn build_model_dependency_request_keeps_explicit_inputs_before_package_facts() {
    let package_facts: serde_json::Value = serde_json::from_str(include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    ))
    .expect("package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("llama.cpp"));
    inputs.insert("model_id".to_string(), serde_json::json!("explicit-model"));
    inputs.insert(
        "task_type_primary".to_string(),
        serde_json::json!("chat_completion"),
    );
    inputs.insert("model_package_facts".to_string(), package_facts);

    let request = TauriTaskExecutor::build_model_dependency_request("llm-inference", &inputs);

    assert_eq!(request.backend_key.as_deref(), None);
    assert_eq!(request.model_id.as_deref(), Some("explicit-model"));
    assert_eq!(
        request.task_type_primary.as_deref(),
        Some("chat_completion")
    );
}

#[test]
fn build_model_dependency_request_ignores_requirements_backend_when_input_missing() {
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

    let request = TauriTaskExecutor::build_model_dependency_request("llm-inference", &inputs);
    assert_eq!(request.backend_key.as_deref(), None);
    assert_eq!(request.model_id, None);
}

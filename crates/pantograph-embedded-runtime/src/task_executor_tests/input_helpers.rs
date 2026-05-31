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

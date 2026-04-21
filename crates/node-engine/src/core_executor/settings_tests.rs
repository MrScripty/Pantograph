use super::super::*;
use tempfile::tempdir;

#[test]
fn test_build_extra_settings_empty() {
    let inputs = HashMap::new();
    let settings = build_extra_settings(&inputs);
    assert!(settings.is_empty());
}

#[test]
fn test_build_extra_settings_uses_defaults() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "denoising_steps", "default": 8},
            {"key": "block_length", "default": 8},
        ]),
    );
    let settings = build_extra_settings(&inputs);
    assert_eq!(settings["denoising_steps"], 8);
    assert_eq!(settings["block_length"], 8);
}

#[test]
fn test_build_extra_settings_port_overrides_default() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "denoising_steps", "default": 8},
        ]),
    );
    // User connected a value to the denoising_steps port
    inputs.insert("denoising_steps".to_string(), serde_json::json!(4));
    let settings = build_extra_settings(&inputs);
    assert_eq!(settings["denoising_steps"], 4);
}

#[test]
fn test_build_extra_settings_skips_null() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "optional_param"},
        ]),
    );
    let settings = build_extra_settings(&inputs);
    assert!(!settings.contains_key("optional_param"));
}

#[test]
fn test_build_extra_settings_resolves_option_object_defaults() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "voice", "default": {"label": "Leo", "value": "expr-voice-5-m"}},
        ]),
    );
    let settings = build_extra_settings(&inputs);
    assert_eq!(settings["voice"], "expr-voice-5-m");
}

#[test]
fn test_build_extra_settings_resolves_allowed_value_label_defaults() {
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
        ]),
    );
    let settings = build_extra_settings(&inputs);
    assert_eq!(settings["voice"], "expr-voice-5-m");
}

#[test]
fn test_build_extra_settings_resolves_option_object_port_values() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "voice", "default": "expr-voice-3-f"},
        ]),
    );
    inputs.insert(
        "voice".to_string(),
        serde_json::json!({"label": "Leo", "value": "expr-voice-5-m"}),
    );
    let settings = build_extra_settings(&inputs);
    assert_eq!(settings["voice"], "expr-voice-5-m");
}

#[test]
fn test_build_extra_settings_resolves_allowed_value_label_ports() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {
                "key": "voice",
                "default": "expr-voice-3-f",
                "constraints": {
                    "allowed_values": [
                        {"label": "Leo", "value": "expr-voice-5-m"}
                    ]
                }
            },
        ]),
    );
    inputs.insert("voice".to_string(), serde_json::json!("Leo"));
    let settings = build_extra_settings(&inputs);
    assert_eq!(settings["voice"], "expr-voice-5-m");
}

#[test]
fn test_expand_settings_numeric_override_flows_into_extra_settings() {
    let mut expand_inputs = HashMap::new();
    expand_inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "speed", "label": "Speed", "param_type": "Number", "default": 1.0}
        ]),
    );

    let expanded = execute_expand_settings(&expand_inputs).unwrap();
    assert_eq!(expanded.get("speed"), Some(&serde_json::json!(1.0)));

    let mut number_inputs = HashMap::new();
    number_inputs.insert("_data".to_string(), serde_json::json!({"value": 1.2}));
    let number_output = execute_number_input(&number_inputs).unwrap();

    let mut inference_inputs = HashMap::new();
    inference_inputs.insert(
        "inference_settings".to_string(),
        expand_inputs["inference_settings"].clone(),
    );
    inference_inputs.insert("speed".to_string(), number_output["value"].clone());

    let settings = build_extra_settings(&inference_inputs);
    assert_eq!(settings.get("speed"), Some(&serde_json::json!(1.2)));
}

#[test]
fn test_execute_expand_settings_empty_schema_passes_through() {
    let mut inputs = HashMap::new();
    inputs.insert("inference_settings".to_string(), serde_json::json!([]));
    let result = execute_expand_settings(&inputs).unwrap();
    assert_eq!(result["inference_settings"], serde_json::json!([]));
    // Only the passthrough output, no parameter ports
    assert_eq!(result.len(), 1);
}

#[test]
fn test_execute_expand_settings_emits_defaults_as_ports() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "denoising_steps", "label": "Denoising Steps", "param_type": "Integer", "default": 8},
            {"key": "block_length", "label": "Block Length", "param_type": "Integer", "default": 16},
        ]),
    );
    let result = execute_expand_settings(&inputs).unwrap();
    // Schema passthrough + 2 parameter outputs
    assert_eq!(result.len(), 3);
    assert_eq!(result["denoising_steps"], 8);
    assert_eq!(result["block_length"], 16);
    // Schema is passed through unchanged
    assert!(result["inference_settings"].is_array());
}

#[test]
fn test_execute_expand_settings_uses_connected_numeric_override() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "denoising_steps", "label": "Denoising Steps", "param_type": "Integer", "default": 8}
        ]),
    );
    inputs.insert("denoising_steps".to_string(), serde_json::json!(12));

    let result = execute_expand_settings(&inputs).unwrap();

    assert_eq!(result["denoising_steps"], 12);
    assert!(result["inference_settings"].is_array());
}

#[test]
fn test_execute_expand_settings_uses_connected_label_override_runtime_value() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {
                "key": "voice",
                "label": "Voice",
                "param_type": "String",
                "default": "Leo",
                "constraints": {
                    "allowed_values": [
                        {"label": "Leo", "value": "expr-voice-5-m"},
                        {"label": "Sage", "value": "expr-voice-7-f"}
                    ]
                }
            }
        ]),
    );
    inputs.insert("voice".to_string(), serde_json::json!("Sage"));

    let result = execute_expand_settings(&inputs).unwrap();

    assert_eq!(result["voice"], "expr-voice-7-f");
    assert!(result["inference_settings"].is_array());
}

#[test]
fn test_execute_expand_settings_resolves_option_object_defaults() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "voice", "label": "Voice", "param_type": "String", "default": {"label": "Leo", "value": "expr-voice-5-m"}},
        ]),
    );
    let result = execute_expand_settings(&inputs).unwrap();
    assert_eq!(result["voice"], "expr-voice-5-m");
    assert!(result["inference_settings"].is_array());
}

#[test]
fn test_execute_expand_settings_resolves_allowed_value_label_defaults() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {
                "key": "voice",
                "label": "Voice",
                "param_type": "String",
                "default": "Leo",
                "constraints": {
                    "allowed_values": [
                        {"label": "Leo", "value": "expr-voice-5-m"}
                    ]
                }
            },
        ]),
    );
    let result = execute_expand_settings(&inputs).unwrap();
    assert_eq!(result["voice"], "expr-voice-5-m");
    assert!(result["inference_settings"].is_array());
}

#[test]
fn test_execute_expand_settings_missing_input_returns_empty_array() {
    let inputs = HashMap::new();
    let result = execute_expand_settings(&inputs).unwrap();
    assert_eq!(result["inference_settings"], serde_json::json!([]));
    assert_eq!(result.len(), 1);
}

#[tokio::test]
async fn test_execute_read_file_rejects_traversal() {
    let root = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let outside_file = outside.path().join("secret.txt");
    std::fs::write(&outside_file, "secret").unwrap();
    let root_path = root.path().to_path_buf();

    let mut inputs = HashMap::new();
    inputs.insert(
        "path".to_string(),
        serde_json::json!(format!(
            "../{}/secret.txt",
            outside.path().file_name().unwrap().to_string_lossy()
        )),
    );

    let result = execute_read_file(Some(&root_path), &inputs).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_write_file_rejects_traversal() {
    let root = tempdir().unwrap();
    let root_path = root.path().to_path_buf();
    let mut inputs = HashMap::new();
    inputs.insert("path".to_string(), serde_json::json!("../secret.txt"));
    inputs.insert("content".to_string(), serde_json::json!("blocked"));

    let result = execute_write_file(Some(&root_path), &inputs).await;
    assert!(result.is_err());
}

#[test]
fn test_read_optional_input_bool_aliases_parses_data_field() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({
            "emit_metadata": "true"
        }),
    );
    let parsed = read_optional_input_bool_aliases(&inputs, &["emit_metadata", "emitMetadata"]);
    assert_eq!(parsed, Some(true));
}

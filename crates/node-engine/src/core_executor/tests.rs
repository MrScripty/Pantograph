use super::*;
use tempfile::tempdir;

#[test]
fn test_resolve_node_type_from_data() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "text-input"}),
    );
    assert_eq!(resolve_node_type("text-input-1", &inputs), "text-input");
}

#[test]
fn test_resolve_node_type_from_task_id() {
    let inputs = HashMap::new();
    assert_eq!(resolve_node_type("text-input-1", &inputs), "text-input");
}

#[test]
fn test_resolve_node_type_no_suffix() {
    let inputs = HashMap::new();
    assert_eq!(resolve_node_type("merge", &inputs), "merge");
}

#[test]
fn test_text_input() {
    let mut inputs = HashMap::new();
    inputs.insert("_data".to_string(), serde_json::json!({"text": "hello"}));
    let result = execute_text_input(&inputs).unwrap();
    assert_eq!(result["text"], "hello");
}

#[test]
fn test_text_input_from_port() {
    let mut inputs = HashMap::new();
    inputs.insert("text".to_string(), serde_json::json!("from port"));
    let result = execute_text_input(&inputs).unwrap();
    assert_eq!(result["text"], "from port");
}

#[test]
fn test_number_input() {
    let mut inputs = HashMap::new();
    inputs.insert("_data".to_string(), serde_json::json!({"value": 1.2}));
    let result = execute_number_input(&inputs).unwrap();
    assert_eq!(result["value"], 1.2);
}

#[test]
fn test_number_input_skips_missing_value() {
    let inputs = HashMap::new();
    let result = execute_number_input(&inputs).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_boolean_input() {
    let mut inputs = HashMap::new();
    inputs.insert("_data".to_string(), serde_json::json!({"value": true}));
    let result = execute_boolean_input(&inputs).unwrap();
    assert_eq!(result["value"], true);
}

#[test]
fn test_boolean_input_skips_missing_value() {
    let inputs = HashMap::new();
    let result = execute_boolean_input(&inputs).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_selection_input() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"value": "expr-voice-5-m"}),
    );
    let result = execute_selection_input(&inputs).unwrap();
    assert_eq!(result["value"], "expr-voice-5-m");
}

#[test]
fn test_selection_input_from_port() {
    let mut inputs = HashMap::new();
    inputs.insert("value".to_string(), serde_json::json!(3));
    let result = execute_selection_input(&inputs).unwrap();
    assert_eq!(result["value"], 3);
}

#[test]
fn test_vector_input_from_array() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"vector": [0.1, 0.2, 0.3]}),
    );
    let result = execute_vector_input(&inputs).unwrap();
    assert_eq!(result["vector"], serde_json::json!([0.1, 0.2, 0.3]));
}

#[test]
fn test_vector_input_from_json_string() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"vector": "[1.0,2.0,3.5]"}),
    );
    let result = execute_vector_input(&inputs).unwrap();
    assert_eq!(result["vector"], serde_json::json!([1.0, 2.0, 3.5]));
}

#[test]
fn test_vector_output_passthrough() {
    let mut inputs = HashMap::new();
    inputs.insert("vector".to_string(), serde_json::json!([1.0, 2.0]));
    let result = execute_vector_output(&inputs).unwrap();
    assert_eq!(result["vector"], serde_json::json!([1.0, 2.0]));
}

#[test]
fn test_text_output() {
    let mut inputs = HashMap::new();
    inputs.insert("text".to_string(), serde_json::json!("output text"));
    let result = execute_text_output(&inputs).unwrap();
    assert_eq!(result["text"], "output text");
}

#[test]
fn test_conditional_true() {
    let mut inputs = HashMap::new();
    inputs.insert("condition".to_string(), serde_json::json!(true));
    inputs.insert("value".to_string(), serde_json::json!("data"));
    let result = execute_conditional(&inputs).unwrap();
    assert_eq!(result["true_out"], "data");
    assert_eq!(result["false_out"], serde_json::Value::Null);
}

#[test]
fn test_conditional_false() {
    let mut inputs = HashMap::new();
    inputs.insert("condition".to_string(), serde_json::json!(false));
    inputs.insert("value".to_string(), serde_json::json!("data"));
    let result = execute_conditional(&inputs).unwrap();
    assert_eq!(result["true_out"], serde_json::Value::Null);
    assert_eq!(result["false_out"], "data");
}

#[test]
fn test_merge_array() {
    let mut inputs = HashMap::new();
    inputs.insert("inputs".to_string(), serde_json::json!(["hello", "world"]));
    let result = execute_merge(&inputs).unwrap();
    assert_eq!(result["merged"], "hello\nworld");
    assert_eq!(result["count"], 2);
}

#[test]
fn test_merge_single() {
    let mut inputs = HashMap::new();
    inputs.insert("inputs".to_string(), serde_json::json!("single"));
    let result = execute_merge(&inputs).unwrap();
    assert_eq!(result["merged"], "single");
    assert_eq!(result["count"], 1);
}

#[test]
fn test_merge_empty() {
    let inputs = HashMap::new();
    let result = execute_merge(&inputs).unwrap();
    assert_eq!(result["merged"], "");
    assert_eq!(result["count"], 0);
}

#[test]
fn test_json_filter_simple_field() {
    let mut inputs = HashMap::new();
    inputs.insert("json".to_string(), serde_json::json!({"name": "test"}));
    inputs.insert("_data".to_string(), serde_json::json!({"path": "name"}));
    let result = execute_json_filter(&inputs).unwrap();
    assert_eq!(result["value"], "test");
    assert_eq!(result["found"], true);
}

#[test]
fn test_json_filter_nested_path() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "json".to_string(),
        serde_json::json!({"a": {"b": {"c": 42}}}),
    );
    inputs.insert("_data".to_string(), serde_json::json!({"path": "a.b.c"}));
    let result = execute_json_filter(&inputs).unwrap();
    assert_eq!(result["value"], 42);
    assert_eq!(result["found"], true);
}

#[test]
fn test_json_filter_array_index() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "json".to_string(),
        serde_json::json!({"items": [10, 20, 30]}),
    );
    inputs.insert("_data".to_string(), serde_json::json!({"path": "items[1]"}));
    let result = execute_json_filter(&inputs).unwrap();
    assert_eq!(result["value"], 20);
    assert_eq!(result["found"], true);
}

#[test]
fn test_json_filter_missing_path() {
    let mut inputs = HashMap::new();
    inputs.insert("json".to_string(), serde_json::json!({"a": 1}));
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"path": "nonexistent"}),
    );
    let result = execute_json_filter(&inputs).unwrap();
    assert_eq!(result["value"], serde_json::Value::Null);
    assert_eq!(result["found"], false);
}

#[test]
fn test_json_filter_empty_path() {
    let mut inputs = HashMap::new();
    let json_val = serde_json::json!({"a": 1});
    inputs.insert("json".to_string(), json_val.clone());
    inputs.insert("_data".to_string(), serde_json::json!({"path": ""}));
    let result = execute_json_filter(&inputs).unwrap();
    assert_eq!(result["value"], json_val);
    assert_eq!(result["found"], true);
}

#[test]
fn test_validator_valid_code() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "code".to_string(),
        serde_json::json!("<script>\nlet { name } = $props();\n</script>\n<p>{name}</p>"),
    );
    let result = execute_validator(&inputs).unwrap();
    assert_eq!(result["valid"], true);
    assert_eq!(result["error"], "");
}

#[test]
fn test_validator_forbidden_pattern() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "code".to_string(),
        serde_json::json!("<script>\nexport let name;\n</script>"),
    );
    let result = execute_validator(&inputs).unwrap();
    assert_eq!(result["valid"], false);
    assert!(result["error"].as_str().unwrap().contains("export let"));
}

#[test]
fn test_validator_unbalanced_script() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "code".to_string(),
        serde_json::json!("<script>\nlet x = 1;\n"),
    );
    let result = execute_validator(&inputs).unwrap();
    assert_eq!(result["valid"], false);
    assert!(result["error"].as_str().unwrap().contains("Unbalanced"));
}

#[test]
fn test_model_provider() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"model_name": "phi-3"}),
    );
    let result = execute_model_provider(&inputs).unwrap();
    assert_eq!(result["model_name"], "phi-3");
}

#[test]
fn test_puma_lib() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({
            "modelPath": "/models/test.gguf",
            "model_id": "llm/example/test",
            "model_type": "llm",
            "task_type_primary": "text-generation",
            "backend_key": "pytorch",
            "recommended_backend": "transformers",
            "selected_binding_ids": ["binding-a", "binding-b"],
            "platform_context": {"os":"linux","arch":"x86_64"},
            "dependency_bindings": [{"binding_id":"binding-a"}],
            "dependency_requirements": {
                "model_id": "llm/example/test",
                "platform_key": "linux-x86_64",
                "dependency_contract_version": 1,
                "validation_state": "resolved",
                "validation_errors": [],
                "bindings": [],
                "selected_binding_ids": []
            },
            "dependency_requirements_id": "requirements-1",
            "inference_settings": [
                {"key": "temperature", "default": 0.6},
                {"key": "top_p", "default": 0.95}
            ]
        }),
    );
    let result = execute_puma_lib(&inputs).unwrap();
    assert_eq!(result["model_path"], "/models/test.gguf");
    assert_eq!(result["model_id"], "llm/example/test");
    assert_eq!(result["model_type"], "llm");
    assert_eq!(result["task_type_primary"], "text-generation");
    assert_eq!(result["backend_key"], "pytorch");
    assert_eq!(result["recommended_backend"], "transformers");
    assert_eq!(
        result["selected_binding_ids"],
        serde_json::json!(["binding-a", "binding-b"])
    );
    assert_eq!(
        result["platform_context"],
        serde_json::json!({"os":"linux","arch":"x86_64"})
    );
    assert_eq!(
        result["dependency_bindings"],
        serde_json::json!([{"binding_id":"binding-a"}])
    );
    assert_eq!(
        result["dependency_requirements"],
        serde_json::json!({
            "model_id": "llm/example/test",
            "platform_key": "linux-x86_64",
            "dependency_contract_version": 1,
            "validation_state": "resolved",
            "validation_errors": [],
            "bindings": [],
            "selected_binding_ids": []
        })
    );
    assert_eq!(result["dependency_requirements_id"], "requirements-1");
    assert_eq!(
        result["inference_settings"],
        serde_json::json!([
            {"key": "temperature", "default": 0.6},
            {"key": "top_p", "default": 0.95}
        ])
    );
}

#[test]
fn test_puma_lib_missing_inference_settings_defaults_to_empty_array() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"modelPath": "/models/test.gguf"}),
    );
    let result = execute_puma_lib(&inputs).unwrap();
    assert_eq!(result["model_path"], "/models/test.gguf");
    assert_eq!(result["inference_settings"], serde_json::json!([]));
}

#[test]
fn test_human_input() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"prompt": "Enter name", "default": "Unknown"}),
    );
    inputs.insert("user_response".to_string(), serde_json::json!("Alice"));
    let result = execute_human_input(&inputs).unwrap();
    assert_eq!(result["prompt"], "Enter name");
    assert_eq!(result["value"], "Alice");
    assert_eq!(result["input"], "Alice");
}

#[test]
fn test_human_input_auto_accepts_default() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"prompt": "Enter name", "default": "Unknown", "auto_accept": true}),
    );
    let result = execute_human_input(&inputs).unwrap();
    assert_eq!(result["prompt"], "Enter name");
    assert_eq!(result["value"], "Unknown");
    assert_eq!(result["input"], "Unknown");
}

#[test]
fn test_tool_executor_is_disabled() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "tool_calls".to_string(),
        serde_json::json!([{"id": "call_1"}, {"id": "call_2"}]),
    );
    let error = execute_tool_executor(&inputs).expect_err("tool execution should be disabled");
    assert!(error.to_string().contains("tool-executor is disabled"));
}

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

#[test]
fn test_execute_vector_output_missing_vector_returns_null() {
    let inputs = HashMap::new();
    let result = execute_vector_output(&inputs).expect("vector output should not fail");
    assert!(result.get("vector").is_some_and(|value| value.is_null()));
}

#[test]
fn test_execute_vector_output_invalid_vector_returns_null() {
    let mut inputs = HashMap::new();
    inputs.insert("vector".to_string(), serde_json::json!("not-a-vector"));

    let result = execute_vector_output(&inputs).expect("vector output should not fail");
    assert!(result.get("vector").is_some_and(|value| value.is_null()));
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_execute_embedding_fails_when_gateway_missing() {
    let mut inputs = HashMap::new();
    inputs.insert("text".to_string(), serde_json::json!("hello"));
    let err = execute_embedding(None, &inputs)
        .await
        .expect_err("embedding should fail fast without gateway");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("InferenceGateway not configured"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[tokio::test]
async fn test_dependency_preflight_skips_llamacpp() {
    let inputs = HashMap::new();
    let extensions = ExecutorExtensions::new();
    let resolved = enforce_dependency_preflight("llamacpp-inference", &inputs, &extensions)
        .await
        .expect("llamacpp preflight should be skipped");
    assert!(resolved.is_none());
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[tokio::test]
async fn test_dependency_preflight_blocks_pytorch_without_resolver() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.gguf"),
    );
    let extensions = ExecutorExtensions::new();
    let err = enforce_dependency_preflight("pytorch-inference", &inputs, &extensions)
        .await
        .expect_err("pytorch preflight should require resolver");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("dependency resolver is not configured"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_canonical_backend_key_normalizes_common_aliases() {
    assert_eq!(
        canonical_backend_key(Some("  onnx-runtime  ")),
        Some("onnx-runtime".to_string())
    );
    assert_eq!(
        canonical_backend_key(Some("llama.cpp")),
        Some("llamacpp".to_string())
    );
    assert_eq!(
        canonical_backend_key(Some("llama_cpp")),
        Some("llamacpp".to_string())
    );
    assert_eq!(
        canonical_backend_key(Some("torch")),
        Some("pytorch".to_string())
    );
    assert_eq!(
        canonical_backend_key(Some("stable-audio")),
        Some("stable_audio".to_string())
    );
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_is_llamacpp_backend_name_accepts_aliases() {
    assert!(is_llamacpp_backend_name("llama.cpp"));
    assert!(is_llamacpp_backend_name("llama_cpp"));
    assert!(is_llamacpp_backend_name("llamacpp"));
    assert!(!is_llamacpp_backend_name("pytorch"));
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_uses_canonical_backend_key() {
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("onnx-runtime"));

    let request = build_model_dependency_request("pytorch-inference", "/tmp/model", &inputs);
    assert_eq!(request.backend_key.as_deref(), Some("onnx-runtime"));
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_prefers_recommended_backend_for_diffusion() {
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("pytorch"));
    inputs.insert(
        "recommended_backend".to_string(),
        serde_json::json!("diffusers"),
    );

    let request = build_model_dependency_request("diffusion-inference", "/tmp/model", &inputs);
    assert_eq!(request.backend_key.as_deref(), Some("diffusers"));
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_infer_task_type_primary_defaults_diffusion_node_to_text_to_image() {
    let inputs = HashMap::new();
    let task = infer_task_type_primary("diffusion-inference", &inputs);
    assert_eq!(task, "text-to-image");
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_defaults_diffusion_backend_to_pytorch() {
    let mut inputs = HashMap::new();
    inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));

    let request = build_model_dependency_request("diffusion-inference", "/tmp/model", &inputs);
    assert_eq!(request.backend_key, None);
    assert_eq!(request.task_type_primary.as_deref(), Some("text-to-image"));
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_parse_reranker_documents_accepts_strings_and_objects() {
    let value = serde_json::json!([
        "first",
        {"text": "second"},
        {"content": "third"},
        {"document": "fourth"}
    ]);
    let documents = parse_reranker_documents(&value).expect("documents should parse");
    assert_eq!(documents, vec!["first", "second", "third", "fourth"]);
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_parse_reranker_documents_rejects_invalid_item() {
    let value = serde_json::json!([{"id": 1}]);
    let error = parse_reranker_documents(&value).expect_err("invalid item should fail");
    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("strings or objects"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_infer_task_type_primary_detects_reranker() {
    let mut inputs = HashMap::new();
    inputs.insert("model_type".to_string(), serde_json::json!("reranker"));
    assert_eq!(infer_task_type_primary("reranker", &inputs), "reranking");
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_parse_reranker_documents_input_accepts_json_string_alias() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "documents_json".to_string(),
        serde_json::json!("[\"alpha\", {\"text\": \"beta\"}]"),
    );
    let documents = parse_reranker_documents_input(&inputs).expect("documents_json should parse");
    assert_eq!(documents, vec!["alpha", "beta"]);
}

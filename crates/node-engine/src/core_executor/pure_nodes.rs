use std::collections::HashMap;

use crate::error::{NodeEngineError, Result};

pub(crate) fn execute_text_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let text = inputs
        .get("_data")
        .and_then(|d| d.get("text"))
        .and_then(|t| t.as_str())
        .or_else(|| inputs.get("text").and_then(|t| t.as_str()))
        .unwrap_or("");

    let mut outputs = HashMap::new();
    outputs.insert("text".to_string(), serde_json::json!(text));
    Ok(outputs)
}

fn parse_number_input_value(value: &serde_json::Value) -> Option<f64> {
    if let Some(number) = value.as_f64() {
        return number.is_finite().then_some(number);
    }

    value
        .as_str()
        .and_then(|raw| raw.parse::<f64>().ok())
        .and_then(|number| number.is_finite().then_some(number))
}

pub(crate) fn execute_number_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let value = inputs
        .get("_data")
        .and_then(|d| d.get("value"))
        .cloned()
        .or_else(|| inputs.get("value").cloned());

    let Some(number) = value.as_ref().and_then(parse_number_input_value) else {
        return Ok(HashMap::new());
    };

    let mut outputs = HashMap::new();
    outputs.insert("value".to_string(), serde_json::json!(number));
    Ok(outputs)
}

fn parse_boolean_input_value(value: &serde_json::Value) -> Option<bool> {
    value.as_bool().or_else(|| match value.as_str()? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    })
}

pub(crate) fn execute_boolean_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let value = inputs
        .get("_data")
        .and_then(|d| d.get("value"))
        .cloned()
        .or_else(|| inputs.get("value").cloned());

    let Some(boolean) = value.as_ref().and_then(parse_boolean_input_value) else {
        return Ok(HashMap::new());
    };

    let mut outputs = HashMap::new();
    outputs.insert("value".to_string(), serde_json::json!(boolean));
    Ok(outputs)
}

pub(crate) fn execute_selection_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let value = inputs
        .get("_data")
        .and_then(|d| d.get("value"))
        .cloned()
        .or_else(|| inputs.get("value").cloned())
        .unwrap_or(serde_json::Value::Null);

    let mut outputs = HashMap::new();
    outputs.insert("value".to_string(), value);
    Ok(outputs)
}

fn parse_embedding_vector_value(value: &serde_json::Value) -> Option<Vec<f64>> {
    let array = value.as_array()?;
    let mut out = Vec::with_capacity(array.len());
    for item in array {
        let number = item.as_f64()?;
        if !number.is_finite() {
            return None;
        }
        out.push(number);
    }
    Some(out)
}

fn parse_embedding_vector_input(value: &serde_json::Value) -> Option<Vec<f64>> {
    if let Some(vector) = parse_embedding_vector_value(value) {
        return Some(vector);
    }
    let raw = value.as_str()?;
    let parsed: serde_json::Value = serde_json::from_str(raw).ok()?;
    parse_embedding_vector_value(&parsed)
}

pub(crate) fn execute_vector_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let raw_vector = inputs
        .get("_data")
        .and_then(|d| d.get("vector"))
        .cloned()
        .or_else(|| inputs.get("vector").cloned())
        .unwrap_or_else(|| serde_json::json!([]));
    let vector = parse_embedding_vector_input(&raw_vector).ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "vector-input expects a JSON array of finite numbers".to_string(),
        )
    })?;

    let mut outputs = HashMap::new();
    outputs.insert("vector".to_string(), serde_json::json!(vector));
    Ok(outputs)
}

pub(crate) fn execute_masked_text_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    // Try to read segments from _data.segments (UI-provided masked segments)
    let segments = inputs
        .get("_data")
        .and_then(|d| d.get("segments"))
        .and_then(|s| s.as_array());

    let masked_prompt = if let Some(segs) = segments {
        // Build a MaskedPrompt from the provided segments
        let prompt_segments: Vec<serde_json::Value> = segs
            .iter()
            .map(|seg| {
                let text = seg.get("text").and_then(|t| t.as_str()).unwrap_or("");
                let masked = seg.get("masked").and_then(|m| m.as_bool()).unwrap_or(false);
                serde_json::json!({ "text": text, "masked": masked })
            })
            .collect();

        serde_json::json!({
            "type": "masked_prompt",
            "segments": prompt_segments
        })
    } else {
        // Fall back to treating the text input as a single masked segment
        let text = inputs
            .get("_data")
            .and_then(|d| d.get("text"))
            .and_then(|t| t.as_str())
            .or_else(|| inputs.get("text").and_then(|t| t.as_str()))
            .unwrap_or("");

        serde_json::json!({
            "type": "masked_prompt",
            "segments": [{ "text": text, "masked": true }]
        })
    };

    let mut outputs = HashMap::new();
    outputs.insert("masked_prompt".to_string(), masked_prompt);
    Ok(outputs)
}

pub(crate) fn execute_text_output(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let text = inputs.get("text").and_then(|t| t.as_str()).unwrap_or("");

    let mut outputs = HashMap::new();
    outputs.insert("text".to_string(), serde_json::json!(text));
    Ok(outputs)
}

pub(crate) fn execute_vector_output(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let raw_vector = inputs
        .get("vector")
        .cloned()
        .or_else(|| inputs.get("_data").and_then(|d| d.get("vector")).cloned());

    let mut outputs = HashMap::new();
    match raw_vector {
        None => {
            outputs.insert("vector".to_string(), serde_json::Value::Null);
        }
        Some(value) if value.is_null() => {
            outputs.insert("vector".to_string(), serde_json::Value::Null);
        }
        Some(value) => {
            if let Some(vector) = parse_embedding_vector_input(&value) {
                outputs.insert("vector".to_string(), serde_json::json!(vector));
            } else {
                log::warn!("vector-output received malformed vector input; emitting null");
                outputs.insert("vector".to_string(), serde_json::Value::Null);
            }
        }
    }
    Ok(outputs)
}

pub(crate) fn execute_image_output(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let image = inputs
        .get("image")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let mut outputs = HashMap::new();
    outputs.insert("image".to_string(), image);
    Ok(outputs)
}

pub(crate) fn execute_audio_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let audio = inputs
        .get("_data")
        .and_then(|d| d.get("audio_data"))
        .cloned()
        .or_else(|| inputs.get("audio_data").cloned())
        .unwrap_or(serde_json::Value::Null);

    let mut outputs = HashMap::new();
    outputs.insert("audio".to_string(), audio);
    Ok(outputs)
}

pub(crate) fn execute_audio_output(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let audio = inputs
        .get("audio")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let mut outputs = HashMap::new();
    outputs.insert("audio".to_string(), audio);
    Ok(outputs)
}

/// Point cloud output — passthrough; the frontend renders the 3D view.
pub(crate) fn execute_point_cloud_output(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let point_cloud = inputs
        .get("point_cloud")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let mut outputs = HashMap::new();
    outputs.insert("point_cloud".to_string(), point_cloud);
    Ok(outputs)
}

pub(crate) fn execute_linked_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let value = inputs
        .get("_data")
        .and_then(|d| d.get("linked_value"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut outputs = HashMap::new();
    outputs.insert("value".to_string(), serde_json::json!(value));
    Ok(outputs)
}

pub(crate) fn execute_image_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let image = inputs
        .get("_data")
        .and_then(|d| d.get("image"))
        .cloned()
        .or_else(|| inputs.get("image").cloned())
        .unwrap_or(serde_json::Value::Null);

    let mut outputs = HashMap::new();
    outputs.insert("image".to_string(), image);
    Ok(outputs)
}

pub(crate) fn execute_component_preview(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let component = inputs
        .get("component")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let props = inputs
        .get("props")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let mut outputs = HashMap::new();
    outputs.insert(
        "rendered".to_string(),
        serde_json::json!({ "component": component, "props": props }),
    );
    Ok(outputs)
}

pub(crate) fn execute_model_provider(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let model_name = inputs
        .get("_data")
        .and_then(|d| d.get("model_name"))
        .and_then(|m| m.as_str())
        .or_else(|| inputs.get("model_name").and_then(|m| m.as_str()))
        .unwrap_or("llama2");

    let mut outputs = HashMap::new();
    outputs.insert("model_name".to_string(), serde_json::json!(model_name));
    outputs.insert(
        "model_info".to_string(),
        serde_json::json!({ "name": model_name, "model_type": "llm" }),
    );

    log::debug!("ModelProvider: providing model '{}'", model_name);
    Ok(outputs)
}

pub(crate) fn execute_puma_lib(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    fn data_string(
        inputs: &HashMap<String, serde_json::Value>,
        snake: &str,
        camel: &str,
    ) -> Option<String> {
        inputs
            .get("_data")
            .and_then(|d| d.get(snake).or_else(|| d.get(camel)))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn data_value(
        inputs: &HashMap<String, serde_json::Value>,
        snake: &str,
        camel: &str,
    ) -> Option<serde_json::Value> {
        inputs
            .get("_data")
            .and_then(|d| d.get(snake).or_else(|| d.get(camel)))
            .cloned()
            .filter(|v| !v.is_null())
    }

    let model_path = inputs
        .get("_data")
        .and_then(|d| d.get("modelPath"))
        .and_then(|m| m.as_str())
        .unwrap_or("");

    let inference_settings = inputs
        .get("_data")
        .and_then(|d| d.get("inference_settings"))
        .or_else(|| inputs.get("inference_settings"))
        .filter(|v| v.is_array())
        .cloned()
        .unwrap_or(serde_json::json!([]));

    let mut outputs = HashMap::new();
    outputs.insert("model_path".to_string(), serde_json::json!(model_path));
    outputs.insert("inference_settings".to_string(), inference_settings);
    if let Some(model_id) = data_string(inputs, "model_id", "modelId") {
        outputs.insert("model_id".to_string(), serde_json::json!(model_id));
    }
    if let Some(model_type) = data_string(inputs, "model_type", "modelType") {
        outputs.insert("model_type".to_string(), serde_json::json!(model_type));
    }
    if let Some(task_type_primary) = data_string(inputs, "task_type_primary", "taskTypePrimary") {
        outputs.insert(
            "task_type_primary".to_string(),
            serde_json::json!(task_type_primary),
        );
    }
    if let Some(backend_key) = data_string(inputs, "backend_key", "backendKey") {
        outputs.insert("backend_key".to_string(), serde_json::json!(backend_key));
    }
    if let Some(recommended_backend) =
        data_string(inputs, "recommended_backend", "recommendedBackend")
    {
        outputs.insert(
            "recommended_backend".to_string(),
            serde_json::json!(recommended_backend),
        );
    }
    if let Some(selected_binding_ids) =
        data_value(inputs, "selected_binding_ids", "selectedBindingIds").filter(|v| v.is_array())
    {
        outputs.insert("selected_binding_ids".to_string(), selected_binding_ids);
    }
    if let Some(platform_context) =
        data_value(inputs, "platform_context", "platformContext").filter(|v| v.is_object())
    {
        outputs.insert("platform_context".to_string(), platform_context);
    }
    if let Some(dependency_bindings) =
        data_value(inputs, "dependency_bindings", "dependencyBindings").filter(|v| v.is_array())
    {
        outputs.insert("dependency_bindings".to_string(), dependency_bindings);
    }
    if let Some(dependency_requirements) =
        data_value(inputs, "dependency_requirements", "dependencyRequirements")
            .filter(|v| v.is_object())
    {
        outputs.insert(
            "dependency_requirements".to_string(),
            dependency_requirements,
        );
    }
    if let Some(dependency_requirements_id) = data_string(
        inputs,
        "dependency_requirements_id",
        "dependencyRequirementsId",
    ) {
        outputs.insert(
            "dependency_requirements_id".to_string(),
            serde_json::json!(dependency_requirements_id),
        );
    }

    log::debug!("PumaLib: providing model path '{}'", model_path);
    Ok(outputs)
}

pub(crate) fn execute_conditional(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let condition = inputs
        .get("condition")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);

    let value = inputs
        .get("value")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let mut outputs = HashMap::new();
    if condition {
        outputs.insert("true_out".to_string(), value);
        outputs.insert("false_out".to_string(), serde_json::Value::Null);
    } else {
        outputs.insert("true_out".to_string(), serde_json::Value::Null);
        outputs.insert("false_out".to_string(), value);
    }
    Ok(outputs)
}

pub(crate) fn execute_merge(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let input_values: Vec<String> =
        if let Some(arr) = inputs.get("inputs").and_then(|v| v.as_array()) {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .filter(|s| !s.trim().is_empty())
                .collect()
        } else if let Some(s) = inputs.get("inputs").and_then(|v| v.as_str()) {
            if s.trim().is_empty() {
                vec![]
            } else {
                vec![s.to_string()]
            }
        } else {
            vec![]
        };

    let merged = input_values.join("\n");
    let count = input_values.len();

    let mut outputs = HashMap::new();
    outputs.insert("merged".to_string(), serde_json::json!(merged));
    outputs.insert("count".to_string(), serde_json::json!(count));
    Ok(outputs)
}

pub(crate) fn execute_human_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let prompt = human_input_prompt(inputs).unwrap_or_else(|| "Please provide input".to_string());
    let value = human_input_value(inputs).unwrap_or_else(|| serde_json::json!(""));

    let mut outputs = HashMap::new();
    outputs.insert("prompt".to_string(), serde_json::json!(prompt));
    outputs.insert("value".to_string(), value.clone());
    outputs.insert("input".to_string(), value);
    Ok(outputs)
}

pub(crate) fn human_input_prompt(inputs: &HashMap<String, serde_json::Value>) -> Option<String> {
    inputs
        .get("prompt")
        .and_then(|prompt| prompt.as_str())
        .map(|prompt| prompt.to_string())
        .or_else(|| {
            inputs
                .get("_data")
                .and_then(|data| data.get("prompt"))
                .and_then(|prompt| prompt.as_str())
                .map(|prompt| prompt.to_string())
        })
}

pub(crate) fn human_input_default_value(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    inputs
        .get("default")
        .filter(|value| !value.is_null())
        .cloned()
        .or_else(|| {
            inputs
                .get("_data")
                .and_then(|data| data.get("default"))
                .filter(|value| !value.is_null())
                .cloned()
        })
}

pub(crate) fn human_input_auto_accept(inputs: &HashMap<String, serde_json::Value>) -> bool {
    inputs
        .get("auto_accept")
        .or_else(|| inputs.get("_data").and_then(|data| data.get("auto_accept")))
        .and_then(parse_boolean_input_value)
        .unwrap_or(false)
}

pub(crate) fn human_input_response_value(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    for key in ["user_response", "user_input"] {
        if let Some(value) = inputs.get(key).filter(|value| !value.is_null()).cloned() {
            return Some(value);
        }
        if let Some(value) = inputs
            .get("_data")
            .and_then(|data| data.get(key))
            .filter(|value| !value.is_null())
            .cloned()
        {
            return Some(value);
        }
    }
    None
}

pub(crate) fn human_input_value(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    human_input_response_value(inputs).or_else(|| {
        human_input_auto_accept(inputs)
            .then(|| human_input_default_value(inputs))
            .flatten()
    })
}

pub(crate) fn execute_tool_executor(
    _inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    Err(NodeEngineError::ExecutionFailed(
        "tool-executor is disabled until backend-owned tool execution is implemented".to_string(),
    ))
}

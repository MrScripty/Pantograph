use std::collections::HashMap;

use crate::error::Result;

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

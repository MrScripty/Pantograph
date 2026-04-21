use std::collections::HashMap;

use crate::error::{NodeEngineError, Result};

use super::{build_extra_settings, build_model_ref_v2};

pub(crate) async fn execute_ollama_inference(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

    let model = inputs
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model input. Connect a Model Provider node.".to_string(),
            )
        })?;

    let system_prompt = inputs.get("system_prompt").and_then(|s| s.as_str());
    let temperature = inputs.get("temperature").and_then(|t| t.as_f64());
    let max_tokens = inputs.get("max_tokens").and_then(|m| m.as_i64());

    let mut request_body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false
    });

    if let Some(sys) = system_prompt {
        request_body["system"] = serde_json::json!(sys);
    }

    let mut options = serde_json::Map::new();
    if let Some(temp) = temperature {
        options.insert("temperature".to_string(), serde_json::json!(temp));
    }
    if let Some(max) = max_tokens {
        options.insert("num_predict".to_string(), serde_json::json!(max));
    }

    // Forward model-specific inference settings into Ollama options
    let extra_settings = build_extra_settings(inputs);
    for (key, value) in &extra_settings {
        options.insert(key.clone(), value.clone());
    }

    if !options.is_empty() {
        request_body["options"] = serde_json::Value::Object(options);
    }

    let client = reqwest::Client::new();
    let url = "http://localhost:11434/api/generate";

    log::debug!(
        "OllamaInference: sending request to {} with model '{}'",
        url,
        model
    );

    let http_response = client
        .post(url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Failed to connect to Ollama server: {}. Is Ollama running?",
                e
            ))
        })?;

    if !http_response.status().is_success() {
        let status = http_response.status();
        let error_body = http_response.text().await.unwrap_or_default();
        return Err(NodeEngineError::ExecutionFailed(format!(
            "Ollama API error ({}): {}",
            status, error_body
        )));
    }

    let response_json: serde_json::Value = http_response.json().await.map_err(|e| {
        NodeEngineError::ExecutionFailed(format!("Failed to parse Ollama response: {}", e))
    })?;

    let response_text = response_json["response"].as_str().unwrap_or("").to_string();

    let model_used = response_json["model"].as_str().unwrap_or(model).to_string();

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response_text));
    outputs.insert("model_used".to_string(), serde_json::json!(model_used));
    let model_ref = build_model_ref_v2(
        None,
        "ollama",
        &model_used,
        &model_used,
        "text-generation",
        inputs,
    );
    let model_ref_value = match serde_json::to_value(model_ref) {
        Ok(v) => v,
        Err(_) => serde_json::json!({
            "contractVersion": 2,
            "engine": "ollama",
            "modelId": model_used.clone(),
            "modelPath": model_used.clone(),
            "taskTypePrimary": "text-generation",
        }),
    };
    outputs.insert("model_ref".to_string(), model_ref_value);

    log::debug!(
        "OllamaInference: completed with {} chars using model '{}'",
        response_text.len(),
        model_used
    );

    Ok(outputs)
}

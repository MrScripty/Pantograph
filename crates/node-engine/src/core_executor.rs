//! Core task executor with built-in node handlers.
//!
//! `CoreTaskExecutor` handles all node types whose logic is not host-specific.
//! Hosts (Tauri, NIF/Elixir) only need to handle nodes that require platform
//! resources (e.g. RAG manager, UI interaction).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

#[cfg(feature = "inference-nodes")]
use inference::InferenceGateway;

use crate::engine::TaskExecutor;
use crate::error::{NodeEngineError, Result};
use crate::events::EventSink;
use crate::extensions::{extension_keys, ExecutorExtensions};
use crate::model_dependencies::{
    DependencyState, ModelDependencyBinding, ModelDependencyRequest, ModelDependencyResolver,
    ModelRefV2,
};

/// Extract the node type from task inputs or infer from the task ID.
///
/// Checks `_data.node_type` first (injected by the graph converter),
/// then falls back to stripping the trailing `-N` suffix from the task ID.
pub fn resolve_node_type(task_id: &str, inputs: &HashMap<String, serde_json::Value>) -> String {
    inputs
        .get("_data")
        .and_then(|d| d.get("node_type"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let parts: Vec<&str> = task_id.rsplitn(2, '-').collect();
            if parts.len() == 2 {
                parts[1].to_string()
            } else {
                task_id.to_string()
            }
        })
}

/// Core task executor that handles all host-independent node types.
///
/// For nodes requiring host-specific resources, wrap this in a
/// `CompositeTaskExecutor` with a host-specific fallback.
pub struct CoreTaskExecutor {
    /// Optional project root for file I/O nodes (read-file, write-file).
    project_root: Option<PathBuf>,
    /// Inference gateway for LLM nodes (llamacpp, llm-inference, vision, unload-model).
    #[cfg(feature = "inference-nodes")]
    gateway: Option<Arc<InferenceGateway>>,
    /// Optional event sink for streaming tokens during inference.
    event_sink: Option<Arc<dyn EventSink>>,
    /// Execution ID for event correlation.
    execution_id: Option<String>,
}

impl CoreTaskExecutor {
    /// Create a new core executor.
    pub fn new() -> Self {
        Self {
            project_root: None,
            #[cfg(feature = "inference-nodes")]
            gateway: None,
            event_sink: None,
            execution_id: None,
        }
    }

    /// Set the project root directory for file I/O nodes.
    pub fn with_project_root(mut self, root: PathBuf) -> Self {
        self.project_root = Some(root);
        self
    }

    /// Set the inference gateway for LLM nodes.
    #[cfg(feature = "inference-nodes")]
    pub fn with_gateway(mut self, gateway: Arc<InferenceGateway>) -> Self {
        self.gateway = Some(gateway);
        self
    }

    /// Set the event sink for streaming tokens during inference.
    pub fn with_event_sink(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Set the execution ID for event correlation.
    pub fn with_execution_id(mut self, id: String) -> Self {
        self.execution_id = Some(id);
        self
    }
}

impl Default for CoreTaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Pure node handlers
// ---------------------------------------------------------------------------

fn execute_text_input(
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

fn execute_number_input(
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

fn execute_boolean_input(
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

fn execute_selection_input(
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

fn execute_vector_input(
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

fn execute_masked_text_input(
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

fn execute_text_output(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let text = inputs.get("text").and_then(|t| t.as_str()).unwrap_or("");

    let mut outputs = HashMap::new();
    outputs.insert("text".to_string(), serde_json::json!(text));
    Ok(outputs)
}

fn execute_vector_output(
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

fn execute_image_output(
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

fn execute_audio_input(
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

fn execute_audio_output(
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
fn execute_point_cloud_output(
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

fn execute_linked_input(
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

fn execute_image_input(
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

fn execute_component_preview(
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

fn execute_model_provider(
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

fn execute_puma_lib(
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

fn execute_conditional(
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

fn execute_merge(
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

fn execute_validator(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let code = inputs
        .get("code")
        .and_then(|c| c.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing code input".to_string()))?;

    let forbidden_patterns: &[(&str, &str)] = &[
        (
            "export let ",
            "Use `let { prop } = $props()` instead of `export let prop`",
        ),
        ("on:click", "Use `onclick` instead of `on:click`"),
        ("on:change", "Use `onchange` instead of `on:change`"),
        ("on:input", "Use `oninput` instead of `on:input`"),
        ("on:submit", "Use `onsubmit` instead of `on:submit`"),
    ];

    // Strip single-line comments before checking patterns
    let code_no_comments: String = code
        .lines()
        .map(|line| {
            if let Some(idx) = line.find("//") {
                &line[..idx]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut valid = true;
    let mut error = String::new();
    let mut category = String::new();

    for (pattern, fix) in forbidden_patterns {
        if code_no_comments.contains(pattern) {
            valid = false;
            error = format!(
                "SVELTE 5 SYNTAX ERROR: Found forbidden pattern '{}'. {}.",
                pattern, fix
            );
            category = "SveltePattern".to_string();
            break;
        }
    }

    if valid {
        let script_opens = code.matches("<script").count();
        let script_closes = code.matches("</script>").count();
        if script_opens != script_closes {
            valid = false;
            error = "Unbalanced <script> tags".to_string();
            category = "SvelteCompiler".to_string();
        }
    }

    let mut outputs = HashMap::new();
    outputs.insert("valid".to_string(), serde_json::json!(valid));
    outputs.insert("error".to_string(), serde_json::json!(error));
    outputs.insert("category".to_string(), serde_json::json!(category));
    Ok(outputs)
}

fn execute_json_filter(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let json = inputs
        .get("json")
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing json input".to_string()))?;

    let path = inputs
        .get("_data")
        .and_then(|d| d.get("path"))
        .and_then(|p| p.as_str())
        .unwrap_or("");

    let (value, found) = extract_json_path(json, path);

    let mut outputs = HashMap::new();
    outputs.insert("value".to_string(), value);
    outputs.insert("found".to_string(), serde_json::json!(found));
    Ok(outputs)
}

/// Extract a value from JSON using a dot-delimited path expression.
///
/// Supports object field access (`field.subfield`), array indexing (`[0]`),
/// and combinations (`field[0].subfield`).
fn extract_json_path(json: &serde_json::Value, path: &str) -> (serde_json::Value, bool) {
    if path.is_empty() {
        return (json.clone(), true);
    }

    let mut current = json;
    let mut remaining = path;

    while !remaining.is_empty() {
        // Handle array indexing: [N]
        if remaining.starts_with('[') {
            if let Some(end) = remaining.find(']') {
                let index_str = &remaining[1..end];
                if let Ok(index) = index_str.parse::<usize>() {
                    if let Some(val) = current.get(index) {
                        current = val;
                        remaining = &remaining[end + 1..];
                        if remaining.starts_with('.') {
                            remaining = &remaining[1..];
                        }
                        continue;
                    }
                }
            }
            return (serde_json::Value::Null, false);
        }

        // Handle object field access
        let (field, rest) = if let Some(dot_pos) = remaining.find('.') {
            let bracket_pos = remaining.find('[').unwrap_or(remaining.len());
            if dot_pos < bracket_pos {
                (&remaining[..dot_pos], &remaining[dot_pos + 1..])
            } else {
                (&remaining[..bracket_pos], &remaining[bracket_pos..])
            }
        } else if let Some(bracket_pos) = remaining.find('[') {
            (&remaining[..bracket_pos], &remaining[bracket_pos..])
        } else {
            (remaining, "")
        };

        if !field.is_empty() {
            if let Some(val) = current.get(field) {
                current = val;
            } else {
                return (serde_json::Value::Null, false);
            }
        }
        remaining = rest;
    }

    (current.clone(), true)
}

fn execute_human_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let prompt = inputs
        .get("_data")
        .and_then(|d| d.get("prompt"))
        .and_then(|p| p.as_str())
        .unwrap_or("Please provide input");

    let user_input = inputs
        .get("user_input")
        .and_then(|i| i.as_str())
        .map(|s| s.to_string());

    let mut outputs = HashMap::new();
    outputs.insert("prompt".to_string(), serde_json::json!(prompt));
    outputs.insert(
        "input".to_string(),
        serde_json::json!(user_input.unwrap_or_default()),
    );
    Ok(outputs)
}

fn execute_tool_executor(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let tool_calls = inputs
        .get("tool_calls")
        .cloned()
        .unwrap_or(serde_json::json!([]));

    let results: Vec<serde_json::Value> = tool_calls
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|call| {
            let id = call.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
            serde_json::json!({
                "tool_call_id": id,
                "result": {"status": "pending", "message": "Tool execution requires external implementation"},
                "success": true,
                "error": null
            })
        })
        .collect();

    let mut outputs = HashMap::new();
    outputs.insert("results".to_string(), serde_json::json!(results));
    outputs.insert("all_success".to_string(), serde_json::json!(true));
    Ok(outputs)
}

// ---------------------------------------------------------------------------
// File I/O handlers (async, use project_root)
// ---------------------------------------------------------------------------

async fn execute_read_file(
    project_root: Option<&PathBuf>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let path = inputs
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing path input".to_string()))?;

    let allowed_root = match project_root {
        Some(root) => root.clone(),
        None => std::env::current_dir().map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to resolve current directory: {e}"))
        })?,
    };
    let full_path =
        crate::path_validation::resolve_path_within_root(path, &allowed_root).map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Invalid read path '{}': {}", path, e))
        })?;

    let content = tokio::fs::read_to_string(&full_path)
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

    let mut outputs = HashMap::new();
    outputs.insert("content".to_string(), serde_json::json!(content));
    outputs.insert(
        "path".to_string(),
        serde_json::json!(full_path.display().to_string()),
    );
    Ok(outputs)
}

async fn execute_write_file(
    project_root: Option<&PathBuf>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let path = inputs
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing path input".to_string()))?;

    let content = inputs
        .get("content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing content input".to_string()))?;

    let allowed_root = match project_root {
        Some(root) => root.clone(),
        None => std::env::current_dir().map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to resolve current directory: {e}"))
        })?,
    };
    let full_path =
        crate::path_validation::resolve_path_within_root(path, &allowed_root).map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Invalid write path '{}': {}", path, e))
        })?;

    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to create directories: {}", e))
        })?;
    }

    tokio::fs::write(&full_path, content)
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

    let mut outputs = HashMap::new();
    outputs.insert("success".to_string(), serde_json::json!(true));
    outputs.insert(
        "path".to_string(),
        serde_json::json!(full_path.display().to_string()),
    );
    Ok(outputs)
}

// ---------------------------------------------------------------------------
// Expand settings — decompose inference schema into individual port outputs
// ---------------------------------------------------------------------------

/// Expand inference settings schema into individual per-parameter outputs.
///
/// Reads the `inference_settings` JSON array, passes it through unchanged,
/// and emits each parameter's default value on a port keyed by `param.key`.
fn execute_expand_settings(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let settings_value = inputs
        .get("inference_settings")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));

    let mut outputs = HashMap::new();
    // Pass through schema unchanged for downstream inference node
    outputs.insert("inference_settings".to_string(), settings_value.clone());

    // Emit each parameter's default as an individual output port
    if let Some(schema) = settings_value.as_array() {
        for param in schema {
            if let Some(key) = param.get("key").and_then(|k| k.as_str()) {
                if let Some(default) = param.get("default") {
                    let default_value =
                        resolve_inference_setting_runtime_value(param, default.clone());
                    if !default_value.is_null() {
                        outputs.insert(key.to_string(), default_value);
                    }
                }
            }
        }
    }

    Ok(outputs)
}

// ---------------------------------------------------------------------------
// Inference settings helper
// ---------------------------------------------------------------------------

/// Build a settings map from the `inference_settings` schema and port inputs.
///
/// The `inference_settings` input carries a JSON array of parameter schemas
/// (from pumas-library). For each param in the schema, uses the connected
/// port value if present, otherwise falls back to the schema's default.
///
/// Supports both legacy primitive defaults and option objects shaped like
/// `{ "label": "...", "value": ... }` by resolving to the runtime value.
fn resolve_inference_setting_runtime_value(
    parameter: &serde_json::Value,
    value: serde_json::Value,
) -> serde_json::Value {
    let normalized = if let serde_json::Value::Object(map) = &value {
        if map.contains_key("label") {
            if let Some(option_value) = map.get("value") {
                option_value.clone()
            } else {
                value
            }
        } else {
            value
        }
    } else {
        value
    };

    let Some(candidate_label) = normalized.as_str() else {
        return normalized;
    };

    let allowed_values = parameter
        .get("constraints")
        .and_then(|constraints| constraints.get("allowed_values"))
        .and_then(|values| values.as_array());
    let Some(allowed_values) = allowed_values else {
        return normalized;
    };

    for option in allowed_values {
        if let serde_json::Value::Object(option_map) = option {
            let option_label = option_map
                .get("label")
                .or_else(|| option_map.get("name"))
                .or_else(|| option_map.get("key"))
                .and_then(|v| v.as_str());
            if option_label == Some(candidate_label) {
                if let Some(option_value) = option_map.get("value") {
                    return option_value.clone();
                }
            }
        }
    }

    normalized
}

fn build_extra_settings(
    inputs: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut settings = HashMap::new();

    let schema: Vec<serde_json::Value> = inputs
        .get("inference_settings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for param in &schema {
        if let Some(key) = param.get("key").and_then(|k| k.as_str()) {
            let value = inputs.get(key).cloned().unwrap_or_else(|| {
                param
                    .get("default")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)
            });
            let runtime_value = resolve_inference_setting_runtime_value(param, value);
            if !runtime_value.is_null() {
                settings.insert(key.to_string(), runtime_value);
            }
        }
    }

    settings
}

fn read_optional_input_string(
    inputs: &HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    inputs
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            inputs
                .get("_data")
                .and_then(|d| d.get(key))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

fn read_optional_input_value(
    inputs: &HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<serde_json::Value> {
    inputs
        .get(key)
        .cloned()
        .or_else(|| inputs.get("_data").and_then(|d| d.get(key)).cloned())
}

fn read_optional_input_string_aliases(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<String> {
    aliases
        .iter()
        .find_map(|key| read_optional_input_string(inputs, key))
}

fn read_optional_input_value_aliases(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<serde_json::Value> {
    aliases
        .iter()
        .find_map(|key| read_optional_input_value(inputs, key))
}

fn read_optional_input_bool(
    inputs: &HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<bool> {
    let value = read_optional_input_value(inputs, key)?;
    if let Some(boolean) = value.as_bool() {
        return Some(boolean);
    }
    value
        .as_str()
        .and_then(|s| match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        })
}

fn read_optional_input_bool_aliases(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<bool> {
    aliases
        .iter()
        .find_map(|key| read_optional_input_bool(inputs, key))
}

fn read_input_dependency_bindings(
    inputs: &HashMap<String, serde_json::Value>,
) -> Vec<ModelDependencyBinding> {
    let Some(raw) = read_optional_input_value(inputs, "dependency_bindings") else {
        return Vec::new();
    };
    if raw.is_null() {
        return Vec::new();
    }
    serde_json::from_value(raw).unwrap_or_default()
}

fn read_input_selected_binding_ids(inputs: &HashMap<String, serde_json::Value>) -> Vec<String> {
    let Some(raw) =
        read_optional_input_value_aliases(inputs, &["selected_binding_ids", "selectedBindingIds"])
    else {
        return Vec::new();
    };

    raw.as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.trim().is_empty())
        .collect()
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
fn infer_task_type_primary(node_type: &str, inputs: &HashMap<String, serde_json::Value>) -> String {
    if let Some(task) =
        read_optional_input_string_aliases(inputs, &["task_type_primary", "taskTypePrimary"])
    {
        if !task.trim().is_empty() {
            return task;
        }
    }

    let model_type = read_optional_input_string_aliases(inputs, &["model_type", "modelType"])
        .or_else(|| {
            inputs
                .get("_data")
                .and_then(|d| d.get("model_type"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
        .to_lowercase();

    if node_type == "audio-generation" || model_type == "audio" {
        return "text-to-audio".to_string();
    }
    if node_type == "diffusion-inference" {
        return "text-to-image".to_string();
    }

    match model_type.as_str() {
        "diffusion" => "text-to-image".to_string(),
        "vision" => "image-to-text".to_string(),
        "embedding" => "feature-extraction".to_string(),
        _ => "text-generation".to_string(),
    }
}

fn build_model_ref_v2(
    resolved: Option<ModelRefV2>,
    engine: &str,
    model_id: &str,
    model_path: &str,
    task_type_primary: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> ModelRefV2 {
    let fallback_dependency_bindings = read_input_dependency_bindings(inputs);
    let fallback_dependency_requirements_id =
        read_optional_input_string(inputs, "dependency_requirements_id");

    let mut model_ref = resolved.unwrap_or(ModelRefV2 {
        contract_version: 2,
        engine: engine.to_string(),
        model_id: model_id.to_string(),
        model_path: model_path.to_string(),
        task_type_primary: task_type_primary.to_string(),
        dependency_bindings: fallback_dependency_bindings.clone(),
        dependency_requirements_id: fallback_dependency_requirements_id.clone(),
    });

    if model_ref.contract_version != 2 {
        model_ref.contract_version = 2;
    }
    if model_ref.engine.trim().is_empty() {
        model_ref.engine = engine.to_string();
    }
    if model_ref.model_id.trim().is_empty() {
        model_ref.model_id = model_id.to_string();
    }
    if model_ref.model_path.trim().is_empty() {
        model_ref.model_path = model_path.to_string();
    }
    if model_ref.task_type_primary.trim().is_empty() {
        model_ref.task_type_primary = task_type_primary.to_string();
    }
    if model_ref.dependency_bindings.is_empty() {
        model_ref.dependency_bindings = fallback_dependency_bindings;
    }
    if model_ref.dependency_requirements_id.is_none() {
        model_ref.dependency_requirements_id = fallback_dependency_requirements_id;
    }

    model_ref
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
fn canonical_backend_key(value: Option<&str>) -> Option<String> {
    let normalized = value
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())?;
    match normalized.as_str() {
        "llama.cpp" | "llama-cpp" | "llamacpp" => Some("llamacpp".to_string()),
        "onnxruntime" | "onnx-runtime" | "onnx_runtime" => Some("onnx-runtime".to_string()),
        "torch" | "pytorch" => Some("pytorch".to_string()),
        "stable-audio" | "stable_audio" => Some("stable_audio".to_string()),
        other => Some(other.to_string()),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
fn infer_backend_key(node_type: &str) -> String {
    match node_type {
        "audio-generation" => "stable_audio".to_string(),
        "pytorch-inference" => "pytorch".to_string(),
        "diffusion-inference" => "pytorch".to_string(),
        "llamacpp-inference" => "llamacpp".to_string(),
        "ollama-inference" => "ollama".to_string(),
        "onnx-inference" => "onnx-runtime".to_string(),
        _ => "pytorch".to_string(),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
fn build_model_dependency_request(
    node_type: &str,
    model_path: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> ModelDependencyRequest {
    let backend_key = read_optional_input_string_aliases(inputs, &["backend_key", "backendKey"])
        .and_then(|value| canonical_backend_key(Some(value.as_str())))
        .unwrap_or_else(|| infer_backend_key(node_type));

    let task_type_primary =
        read_optional_input_string_aliases(inputs, &["task_type_primary", "taskTypePrimary"])
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| infer_task_type_primary(node_type, inputs));

    ModelDependencyRequest {
        node_type: node_type.to_string(),
        model_path: model_path.to_string(),
        model_id: read_optional_input_string_aliases(inputs, &["model_id", "modelId"]),
        model_type: read_optional_input_string_aliases(inputs, &["model_type", "modelType"]),
        task_type_primary: Some(task_type_primary),
        backend_key: Some(backend_key),
        platform_context: read_optional_input_value_aliases(
            inputs,
            &["platform_context", "platformContext"],
        ),
        selected_binding_ids: read_input_selected_binding_ids(inputs),
        dependency_override_patches: Vec::new(),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
async fn enforce_dependency_preflight(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
) -> Result<Option<ModelRefV2>> {
    if node_type != "pytorch-inference"
        && node_type != "diffusion-inference"
        && node_type != "audio-generation"
    {
        return Ok(None);
    }

    let Some(resolver) = extensions
        .get::<Arc<dyn ModelDependencyResolver>>(extension_keys::MODEL_DEPENDENCY_RESOLVER)
    else {
        return Err(NodeEngineError::ExecutionFailed(
            "Dependency preflight blocked execution: dependency resolver is not configured"
                .to_string(),
        ));
    };

    let model_path = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?;

    let request = build_model_dependency_request(node_type, model_path, inputs);
    let requirements = resolver
        .resolve_model_dependency_requirements(request.clone())
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Dependency preflight requirements resolution failed for '{}': {}",
                node_type, e
            ))
        })?;

    let status = resolver
        .check_dependencies(request.clone())
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Dependency preflight check failed for '{}': {}",
                node_type, e
            ))
        })?;

    if status.state != DependencyState::Ready {
        let payload = serde_json::json!({
            "kind": "dependency_preflight",
            "node_type": node_type,
            "model_path": model_path,
            "validation_state": requirements.validation_state,
            "validation_errors": requirements.validation_errors,
            "selected_binding_ids": requirements.selected_binding_ids,
            "state": status.state,
            "code": status.code,
            "bindings": status.bindings,
            "message": status.message,
        });
        return Err(NodeEngineError::ExecutionFailed(format!(
            "Dependency preflight blocked execution: {}",
            payload
        )));
    }

    let resolved = resolver
        .resolve_model_ref(request, Some(requirements))
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Dependency preflight failed to resolve model_ref: {}",
                e
            ))
        })?;
    if let Some(ref model_ref) = resolved {
        model_ref
            .validate()
            .map_err(NodeEngineError::ExecutionFailed)?;
    }

    Ok(resolved)
}

// ---------------------------------------------------------------------------
// Ollama (pure HTTP, no gateway needed)
// ---------------------------------------------------------------------------

async fn execute_ollama_inference(
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

// ---------------------------------------------------------------------------
// TaskExecutor implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl TaskExecutor for CoreTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &graph_flow::Context,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let node_type = resolve_node_type(task_id, &inputs);
        let _ = extensions;

        log::debug!(
            "CoreTaskExecutor: executing '{}' (type '{}')",
            task_id,
            node_type
        );

        match node_type.as_str() {
            // Input nodes
            "text-input" => execute_text_input(&inputs),
            "number-input" => execute_number_input(&inputs),
            "boolean-input" => execute_boolean_input(&inputs),
            "selection-input" => execute_selection_input(&inputs),
            "vector-input" => execute_vector_input(&inputs),
            "masked-text-input" => execute_masked_text_input(&inputs),
            "linked-input" => execute_linked_input(&inputs),
            "image-input" => execute_image_input(&inputs),
            "audio-input" => execute_audio_input(&inputs),

            // Output nodes
            "text-output" => execute_text_output(&inputs),
            "vector-output" => execute_vector_output(&inputs),
            "image-output" => execute_image_output(&inputs),
            "audio-output" => execute_audio_output(&inputs),
            "point-cloud-output" => execute_point_cloud_output(&inputs),
            "component-preview" => execute_component_preview(&inputs),

            // Model/provider nodes
            "model-provider" => execute_model_provider(&inputs),
            "puma-lib" => execute_puma_lib(&inputs),

            // Control flow nodes
            "conditional" => execute_conditional(&inputs),
            "merge" => execute_merge(&inputs),

            // Processing nodes
            "validator" => execute_validator(&inputs),
            "json-filter" => execute_json_filter(&inputs),
            "expand-settings" => execute_expand_settings(&inputs),

            // File I/O nodes
            "read-file" => execute_read_file(self.project_root.as_ref(), &inputs).await,
            "write-file" => execute_write_file(self.project_root.as_ref(), &inputs).await,

            // Interaction nodes
            "human-input" => execute_human_input(&inputs),
            "tool-executor" => execute_tool_executor(&inputs),

            // Pure HTTP inference
            "ollama-inference" => execute_ollama_inference(&inputs).await,

            // Gateway-backed inference nodes (require `inference-nodes` feature)
            #[cfg(feature = "inference-nodes")]
            "embedding" => execute_embedding(self.gateway.as_ref(), &inputs).await,
            #[cfg(feature = "inference-nodes")]
            "llamacpp-inference" => {
                let resolved_model_ref =
                    enforce_dependency_preflight("llamacpp-inference", &inputs, extensions).await?;
                let exec_id = self.execution_id.as_deref().unwrap_or("unknown");
                execute_llamacpp_inference(
                    self.gateway.as_ref(),
                    &inputs,
                    task_id,
                    self.event_sink.as_ref(),
                    exec_id,
                    resolved_model_ref,
                )
                .await
            }
            #[cfg(feature = "inference-nodes")]
            "llm-inference" => {
                let exec_id = self.execution_id.as_deref().unwrap_or("unknown");
                execute_llm_inference(
                    self.gateway.as_ref(),
                    &inputs,
                    task_id,
                    self.event_sink.as_ref(),
                    exec_id,
                )
                .await
            }
            #[cfg(feature = "inference-nodes")]
            "vision-analysis" => execute_vision_analysis(self.gateway.as_ref(), &inputs).await,
            #[cfg(feature = "inference-nodes")]
            "unload-model" => execute_unload_model(self.gateway.as_ref(), &inputs).await,

            // KV cache operations (require inference-nodes feature)
            #[cfg(feature = "inference-nodes")]
            "kv-cache-save" => execute_kv_cache_save(&inputs, extensions).await,
            #[cfg(feature = "inference-nodes")]
            "kv-cache-load" => execute_kv_cache_load(&inputs, extensions).await,
            #[cfg(feature = "inference-nodes")]
            "kv-cache-truncate" => execute_kv_cache_truncate(&inputs, extensions).await,

            // PyTorch inference (in-process via PyO3)
            #[cfg(feature = "pytorch-nodes")]
            "pytorch-inference" => {
                let resolved_model_ref =
                    enforce_dependency_preflight("pytorch-inference", &inputs, extensions).await?;
                let exec_id = self.execution_id.as_deref().unwrap_or("unknown");
                execute_pytorch_inference(
                    &inputs,
                    task_id,
                    self.event_sink.as_ref(),
                    exec_id,
                    resolved_model_ref,
                )
                .await
            }

            // Audio generation (in-process via PyO3 + Stable Audio)
            #[cfg(feature = "audio-nodes")]
            "audio-generation" => {
                let resolved_model_ref =
                    enforce_dependency_preflight("audio-generation", &inputs, extensions).await?;
                execute_audio_generation(&inputs, resolved_model_ref).await
            }

            // Unknown — signal that this node requires a host-specific executor
            _ => Err(NodeEngineError::ExecutionFailed(format!(
                "Node type '{}' requires host-specific executor",
                node_type
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Gateway-backed inference handlers (behind feature flag)
// ---------------------------------------------------------------------------

#[cfg(feature = "inference-nodes")]
fn require_gateway(gateway: Option<&Arc<InferenceGateway>>) -> Result<&Arc<InferenceGateway>> {
    gateway.ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "InferenceGateway not configured: requires host-specific executor".to_string(),
        )
    })
}

/// Resolve a model path that may be a directory to the actual `.gguf` file inside.
///
/// pumas-library stores directory paths; llama.cpp needs the `.gguf` file.
#[cfg(feature = "inference-nodes")]
fn resolve_gguf_path(path: &str) -> Result<String> {
    let p = std::path::Path::new(path);
    if p.is_dir() {
        let gguf = std::fs::read_dir(p)
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Cannot read model directory '{}': {}",
                    path, e
                ))
            })?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
            })
            .ok_or_else(|| {
                NodeEngineError::ExecutionFailed(format!(
                    "No .gguf file found in model directory '{}'",
                    path
                ))
            })?;
        Ok(gguf.path().to_string_lossy().into_owned())
    } else {
        Ok(path.to_string())
    }
}

#[cfg(feature = "inference-nodes")]
async fn execute_llamacpp_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
    resolved_model_ref: Option<ModelRefV2>,
) -> Result<HashMap<String, serde_json::Value>> {
    use futures_util::StreamExt;

    let gw = require_gateway(gateway)?;

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

    let model_path_raw = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?;

    let model_path = resolve_gguf_path(model_path_raw)?;
    let system_prompt = inputs.get("system_prompt").and_then(|s| s.as_str());
    let temperature = inputs
        .get("temperature")
        .and_then(|t| t.as_f64())
        .unwrap_or(0.7);
    let max_tokens = inputs
        .get("max_tokens")
        .and_then(|m| m.as_i64())
        .unwrap_or(512);

    // Read model-specific inference settings
    let extra_settings = build_extra_settings(inputs);

    // Ensure gateway is ready — start if needed
    if !gw.is_ready().await {
        let mut config = inference::BackendConfig {
            model_path: Some(PathBuf::from(&model_path)),
            device: Some("auto".to_string()),
            gpu_layers: Some(-1),
            embedding_mode: false,
            ..Default::default()
        };

        // Apply model-specific settings to backend config
        if let Some(v) = extra_settings.get("gpu_layers").and_then(|v| v.as_i64()) {
            config.gpu_layers = Some(v as i32);
        }
        if let Some(v) = extra_settings
            .get("context_length")
            .and_then(|v| v.as_i64())
        {
            config.context_size = Some(v as u32);
        }

        log::info!(
            "LlamaCppInference: starting server with model '{}'",
            model_path
        );
        gw.start(&config).await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to start llama.cpp server: {}", e))
        })?;

        // Wait for readiness with timeout
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !gw.is_ready().await {
            if std::time::Instant::now() > deadline {
                return Err(NodeEngineError::ExecutionFailed(
                    "Timeout waiting for llama.cpp server to start".to_string(),
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        log::info!("LlamaCppInference: server is ready");
    }

    let base_url = gw.base_url().await.ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "llama.cpp server started but no URL available".to_string(),
        )
    })?;

    let full_prompt = if let Some(sys) = system_prompt {
        format!("{}\n\n{}", sys, prompt)
    } else {
        prompt.to_string()
    };

    let streaming = event_sink.is_some();
    let request_body = serde_json::json!({
        "prompt": full_prompt,
        "n_predict": max_tokens,
        "temperature": temperature,
        "stop": ["</s>", "<|im_end|>", "<|end|>"],
        "stream": streaming
    });

    let client = reqwest::Client::new();
    let url = format!("{}/completion", base_url);

    log::debug!(
        "LlamaCppInference: sending request to {} (stream={})",
        url,
        streaming
    );

    let http_response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Failed to connect to llama.cpp server at {}: {}",
                url, e
            ))
        })?;

    if !http_response.status().is_success() {
        let status = http_response.status();
        let error_body = http_response.text().await.unwrap_or_default();
        return Err(NodeEngineError::ExecutionFailed(format!(
            "llama.cpp API error ({}): {}",
            status, error_body
        )));
    }

    let response_text = if let Some(sink) = event_sink {
        // Streaming path: parse SSE and emit per-token events
        let mut full_response = String::new();
        let mut byte_stream = http_response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                NodeEngineError::ExecutionFailed(format!("Stream read error: {}", e))
            })?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines from buffer
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if let Some(token) = parse_llamacpp_sse_content(&line) {
                    full_response.push_str(&token);
                    let _ = sink.send(crate::WorkflowEvent::task_stream(
                        task_id,
                        execution_id,
                        "response",
                        serde_json::json!(token),
                    ));
                }
            }
        }
        // Process any remaining data in buffer
        let line = buffer.trim().to_string();
        if let Some(token) = parse_llamacpp_sse_content(&line) {
            full_response.push_str(&token);
            let _ = sink.send(crate::WorkflowEvent::task_stream(
                task_id,
                execution_id,
                "response",
                serde_json::json!(token),
            ));
        }

        full_response
    } else {
        // Non-streaming path: collect entire response
        let response_json: serde_json::Value = http_response.json().await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to parse llama.cpp response: {}", e))
        })?;
        response_json["content"].as_str().unwrap_or("").to_string()
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response_text));
    outputs.insert("model_path".to_string(), serde_json::json!(model_path));
    let task_type_primary = infer_task_type_primary("llamacpp-inference", inputs);
    let model_ref = build_model_ref_v2(
        resolved_model_ref,
        "llamacpp",
        &model_path,
        &model_path,
        &task_type_primary,
        inputs,
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::to_value(model_ref).unwrap_or_else(|_| {
            serde_json::json!({
                "contractVersion": 2,
                "engine": "llamacpp",
                "modelId": model_path,
                "modelPath": model_path,
                "taskTypePrimary": task_type_primary,
            })
        }),
    );
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
async fn execute_embedding(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;

    let text = inputs
        .get("text")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing text input".to_string()))?;
    if text.trim().is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Embedding input text cannot be empty".to_string(),
        ));
    }

    let backend_name = gw.current_backend_name().await;
    if backend_name != "llama.cpp" {
        return Err(NodeEngineError::ExecutionFailed(format!(
            "LlamaCpp Embedding blocked execution: active backend '{}' is not supported",
            backend_name
        )));
    }

    let model = read_optional_input_string_aliases(
        inputs,
        &["model", "model_name", "modelName", "model_id", "modelId"],
    )
    .filter(|s| !s.trim().is_empty())
    .unwrap_or_else(|| "default".to_string());

    if !gw.is_ready().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: backend is not ready. Start llama.cpp in embedding mode (`--embeddings`) first".to_string(),
        ));
    }
    if !gw.is_embedding_mode().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: backend is running in inference mode. Restart with `--embeddings`".to_string(),
        ));
    }
    let capabilities = gw.capabilities().await;
    if !capabilities.embeddings {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: active backend does not support embeddings"
                .to_string(),
        ));
    }

    let emit_metadata =
        read_optional_input_bool_aliases(inputs, &["emit_metadata", "emitMetadata"])
            .unwrap_or(false);

    let start = std::time::Instant::now();
    let results = gw
        .embeddings(vec![text.to_string()], &model)
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("LlamaCpp Embedding request failed: {}", e))
        })?;
    let embedding = results.first().ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned no vectors for input text".to_string(),
        )
    })?;
    if embedding.vector.is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned an empty vector".to_string(),
        ));
    }
    if embedding.vector.iter().any(|v| !v.is_finite()) {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned invalid vector values".to_string(),
        ));
    }

    let mut outputs = HashMap::new();
    outputs.insert("embedding".to_string(), serde_json::json!(embedding.vector));
    if emit_metadata {
        outputs.insert(
            "metadata".to_string(),
            serde_json::json!({
                "backend": "llamacpp",
                "model": model,
                "vector_length": embedding.vector.len(),
                "duration_ms": start.elapsed().as_millis(),
            }),
        );
    }

    Ok(outputs)
}

/// Parse a llama.cpp `/completion` SSE data line into a content token.
///
/// llama.cpp streams `data: {"content": "token", ...}` per line.
#[cfg(feature = "inference-nodes")]
fn parse_llamacpp_sse_content(line: &str) -> Option<String> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    json.get("content")
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Parse an OpenAI-compatible `/v1/chat/completions` SSE data line into a content token.
///
/// Streams `data: {"choices": [{"delta": {"content": "token"}}]}` per line.
#[cfg(feature = "inference-nodes")]
fn parse_openai_sse_content(line: &str) -> Option<String> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    json.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("delta"))
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[cfg(feature = "inference-nodes")]
async fn execute_llm_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
) -> Result<HashMap<String, serde_json::Value>> {
    use futures_util::StreamExt;

    let gw = require_gateway(gateway)?;

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

    let system_prompt = inputs.get("system_prompt").and_then(|p| p.as_str());
    let extra_context = inputs.get("context").and_then(|c| c.as_str());

    if !gw.is_ready().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LLM server is not ready".to_string(),
        ));
    }

    let base_url = gw.base_url().await.ok_or_else(|| {
        NodeEngineError::ExecutionFailed("No LLM server URL available".to_string())
    })?;

    let full_prompt = if let Some(ctx) = extra_context {
        format!("{}\n\nContext:\n{}", prompt, ctx)
    } else {
        prompt.to_string()
    };

    let mut messages = Vec::new();
    if let Some(sys) = system_prompt {
        messages.push(serde_json::json!({"role": "system", "content": sys}));
    }
    messages.push(serde_json::json!({"role": "user", "content": full_prompt}));

    let streaming = event_sink.is_some();
    let mut request_body = serde_json::json!({
        "model": "gpt-4",
        "messages": messages,
        "stream": streaming
    });

    // Forward model-specific inference settings into the request body
    let extra_settings = build_extra_settings(inputs);
    for (key, value) in &extra_settings {
        request_body[key] = value.clone();
    }

    let client = reqwest::Client::new();
    let http_response = client
        .post(format!("{}/v1/chat/completions", base_url))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("LLM request failed: {}", e)))?;

    if !http_response.status().is_success() {
        let error = http_response.text().await.unwrap_or_default();
        return Err(NodeEngineError::ExecutionFailed(format!(
            "LLM error: {}",
            error
        )));
    }

    let response = if let Some(sink) = event_sink {
        // Streaming path: parse SSE and emit per-token events
        let mut full_response = String::new();
        let mut byte_stream = http_response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                NodeEngineError::ExecutionFailed(format!("Stream read error: {}", e))
            })?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if let Some(token) = parse_openai_sse_content(&line) {
                    full_response.push_str(&token);
                    let _ = sink.send(crate::WorkflowEvent::task_stream(
                        task_id,
                        execution_id,
                        "response",
                        serde_json::json!(token),
                    ));
                }
            }
        }
        let line = buffer.trim().to_string();
        if let Some(token) = parse_openai_sse_content(&line) {
            full_response.push_str(&token);
            let _ = sink.send(crate::WorkflowEvent::task_stream(
                task_id,
                execution_id,
                "response",
                serde_json::json!(token),
            ));
        }

        full_response
    } else {
        // Non-streaming path: collect entire response
        let json: serde_json::Value = http_response.json().await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to parse response: {}", e))
        })?;
        json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response));
    outputs.insert("stream".to_string(), serde_json::Value::Null);
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
async fn execute_vision_analysis(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;

    let image_base64 = inputs
        .get("image")
        .and_then(|i| i.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing image input".to_string()))?;

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

    if !gw.is_ready().await {
        return Err(NodeEngineError::ExecutionFailed(
            "Vision server is not ready".to_string(),
        ));
    }

    let base_url = gw.base_url().await.ok_or_else(|| {
        NodeEngineError::ExecutionFailed("No vision server URL available".to_string())
    })?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", base_url))
        .json(&serde_json::json!({
            "model": "gpt-4-vision-preview",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", image_base64)
                        }
                    }
                ]
            }],
            "max_tokens": 4096
        }))
        .send()
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Vision request failed: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(NodeEngineError::ExecutionFailed(format!(
            "Vision API error: {}",
            error_text
        )));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| {
        NodeEngineError::ExecutionFailed(format!("Failed to parse response: {}", e))
    })?;

    let analysis = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let mut outputs = HashMap::new();
    outputs.insert("analysis".to_string(), serde_json::json!(analysis));
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
async fn execute_unload_model(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let model_ref_value = inputs.get("model_ref").ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "Missing model_ref input. Connect an inference node's Model Reference output."
                .to_string(),
        )
    })?;
    let model_ref =
        ModelRefV2::validate_value(model_ref_value).map_err(NodeEngineError::ExecutionFailed)?;

    let engine = model_ref.engine.as_str();
    let model_id = model_ref.model_id.as_str();

    let trigger_value = inputs
        .get("trigger")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    log::info!(
        "UnloadModel: unloading '{}' from engine '{}'",
        model_id,
        engine
    );

    match engine {
        "llamacpp" => {
            let gw = require_gateway(gateway)?;
            gw.stop().await;
            log::info!(
                "UnloadModel: llama.cpp server stopped for model '{}'",
                model_id
            );
        }
        "ollama" => {
            let client = reqwest::Client::new();
            let url = "http://localhost:11434/api/generate";
            let request_body = serde_json::json!({
                "model": model_id,
                "keep_alive": 0
            });

            match client.post(url).json(&request_body).send().await {
                Ok(resp) if resp.status().is_success() => {
                    log::info!(
                        "UnloadModel: Ollama model '{}' unloaded from VRAM",
                        model_id
                    );
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    log::warn!(
                        "UnloadModel: Ollama unload returned {} for model '{}': {}",
                        status,
                        model_id,
                        body
                    );
                }
                Err(e) => {
                    return Err(NodeEngineError::ExecutionFailed(format!(
                        "Failed to connect to Ollama server to unload model '{}': {}",
                        model_id, e
                    )));
                }
            }
        }
        #[cfg(feature = "pytorch-nodes")]
        "pytorch" => {
            use pyo3::types::PyAnyMethods;
            // Unload via PyO3 in-process call to the Python worker
            let model_id_owned = model_id.to_string();
            tokio::task::spawn_blocking(move || {
                pyo3::Python::with_gil(|py| {
                    if let Ok(worker) = py.import("pantograph_torch_worker") {
                        let _ = worker.call_method0("unload_model");
                    }
                });
            })
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to unload PyTorch model '{}': {}",
                    model_id_owned, e
                ))
            })?;
            log::info!("UnloadModel: PyTorch model '{}' unloaded", model_id);
        }
        #[cfg(feature = "audio-nodes")]
        "stable_audio" => {
            use pyo3::types::PyAnyMethods;
            let model_id_owned = model_id.to_string();
            tokio::task::spawn_blocking(move || {
                pyo3::Python::with_gil(|py| {
                    if let Ok(worker) = py.import("pantograph_audio_worker") {
                        let _ = worker.call_method0("unload_model");
                    }
                });
            })
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to unload audio model '{}': {}",
                    model_id_owned, e
                ))
            })?;
            log::info!("UnloadModel: audio model '{}' unloaded", model_id);
        }
        "onnx-runtime" | "onnxruntime" => {
            log::info!(
                "UnloadModel: onnx-runtime model '{}' does not keep a shared runtime session",
                model_id
            );
        }
        other => {
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Unknown inference engine '{}'. Supported: llamacpp, ollama, pytorch, stable_audio, onnx-runtime",
                other
            )));
        }
    }

    let status_msg = format!("Model '{}' unloaded from {}", model_id, engine);

    let mut outputs = HashMap::new();
    outputs.insert("status".to_string(), serde_json::json!(status_msg));
    outputs.insert("trigger_passthrough".to_string(), trigger_value);
    Ok(outputs)
}

/// Ensure the PyTorch worker module (and its sibling modules) are loaded into
/// the Python interpreter.  Safe to call multiple times — only the first call
/// actually loads.
#[cfg(feature = "pytorch-nodes")]
fn ensure_torch_worker_initialised(py: pyo3::Python<'_>) -> std::result::Result<(), String> {
    if py.import("pantograph_torch_worker").is_ok() {
        return Ok(());
    }

    use pyo3::types::PyAnyMethods;

    let sys = py
        .import("sys")
        .map_err(|e| format!("Failed to import sys: {}", e))?;
    let modules = sys
        .getattr("modules")
        .map_err(|e| format!("Failed to get sys.modules: {}", e))?;

    // Register sibling modules first so worker.py's imports resolve
    let bd_code = std::ffi::CString::new(include_str!("../../inference/torch/block_diffusion.py"))
        .map_err(|e| format!("Invalid block_diffusion source: {}", e))?;
    let bd_module =
        pyo3::types::PyModule::from_code(py, &bd_code, c"block_diffusion.py", c"block_diffusion")
            .map_err(|e| format!("Failed to load block_diffusion: {}", e))?;
    modules
        .set_item("block_diffusion", &bd_module)
        .map_err(|e| format!("Failed to register block_diffusion: {}", e))?;

    let ar_code = std::ffi::CString::new(include_str!("../../inference/torch/autoregressive.py"))
        .map_err(|e| format!("Invalid autoregressive source: {}", e))?;
    let ar_module =
        pyo3::types::PyModule::from_code(py, &ar_code, c"autoregressive.py", c"autoregressive")
            .map_err(|e| format!("Failed to load autoregressive: {}", e))?;
    modules
        .set_item("autoregressive", &ar_module)
        .map_err(|e| format!("Failed to register autoregressive: {}", e))?;

    // Now load the worker module (which imports from block_diffusion and autoregressive)
    let code = std::ffi::CString::new(include_str!("../../inference/torch/worker.py"))
        .map_err(|e| format!("Invalid worker source: {}", e))?;
    pyo3::types::PyModule::from_code(
        py,
        &code,
        c"pantograph_torch_worker",
        c"pantograph_torch_worker",
    )
    .map_err(|e| format!("Failed to load worker: {}", e))?;

    log::info!(
        "PyTorch worker module initialised (with block_diffusion + autoregressive siblings)"
    );
    Ok(())
}

/// Ensure the Stable Audio worker module (and its sibling) are loaded into
/// the Python interpreter.  Safe to call multiple times — only the first call
/// actually loads.
#[cfg(feature = "audio-nodes")]
fn ensure_audio_worker_initialised(py: pyo3::Python<'_>) -> std::result::Result<(), String> {
    if py.import("pantograph_audio_worker").is_ok() {
        return Ok(());
    }

    use pyo3::types::PyAnyMethods;

    let sys = py
        .import("sys")
        .map_err(|e| format!("Failed to import sys: {}", e))?;
    let modules = sys
        .getattr("modules")
        .map_err(|e| format!("Failed to get sys.modules: {}", e))?;

    // Register sibling module first so worker.py's imports resolve
    let sa_code = std::ffi::CString::new(include_str!("../../inference/audio/stable_audio.py"))
        .map_err(|e| format!("Invalid stable_audio source: {}", e))?;
    let sa_module =
        pyo3::types::PyModule::from_code(py, &sa_code, c"stable_audio.py", c"stable_audio")
            .map_err(|e| format!("Failed to load stable_audio: {}", e))?;
    modules
        .set_item("stable_audio", &sa_module)
        .map_err(|e| format!("Failed to register stable_audio: {}", e))?;

    // Now load the worker module (which imports from stable_audio)
    let code = std::ffi::CString::new(include_str!("../../inference/audio/worker.py"))
        .map_err(|e| format!("Invalid audio worker source: {}", e))?;
    pyo3::types::PyModule::from_code(
        py,
        &code,
        c"pantograph_audio_worker",
        c"pantograph_audio_worker",
    )
    .map_err(|e| format!("Failed to load audio worker: {}", e))?;

    log::info!("Audio worker module initialised (with stable_audio sibling)");
    Ok(())
}

#[cfg(feature = "pytorch-nodes")]
async fn execute_pytorch_inference(
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
    resolved_model_ref: Option<ModelRefV2>,
) -> Result<HashMap<String, serde_json::Value>> {
    // Detect if the prompt input is a masked prompt JSON object
    let masked_prompt_json = inputs
        .get("prompt")
        .filter(|p| p.get("type").and_then(|t| t.as_str()) == Some("masked_prompt"))
        .map(|p| serde_json::to_string(p).unwrap_or_default());

    let prompt = if let Some(p_str) = inputs.get("prompt").and_then(|p| p.as_str()) {
        p_str.to_string()
    } else if let Some(p_obj) = inputs.get("prompt") {
        // For masked prompt objects, concatenate all segment texts as the plain prompt
        if let Some(segments) = p_obj.get("segments").and_then(|s| s.as_array()) {
            segments
                .iter()
                .filter_map(|seg| seg.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        } else {
            return Err(NodeEngineError::ExecutionFailed(
                "Missing prompt input: not a string or masked prompt".to_string(),
            ));
        }
    } else {
        return Err(NodeEngineError::ExecutionFailed(
            "Missing prompt input".to_string(),
        ));
    };

    let model_path = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?
        .to_string();

    let system_prompt = inputs
        .get("system_prompt")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let temperature = inputs
        .get("temperature")
        .and_then(|t| t.as_f64())
        .unwrap_or(0.7);
    let max_tokens = inputs
        .get("max_tokens")
        .and_then(|m| m.as_i64())
        .unwrap_or(512);
    let device = inputs
        .get("device")
        .and_then(|d| d.as_str())
        .unwrap_or("auto")
        .to_string();
    let model_type = inputs
        .get("model_type")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());

    let model_name = std::path::Path::new(&model_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("pytorch-model")
        .to_string();

    // Phase 1: Check if model is already loaded, load if needed
    {
        let mp = model_path.clone();
        let dev = device.clone();
        let mt = model_type.clone();

        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| -> std::result::Result<(), String> {
                use pyo3::types::{PyAnyMethods, PyDictMethods};

                // Ensure worker + sibling modules are initialised
                ensure_torch_worker_initialised(py)?;
                let worker = py
                    .import("pantograph_torch_worker")
                    .map_err(|e| format!("Failed to import worker: {}", e))?;

                // Check if the correct model is already loaded
                let info = worker
                    .call_method0("get_loaded_info")
                    .map_err(|e| format!("get_loaded_info failed: {}", e))?;

                let needs_load = if info.is_none() {
                    true
                } else {
                    let loaded_path: String = info
                        .get_item("model_path")
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                        .unwrap_or_default();
                    loaded_path != mp
                };

                if needs_load {
                    log::info!("PyTorchInference: loading model from '{}'", mp);
                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("model_path", &mp).unwrap();
                    kwargs.set_item("device", &dev).unwrap();
                    if let Some(ref mt_val) = mt {
                        kwargs.set_item("model_type", mt_val).unwrap();
                    }
                    worker
                        .call_method("load_model", (), Some(&kwargs))
                        .map_err(|e| format!("Model load failed: {}", e))?;
                    log::info!("PyTorchInference: model loaded successfully");
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(|e| NodeEngineError::ExecutionFailed(e))?;
    }

    // Read model-specific inference settings to forward as Python kwargs
    let extra_settings = build_extra_settings(inputs);
    // Keep top_p explicit even when inference_settings schema is missing.
    let top_p = inputs
        .get("top_p")
        .and_then(|v| v.as_f64())
        .or_else(|| extra_settings.get("top_p").and_then(|v| v.as_f64()))
        .unwrap_or(0.95);

    // Phase 2: Generate — streaming or non-streaming
    let response_text = if let Some(sink) = event_sink {
        // Streaming: iterate Python generator via mpsc channel
        // Channel carries (mode, text) tuples: mode is "append" or "replace"
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<std::result::Result<(String, String), String>>(32);
        let p = prompt.clone();
        let sp = system_prompt.clone();
        let mpj = masked_prompt_json.clone();
        let extra = extra_settings.clone();
        let top_p = top_p;

        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| {
                use pyo3::types::{PyAnyMethods, PyDictMethods, PyTypeMethods};

                if let Err(e) = ensure_torch_worker_initialised(py) {
                    let _ = tx.blocking_send(Err(e));
                    return;
                }
                let worker = match py.import("pantograph_torch_worker") {
                    Ok(w) => w,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(format!("Failed to get worker: {}", e)));
                        return;
                    }
                };

                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("prompt", &p).unwrap();
                if let Some(ref sys) = sp {
                    kwargs.set_item("system_prompt", sys).unwrap();
                }
                kwargs.set_item("max_tokens", max_tokens).unwrap();
                kwargs.set_item("temperature", temperature).unwrap();
                kwargs.set_item("top_p", top_p).unwrap();
                if let Some(ref mpj_val) = mpj {
                    kwargs.set_item("masked_prompt_json", mpj_val).unwrap();
                }

                // Forward model-specific inference settings as kwargs
                for (key, value) in &extra {
                    if let Some(n) = value.as_i64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(n) = value.as_f64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(s) = value.as_str() {
                        kwargs.set_item(key.as_str(), s).unwrap();
                    } else if let Some(b) = value.as_bool() {
                        kwargs.set_item(key.as_str(), b).unwrap();
                    }
                }

                let generator = match worker.call_method("generate_tokens", (), Some(&kwargs)) {
                    Ok(g) => g,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(format!("Failed to create generator: {}", e)));
                        return;
                    }
                };

                let iter = match generator.try_iter() {
                    Ok(it) => it,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(format!("Generator not iterable: {}", e)));
                        return;
                    }
                };

                for item in iter {
                    match item {
                        Ok(token_obj) => {
                            // Try dict first: {"mode": "append"|"replace", "text": "..."}
                            let result =
                                if let Ok(dict) = token_obj.downcast::<pyo3::types::PyDict>() {
                                    let mode = dict
                                        .get_item("mode")
                                        .ok()
                                        .flatten()
                                        .and_then(|v| v.extract::<String>().ok())
                                        .unwrap_or_else(|| "append".to_string());
                                    let text = dict
                                        .get_item("text")
                                        .ok()
                                        .flatten()
                                        .and_then(|v| v.extract::<String>().ok())
                                        .unwrap_or_default();
                                    Ok((mode, text))
                                } else if let Ok(text) = token_obj.extract::<String>() {
                                    // Backwards compat: plain string → append
                                    Ok(("append".to_string(), text))
                                } else {
                                    Err(format!(
                                    "Token extraction failed: expected dict or string, got {:?}",
                                    token_obj.get_type().name()
                                ))
                                };
                            if tx.blocking_send(result).is_err() {
                                return;
                            }
                        }
                        Err(e) => {
                            let _ = tx.blocking_send(Err(format!("Generator error: {}", e)));
                            return;
                        }
                    }
                }
            });
        });

        let mut full_response = String::new();
        while let Some(token_result) = rx.recv().await {
            let (mode, text) = token_result.map_err(|e| {
                NodeEngineError::ExecutionFailed(format!("PyTorch generation error: {}", e))
            })?;
            if mode == "replace" {
                full_response = text.clone();
            } else {
                full_response.push_str(&text);
            }
            let _ = sink.send(crate::WorkflowEvent::task_stream(
                task_id,
                execution_id,
                "stream",
                serde_json::json!({"mode": mode, "text": text}),
            ));
        }

        full_response
    } else {
        // Non-streaming: single blocking call
        let p = prompt.clone();
        let sp = system_prompt.clone();
        let mpj = masked_prompt_json.clone();
        let extra = extra_settings;
        let top_p = top_p;

        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| -> std::result::Result<String, String> {
                use pyo3::types::{PyAnyMethods, PyDictMethods};

                ensure_torch_worker_initialised(py)?;
                let worker = py
                    .import("pantograph_torch_worker")
                    .map_err(|e| format!("Failed to get worker: {}", e))?;

                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("prompt", &p).unwrap();
                if let Some(ref sys) = sp {
                    kwargs.set_item("system_prompt", sys).unwrap();
                }
                kwargs.set_item("max_tokens", max_tokens).unwrap();
                kwargs.set_item("temperature", temperature).unwrap();
                kwargs.set_item("top_p", top_p).unwrap();
                if let Some(ref mpj_val) = mpj {
                    kwargs.set_item("masked_prompt_json", mpj_val).unwrap();
                }

                // Forward model-specific inference settings as kwargs
                for (key, value) in &extra {
                    if let Some(n) = value.as_i64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(n) = value.as_f64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(s) = value.as_str() {
                        kwargs.set_item(key.as_str(), s).unwrap();
                    } else if let Some(b) = value.as_bool() {
                        kwargs.set_item(key.as_str(), b).unwrap();
                    }
                }

                let result = worker
                    .call_method("generate", (), Some(&kwargs))
                    .map_err(|e| format!("Generation failed: {}", e))?;

                result
                    .extract::<String>()
                    .map_err(|e| format!("Failed to extract result: {}", e))
            })
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(|e| NodeEngineError::ExecutionFailed(e))?
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response_text));
    let task_type_primary = infer_task_type_primary("pytorch-inference", inputs);
    let model_ref = build_model_ref_v2(
        resolved_model_ref,
        "pytorch",
        &model_name,
        &model_path,
        &task_type_primary,
        inputs,
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::to_value(model_ref).unwrap_or_else(|_| {
            serde_json::json!({
                "contractVersion": 2,
                "engine": "pytorch",
                "modelId": model_name,
                "modelPath": model_path,
                "taskTypePrimary": task_type_primary,
            })
        }),
    );
    Ok(outputs)
}

// ---------------------------------------------------------------------------
// Audio generation handler (behind audio-nodes feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "audio-nodes")]
async fn execute_audio_generation(
    inputs: &HashMap<String, serde_json::Value>,
    resolved_model_ref: Option<ModelRefV2>,
) -> Result<HashMap<String, serde_json::Value>> {
    let model_path = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?
        .to_string();

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?
        .to_string();

    let duration = inputs
        .get("duration")
        .and_then(|d| d.as_f64())
        .unwrap_or(30.0);
    let steps = inputs
        .get("num_inference_steps")
        .and_then(|s| s.as_i64())
        .unwrap_or(100);
    let guidance_scale = inputs
        .get("guidance_scale")
        .and_then(|g| g.as_f64())
        .unwrap_or(7.0);
    let seed = inputs.get("seed").and_then(|s| s.as_i64()).unwrap_or(-1);

    // Phase 1: Load model if needed
    {
        let mp = model_path.clone();
        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| -> std::result::Result<(), String> {
                use pyo3::types::{PyAnyMethods, PyDictMethods};

                ensure_audio_worker_initialised(py)?;
                let worker = py
                    .import("pantograph_audio_worker")
                    .map_err(|e| format!("Failed to import audio worker: {}", e))?;

                let info = worker
                    .call_method0("get_loaded_info")
                    .map_err(|e| format!("get_loaded_info failed: {}", e))?;

                let needs_load = if info.is_none() {
                    true
                } else {
                    let loaded_path: String = info
                        .get_item("model_path")
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                        .unwrap_or_default();
                    loaded_path != mp
                };

                if needs_load {
                    log::info!("AudioGeneration: loading model from '{}'", mp);
                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("model_path", &mp).unwrap();
                    kwargs.set_item("device", "auto").unwrap();
                    worker
                        .call_method("load_model", (), Some(&kwargs))
                        .map_err(|e| format!("Audio model load failed: {}", e))?;
                    log::info!("AudioGeneration: model loaded successfully");
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(NodeEngineError::ExecutionFailed)?;
    }

    // Phase 2: Generate audio
    let mut result = {
        let p = prompt;
        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(
                |py| -> std::result::Result<HashMap<String, serde_json::Value>, String> {
                    use pyo3::types::PyAnyMethods;

                    let worker = py
                        .import("pantograph_audio_worker")
                        .map_err(|e| format!("Failed to get audio worker: {}", e))?;

                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("prompt", &p).unwrap();
                    kwargs.set_item("duration", duration).unwrap();
                    kwargs.set_item("steps", steps).unwrap();
                    kwargs.set_item("guidance_scale", guidance_scale).unwrap();
                    kwargs.set_item("seed", seed).unwrap();

                    let result = worker
                        .call_method("generate_audio_from_text", (), Some(&kwargs))
                        .map_err(|e| format!("Audio generation failed: {}", e))?;

                    // Extract dict fields
                    let audio_base64: String = result
                        .get_item("audio_base64")
                        .ok()
                        .and_then(|v| v.extract().ok())
                        .unwrap_or_default();
                    let duration_seconds: f64 = result
                        .get_item("duration_seconds")
                        .ok()
                        .and_then(|v| v.extract().ok())
                        .unwrap_or(0.0);
                    let sample_rate: i64 = result
                        .get_item("sample_rate")
                        .ok()
                        .and_then(|v| v.extract().ok())
                        .unwrap_or(44100);

                    let mut outputs = HashMap::new();
                    outputs.insert("audio".to_string(), serde_json::json!(audio_base64));
                    outputs.insert(
                        "duration_seconds".to_string(),
                        serde_json::json!(duration_seconds),
                    );
                    outputs.insert("sample_rate".to_string(), serde_json::json!(sample_rate));
                    Ok(outputs)
                },
            )
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(NodeEngineError::ExecutionFailed)?
    };

    let model_name = std::path::Path::new(&model_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio-model")
        .to_string();
    let model_ref = build_model_ref_v2(
        resolved_model_ref,
        "stable_audio",
        &model_name,
        &model_path,
        "text-to-audio",
        inputs,
    );
    result.insert(
        "model_ref".to_string(),
        serde_json::to_value(model_ref).unwrap_or_else(|_| {
            serde_json::json!({
                "contractVersion": 2,
                "engine": "stable_audio",
                "modelId": model_name,
                "modelPath": model_path,
                "taskTypePrimary": "text-to-audio",
            })
        }),
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// KV Cache handlers (behind inference-nodes feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "inference-nodes")]
async fn execute_kv_cache_save(
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    use inference::kv_cache::{
        KvCacheEntry, KvCacheMetadata, KvCacheStore, ModelFingerprint, StoragePolicy,
    };

    let store = extensions
        .get::<Arc<KvCacheStore>>(crate::extension_keys::KV_CACHE_STORE)
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "KvCacheStore not configured in executor extensions".to_string(),
            )
        })?;

    // Parse required inputs
    let cache_data_val = inputs
        .get("cache_data")
        .ok_or_else(|| NodeEngineError::MissingInput("cache_data".to_string()))?;
    let data_bytes: Vec<u8> = serde_json::from_value(cache_data_val.clone())?;

    let fingerprint_val = inputs
        .get("model_fingerprint")
        .ok_or_else(|| NodeEngineError::MissingInput("model_fingerprint".to_string()))?;
    let model_fingerprint: ModelFingerprint = serde_json::from_value(fingerprint_val.clone())?;

    // Parse optional inputs
    let label = inputs
        .get("label")
        .and_then(|v| v.as_str())
        .map(String::from);
    let compressed = inputs
        .get("compressed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let backend_hint = inputs
        .get("_data")
        .and_then(|d| d.get("backend_hint"))
        .and_then(|b| b.as_str())
        .unwrap_or("unknown")
        .to_string();

    let policy_str = inputs
        .get("storage_policy")
        .and_then(|v| v.as_str())
        .unwrap_or("memory");
    let policy = match policy_str {
        "disk" => StoragePolicy::DiskOnly,
        "both" => StoragePolicy::MemoryAndDisk,
        _ => StoragePolicy::MemoryOnly,
    };

    let cache_dir = inputs.get("cache_dir").and_then(|v| v.as_str());

    // Parse optional markers
    let markers: Vec<inference::kv_cache::CacheMarker> = inputs
        .get("markers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let token_count = data_bytes.len(); // approximate: actual token count depends on backend

    let entry = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: String::new(),
            label,
            model_fingerprint,
            backend_hint,
            token_count,
            markers,
            created_at: 0,
            updated_at: 0,
            compressed,
            extra: serde_json::json!({}),
        },
        data: data_bytes,
    };

    let cache_id = if let Some(dir) = cache_dir {
        store
            .save_to(entry, std::path::PathBuf::from(dir), Some(policy))
            .await
    } else {
        store.save(entry, Some(policy)).await
    }
    .map_err(|e| NodeEngineError::ExecutionFailed(format!("KV cache save failed: {e}")))?;

    let metadata = store
        .get_metadata(&cache_id)
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Failed to read metadata: {e}")))?;
    let metadata_json = serde_json::to_value(&metadata)?;

    let mut outputs = HashMap::new();
    outputs.insert("cache_id".to_string(), serde_json::json!(cache_id));
    outputs.insert("metadata".to_string(), metadata_json);
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
async fn execute_kv_cache_load(
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    use inference::kv_cache::{KvCacheStore, ModelFingerprint};

    let store = extensions
        .get::<Arc<KvCacheStore>>(crate::extension_keys::KV_CACHE_STORE)
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "KvCacheStore not configured in executor extensions".to_string(),
            )
        })?;

    let cache_id = inputs
        .get("cache_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| NodeEngineError::MissingInput("cache_id".to_string()))?;

    let fingerprint_val = inputs
        .get("model_fingerprint")
        .ok_or_else(|| NodeEngineError::MissingInput("model_fingerprint".to_string()))?;
    let fingerprint: ModelFingerprint = serde_json::from_value(fingerprint_val.clone())?;

    let mut outputs = HashMap::new();
    match store.load(cache_id, &fingerprint).await {
        Ok(entry) => {
            let metadata_json = serde_json::to_value(&entry.metadata)?;
            let data_json = serde_json::to_value(&entry.data)?;
            outputs.insert("cache_data".to_string(), data_json);
            outputs.insert("metadata".to_string(), metadata_json);
            outputs.insert("valid".to_string(), serde_json::json!(true));
        }
        Err(e) => {
            log::warn!("KV cache load failed for '{}': {}", cache_id, e);
            outputs.insert("cache_data".to_string(), serde_json::Value::Null);
            outputs.insert(
                "metadata".to_string(),
                serde_json::json!({"cache_id": cache_id}),
            );
            outputs.insert("valid".to_string(), serde_json::json!(false));
        }
    }
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
async fn execute_kv_cache_truncate(
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    use inference::kv_cache::KvCacheStore;

    let store = extensions
        .get::<Arc<KvCacheStore>>(crate::extension_keys::KV_CACHE_STORE)
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "KvCacheStore not configured in executor extensions".to_string(),
            )
        })?;

    let cache_id = inputs
        .get("cache_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| NodeEngineError::MissingInput("cache_id".to_string()))?;

    let marker_name = inputs.get("marker_name").and_then(|v| v.as_str());
    let token_position = inputs.get("token_position").and_then(|v| v.as_f64());

    // Truncation requires a backend-specific KvCacheCodec.
    // Until concrete codec implementations exist, return an error.
    if marker_name.is_some() || token_position.is_some() {
        return Err(NodeEngineError::ExecutionFailed(
            "KV cache truncation requires a backend-specific KvCacheCodec. \
             No codec is currently available. Connect an inference backend first."
                .to_string(),
        ));
    }

    // No truncation target — pass through with current metadata
    let metadata = store
        .get_metadata(cache_id)
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Failed to load metadata: {e}")))?;
    let metadata_json = serde_json::to_value(&metadata)?;

    let mut outputs = HashMap::new();
    outputs.insert("cache_id".to_string(), serde_json::json!(cache_id));
    outputs.insert("metadata".to_string(), metadata_json);
    Ok(outputs)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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
            serde_json::json!({"prompt": "Enter name"}),
        );
        inputs.insert("user_input".to_string(), serde_json::json!("Alice"));
        let result = execute_human_input(&inputs).unwrap();
        assert_eq!(result["prompt"], "Enter name");
        assert_eq!(result["input"], "Alice");
    }

    #[test]
    fn test_tool_executor_stub() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "tool_calls".to_string(),
            serde_json::json!([{"id": "call_1"}, {"id": "call_2"}]),
        );
        let result = execute_tool_executor(&inputs).unwrap();
        assert_eq!(result["all_success"], true);
        let results = result["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
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
            canonical_backend_key(Some("torch")),
            Some("pytorch".to_string())
        );
        assert_eq!(
            canonical_backend_key(Some("stable-audio")),
            Some("stable_audio".to_string())
        );
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
        assert_eq!(request.backend_key.as_deref(), Some("pytorch"));
        assert_eq!(request.task_type_primary.as_deref(), Some("text-to-image"));
    }
}

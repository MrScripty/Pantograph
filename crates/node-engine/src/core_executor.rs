//! Core task executor with built-in node handlers.
//!
//! `CoreTaskExecutor` handles all node types whose logic is not host-specific.
//! Hosts (Tauri, NIF/Elixir) only need to handle nodes that require platform
//! resources (e.g. RAG manager, UI interaction).

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;

use crate::engine::TaskExecutor;
use crate::error::{NodeEngineError, Result};
use crate::extensions::ExecutorExtensions;

/// Extract the node type from task inputs or infer from the task ID.
///
/// Checks `_data.node_type` first (injected by the graph converter),
/// then falls back to stripping the trailing `-N` suffix from the task ID.
pub fn resolve_node_type(
    task_id: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> String {
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
}

impl CoreTaskExecutor {
    /// Create a new core executor.
    pub fn new() -> Self {
        Self {
            project_root: None,
        }
    }

    /// Set the project root directory for file I/O nodes.
    pub fn with_project_root(mut self, root: PathBuf) -> Self {
        self.project_root = Some(root);
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

fn execute_text_input(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
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

fn execute_text_output(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
    let text = inputs
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("");

    let mut outputs = HashMap::new();
    outputs.insert("text".to_string(), serde_json::json!(text));
    Ok(outputs)
}

fn execute_linked_input(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
    let value = inputs
        .get("_data")
        .and_then(|d| d.get("linked_value"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut outputs = HashMap::new();
    outputs.insert("value".to_string(), serde_json::json!(value));
    Ok(outputs)
}

fn execute_image_input(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
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

fn execute_component_preview(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
    let component = inputs.get("component").cloned().unwrap_or(serde_json::Value::Null);
    let props = inputs.get("props").cloned().unwrap_or(serde_json::json!({}));

    let mut outputs = HashMap::new();
    outputs.insert(
        "rendered".to_string(),
        serde_json::json!({ "component": component, "props": props }),
    );
    Ok(outputs)
}

fn execute_model_provider(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
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

fn execute_puma_lib(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
    let model_path = inputs
        .get("_data")
        .and_then(|d| d.get("modelPath"))
        .and_then(|m| m.as_str())
        .unwrap_or("");

    let mut outputs = HashMap::new();
    outputs.insert("model_path".to_string(), serde_json::json!(model_path));

    log::debug!("PumaLib: providing model path '{}'", model_path);
    Ok(outputs)
}

fn execute_conditional(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
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

fn execute_merge(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
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

fn execute_validator(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
    let code = inputs
        .get("code")
        .and_then(|c| c.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing code input".to_string()))?;

    let forbidden_patterns: &[(&str, &str)] = &[
        ("export let ", "Use `let { prop } = $props()` instead of `export let prop`"),
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

fn execute_json_filter(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
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

fn execute_human_input(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
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

fn execute_tool_executor(inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>> {
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

    let full_path = if std::path::Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else if let Some(root) = project_root {
        root.join(path)
    } else {
        PathBuf::from(path)
    };

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

    let full_path = if std::path::Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else if let Some(root) = project_root {
        root.join(path)
    } else {
        PathBuf::from(path)
    };

    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| {
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
    if !options.is_empty() {
        request_body["options"] = serde_json::Value::Object(options);
    }

    let client = reqwest::Client::new();
    let url = "http://localhost:11434/api/generate";

    log::debug!("OllamaInference: sending request to {} with model '{}'", url, model);

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

    let response_text = response_json["response"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let model_used = response_json["model"]
        .as_str()
        .unwrap_or(model)
        .to_string();

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response_text));
    outputs.insert("model_used".to_string(), serde_json::json!(model_used));
    outputs.insert(
        "model_ref".to_string(),
        serde_json::json!({"engine": "ollama", "model_id": model_used}),
    );

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
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let node_type = resolve_node_type(task_id, &inputs);

        log::debug!(
            "CoreTaskExecutor: executing '{}' (type '{}')",
            task_id,
            node_type
        );

        match node_type.as_str() {
            // Input nodes
            "text-input" => execute_text_input(&inputs),
            "linked-input" => execute_linked_input(&inputs),
            "image-input" => execute_image_input(&inputs),

            // Output nodes
            "text-output" => execute_text_output(&inputs),
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

            // File I/O nodes
            "read-file" => execute_read_file(self.project_root.as_ref(), &inputs).await,
            "write-file" => execute_write_file(self.project_root.as_ref(), &inputs).await,

            // Interaction nodes
            "human-input" => execute_human_input(&inputs),
            "tool-executor" => execute_tool_executor(&inputs),

            // Pure HTTP inference
            "ollama-inference" => execute_ollama_inference(&inputs).await,

            // Unknown — signal that this node requires a host-specific executor
            _ => Err(NodeEngineError::ExecutionFailed(format!(
                "Node type '{}' requires host-specific executor",
                node_type
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
        inputs.insert(
            "inputs".to_string(),
            serde_json::json!(["hello", "world"]),
        );
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
        inputs.insert(
            "_data".to_string(),
            serde_json::json!({"path": "items[1]"}),
        );
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
            serde_json::json!({"modelPath": "/models/test.gguf"}),
        );
        let result = execute_puma_lib(&inputs).unwrap();
        assert_eq!(result["model_path"], "/models/test.gguf");
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
}

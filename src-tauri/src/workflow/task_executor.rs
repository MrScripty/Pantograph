//! Task executor for bridging node-engine tasks with Tauri resources
//!
//! This module implements the TaskExecutor trait from node-engine,
//! providing access to Pantograph's InferenceGateway, RagManager, and
//! other Tauri-specific resources.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use node_engine::{Context, NodeEngineError, Result, TaskExecutor};
use tokio::sync::RwLock;

use crate::agent::rag::RagManager;
use crate::llm::gateway::InferenceGateway;

/// Context keys for storing Pantograph-specific resources in graph-flow Context
pub mod context_keys {
    pub const PROJECT_ROOT: &str = "pantograph:project_root";
    pub const LLM_BASE_URL: &str = "pantograph:llm_base_url";
    pub const LLM_READY: &str = "pantograph:llm_ready";
    pub const RAG_DOCUMENTS: &str = "pantograph:rag_documents";
}

/// Task executor that bridges node-engine with Pantograph resources
///
/// This executor dispatches task execution based on node type,
/// using the appropriate Pantograph services (gateway, RAG, etc.).
pub struct PantographTaskExecutor {
    /// Gateway for LLM inference
    gateway: Arc<InferenceGateway>,
    /// RAG manager for document search
    rag_manager: Arc<RwLock<RagManager>>,
    /// Project root directory
    project_root: PathBuf,
}

impl PantographTaskExecutor {
    /// Create a new Pantograph task executor
    pub fn new(
        gateway: Arc<InferenceGateway>,
        rag_manager: Arc<RwLock<RagManager>>,
        project_root: PathBuf,
    ) -> Self {
        Self {
            gateway,
            rag_manager,
            project_root,
        }
    }

    /// Get the LLM base URL if available
    pub async fn llm_base_url(&self) -> Option<String> {
        self.gateway.base_url().await
    }

    /// Check if LLM server is ready
    pub async fn is_llm_ready(&self) -> bool {
        self.gateway.is_ready().await
    }

    /// Execute a text input task (passthrough)
    async fn execute_text_input(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        // Get text from _data or direct input
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

    /// Execute an image input task (passthrough)
    async fn execute_image_input(
        &self,
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

    /// Execute a text output task (passthrough for display)
    async fn execute_text_output(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let text = inputs
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("");

        let mut outputs = HashMap::new();
        outputs.insert("text".to_string(), serde_json::json!(text));
        Ok(outputs)
    }

    /// Execute a component preview task
    async fn execute_component_preview(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let component = inputs.get("component").cloned().unwrap_or(serde_json::Value::Null);
        let props = inputs.get("props").cloned().unwrap_or(serde_json::json!({}));

        let mut outputs = HashMap::new();
        outputs.insert("rendered".to_string(), serde_json::json!({
            "component": component,
            "props": props
        }));
        Ok(outputs)
    }

    /// Execute an LLM inference task
    async fn execute_llm_inference(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let prompt = inputs
            .get("prompt")
            .and_then(|p| p.as_str())
            .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

        let system_prompt = inputs
            .get("system_prompt")
            .and_then(|p| p.as_str());

        let extra_context = inputs
            .get("context")
            .and_then(|c| c.as_str());

        // Check if LLM is ready
        if !self.is_llm_ready().await {
            return Err(NodeEngineError::ExecutionFailed(
                "LLM server is not ready".to_string(),
            ));
        }

        let base_url = self.llm_base_url().await.ok_or_else(|| {
            NodeEngineError::ExecutionFailed("No LLM server URL available".to_string())
        })?;

        // Build full prompt with context
        let full_prompt = if let Some(ctx) = extra_context {
            format!("{}\n\nContext:\n{}", prompt, ctx)
        } else {
            prompt.to_string()
        };

        // Build messages
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }
        messages.push(serde_json::json!({
            "role": "user",
            "content": full_prompt
        }));

        // Make request
        let client = reqwest::Client::new();
        let http_response = client
            .post(format!("{}/v1/chat/completions", base_url))
            .json(&serde_json::json!({
                "model": "gpt-4",
                "messages": messages,
                "stream": false
            }))
            .send()
            .await
            .map_err(|e| NodeEngineError::ExecutionFailed(format!("LLM request failed: {}", e)))?;

        if !http_response.status().is_success() {
            let error = http_response.text().await.unwrap_or_default();
            return Err(NodeEngineError::ExecutionFailed(format!("LLM error: {}", error)));
        }

        let json: serde_json::Value = http_response.json().await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to parse response: {}", e))
        })?;

        let response = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let mut outputs = HashMap::new();
        outputs.insert("response".to_string(), serde_json::json!(response));
        outputs.insert("stream".to_string(), serde_json::Value::Null);
        Ok(outputs)
    }

    /// Execute a vision analysis task
    async fn execute_vision_analysis(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let image_base64 = inputs
            .get("image")
            .and_then(|i| i.as_str())
            .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing image input".to_string()))?;

        let prompt = inputs
            .get("prompt")
            .and_then(|p| p.as_str())
            .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

        if !self.is_llm_ready().await {
            return Err(NodeEngineError::ExecutionFailed(
                "Vision server is not ready".to_string(),
            ));
        }

        let base_url = self.llm_base_url().await.ok_or_else(|| {
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

    /// Execute a RAG search task
    async fn execute_rag_search(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let query = inputs
            .get("query")
            .and_then(|q| q.as_str())
            .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing query input".to_string()))?;

        let limit = inputs
            .get("limit")
            .and_then(|l| l.as_f64())
            .map(|l| l as usize)
            .unwrap_or(5);

        let rag_manager = self.rag_manager.read().await;
        let docs = rag_manager
            .search_as_docs(query, limit)
            .await
            .map_err(|e| NodeEngineError::ExecutionFailed(format!("RAG search failed: {}", e)))?;

        // Build context string
        let context_str = docs
            .iter()
            .map(|doc| format!("## {}\n{}", doc.title, doc.content))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        let mut outputs = HashMap::new();
        outputs.insert("documents".to_string(), serde_json::to_value(&docs).unwrap());
        outputs.insert("context".to_string(), serde_json::json!(context_str));
        Ok(outputs)
    }

    /// Execute a read file task
    async fn execute_read_file(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let path = inputs
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing path input".to_string()))?;

        // Resolve relative paths against project root
        let full_path = if std::path::Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.project_root.join(path)
        };

        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| NodeEngineError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        let mut outputs = HashMap::new();
        outputs.insert("content".to_string(), serde_json::json!(content));
        outputs.insert("path".to_string(), serde_json::json!(full_path.display().to_string()));
        Ok(outputs)
    }

    /// Execute a write file task
    async fn execute_write_file(
        &self,
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

        // Resolve relative paths against project root
        let full_path = if std::path::Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.project_root.join(path)
        };

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| NodeEngineError::ExecutionFailed(format!("Failed to create directories: {}", e)))?;
        }

        tokio::fs::write(&full_path, content)
            .await
            .map_err(|e| NodeEngineError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

        let mut outputs = HashMap::new();
        outputs.insert("success".to_string(), serde_json::json!(true));
        outputs.insert("path".to_string(), serde_json::json!(full_path.display().to_string()));
        Ok(outputs)
    }

    /// Execute a human input task (returns placeholder, requires WaitForInput handling)
    async fn execute_human_input(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        // Get prompt and any existing input
        let prompt = inputs
            .get("_data")
            .and_then(|d| d.get("prompt"))
            .and_then(|p| p.as_str())
            .unwrap_or("Please provide input");

        // Check if input was already provided
        let user_input = inputs
            .get("user_input")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string());

        let mut outputs = HashMap::new();
        outputs.insert("prompt".to_string(), serde_json::json!(prompt));
        outputs.insert("input".to_string(), serde_json::json!(user_input.unwrap_or_default()));
        Ok(outputs)
    }

    /// Execute a tool executor task
    async fn execute_tool_executor(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let tool_calls = inputs
            .get("tool_calls")
            .cloned()
            .unwrap_or(serde_json::json!([]));

        // For now, return placeholder results
        // Actual tool execution would be handled by the orchestration layer
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

    /// Execute a conditional task
    async fn execute_conditional(
        &self,
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

    /// Execute a merge task
    async fn execute_merge(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        // Get inputs - may be array or single value
        let input_values: Vec<String> = if let Some(arr) = inputs.get("inputs").and_then(|v| v.as_array()) {
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

    /// Execute a validator task
    async fn execute_validator(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let code = inputs
            .get("code")
            .and_then(|c| c.as_str())
            .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing code input".to_string()))?;

        // Perform pattern-based validation (same as ValidatorTask)
        let forbidden_patterns: &[(&str, &str)] = &[
            ("export let ", "Use `let { prop } = $props()` instead of `export let prop`"),
            ("on:click", "Use `onclick` instead of `on:click`"),
            ("on:change", "Use `onchange` instead of `on:change`"),
            ("on:input", "Use `oninput` instead of `on:input`"),
            ("on:submit", "Use `onsubmit` instead of `on:submit`"),
        ];

        // Strip comments
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

        // Check for unbalanced script tags
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

    /// Execute a JSON filter task
    async fn execute_json_filter(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let json = inputs
            .get("json")
            .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing json input".to_string()))?;

        // Get path from _data (node configuration)
        let path = inputs
            .get("_data")
            .and_then(|d| d.get("path"))
            .and_then(|p| p.as_str())
            .unwrap_or("");

        let (value, found) = Self::extract_json_path(json, path);

        let mut outputs = HashMap::new();
        outputs.insert("value".to_string(), value);
        outputs.insert("found".to_string(), serde_json::json!(found));
        Ok(outputs)
    }

    /// Extract a value from JSON using a path expression
    fn extract_json_path(json: &serde_json::Value, path: &str) -> (serde_json::Value, bool) {
        if path.is_empty() {
            return (json.clone(), true);
        }

        let mut current = json;
        let mut remaining = path;

        while !remaining.is_empty() {
            // Handle array indexing at start: [0]
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
}

#[async_trait]
impl TaskExecutor for PantographTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
    ) -> Result<HashMap<String, serde_json::Value>> {
        // Extract node_type from _data if available, otherwise infer from task_id
        let node_type = inputs
            .get("_data")
            .and_then(|d| d.get("node_type"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        // Use the node_type from data, or try to get it from a graph lookup
        // For now, we'll need the graph to pass the node_type in _data
        let node_type = node_type.unwrap_or_else(|| {
            // Try to infer from task_id pattern (e.g., "llm-inference-1")
            let parts: Vec<&str> = task_id.split('-').collect();
            if parts.len() >= 2 {
                parts[..parts.len() - 1].join("-")
            } else {
                task_id.to_string()
            }
        });

        log::debug!(
            "Executing task '{}' with type '{}' and {} inputs",
            task_id,
            node_type,
            inputs.len()
        );

        // Dispatch to appropriate handler based on node type
        match node_type.as_str() {
            "text-input" => self.execute_text_input(&inputs).await,
            "image-input" => self.execute_image_input(&inputs).await,
            "text-output" => self.execute_text_output(&inputs).await,
            "component-preview" => self.execute_component_preview(&inputs).await,
            "llm-inference" => self.execute_llm_inference(&inputs).await,
            "vision-analysis" => self.execute_vision_analysis(&inputs).await,
            "rag-search" => self.execute_rag_search(&inputs).await,
            "read-file" => self.execute_read_file(&inputs).await,
            "write-file" => self.execute_write_file(&inputs).await,
            "human-input" => self.execute_human_input(&inputs).await,
            // New control nodes
            "tool-executor" => self.execute_tool_executor(&inputs).await,
            "conditional" => self.execute_conditional(&inputs).await,
            "merge" => self.execute_merge(&inputs).await,
            // New processing nodes
            "validator" => self.execute_validator(&inputs).await,
            "json-filter" => self.execute_json_filter(&inputs).await,
            _ => {
                log::warn!("Unknown node type: {}", node_type);
                // Return empty outputs for unknown types
                Ok(HashMap::new())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require mocking InferenceGateway and RagManager
}

//! Tauri-specific task executor for host-dependent node types.
//!
//! Only handles node types that require Tauri-specific resources
//! (e.g. RagManager). All other nodes are handled by
//! `CoreTaskExecutor` via `CompositeTaskExecutor`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use node_engine::{
    core_executor::resolve_node_type, Context, NodeEngineError, Result, TaskExecutor,
};
use tokio::sync::RwLock;

use crate::agent::rag::RagManager;
use crate::workflow::python_runtime::{
    PythonNodeExecutionRequest, PythonRuntimeAdapter, UnconfiguredPythonRuntimeAdapter,
};

/// Tauri-specific task executor that handles only host-dependent nodes.
///
/// Currently handles:
/// - `rag-search`: requires `RagManager` (Tauri-managed state)
///
/// All other node types should be handled by `CoreTaskExecutor` via
/// `CompositeTaskExecutor`. Unknown types return the sentinel error
/// that `CompositeTaskExecutor` uses for fallthrough.
pub struct TauriTaskExecutor {
    /// RAG manager for document search
    rag_manager: Arc<RwLock<RagManager>>,
    /// Host adapter for python-backed nodes (pytorch/audio).
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
}

impl TauriTaskExecutor {
    /// Create a new Tauri-specific task executor.
    pub fn new(rag_manager: Arc<RwLock<RagManager>>) -> Self {
        Self::with_python_runtime(rag_manager, Arc::new(UnconfiguredPythonRuntimeAdapter))
    }

    /// Create a task executor with a custom python runtime adapter.
    pub fn with_python_runtime(
        rag_manager: Arc<RwLock<RagManager>>,
        python_runtime: Arc<dyn PythonRuntimeAdapter>,
    ) -> Self {
        Self {
            rag_manager,
            python_runtime,
        }
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
        outputs.insert(
            "documents".to_string(),
            serde_json::to_value(&docs).unwrap(),
        );
        outputs.insert("context".to_string(), serde_json::json!(context_str));
        Ok(outputs)
    }

    fn collect_model_ref_env_ids(inputs: &HashMap<String, serde_json::Value>) -> Vec<String> {
        let model_ref = inputs
            .get("model_ref")
            .or_else(|| inputs.get("_data").and_then(|v| v.get("model_ref")));
        let Some(bindings) = model_ref
            .and_then(|v| v.get("dependency_bindings"))
            .and_then(|v| v.as_array())
        else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for binding in bindings {
            if let Some(env_id) = binding.get("env_id").and_then(|v| v.as_str()) {
                let trimmed = env_id.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    async fn execute_python_node(
        &self,
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let request = PythonNodeExecutionRequest {
            node_type: node_type.to_string(),
            inputs: inputs.clone(),
            env_ids: Self::collect_model_ref_env_ids(inputs),
        };
        self.python_runtime
            .execute_node(request)
            .await
            .map_err(NodeEngineError::ExecutionFailed)
    }
}

#[async_trait]
impl TaskExecutor for TauriTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &node_engine::ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let node_type = resolve_node_type(task_id, &inputs);

        match node_type.as_str() {
            "rag-search" => self.execute_rag_search(&inputs).await,
            "pytorch-inference" | "audio-generation" => {
                self.execute_python_node(&node_type, &inputs).await
            }
            _ => {
                // Signal to CompositeTaskExecutor that this node type
                // requires host-specific executor (i.e., fall through to core)
                Err(NodeEngineError::ExecutionFailed(format!(
                    "Node type '{}' requires host-specific executor",
                    node_type
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require RagManager which needs runtime setup.
}

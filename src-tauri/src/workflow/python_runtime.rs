//! Host-side adapter boundary for Python-backed workflow nodes.
//!
//! Python execution must remain out-of-process and consumer-managed so Pantograph
//! itself does not depend on a specific Python runtime or environment layout.

use std::collections::HashMap;

use async_trait::async_trait;

/// Request payload forwarded from workflow node execution into the host adapter.
#[derive(Debug, Clone)]
pub struct PythonNodeExecutionRequest {
    pub node_type: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub env_ids: Vec<String>,
}

/// Host adapter interface for Python-backed node execution.
#[async_trait]
pub trait PythonRuntimeAdapter: Send + Sync {
    async fn execute_node(
        &self,
        request: PythonNodeExecutionRequest,
    ) -> Result<HashMap<String, serde_json::Value>, String>;
}

/// Default adapter used until a process-based runtime is configured.
pub struct UnconfiguredPythonRuntimeAdapter;

#[async_trait]
impl PythonRuntimeAdapter for UnconfiguredPythonRuntimeAdapter {
    async fn execute_node(
        &self,
        request: PythonNodeExecutionRequest,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let input_hint = if request.inputs.is_empty() {
            "No node inputs were provided.".to_string()
        } else {
            format!("Input payload keys: {}", request.inputs.len())
        };
        let env_hint = if request.env_ids.is_empty() {
            "No dependency env_id was provided in model_ref.".to_string()
        } else {
            format!("Resolved dependency env_id(s): {}", request.env_ids.join(", "))
        };

        Err(format!(
            "Node '{}' requires the external Python runtime adapter. \
In-process Python execution is disabled in the default Pantograph build. {} {}",
            request.node_type, env_hint, input_hint
        ))
    }
}

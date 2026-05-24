//! One-task core execution plumbing for scheduler-owned orchestration.
//!
//! This module lets application-layer schedulers execute one explicit
//! host-independent node through `CoreTaskExecutor` without constructing
//! graph-flow context, executor extensions, workflow sessions, demand execution,
//! or runtime inference dispatch outside node-engine.

use std::collections::HashMap;

use serde_json::{Map, Value};
use thiserror::Error;

use crate::core_executor::{resolve_node_type, CoreTaskExecutor};
use crate::engine::TaskExecutor;
use crate::error::NodeEngineError;
use crate::extensions::ExecutorExtensions;

const SINGLE_TASK_ID_MAX_LEN: usize = 256;
const SINGLE_TASK_NODE_TYPE_MAX_LEN: usize = 128;

/// Result type for node-engine single-task execution.
pub type NodeEngineSingleTaskResult<T> = std::result::Result<T, NodeEngineSingleTaskError>;

/// Validated request for executing one explicit core task.
#[derive(Debug, Clone)]
#[must_use]
pub struct NodeEngineSingleTaskRequest {
    task_id: String,
    node_type: String,
    inputs: HashMap<String, Value>,
}

impl NodeEngineSingleTaskRequest {
    /// Builds a single-task request from scheduler-adapter-owned input values.
    pub fn try_new(
        task_id: impl Into<String>,
        node_type: impl Into<String>,
        inputs: HashMap<String, Value>,
    ) -> NodeEngineSingleTaskResult<Self> {
        let task_id = validate_text_field("task_id", task_id.into(), SINGLE_TASK_ID_MAX_LEN)?;
        let node_type =
            validate_text_field("node_type", node_type.into(), SINGLE_TASK_NODE_TYPE_MAX_LEN)?;
        Ok(Self {
            task_id,
            node_type,
            inputs,
        })
    }

    /// Task identifier used only for execution correlation.
    #[must_use]
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Explicit node type supplied by immutable task-definition facts.
    #[must_use]
    pub fn node_type(&self) -> &str {
        &self.node_type
    }

    /// Scheduler-adapter-converted node inputs.
    #[must_use]
    pub fn inputs(&self) -> &HashMap<String, Value> {
        &self.inputs
    }
}

/// Raw node-engine output map for one core task.
#[derive(Debug, Clone)]
#[must_use]
pub struct NodeEngineSingleTaskResponse {
    outputs: HashMap<String, Value>,
}

impl NodeEngineSingleTaskResponse {
    /// Borrow the raw node-engine outputs.
    #[must_use]
    pub fn outputs(&self) -> &HashMap<String, Value> {
        &self.outputs
    }

    /// Consume the response and return the raw output map.
    #[must_use]
    pub fn into_outputs(self) -> HashMap<String, Value> {
        self.outputs
    }
}

/// Typed failures from the node-engine single-task API.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NodeEngineSingleTaskError {
    /// A request field was not valid for one-task execution.
    #[error("invalid single-task request field '{field}': {message}")]
    InvalidRequest {
        field: &'static str,
        message: String,
    },

    /// Core node-type resolution did not match the explicit task definition.
    #[error(
        "single-task node type mismatch for task '{task_id}': expected '{expected}', resolved '{actual}'"
    )]
    NodeTypeMismatch {
        task_id: String,
        expected: String,
        actual: String,
    },

    /// Underlying core task execution failed.
    #[error("single-task core execution failed: {0}")]
    Execution(#[from] NodeEngineError),
}

/// Execute one explicit host-independent core task.
pub async fn execute_core_task_once(
    request: NodeEngineSingleTaskRequest,
) -> NodeEngineSingleTaskResult<NodeEngineSingleTaskResponse> {
    let NodeEngineSingleTaskRequest {
        task_id,
        node_type,
        mut inputs,
    } = request;

    inject_explicit_node_type(&mut inputs, &node_type)?;
    let resolved_node_type = resolve_node_type(&task_id, &inputs);
    if resolved_node_type != node_type {
        return Err(NodeEngineSingleTaskError::NodeTypeMismatch {
            task_id,
            expected: node_type,
            actual: resolved_node_type,
        });
    }

    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let executor = CoreTaskExecutor::new();
    let outputs = executor
        .execute_task(&task_id, inputs, &context, &extensions)
        .await?;
    Ok(NodeEngineSingleTaskResponse { outputs })
}

fn inject_explicit_node_type(
    inputs: &mut HashMap<String, Value>,
    node_type: &str,
) -> NodeEngineSingleTaskResult<()> {
    match inputs.get_mut("_data") {
        Some(Value::Object(data)) => {
            data.insert(
                "node_type".to_string(),
                Value::String(node_type.to_string()),
            );
            Ok(())
        }
        Some(_) => Err(NodeEngineSingleTaskError::InvalidRequest {
            field: "_data",
            message: "_data must be a JSON object when supplied".to_string(),
        }),
        None => {
            let mut data = Map::new();
            data.insert(
                "node_type".to_string(),
                Value::String(node_type.to_string()),
            );
            inputs.insert("_data".to_string(), Value::Object(data));
            Ok(())
        }
    }
}

fn validate_text_field(
    field: &'static str,
    value: String,
    max_len: usize,
) -> NodeEngineSingleTaskResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(NodeEngineSingleTaskError::InvalidRequest {
            field,
            message: "must be non-empty".to_string(),
        });
    }
    if value.len() > max_len {
        return Err(NodeEngineSingleTaskError::InvalidRequest {
            field,
            message: format!("must be at most {max_len} bytes"),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(NodeEngineSingleTaskError::InvalidRequest {
            field,
            message: "must not contain control characters".to_string(),
        });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(
        task_id: &str,
        node_type: &str,
        inputs: HashMap<String, Value>,
    ) -> NodeEngineSingleTaskRequest {
        NodeEngineSingleTaskRequest::try_new(task_id, node_type, inputs).expect("valid request")
    }

    #[tokio::test]
    async fn execute_core_task_once_runs_text_input_without_external_context() {
        let mut inputs = HashMap::new();
        inputs.insert("text".to_string(), Value::String("hello".to_string()));

        let response = execute_core_task_once(request("node-1", "text-input", inputs))
            .await
            .expect("text-input response");

        assert_eq!(
            response.outputs().get("text"),
            Some(&Value::String("hello".to_string()))
        );
    }

    #[tokio::test]
    async fn execute_core_task_once_runs_text_output_without_external_context() {
        let mut inputs = HashMap::new();
        inputs.insert("text".to_string(), Value::String("done".to_string()));

        let response = execute_core_task_once(request("node-2", "text-output", inputs))
            .await
            .expect("text-output response");

        assert_eq!(
            response.outputs().get("text"),
            Some(&Value::String("done".to_string()))
        );
    }

    #[tokio::test]
    async fn execute_core_task_once_runs_boolean_input_without_external_context() {
        let mut inputs = HashMap::new();
        inputs.insert("value".to_string(), Value::Bool(true));

        let response = execute_core_task_once(request("node-3", "boolean-input", inputs))
            .await
            .expect("boolean-input response");

        assert_eq!(response.outputs().get("value"), Some(&Value::Bool(true)));
    }

    #[tokio::test]
    async fn explicit_node_type_overrides_task_id_suffix_fallback() {
        let mut inputs = HashMap::new();
        inputs.insert("text".to_string(), Value::String("explicit".to_string()));

        let response = execute_core_task_once(request("boolean-input-1", "text-input", inputs))
            .await
            .expect("explicit node type response");

        assert_eq!(
            response.outputs().get("text"),
            Some(&Value::String("explicit".to_string()))
        );
        assert!(!response.outputs().contains_key("value"));
    }

    #[tokio::test]
    async fn caller_supplied_data_node_type_is_not_execution_authority() {
        let mut data = Map::new();
        data.insert(
            "node_type".to_string(),
            Value::String("boolean-input".to_string()),
        );
        data.insert("text".to_string(), Value::String("from data".to_string()));
        let mut inputs = HashMap::new();
        inputs.insert("_data".to_string(), Value::Object(data));

        let response = execute_core_task_once(request("task-1", "text-input", inputs))
            .await
            .expect("caller node type overridden");

        assert_eq!(
            response.outputs().get("text"),
            Some(&Value::String("from data".to_string()))
        );
    }

    #[tokio::test]
    async fn non_object_data_fails_closed() {
        let mut inputs = HashMap::new();
        inputs.insert("_data".to_string(), Value::String("not object".to_string()));

        let error = execute_core_task_once(request("task-1", "text-input", inputs))
            .await
            .expect_err("invalid _data should fail");

        assert!(matches!(
            error,
            NodeEngineSingleTaskError::InvalidRequest { field: "_data", .. }
        ));
    }

    #[test]
    fn request_rejects_blank_task_id() {
        let error = NodeEngineSingleTaskRequest::try_new(" ", "text-input", HashMap::new())
            .expect_err("blank task id should fail");

        assert!(matches!(
            error,
            NodeEngineSingleTaskError::InvalidRequest {
                field: "task_id",
                ..
            }
        ));
    }

    #[test]
    fn request_rejects_blank_node_type() {
        let error = NodeEngineSingleTaskRequest::try_new("task-1", " ", HashMap::new())
            .expect_err("blank node type should fail");

        assert!(matches!(
            error,
            NodeEngineSingleTaskError::InvalidRequest {
                field: "node_type",
                ..
            }
        ));
    }
}

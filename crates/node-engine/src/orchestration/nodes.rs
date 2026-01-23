//! Orchestration node execution logic.
//!
//! This module provides the execution logic for each orchestration node type.

use super::types::{
    ConditionConfig, DataGraphConfig, LoopConfig, OrchestrationNode, OrchestrationNodeType,
};
use crate::{NodeEngineError, Result};
use serde_json::Value;
use std::collections::HashMap;

/// Context for orchestration execution, holding data passed between nodes.
#[derive(Debug, Clone, Default)]
pub struct OrchestrationContext {
    /// Data values accessible by key.
    data: HashMap<String, Value>,
    /// Current loop iteration counts (keyed by loop node ID).
    loop_iterations: HashMap<String, u32>,
}

impl OrchestrationContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context with initial data.
    pub fn with_data(data: HashMap<String, Value>) -> Self {
        Self {
            data,
            loop_iterations: HashMap::new(),
        }
    }

    /// Get a value from the context.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Set a value in the context.
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.data.insert(key.into(), value);
    }

    /// Remove a value from the context.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.data.remove(key)
    }

    /// Check if a key exists in the context.
    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Get all data as a reference.
    pub fn data(&self) -> &HashMap<String, Value> {
        &self.data
    }

    /// Get all data, consuming the context.
    pub fn into_data(self) -> HashMap<String, Value> {
        self.data
    }

    /// Merge another context's data into this one.
    pub fn merge(&mut self, other: &OrchestrationContext) {
        for (key, value) in &other.data {
            self.data.insert(key.clone(), value.clone());
        }
    }

    /// Get the current iteration count for a loop node.
    pub fn get_loop_iteration(&self, loop_node_id: &str) -> u32 {
        self.loop_iterations.get(loop_node_id).copied().unwrap_or(0)
    }

    /// Increment and return the iteration count for a loop node.
    pub fn increment_loop_iteration(&mut self, loop_node_id: &str) -> u32 {
        let count = self.loop_iterations.entry(loop_node_id.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    /// Reset the iteration count for a loop node.
    pub fn reset_loop_iteration(&mut self, loop_node_id: &str) {
        self.loop_iterations.remove(loop_node_id);
    }
}

/// Result of executing an orchestration node.
#[derive(Debug, Clone)]
pub struct NodeExecutionResult {
    /// The handle to follow for the next node (e.g., "next", "true", "false", "iteration", "complete").
    pub next_handle: String,
    /// Updated context data (will be merged into the main context).
    pub context_updates: HashMap<String, Value>,
    /// Optional message describing what happened.
    pub message: Option<String>,
}

impl NodeExecutionResult {
    /// Create a result that continues to the "next" handle.
    pub fn next() -> Self {
        Self {
            next_handle: "next".to_string(),
            context_updates: HashMap::new(),
            message: None,
        }
    }

    /// Create a result that follows a specific handle.
    pub fn handle(handle: impl Into<String>) -> Self {
        Self {
            next_handle: handle.into(),
            context_updates: HashMap::new(),
            message: None,
        }
    }

    /// Add context updates to the result.
    pub fn with_updates(mut self, updates: HashMap<String, Value>) -> Self {
        self.context_updates = updates;
        self
    }

    /// Add a single context update.
    pub fn with_update(mut self, key: impl Into<String>, value: Value) -> Self {
        self.context_updates.insert(key.into(), value);
        self
    }

    /// Add a message to the result.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Execute a Start node.
///
/// Start nodes simply pass through to the next node.
pub fn execute_start(_node: &OrchestrationNode, _context: &OrchestrationContext) -> Result<NodeExecutionResult> {
    Ok(NodeExecutionResult::next().with_message("Orchestration started"))
}

/// Execute an End node.
///
/// End nodes signal completion of the orchestration.
pub fn execute_end(_node: &OrchestrationNode, _context: &OrchestrationContext) -> Result<NodeExecutionResult> {
    // End nodes don't have a next handle - this signals completion
    Ok(NodeExecutionResult::handle("").with_message("Orchestration completed"))
}

/// Execute a Condition node.
///
/// Condition nodes evaluate a boolean condition and branch accordingly.
pub fn execute_condition(node: &OrchestrationNode, context: &OrchestrationContext) -> Result<NodeExecutionResult> {
    // Parse the condition config
    let config: ConditionConfig = serde_json::from_value(node.config.clone())
        .map_err(|e| NodeEngineError::failed(format!("Invalid condition config: {}", e)))?;

    // Get the value to check from context
    let value = context.get(&config.condition_key);

    let condition_met = match (value, &config.expected_value) {
        // No value in context - condition is false
        (None, _) => false,
        // Check for truthy value
        (Some(val), None) => is_truthy(val),
        // Check for specific value match
        (Some(val), Some(expected)) => val == expected,
    };

    let handle = if condition_met { "true" } else { "false" };
    Ok(NodeExecutionResult::handle(handle)
        .with_message(format!("Condition '{}' evaluated to {}", config.condition_key, condition_met)))
}

/// Execute a Loop node.
///
/// Loop nodes manage iteration, checking exit conditions and max iterations.
pub fn execute_loop(node: &OrchestrationNode, context: &mut OrchestrationContext) -> Result<NodeExecutionResult> {
    // Parse the loop config
    let config: LoopConfig = serde_json::from_value(node.config.clone()).unwrap_or_default();

    // Increment iteration counter
    let iteration = context.increment_loop_iteration(&node.id);

    // Check max iterations
    if config.max_iterations > 0 && iteration > config.max_iterations {
        context.reset_loop_iteration(&node.id);
        return Ok(NodeExecutionResult::handle("complete")
            .with_message(format!("Loop completed after {} iterations (max reached)", iteration - 1)));
    }

    // Check exit condition if specified
    if let Some(exit_key) = &config.exit_condition_key {
        if let Some(exit_value) = context.get(exit_key) {
            if is_truthy(exit_value) {
                context.reset_loop_iteration(&node.id);
                return Ok(NodeExecutionResult::handle("complete")
                    .with_message(format!("Loop completed after {} iterations (exit condition met)", iteration - 1)));
            }
        }
    }

    // Continue iterating
    Ok(NodeExecutionResult::handle("iteration")
        .with_update(&config.iteration_key, Value::Number(iteration.into()))
        .with_message(format!("Loop iteration {}", iteration)))
}

/// Execute a Merge node.
///
/// Merge nodes combine multiple execution paths. They simply pass through.
pub fn execute_merge(_node: &OrchestrationNode, _context: &OrchestrationContext) -> Result<NodeExecutionResult> {
    Ok(NodeExecutionResult::next().with_message("Paths merged"))
}

/// Execute a DataGraph node.
///
/// This returns a placeholder result - actual data graph execution
/// must be handled by the executor which has access to the data graphs.
pub fn prepare_data_graph_execution(node: &OrchestrationNode) -> Result<DataGraphConfig> {
    // Parse the data graph config
    let config: DataGraphConfig = serde_json::from_value(node.config.clone())
        .map_err(|e| NodeEngineError::failed(format!("Invalid data graph config: {}", e)))?;

    Ok(config)
}

/// Check if a JSON value is "truthy".
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty() && s != "false" && s != "0",
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Execute an orchestration node based on its type.
///
/// Note: DataGraph nodes are handled specially by the executor,
/// as they require access to the data graph engine.
pub fn execute_node(
    node: &OrchestrationNode,
    context: &mut OrchestrationContext,
) -> Result<NodeExecutionResult> {
    match node.node_type {
        OrchestrationNodeType::Start => execute_start(node, context),
        OrchestrationNodeType::End => execute_end(node, context),
        OrchestrationNodeType::Condition => execute_condition(node, context),
        OrchestrationNodeType::Loop => execute_loop(node, context),
        OrchestrationNodeType::Merge => execute_merge(node, context),
        OrchestrationNodeType::DataGraph => {
            // DataGraph nodes need special handling - return a placeholder
            // The executor will intercept this and run the actual data graph
            Ok(NodeExecutionResult::next().with_message("Data graph execution pending"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_operations() {
        let mut ctx = OrchestrationContext::new();

        ctx.set("key1", Value::String("value1".to_string()));
        assert_eq!(ctx.get("key1"), Some(&Value::String("value1".to_string())));
        assert!(ctx.contains("key1"));

        ctx.remove("key1");
        assert!(!ctx.contains("key1"));
    }

    #[test]
    fn test_loop_iteration() {
        let mut ctx = OrchestrationContext::new();

        assert_eq!(ctx.get_loop_iteration("loop1"), 0);
        assert_eq!(ctx.increment_loop_iteration("loop1"), 1);
        assert_eq!(ctx.increment_loop_iteration("loop1"), 2);
        assert_eq!(ctx.get_loop_iteration("loop1"), 2);

        ctx.reset_loop_iteration("loop1");
        assert_eq!(ctx.get_loop_iteration("loop1"), 0);
    }

    #[test]
    fn test_is_truthy() {
        assert!(!is_truthy(&Value::Null));
        assert!(!is_truthy(&Value::Bool(false)));
        assert!(is_truthy(&Value::Bool(true)));
        assert!(!is_truthy(&Value::Number(0.into())));
        assert!(is_truthy(&Value::Number(1.into())));
        assert!(!is_truthy(&Value::String("".to_string())));
        assert!(is_truthy(&Value::String("hello".to_string())));
        assert!(!is_truthy(&Value::String("false".to_string())));
        assert!(!is_truthy(&Value::Array(vec![])));
        assert!(is_truthy(&Value::Array(vec![Value::Null])));
    }

    #[test]
    fn test_execute_start() {
        let node = OrchestrationNode::new("start", OrchestrationNodeType::Start, (0.0, 0.0));
        let ctx = OrchestrationContext::new();

        let result = execute_start(&node, &ctx).unwrap();
        assert_eq!(result.next_handle, "next");
    }

    #[test]
    fn test_execute_end() {
        let node = OrchestrationNode::new("end", OrchestrationNodeType::End, (0.0, 0.0));
        let ctx = OrchestrationContext::new();

        let result = execute_end(&node, &ctx).unwrap();
        assert_eq!(result.next_handle, "");
    }

    #[test]
    fn test_execute_condition_true() {
        let config = serde_json::json!({
            "conditionKey": "isValid"
        });
        let node = OrchestrationNode::with_config(
            "cond",
            OrchestrationNodeType::Condition,
            (0.0, 0.0),
            config,
        );

        let mut ctx = OrchestrationContext::new();
        ctx.set("isValid", Value::Bool(true));

        let result = execute_condition(&node, &ctx).unwrap();
        assert_eq!(result.next_handle, "true");
    }

    #[test]
    fn test_execute_condition_false() {
        let config = serde_json::json!({
            "conditionKey": "isValid"
        });
        let node = OrchestrationNode::with_config(
            "cond",
            OrchestrationNodeType::Condition,
            (0.0, 0.0),
            config,
        );

        let mut ctx = OrchestrationContext::new();
        ctx.set("isValid", Value::Bool(false));

        let result = execute_condition(&node, &ctx).unwrap();
        assert_eq!(result.next_handle, "false");
    }

    #[test]
    fn test_execute_loop_iterations() {
        let config = serde_json::json!({
            "maxIterations": 3,
            "iterationKey": "i"
        });
        let node = OrchestrationNode::with_config(
            "loop1",
            OrchestrationNodeType::Loop,
            (0.0, 0.0),
            config,
        );

        let mut ctx = OrchestrationContext::new();

        // First iteration
        let result = execute_loop(&node, &mut ctx).unwrap();
        assert_eq!(result.next_handle, "iteration");

        // Second iteration
        let result = execute_loop(&node, &mut ctx).unwrap();
        assert_eq!(result.next_handle, "iteration");

        // Third iteration
        let result = execute_loop(&node, &mut ctx).unwrap();
        assert_eq!(result.next_handle, "iteration");

        // Fourth iteration - should complete
        let result = execute_loop(&node, &mut ctx).unwrap();
        assert_eq!(result.next_handle, "complete");
    }

    #[test]
    fn test_execute_loop_exit_condition() {
        let config = serde_json::json!({
            "maxIterations": 10,
            "exitConditionKey": "done"
        });
        let node = OrchestrationNode::with_config(
            "loop1",
            OrchestrationNodeType::Loop,
            (0.0, 0.0),
            config,
        );

        let mut ctx = OrchestrationContext::new();

        // First iteration - no exit condition set
        let result = execute_loop(&node, &mut ctx).unwrap();
        assert_eq!(result.next_handle, "iteration");

        // Set exit condition
        ctx.set("done", Value::Bool(true));

        // Second iteration - should complete due to exit condition
        let result = execute_loop(&node, &mut ctx).unwrap();
        assert_eq!(result.next_handle, "complete");
    }
}

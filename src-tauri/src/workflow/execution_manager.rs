//! Execution state manager for tracking workflow executions
//!
//! This module manages the lifecycle of workflow executions, including:
//! - Tracking active executions with their WorkflowExecutor instances
//! - Managing undo/redo stacks per execution
//! - Cleaning up stale executions

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use node_engine::{EventSink, UndoStack, WorkflowExecutor, WorkflowGraph};
use tokio::sync::RwLock;

/// State for a single workflow execution
pub struct ExecutionState {
    /// The workflow executor (contains DemandEngine, graph, context)
    pub executor: WorkflowExecutor,
    /// Undo/redo stack for this execution
    pub undo_stack: UndoStack,
    /// When this execution was created
    pub created_at: Instant,
    /// When this execution was last accessed
    pub last_accessed: Instant,
}

impl ExecutionState {
    /// Create a new execution state
    pub fn new(executor: WorkflowExecutor) -> Self {
        let now = Instant::now();
        Self {
            executor,
            undo_stack: UndoStack::default(),
            created_at: now,
            last_accessed: now,
        }
    }

    /// Update the last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// Check if this execution is stale (hasn't been accessed recently)
    pub fn is_stale(&self, timeout: Duration) -> bool {
        self.last_accessed.elapsed() > timeout
    }

    /// Push the current graph state to the undo stack
    pub async fn push_undo_snapshot(&mut self) -> node_engine::Result<()> {
        let graph = self.executor.get_graph_snapshot().await;
        self.undo_stack.push(&graph)
    }

    /// Undo to the previous graph state
    pub async fn undo(&mut self) -> Option<node_engine::Result<WorkflowGraph>> {
        match self.undo_stack.undo() {
            Some(Ok(graph)) => {
                self.executor.restore_graph_snapshot(graph.clone()).await;
                Some(Ok(graph))
            }
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }

    /// Redo to the next graph state
    pub async fn redo(&mut self) -> Option<node_engine::Result<WorkflowGraph>> {
        match self.undo_stack.redo() {
            Some(Ok(graph)) => {
                self.executor.restore_graph_snapshot(graph.clone()).await;
                Some(Ok(graph))
            }
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
    }
}

/// Manager for all workflow executions
pub struct ExecutionManager {
    /// Active executions keyed by execution ID
    executions: RwLock<HashMap<String, ExecutionState>>,
    /// Timeout for cleaning up stale executions
    stale_timeout: Duration,
}

impl ExecutionManager {
    /// Create a new execution manager
    pub fn new() -> Self {
        Self {
            executions: RwLock::new(HashMap::new()),
            stale_timeout: Duration::from_secs(5 * 60), // 5 minutes default
        }
    }

    /// Create a new execution manager with a custom stale timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            executions: RwLock::new(HashMap::new()),
            stale_timeout: timeout,
        }
    }

    /// Create a new execution and return its ID
    pub async fn create_execution(
        &self,
        execution_id: impl Into<String>,
        graph: WorkflowGraph,
        event_sink: Arc<dyn EventSink>,
    ) -> String {
        let execution_id = execution_id.into();
        let executor = WorkflowExecutor::new(&execution_id, graph, event_sink);
        let state = ExecutionState::new(executor);

        let mut executions = self.executions.write().await;
        executions.insert(execution_id.clone(), state);

        execution_id
    }

    /// Get direct access to the executions map for async operations
    pub async fn executions(&self) -> tokio::sync::RwLockWriteGuard<'_, HashMap<String, ExecutionState>> {
        self.executions.write().await
    }

    /// Get an execution by ID, updating its last accessed time
    pub async fn get_execution(&self, execution_id: &str) -> Option<()> {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.touch();
            Some(())
        } else {
            None
        }
    }

    /// Execute a synchronous function with the execution state
    ///
    /// This is for simple operations that don't need async.
    pub async fn with_execution<F, R>(&self, execution_id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut ExecutionState) -> R,
    {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.touch();
            Some(f(state))
        } else {
            None
        }
    }

    /// Remove an execution by ID
    pub async fn remove_execution(&self, execution_id: &str) -> Option<ExecutionState> {
        let mut executions = self.executions.write().await;
        executions.remove(execution_id)
    }

    /// Clean up stale executions
    pub async fn cleanup_stale(&self) -> usize {
        let mut executions = self.executions.write().await;
        let stale_ids: Vec<String> = executions
            .iter()
            .filter(|(_, state)| state.is_stale(self.stale_timeout))
            .map(|(id, _)| id.clone())
            .collect();

        let count = stale_ids.len();
        for id in stale_ids {
            executions.remove(&id);
            log::debug!("Cleaned up stale execution: {}", id);
        }

        count
    }

    /// Get the number of active executions
    pub async fn execution_count(&self) -> usize {
        self.executions.read().await.len()
    }

    /// Check if an execution exists
    pub async fn has_execution(&self, execution_id: &str) -> bool {
        self.executions.read().await.contains_key(execution_id)
    }

    /// Get the undo/redo state for an execution
    pub async fn get_undo_redo_state(&self, execution_id: &str) -> Option<UndoRedoState> {
        let executions = self.executions.read().await;
        executions.get(execution_id).map(|state| UndoRedoState {
            can_undo: state.can_undo(),
            can_redo: state.can_redo(),
            undo_count: state.undo_stack.len(),
        })
    }
}

impl Default for ExecutionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// State of undo/redo for an execution
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoRedoState {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_count: usize,
}

/// Shared execution manager type for Tauri state
pub type SharedExecutionManager = Arc<ExecutionManager>;

#[cfg(test)]
mod tests {
    use super::*;
    use node_engine::NullEventSink;

    fn make_test_graph() -> WorkflowGraph {
        WorkflowGraph::new("test", "Test Workflow")
    }

    #[tokio::test]
    async fn test_create_and_get_execution() {
        let manager = ExecutionManager::new();
        let graph = make_test_graph();
        let event_sink = Arc::new(NullEventSink);

        let id = manager
            .create_execution("exec-1", graph, event_sink)
            .await;

        assert_eq!(id, "exec-1");
        assert!(manager.has_execution("exec-1").await);
        assert!(!manager.has_execution("exec-2").await);
    }

    #[tokio::test]
    async fn test_remove_execution() {
        let manager = ExecutionManager::new();
        let graph = make_test_graph();
        let event_sink = Arc::new(NullEventSink);

        manager
            .create_execution("exec-1", graph, event_sink)
            .await;

        assert!(manager.has_execution("exec-1").await);

        manager.remove_execution("exec-1").await;

        assert!(!manager.has_execution("exec-1").await);
    }

    #[tokio::test]
    async fn test_cleanup_stale() {
        let manager = ExecutionManager::with_timeout(Duration::from_millis(10));
        let graph = make_test_graph();
        let event_sink = Arc::new(NullEventSink);

        manager
            .create_execution("exec-1", graph, event_sink)
            .await;

        assert_eq!(manager.execution_count().await, 1);

        // Wait for execution to become stale
        tokio::time::sleep(Duration::from_millis(20)).await;

        let cleaned = manager.cleanup_stale().await;
        assert_eq!(cleaned, 1);
        assert_eq!(manager.execution_count().await, 0);
    }

    #[tokio::test]
    async fn test_with_execution() {
        let manager = ExecutionManager::new();
        let graph = make_test_graph();
        let event_sink = Arc::new(NullEventSink);

        manager
            .create_execution("exec-1", graph, event_sink)
            .await;

        let result = manager
            .with_execution("exec-1", |state| state.can_undo())
            .await;

        assert_eq!(result, Some(false)); // No undo history yet
    }
}

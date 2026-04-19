//! Execution state manager for tracking workflow executions
//!
//! This module manages the lifecycle of workflow executions, including:
//! - Tracking active executions with their WorkflowExecutor instances
//! - Managing undo/redo stacks per execution
//! - Cleaning up stale executions

mod state;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use node_engine::{EventSink, WorkflowExecutor, WorkflowGraph};
pub use state::{ExecutionHandle, ExecutionState, UndoRedoState};
use tokio::sync::{Mutex, RwLock};

/// Manager for all workflow executions
pub struct ExecutionManager {
    /// Active executions keyed by execution ID
    executions: RwLock<HashMap<String, ExecutionHandle>>,
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
        let state = Arc::new(Mutex::new(ExecutionState::new(executor)));

        let mut executions = self.executions.write().await;
        executions.insert(execution_id.clone(), state);

        execution_id
    }

    /// Get an execution handle by ID.
    pub async fn get_execution_handle(&self, execution_id: &str) -> Option<ExecutionHandle> {
        let executions = self.executions.read().await;
        executions.get(execution_id).cloned()
    }

    /// Get an execution by ID, updating its last accessed time
    pub async fn get_execution(&self, execution_id: &str) -> Option<()> {
        let handle = self.get_execution_handle(execution_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        Some(())
    }

    /// Execute a synchronous function with the execution state
    ///
    /// This is for simple operations that don't need async.
    pub async fn with_execution<F, R>(&self, execution_id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut ExecutionState) -> R,
    {
        let handle = self.get_execution_handle(execution_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        Some(f(&mut state))
    }

    /// Remove an execution by ID
    pub async fn remove_execution(&self, execution_id: &str) -> Option<ExecutionHandle> {
        let mut executions = self.executions.write().await;
        executions.remove(execution_id)
    }

    /// Clean up stale executions
    pub async fn cleanup_stale(&self) -> usize {
        let handles: Vec<(String, ExecutionHandle)> = {
            let executions = self.executions.read().await;
            executions
                .iter()
                .map(|(id, handle)| (id.clone(), handle.clone()))
                .collect()
        };

        let mut stale_ids = Vec::new();
        for (id, handle) in handles {
            if handle.lock().await.is_stale(self.stale_timeout) {
                stale_ids.push(id);
            }
        }

        let count = stale_ids.len();
        let mut executions = self.executions.write().await;
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
        let handle = self.get_execution_handle(execution_id).await?;
        let state = handle.lock().await;
        Some(state.undo_redo_state())
    }
}

impl Default for ExecutionManager {
    fn default() -> Self {
        Self::new()
    }
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

        let id = manager.create_execution("exec-1", graph, event_sink).await;

        assert_eq!(id, "exec-1");
        assert!(manager.has_execution("exec-1").await);
        assert!(!manager.has_execution("exec-2").await);
    }

    #[tokio::test]
    async fn test_remove_execution() {
        let manager = ExecutionManager::new();
        let graph = make_test_graph();
        let event_sink = Arc::new(NullEventSink);

        manager.create_execution("exec-1", graph, event_sink).await;

        assert!(manager.has_execution("exec-1").await);

        manager.remove_execution("exec-1").await;

        assert!(!manager.has_execution("exec-1").await);
    }

    #[tokio::test]
    async fn test_cleanup_stale() {
        let manager = ExecutionManager::with_timeout(Duration::from_millis(10));
        let graph = make_test_graph();
        let event_sink = Arc::new(NullEventSink);

        manager.create_execution("exec-1", graph, event_sink).await;

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

        manager.create_execution("exec-1", graph, event_sink).await;

        let result = manager
            .with_execution("exec-1", |state| state.can_undo())
            .await;

        assert_eq!(result, Some(false)); // No undo history yet
    }

    #[tokio::test]
    async fn test_execution_handle_lock_does_not_block_other_handle_lookups() {
        let manager = ExecutionManager::new();
        let graph = make_test_graph();
        let event_sink = Arc::new(NullEventSink);

        manager
            .create_execution("exec-1", graph.clone(), event_sink.clone())
            .await;
        manager.create_execution("exec-2", graph, event_sink).await;

        let handle = manager
            .get_execution_handle("exec-1")
            .await
            .expect("execution handle should exist");
        let _locked = handle.lock().await;

        let lookup = tokio::time::timeout(
            Duration::from_millis(50),
            manager.get_execution_handle("exec-2"),
        )
        .await;

        assert!(lookup.is_ok());
        assert!(lookup.expect("lookup should complete").is_some());
    }
}

use std::sync::Arc;
use std::time::{Duration, Instant};

use node_engine::{UndoStack, WorkflowExecutor, WorkflowGraph};
use tokio::sync::Mutex;

/// State for a single workflow execution.
pub struct ExecutionState {
    /// The workflow executor (contains DemandEngine, graph, context).
    pub executor: WorkflowExecutor,
    /// Undo/redo stack for this execution.
    pub undo_stack: UndoStack,
    /// When this execution was created.
    pub created_at: Instant,
    /// When this execution was last accessed.
    pub last_accessed: Instant,
}

pub type ExecutionHandle = Arc<Mutex<ExecutionState>>;

impl ExecutionState {
    /// Create a new execution state.
    pub fn new(executor: WorkflowExecutor) -> Self {
        let now = Instant::now();
        Self {
            executor,
            undo_stack: UndoStack::default(),
            created_at: now,
            last_accessed: now,
        }
    }

    /// Update the last accessed time.
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// Check if this execution is stale.
    pub fn is_stale(&self, timeout: Duration) -> bool {
        self.last_accessed.elapsed() > timeout
    }

    /// Push the current graph state to the undo stack.
    pub async fn push_undo_snapshot(&mut self) -> node_engine::Result<()> {
        let graph = self.executor.get_graph_snapshot().await;
        self.undo_stack.push(&graph)
    }

    /// Undo to the previous graph state.
    pub async fn undo(&mut self) -> Option<node_engine::Result<WorkflowGraph>> {
        match self.undo_stack.undo() {
            Some(Ok(graph)) => {
                self.executor.restore_graph_snapshot(graph.clone()).await;
                Some(Ok(graph))
            }
            Some(Err(error)) => Some(Err(error)),
            None => None,
        }
    }

    /// Redo to the next graph state.
    pub async fn redo(&mut self) -> Option<node_engine::Result<WorkflowGraph>> {
        match self.undo_stack.redo() {
            Some(Ok(graph)) => {
                self.executor.restore_graph_snapshot(graph.clone()).await;
                Some(Ok(graph))
            }
            Some(Err(error)) => Some(Err(error)),
            None => None,
        }
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
    }

    /// Return the transport-facing undo/redo state projection.
    pub fn undo_redo_state(&self) -> UndoRedoState {
        UndoRedoState {
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            undo_count: self.undo_stack.len(),
        }
    }
}

/// State of undo/redo for an execution.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoRedoState {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_count: usize,
}

#[cfg(test)]
mod tests {
    use super::ExecutionState;
    use node_engine::{NullEventSink, WorkflowExecutor, WorkflowGraph};
    use std::sync::Arc;
    use std::time::Duration;

    fn make_state() -> ExecutionState {
        let executor = WorkflowExecutor::new(
            "exec-1",
            WorkflowGraph::new("test", "Test Workflow"),
            Arc::new(NullEventSink),
        );
        ExecutionState::new(executor)
    }

    #[test]
    fn undo_redo_state_defaults_empty() {
        let state = make_state();
        let undo_redo = state.undo_redo_state();

        assert!(!undo_redo.can_undo);
        assert!(!undo_redo.can_redo);
        assert_eq!(undo_redo.undo_count, 0);
    }

    #[test]
    fn execution_state_reports_stale_against_timeout() {
        let mut state = make_state();
        state.last_accessed -= Duration::from_secs(10);

        assert!(state.is_stale(Duration::from_secs(5)));
        assert!(!state.is_stale(Duration::from_secs(15)));
    }
}

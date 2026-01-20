//! Undo/redo system using compressed snapshots
//!
//! This module provides a simple but effective undo/redo system
//! based on compressed immutable snapshots of the workflow graph.
//!
//! # Design Choice: Snapshots vs Command Pattern
//!
//! We use snapshots instead of the command pattern because:
//! - No need to implement inverse operations for every change
//! - Works reliably with any graph mutation
//! - zstd compression is fast (~500MB/s) and effective (~10x reduction)
//! - Simple to understand and debug

use std::collections::VecDeque;

use crate::error::{NodeEngineError, Result};
use crate::types::WorkflowGraph;

/// Undo/redo stack using compressed snapshots
pub struct UndoStack {
    /// Compressed graph states (zstd)
    snapshots: VecDeque<Vec<u8>>,
    /// Current position in the stack
    current: usize,
    /// Maximum number of snapshots to keep
    max_snapshots: usize,
}

impl UndoStack {
    /// Create a new undo stack with the specified maximum size
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: VecDeque::new(),
            current: 0,
            max_snapshots: max_snapshots.max(1), // At least 1 snapshot
        }
    }

    /// Push a new snapshot onto the stack
    ///
    /// This truncates any redo history (snapshots after current position).
    pub fn push(&mut self, graph: &WorkflowGraph) -> Result<()> {
        let json = serde_json::to_vec(graph)?;
        let compressed = zstd::encode_all(&json[..], 3)
            .map_err(|e| NodeEngineError::Compression(e.to_string()))?;

        // Truncate any redo history
        while self.snapshots.len() > self.current + 1 {
            self.snapshots.pop_back();
        }

        // Add new snapshot
        self.snapshots.push_back(compressed);
        self.current = self.snapshots.len() - 1;

        // Trim old snapshots if over limit
        while self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
            if self.current > 0 {
                self.current -= 1;
            }
        }

        Ok(())
    }

    /// Undo: move back one snapshot
    ///
    /// Returns the previous graph state, or None if at the beginning.
    pub fn undo(&mut self) -> Option<Result<WorkflowGraph>> {
        if self.current > 0 {
            self.current -= 1;
            Some(self.decompress(self.current))
        } else {
            None
        }
    }

    /// Redo: move forward one snapshot
    ///
    /// Returns the next graph state, or None if at the end.
    pub fn redo(&mut self) -> Option<Result<WorkflowGraph>> {
        if self.current + 1 < self.snapshots.len() {
            self.current += 1;
            Some(self.decompress(self.current))
        } else {
            None
        }
    }

    /// Get the current graph state without modifying the stack
    pub fn current(&self) -> Option<Result<WorkflowGraph>> {
        if self.snapshots.is_empty() {
            None
        } else {
            Some(self.decompress(self.current))
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.current + 1 < self.snapshots.len()
    }

    /// Get the number of snapshots
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Clear all snapshots
    pub fn clear(&mut self) {
        self.snapshots.clear();
        self.current = 0;
    }

    /// Get the total compressed size of all snapshots
    pub fn compressed_size(&self) -> usize {
        self.snapshots.iter().map(|s| s.len()).sum()
    }

    /// Decompress a snapshot at the given index
    fn decompress(&self, index: usize) -> Result<WorkflowGraph> {
        let compressed = &self.snapshots[index];
        let json = zstd::decode_all(&compressed[..])
            .map_err(|e| NodeEngineError::Compression(e.to_string()))?;
        let graph: WorkflowGraph = serde_json::from_slice(&json)?;
        Ok(graph)
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(100) // Default to 100 snapshots
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GraphNode;

    fn make_graph(name: &str) -> WorkflowGraph {
        let mut graph = WorkflowGraph::new("test", name);
        graph.nodes.push(GraphNode {
            id: "node1".to_string(),
            node_type: "test".to_string(),
            data: serde_json::json!({"name": name}),
            position: (0.0, 0.0),
        });
        graph
    }

    #[test]
    fn test_push_and_undo() {
        let mut stack = UndoStack::new(10);

        let graph1 = make_graph("first");
        let graph2 = make_graph("second");
        let graph3 = make_graph("third");

        stack.push(&graph1).unwrap();
        stack.push(&graph2).unwrap();
        stack.push(&graph3).unwrap();

        // Should be at "third"
        let current = stack.current().unwrap().unwrap();
        assert_eq!(current.name, "third");

        // Undo to "second"
        let undone = stack.undo().unwrap().unwrap();
        assert_eq!(undone.name, "second");

        // Undo to "first"
        let undone = stack.undo().unwrap().unwrap();
        assert_eq!(undone.name, "first");

        // Can't undo further
        assert!(stack.undo().is_none());
    }

    #[test]
    fn test_redo() {
        let mut stack = UndoStack::new(10);

        let graph1 = make_graph("first");
        let graph2 = make_graph("second");

        stack.push(&graph1).unwrap();
        stack.push(&graph2).unwrap();

        stack.undo(); // Go to "first"

        // Redo to "second"
        let redone = stack.redo().unwrap().unwrap();
        assert_eq!(redone.name, "second");

        // Can't redo further
        assert!(stack.redo().is_none());
    }

    #[test]
    fn test_push_truncates_redo() {
        let mut stack = UndoStack::new(10);

        let graph1 = make_graph("first");
        let graph2 = make_graph("second");
        let graph3 = make_graph("third");

        stack.push(&graph1).unwrap();
        stack.push(&graph2).unwrap();
        stack.undo(); // Go to "first"

        // Push new graph - should truncate "second"
        stack.push(&graph3).unwrap();

        // Can't redo anymore
        assert!(!stack.can_redo());

        // Current is "third"
        let current = stack.current().unwrap().unwrap();
        assert_eq!(current.name, "third");
    }

    #[test]
    fn test_max_snapshots() {
        let mut stack = UndoStack::new(3);

        for i in 0..5 {
            let graph = make_graph(&format!("graph_{}", i));
            stack.push(&graph).unwrap();
        }

        // Should only have 3 snapshots
        assert_eq!(stack.len(), 3);

        // Should have graph_2, graph_3, graph_4 (oldest trimmed)
        let current = stack.current().unwrap().unwrap();
        assert_eq!(current.name, "graph_4");

        // Can only undo twice (to graph_3 and graph_2)
        stack.undo();
        stack.undo();
        assert!(!stack.can_undo());
    }

    #[test]
    fn test_can_undo_redo() {
        let mut stack = UndoStack::new(10);

        assert!(!stack.can_undo());
        assert!(!stack.can_redo());

        stack.push(&make_graph("first")).unwrap();
        assert!(!stack.can_undo()); // Only one snapshot
        assert!(!stack.can_redo());

        stack.push(&make_graph("second")).unwrap();
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        stack.undo();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
    }
}

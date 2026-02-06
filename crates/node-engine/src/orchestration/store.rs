//! Orchestration storage with file persistence.
//!
//! This module provides persistent storage for orchestration graphs,
//! enabling the two-level workflow system to load orchestrations on startup.

use super::types::{OrchestrationGraph, OrchestrationGraphId};
use crate::{Result, WorkflowGraph};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Metadata for an orchestration graph (for listing).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationGraphMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub node_count: usize,
}

/// In-memory orchestration store with optional file persistence.
///
/// The store maintains orchestration graphs in memory for fast access,
/// with optional JSON file persistence for durability across restarts.
///
/// # Example
///
/// ```ignore
/// use node_engine::OrchestrationStore;
///
/// // Create a persistent store
/// let mut store = OrchestrationStore::with_persistence(".pantograph/orchestrations");
///
/// // Load existing orchestrations from disk
/// let count = store.load_from_disk()?;
/// println!("Loaded {} orchestrations", count);
///
/// // Insert a new orchestration (automatically persisted)
/// store.insert(my_orchestration)?;
/// ```
#[derive(Debug, Default)]
pub struct OrchestrationStore {
    /// Stored orchestration graphs, keyed by ID.
    graphs: HashMap<String, OrchestrationGraph>,
    /// Mapping from data graph node IDs to their workflow graphs.
    data_graphs: HashMap<String, WorkflowGraph>,
    /// Optional path for file persistence.
    persist_path: Option<PathBuf>,
}

impl OrchestrationStore {
    /// Create a new in-memory store without persistence.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a store that persists to the given directory.
    ///
    /// The directory will be created if it doesn't exist when saving.
    pub fn with_persistence(path: impl AsRef<Path>) -> Self {
        Self {
            graphs: HashMap::new(),
            data_graphs: HashMap::new(),
            persist_path: Some(path.as_ref().to_path_buf()),
        }
    }

    /// Load all orchestrations from the persistence directory.
    ///
    /// Returns the number of orchestrations loaded.
    pub fn load_from_disk(&mut self) -> Result<usize> {
        let Some(ref path) = self.persist_path else {
            return Ok(0);
        };

        if !path.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.extension().map_or(false, |e| e == "json") {
                let content = std::fs::read_to_string(&file_path)?;
                match serde_json::from_str::<OrchestrationGraph>(&content) {
                    Ok(graph) => {
                        log::info!("Loaded orchestration '{}' from {:?}", graph.id, file_path);
                        self.graphs.insert(graph.id.clone(), graph);
                        count += 1;
                    }
                    Err(e) => {
                        log::warn!("Failed to parse orchestration from {:?}: {}", file_path, e);
                    }
                }
            }
        }
        Ok(count)
    }

    /// Save an orchestration to disk (if persistence is enabled).
    fn save_to_disk(&self, graph: &OrchestrationGraph) -> Result<()> {
        let Some(ref path) = self.persist_path else {
            return Ok(());
        };

        std::fs::create_dir_all(path)?;
        let file_path = path.join(format!("{}.json", &graph.id));
        let content = serde_json::to_string_pretty(graph)?;
        std::fs::write(&file_path, content)?;
        log::debug!("Saved orchestration '{}' to {:?}", graph.id, file_path);
        Ok(())
    }

    /// Delete an orchestration from disk (if persistence is enabled).
    fn delete_from_disk(&self, id: &str) -> Result<()> {
        let Some(ref path) = self.persist_path else {
            return Ok(());
        };

        let file_path = path.join(format!("{}.json", id));
        if file_path.exists() {
            std::fs::remove_file(&file_path)?;
            log::debug!("Deleted orchestration '{}' from {:?}", id, file_path);
        }
        Ok(())
    }

    // =========================================================================
    // Graph access methods
    // =========================================================================

    /// Get an orchestration graph by ID.
    pub fn get_graph(&self, id: &str) -> Option<&OrchestrationGraph> {
        self.graphs.get(id)
    }

    /// Get a mutable reference to an orchestration graph by ID.
    pub fn get_graph_mut(&mut self, id: &str) -> Option<&mut OrchestrationGraph> {
        self.graphs.get_mut(id)
    }

    /// Insert or update an orchestration graph.
    ///
    /// The graph is automatically persisted to disk if persistence is enabled.
    pub fn insert_graph(&mut self, graph: OrchestrationGraph) -> Result<()> {
        self.save_to_disk(&graph)?;
        self.graphs.insert(graph.id.clone(), graph);
        Ok(())
    }

    /// Remove an orchestration graph by ID.
    ///
    /// Returns the removed graph if it existed.
    pub fn remove_graph(&mut self, id: &str) -> Result<Option<OrchestrationGraph>> {
        self.delete_from_disk(id)?;
        Ok(self.graphs.remove(id))
    }

    /// List all orchestration graphs.
    pub fn list_graphs(&self) -> Vec<OrchestrationGraphMetadata> {
        self.graphs
            .values()
            .map(|g| OrchestrationGraphMetadata {
                id: g.id.clone(),
                name: g.name.clone(),
                description: g.description.clone(),
                node_count: g.nodes.len(),
            })
            .collect()
    }

    /// Get all orchestration graph IDs.
    pub fn graph_ids(&self) -> Vec<OrchestrationGraphId> {
        self.graphs.keys().cloned().collect()
    }

    /// Check if an orchestration exists.
    pub fn contains(&self, id: &str) -> bool {
        self.graphs.contains_key(id)
    }

    // =========================================================================
    // Data graph methods (for DataGraph node references)
    // =========================================================================

    /// Get a data graph by ID.
    pub fn get_data_graph(&self, id: &str) -> Option<&WorkflowGraph> {
        self.data_graphs.get(id)
    }

    /// Insert a data graph (for execution of DataGraph nodes).
    pub fn insert_data_graph(&mut self, id: String, graph: WorkflowGraph) {
        self.data_graphs.insert(id, graph);
    }

    /// Remove a data graph.
    pub fn remove_data_graph(&mut self, id: &str) -> Option<WorkflowGraph> {
        self.data_graphs.remove(id)
    }

    /// Clear all data graphs from memory.
    pub fn clear_data_graphs(&mut self) {
        self.data_graphs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::types::{OrchestrationNode, OrchestrationNodeType};
    use tempfile::TempDir;

    fn create_test_orchestration(id: &str, name: &str) -> OrchestrationGraph {
        let mut graph = OrchestrationGraph::new(id, name);
        graph.nodes.push(OrchestrationNode::new(
            "start",
            OrchestrationNodeType::Start,
            (0.0, 0.0),
        ));
        graph.nodes.push(OrchestrationNode::new(
            "end",
            OrchestrationNodeType::End,
            (100.0, 0.0),
        ));
        graph
    }

    #[test]
    fn test_in_memory_store() {
        let mut store = OrchestrationStore::new();

        // Insert
        let graph = create_test_orchestration("test-1", "Test Orchestration");
        store.insert_graph(graph).unwrap();

        // Get
        assert!(store.get_graph("test-1").is_some());
        assert!(store.get_graph("nonexistent").is_none());

        // List
        let list = store.list_graphs();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "test-1");

        // Remove
        let removed = store.remove_graph("test-1").unwrap();
        assert!(removed.is_some());
        assert!(store.get_graph("test-1").is_none());
    }

    #[test]
    fn test_persistent_store() {
        let temp_dir = TempDir::new().unwrap();
        let persist_path = temp_dir.path().join("orchestrations");

        // Create and populate store
        {
            let mut store = OrchestrationStore::with_persistence(&persist_path);
            let graph = create_test_orchestration("persist-test", "Persistent Test");
            store.insert_graph(graph).unwrap();
        }

        // Create new store and load from disk
        {
            let mut store = OrchestrationStore::with_persistence(&persist_path);
            let count = store.load_from_disk().unwrap();
            assert_eq!(count, 1);
            assert!(store.get_graph("persist-test").is_some());
        }
    }

    #[test]
    fn test_data_graph_storage() {
        let mut store = OrchestrationStore::new();

        let workflow = WorkflowGraph::new("test-wf", "Test Workflow");

        store.insert_data_graph("my-workflow".to_string(), workflow);
        assert!(store.get_data_graph("my-workflow").is_some());

        store.remove_data_graph("my-workflow");
        assert!(store.get_data_graph("my-workflow").is_none());
    }
}

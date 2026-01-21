//! Vector Database Task
//!
//! Provides a reference to a LanceDB vector database for use with RAG Search.
//! This task outputs the database path and can optionally store vectors.
//!
//! # Inputs (from context)
//! - `{task_id}.input.database_path` - Path to the database (set by UI)
//! - `{task_id}.input.vector` (optional) - Vectors to store
//! - `{task_id}.input.data` (optional) - Corresponding data to store
//!
//! # Outputs (to context)
//! - `{task_id}.output.database` - Database path for RAG Search

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Vector Database Task
///
/// Outputs a database path that can be connected to RAG Search nodes.
/// Optionally stores vectors and their corresponding data.
#[derive(Clone)]
pub struct VectorDbTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl VectorDbTask {
    /// Port ID for database path input
    pub const PORT_DATABASE_PATH: &'static str = "database_path";
    /// Port ID for vector input
    pub const PORT_VECTOR: &'static str = "vector";
    /// Port ID for data input
    pub const PORT_DATA: &'static str = "data";
    /// Port ID for database output
    pub const PORT_DATABASE: &'static str = "database";

    /// Create a new vector database task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }

    /// Get the task ID
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

impl TaskDescriptor for VectorDbTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "vector-db".to_string(),
            category: NodeCategory::Input,
            label: "Vector Database".to_string(),
            description: "Select or create a vector database for RAG".to_string(),
            inputs: vec![
                PortMetadata::optional(
                    Self::PORT_DATABASE_PATH,
                    "Database Path",
                    PortDataType::String,
                ),
                PortMetadata::optional(Self::PORT_VECTOR, "Vector", PortDataType::Embedding)
                    .multiple(),
                PortMetadata::optional(Self::PORT_DATA, "Data", PortDataType::String).multiple(),
            ],
            outputs: vec![PortMetadata::optional(
                Self::PORT_DATABASE,
                "Database",
                PortDataType::VectorDb,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Task for VectorDbTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get database path from node data (set by UI dropdown selection)
        let db_path_key = ContextKeys::input(&self.task_id, "database_path");
        let db_path: Option<String> = context.get(&db_path_key).await;

        // Get optional vectors and data to store
        let vector_key = ContextKeys::input(&self.task_id, "vector");
        let data_key = ContextKeys::input(&self.task_id, "data");

        let vectors: Option<Vec<Vec<f64>>> = context.get(&vector_key).await;
        let data: Option<Vec<String>> = context.get(&data_key).await;

        // Log if we have vectors to store (actual storage would require Tauri integration)
        if let (Some(vecs), Some(content)) = (&vectors, &data) {
            log::info!(
                "VectorDbTask {}: {} vectors and {} data entries to store",
                self.task_id,
                vecs.len(),
                content.len()
            );
            // TODO: Store vectors via Tauri command
            // This would require communicating with the RagManager through a channel
        }

        // Output the database path
        let output_key = ContextKeys::output(&self.task_id, "database");
        let path = db_path.unwrap_or_default();
        context.set(&output_key, path.clone()).await;

        log::debug!(
            "VectorDbTask {}: outputting database path: {}",
            self.task_id,
            path
        );

        Ok(TaskResult::new(
            Some(format!(
                "Database: {}",
                if path.is_empty() { "default" } else { &path }
            )),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = VectorDbTask::new("my_db");
        assert_eq!(task.id(), "my_db");
    }

    #[tokio::test]
    async fn test_output_database_path() {
        let task = VectorDbTask::new("test_db");
        let context = Context::new();

        // Set database path
        let path_key = ContextKeys::input("test_db", "database_path");
        context.set(&path_key, "/path/to/db".to_string()).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify output
        let output_key = ContextKeys::output("test_db", "database");
        let output_path: Option<String> = context.get(&output_key).await;
        assert_eq!(output_path, Some("/path/to/db".to_string()));
    }

    #[tokio::test]
    async fn test_empty_path_defaults() {
        let task = VectorDbTask::new("test_db");
        let context = Context::new();

        // Run without setting path
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Should output empty string
        let output_key = ContextKeys::output("test_db", "database");
        let output_path: Option<String> = context.get(&output_key).await;
        assert_eq!(output_path, Some(String::new()));
    }
}

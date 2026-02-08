//! LanceDB Task
//!
//! Vector database operations for storing and querying embeddings.
//! Replaces the monolithic RagSearchTask with atomic database operations.
//!
//! # Inputs (from context)
//! - `{task_id}.input.database_path` - Path to the LanceDB database
//! - `{task_id}.input.query_embedding` - Embedding vector for similarity search
//! - `{task_id}.input.store_embedding` - Embedding to store
//! - `{task_id}.input.store_data` - Data to store alongside embedding
//! - `{task_id}.input.limit` - Maximum number of results
//!
//! # Outputs (to context)
//! - `{task_id}.output.results` - Array of search results
//! - `{task_id}.output.database` - Database path for chaining

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// A search result from the vector database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The stored text/data
    pub content: String,
    /// Similarity score (higher is more similar)
    pub score: f64,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Configuration for the LanceDB task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanceDbConfig {
    /// Table name within the database
    pub table_name: String,
    /// Default number of results to return
    pub default_limit: usize,
}

impl Default for LanceDbConfig {
    fn default() -> Self {
        Self {
            table_name: "embeddings".to_string(),
            default_limit: 5,
        }
    }
}

/// LanceDB Task
///
/// Provides atomic vector database operations:
/// - Query: Search for similar vectors
/// - Store: Add new vectors to the database
///
/// This replaces the monolithic RagSearchTask and allows users to compose
/// their own RAG workflows by connecting Embedding → LanceDB → Inference.
#[derive(Clone)]
pub struct LanceDbTask {
    /// Unique identifier for this task instance
    task_id: String,
    /// Configuration
    config: Option<LanceDbConfig>,
}

impl LanceDbTask {
    /// Port ID for database path input
    pub const PORT_DATABASE_PATH: &'static str = "database_path";
    /// Port ID for query embedding input
    pub const PORT_QUERY_EMBEDDING: &'static str = "query_embedding";
    /// Port ID for store embedding input
    pub const PORT_STORE_EMBEDDING: &'static str = "store_embedding";
    /// Port ID for store data input
    pub const PORT_STORE_DATA: &'static str = "store_data";
    /// Port ID for limit input
    pub const PORT_LIMIT: &'static str = "limit";
    /// Port ID for results output
    pub const PORT_RESULTS: &'static str = "results";
    /// Port ID for database output (for chaining)
    pub const PORT_DATABASE: &'static str = "database";

    /// Create a new LanceDB task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            config: None,
        }
    }

    /// Create with configuration
    pub fn with_config(task_id: impl Into<String>, config: LanceDbConfig) -> Self {
        Self {
            task_id: task_id.into(),
            config: Some(config),
        }
    }

    /// Get the task ID
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

impl TaskDescriptor for LanceDbTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "lancedb".to_string(),
            category: NodeCategory::Tool,
            label: "LanceDB".to_string(),
            description: "Vector database operations (store/query)".to_string(),
            inputs: vec![
                PortMetadata::optional(
                    Self::PORT_DATABASE_PATH,
                    "Database Path",
                    PortDataType::String,
                ),
                PortMetadata::optional(
                    Self::PORT_QUERY_EMBEDDING,
                    "Query Embedding",
                    PortDataType::Embedding,
                ),
                PortMetadata::optional(
                    Self::PORT_STORE_EMBEDDING,
                    "Store Embedding",
                    PortDataType::Embedding,
                ),
                PortMetadata::optional(Self::PORT_STORE_DATA, "Store Data", PortDataType::String),
                PortMetadata::optional(Self::PORT_LIMIT, "Limit", PortDataType::Number),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_RESULTS, "Results", PortDataType::Document)
                    .multiple(),
                PortMetadata::optional(Self::PORT_DATABASE, "Database", PortDataType::VectorDb),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(LanceDbTask::descriptor));

#[async_trait]
impl Task for LanceDbTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get configuration
        let config = if let Some(ref cfg) = self.config {
            cfg.clone()
        } else {
            let config_key = ContextKeys::meta(&self.task_id, "config");
            context
                .get::<LanceDbConfig>(&config_key)
                .await
                .unwrap_or_default()
        };

        // Get database path
        let db_path_key = ContextKeys::input(&self.task_id, Self::PORT_DATABASE_PATH);
        let db_path: String = context
            .get(&db_path_key)
            .await
            .unwrap_or_else(|| "default.lancedb".to_string());

        // Get limit
        let limit_key = ContextKeys::input(&self.task_id, Self::PORT_LIMIT);
        let limit: usize = context
            .get::<f64>(&limit_key)
            .await
            .map(|n| n as usize)
            .unwrap_or(config.default_limit);

        // Check for store operation
        let store_embedding_key = ContextKeys::input(&self.task_id, Self::PORT_STORE_EMBEDDING);
        let store_data_key = ContextKeys::input(&self.task_id, Self::PORT_STORE_DATA);

        let store_embedding: Option<Vec<f64>> = context.get(&store_embedding_key).await;
        let store_data: Option<String> = context.get(&store_data_key).await;

        if let (Some(embedding), Some(data)) = (&store_embedding, &store_data) {
            log::info!(
                "LanceDbTask {}: storing {}-dimensional embedding with data at '{}'",
                self.task_id,
                embedding.len(),
                db_path
            );

            // TODO: Implement actual LanceDB storage
            // This would use lancedb crate to:
            // 1. Open/create the database
            // 2. Create table if needed
            // 3. Insert the embedding and data

            // For now, just log the operation
            log::debug!(
                "LanceDbTask {}: would store embedding with {} chars of data",
                self.task_id,
                data.len()
            );
        }

        // Check for query operation
        let query_embedding_key = ContextKeys::input(&self.task_id, Self::PORT_QUERY_EMBEDDING);
        let query_embedding: Option<Vec<f64>> = context.get(&query_embedding_key).await;

        let results: Vec<SearchResult> = Vec::new();

        if let Some(embedding) = &query_embedding {
            log::info!(
                "LanceDbTask {}: querying with {}-dimensional embedding (limit: {})",
                self.task_id,
                embedding.len(),
                limit
            );

            // TODO: Implement actual LanceDB query
            // This would use lancedb crate to:
            // 1. Open the database
            // 2. Search for similar vectors
            // 3. Return top-k results

            // For now, return empty results with a debug message
            log::debug!(
                "LanceDbTask {}: query not implemented - returning empty results",
                self.task_id
            );
        }

        // Store outputs in context
        let results_key = ContextKeys::output(&self.task_id, Self::PORT_RESULTS);
        context.set(&results_key, results.clone()).await;

        let database_key = ContextKeys::output(&self.task_id, Self::PORT_DATABASE);
        context.set(&database_key, db_path.clone()).await;

        log::debug!(
            "LanceDbTask {}: completed with {} results",
            self.task_id,
            results.len()
        );

        Ok(TaskResult::new(
            Some(format!("LanceDB: {} results from '{}'", results.len(), db_path)),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = LanceDbTask::new("my_db");
        assert_eq!(task.id(), "my_db");
    }

    #[test]
    fn test_with_config() {
        let config = LanceDbConfig {
            table_name: "custom_table".to_string(),
            default_limit: 10,
        };
        let task = LanceDbTask::with_config("task1", config);
        assert_eq!(task.config.as_ref().unwrap().table_name, "custom_table");
        assert_eq!(task.config.as_ref().unwrap().default_limit, 10);
    }

    #[test]
    fn test_default_config() {
        let config = LanceDbConfig::default();
        assert_eq!(config.table_name, "embeddings");
        assert_eq!(config.default_limit, 5);
    }

    #[test]
    fn test_descriptor() {
        let meta = LanceDbTask::descriptor();
        assert_eq!(meta.node_type, "lancedb");
        assert_eq!(meta.category, NodeCategory::Tool);
        assert_eq!(meta.inputs.len(), 5);
        assert_eq!(meta.outputs.len(), 2);
    }

    #[test]
    fn test_search_result_serialize() {
        let result = SearchResult {
            content: "test content".to_string(),
            score: 0.95,
            metadata: Some(serde_json::json!({"source": "test.txt"})),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test content"));
        assert!(json.contains("0.95"));
        assert!(json.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_default_database_path() {
        let task = LanceDbTask::new("test_db");
        let context = Context::new();

        // Run without setting any inputs
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Should output default database path
        let db_key = ContextKeys::output("test_db", "database");
        let db_path: Option<String> = context.get(&db_key).await;
        assert_eq!(db_path, Some("default.lancedb".to_string()));
    }

    #[tokio::test]
    async fn test_custom_database_path() {
        let task = LanceDbTask::new("test_db");
        let context = Context::new();

        // Set custom database path
        let path_key = ContextKeys::input("test_db", "database_path");
        context.set(&path_key, "/custom/path.lancedb".to_string()).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Should output the custom path
        let db_key = ContextKeys::output("test_db", "database");
        let db_path: Option<String> = context.get(&db_key).await;
        assert_eq!(db_path, Some("/custom/path.lancedb".to_string()));
    }
}

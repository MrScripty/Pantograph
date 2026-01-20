//! RAG Search Task
//!
//! Performs semantic search using vector embeddings.
//! This task interfaces with a RAG (Retrieval-Augmented Generation) system
//! to find relevant documents based on a query.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use serde::{Deserialize, Serialize};

use super::ContextKeys;

/// A document returned from RAG search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagDocument {
    /// Document title or filename
    pub title: String,
    /// Document content
    pub content: String,
    /// Similarity score (0.0 to 1.0)
    pub score: Option<f32>,
    /// Original source path or URI
    pub source: Option<String>,
}

/// Configuration for RAG search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Embedding API base URL
    pub embedding_url: String,
    /// Embedding model name
    pub embedding_model: String,
    /// Vector database URL (e.g., LanceDB path)
    pub vector_db_url: Option<String>,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            embedding_url: "http://localhost:8080".to_string(),
            embedding_model: "text-embedding-ada-002".to_string(),
            vector_db_url: None,
        }
    }
}

/// RAG Search Task
///
/// Searches indexed documents for relevant content using semantic search.
/// This is a simplified implementation - in production, this would
/// interface with a vector database like LanceDB.
///
/// # Inputs (from context)
/// - `{task_id}.input.query` (required) - The search query
/// - `{task_id}.input.limit` (optional) - Maximum number of results (default: 5)
/// - `{task_id}.input.documents` (optional) - Pre-indexed documents to search
///
/// # Outputs (to context)
/// - `{task_id}.output.documents` - Array of RagDocument results
/// - `{task_id}.output.context` - Formatted context string for LLM prompts
#[derive(Clone)]
pub struct RagSearchTask {
    /// Unique identifier for this task instance
    task_id: String,
    /// Configuration (for future vector DB integration)
    #[allow(dead_code)]
    config: Option<RagConfig>,
}

impl RagSearchTask {
    /// Create a new RAG search task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            config: None,
        }
    }

    /// Create with configuration
    pub fn with_config(task_id: impl Into<String>, config: RagConfig) -> Self {
        Self {
            task_id: task_id.into(),
            config: Some(config),
        }
    }

    /// Get the task ID
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Simple keyword-based search (fallback when no vector DB is configured)
    fn keyword_search(query: &str, documents: &[RagDocument], limit: usize) -> Vec<RagDocument> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored_docs: Vec<(f32, RagDocument)> = documents
            .iter()
            .map(|doc| {
                let content_lower = doc.content.to_lowercase();
                let title_lower = doc.title.to_lowercase();

                // Simple scoring: count matching words
                let mut score = 0.0f32;
                for word in &query_words {
                    if content_lower.contains(word) {
                        score += 1.0;
                    }
                    if title_lower.contains(word) {
                        score += 2.0; // Title matches weighted higher
                    }
                }

                // Normalize by query length
                score /= query_words.len().max(1) as f32;

                let mut result = doc.clone();
                result.score = Some(score);
                (score, result)
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        // Sort by score descending
        scored_docs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Take top results
        scored_docs.into_iter().take(limit).map(|(_, doc)| doc).collect()
    }
}

#[async_trait]
impl Task for RagSearchTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: query
        let query_key = ContextKeys::input(&self.task_id, "query");
        let query: String = context.get(&query_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'query' at key '{}'",
                query_key
            ))
        })?;

        // Get optional limit
        let limit_key = ContextKeys::input(&self.task_id, "limit");
        let limit: usize = context
            .get::<f64>(&limit_key)
            .await
            .map(|n| n as usize)
            .unwrap_or(5);

        // Get pre-indexed documents from context (if available)
        let docs_key = ContextKeys::input(&self.task_id, "documents");
        let documents: Vec<RagDocument> = context.get(&docs_key).await.unwrap_or_default();

        log::debug!(
            "RagSearchTask {}: searching {} documents for '{}'",
            self.task_id,
            documents.len(),
            query.chars().take(50).collect::<String>()
        );

        // Perform search
        // In a full implementation, this would:
        // 1. Get embedding for query via embedding API
        // 2. Search vector database for similar documents
        // 3. Return ranked results
        //
        // For now, we do a simple keyword-based fallback search
        let results = Self::keyword_search(&query, &documents, limit);

        // Build context string from documents
        let context_str = results
            .iter()
            .map(|doc| format!("## {}\n{}", doc.title, doc.content))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        // Store outputs in context
        let docs_output_key = ContextKeys::output(&self.task_id, "documents");
        context.set(&docs_output_key, results.clone()).await;

        let context_output_key = ContextKeys::output(&self.task_id, "context");
        context.set(&context_output_key, context_str.clone()).await;

        log::debug!(
            "RagSearchTask {}: found {} documents",
            self.task_id,
            results.len()
        );

        Ok(TaskResult::new(
            Some(format!("Found {} documents", results.len())),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = RagSearchTask::new("my_rag");
        assert_eq!(task.id(), "my_rag");
    }

    #[test]
    fn test_keyword_search() {
        let docs = vec![
            RagDocument {
                title: "Rust Programming".to_string(),
                content: "Rust is a systems programming language".to_string(),
                score: None,
                source: None,
            },
            RagDocument {
                title: "Python Guide".to_string(),
                content: "Python is great for scripting".to_string(),
                score: None,
                source: None,
            },
            RagDocument {
                title: "Rust Async".to_string(),
                content: "Async programming in Rust with tokio".to_string(),
                score: None,
                source: None,
            },
        ];

        let results = RagSearchTask::keyword_search("rust programming", &docs, 5);

        // Should find documents with "rust" or "programming"
        assert!(!results.is_empty());
        // First result should be most relevant (matches both words)
        assert!(results[0].title.contains("Rust"));
    }

    #[tokio::test]
    async fn test_search_with_documents() {
        let task = RagSearchTask::new("test_rag");
        let context = Context::new();

        // Set query
        let query_key = ContextKeys::input("test_rag", "query");
        context.set(&query_key, "rust async".to_string()).await;

        // Set documents
        let docs_key = ContextKeys::input("test_rag", "documents");
        let docs = vec![
            RagDocument {
                title: "Async Rust".to_string(),
                content: "Learn async programming in Rust".to_string(),
                score: None,
                source: None,
            },
            RagDocument {
                title: "Python Basics".to_string(),
                content: "Getting started with Python".to_string(),
                score: None,
                source: None,
            },
        ];
        context.set(&docs_key, docs).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify documents output
        let output_key = ContextKeys::output("test_rag", "documents");
        let output_docs: Option<Vec<RagDocument>> = context.get(&output_key).await;
        assert!(output_docs.is_some());
        let docs = output_docs.unwrap();
        assert!(!docs.is_empty());
        assert!(docs[0].title.contains("Async") || docs[0].title.contains("Rust"));
    }

    #[tokio::test]
    async fn test_empty_documents() {
        let task = RagSearchTask::new("test_rag");
        let context = Context::new();

        // Set query but no documents
        let query_key = ContextKeys::input("test_rag", "query");
        context.set(&query_key, "test query".to_string()).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Should return empty results
        let output_key = ContextKeys::output("test_rag", "documents");
        let output_docs: Option<Vec<RagDocument>> = context.get(&output_key).await;
        assert!(output_docs.is_some());
        assert!(output_docs.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_missing_query_error() {
        let task = RagSearchTask::new("test_rag");
        let context = Context::new();

        // Run without setting query - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_context_string_output() {
        let task = RagSearchTask::new("test_rag");
        let context = Context::new();

        // Set query
        let query_key = ContextKeys::input("test_rag", "query");
        context.set(&query_key, "test".to_string()).await;

        // Set documents
        let docs_key = ContextKeys::input("test_rag", "documents");
        let docs = vec![RagDocument {
            title: "Test Doc".to_string(),
            content: "This is a test document".to_string(),
            score: None,
            source: None,
        }];
        context.set(&docs_key, docs).await;

        // Run task
        task.run(context.clone()).await.unwrap();

        // Verify context string
        let context_key = ContextKeys::output("test_rag", "context");
        let context_str: Option<String> = context.get(&context_key).await;
        assert!(context_str.is_some());
        let ctx = context_str.unwrap();
        assert!(ctx.contains("## Test Doc"));
        assert!(ctx.contains("This is a test document"));
    }
}

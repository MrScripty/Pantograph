//! Write File Task
//!
//! Writes content to a file in the project.
//! Creates parent directories if needed.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use std::path::PathBuf;
use tokio::fs;

use super::ContextKeys;

/// Write File Task
///
/// Writes content to a file relative to the project root.
/// Creates parent directories if they don't exist.
///
/// # Inputs (from context)
/// - `{task_id}.input.path` (required) - File path to write
/// - `{task_id}.input.content` (required) - Content to write
/// - `{task_id}.input.project_root` (optional) - Project root directory
///
/// # Outputs (to context)
/// - `{task_id}.output.success` - Whether the write succeeded
/// - `{task_id}.output.path` - The path that was written to
#[derive(Clone)]
pub struct WriteFileTask {
    /// Unique identifier for this task instance
    task_id: String,
    /// Default project root if not specified in context
    default_project_root: Option<PathBuf>,
}

impl WriteFileTask {
    /// Create a new write file task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            default_project_root: None,
        }
    }

    /// Create with a default project root
    pub fn with_project_root(task_id: impl Into<String>, root: PathBuf) -> Self {
        Self {
            task_id: task_id.into(),
            default_project_root: Some(root),
        }
    }

    /// Get the task ID
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

#[async_trait]
impl Task for WriteFileTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: path
        let path_key = ContextKeys::input(&self.task_id, "path");
        let path_str: String = context.get(&path_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'path' at key '{}'",
                path_key
            ))
        })?;

        // Get required input: content
        let content_key = ContextKeys::input(&self.task_id, "content");
        let content: String = context.get(&content_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'content' at key '{}'",
                content_key
            ))
        })?;

        // Get project root from context or use default
        let project_root_key = ContextKeys::input(&self.task_id, "project_root");
        let project_root: PathBuf = context
            .get::<String>(&project_root_key)
            .await
            .map(PathBuf::from)
            .or_else(|| self.default_project_root.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        // Resolve path relative to project root
        let full_path = project_root.join(&path_str);

        log::debug!(
            "WriteFileTask {}: writing {} bytes to '{}'",
            self.task_id,
            content.len(),
            full_path.display()
        );

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    GraphError::TaskExecutionFailed(format!(
                        "Failed to create directories for '{}': {}",
                        full_path.display(),
                        e
                    ))
                })?;
            }
        }

        // Write the file
        fs::write(&full_path, &content).await.map_err(|e| {
            GraphError::TaskExecutionFailed(format!(
                "Failed to write file '{}': {}",
                full_path.display(),
                e
            ))
        })?;

        // Store outputs in context
        let success_key = ContextKeys::output(&self.task_id, "success");
        context.set(&success_key, true).await;

        let output_path_key = ContextKeys::output(&self.task_id, "path");
        context.set(&output_path_key, path_str.clone()).await;

        log::debug!(
            "WriteFileTask {}: successfully wrote {} bytes",
            self.task_id,
            content.len()
        );

        Ok(TaskResult::new(Some(path_str), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_task_id() {
        let task = WriteFileTask::new("my_writer");
        assert_eq!(task.id(), "my_writer");
    }

    #[tokio::test]
    async fn test_write_file() {
        let dir = tempdir().unwrap();
        let task = WriteFileTask::with_project_root("test_writer", dir.path().to_path_buf());
        let context = Context::new();

        // Set inputs
        let path_key = ContextKeys::input("test_writer", "path");
        context.set(&path_key, "output.txt".to_string()).await;

        let content_key = ContextKeys::input("test_writer", "content");
        context
            .set(&content_key, "Hello, file!".to_string())
            .await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify success output
        let success_key = ContextKeys::output("test_writer", "success");
        let success: Option<bool> = context.get(&success_key).await;
        assert_eq!(success, Some(true));

        // Verify file was created
        let file_path = dir.path().join("output.txt");
        assert!(file_path.exists());

        // Verify content
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, file!");
    }

    #[tokio::test]
    async fn test_write_with_subdirectories() {
        let dir = tempdir().unwrap();
        let task = WriteFileTask::with_project_root("test_writer", dir.path().to_path_buf());
        let context = Context::new();

        // Set inputs with nested path
        let path_key = ContextKeys::input("test_writer", "path");
        context
            .set(&path_key, "subdir/nested/output.txt".to_string())
            .await;

        let content_key = ContextKeys::input("test_writer", "content");
        context
            .set(&content_key, "Nested content".to_string())
            .await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify file was created in nested directory
        let file_path = dir.path().join("subdir/nested/output.txt");
        assert!(file_path.exists());

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Nested content");
    }

    #[tokio::test]
    async fn test_missing_path_error() {
        let task = WriteFileTask::new("test_writer");
        let context = Context::new();

        // Set content but not path
        let content_key = ContextKeys::input("test_writer", "content");
        context.set(&content_key, "content".to_string()).await;

        // Run without setting path - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_content_error() {
        let task = WriteFileTask::new("test_writer");
        let context = Context::new();

        // Set path but not content
        let path_key = ContextKeys::input("test_writer", "path");
        context.set(&path_key, "output.txt".to_string()).await;

        // Run without setting content - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}

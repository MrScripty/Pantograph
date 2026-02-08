//! Read File Task
//!
//! Reads content from a file in the project.
//! Supports reading relative to a configurable project root.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use std::path::PathBuf;
use tokio::fs;

/// Read File Task
///
/// Reads content from a file relative to the project root.
///
/// # Inputs (from context)
/// - `{task_id}.input.path` (required) - File path to read
/// - `{task_id}.input.project_root` (optional) - Project root directory
///
/// # Outputs (to context)
/// - `{task_id}.output.content` - The file content
/// - `{task_id}.output.exists` - Whether the file exists
#[derive(Clone)]
pub struct ReadFileTask {
    /// Unique identifier for this task instance
    task_id: String,
    /// Default project root if not specified in context
    default_project_root: Option<PathBuf>,
}

impl ReadFileTask {
    /// Port ID for path input
    pub const PORT_PATH: &'static str = "path";
    /// Port ID for project root input
    pub const PORT_PROJECT_ROOT: &'static str = "project_root";
    /// Port ID for content output
    pub const PORT_CONTENT: &'static str = "content";
    /// Port ID for exists output
    pub const PORT_EXISTS: &'static str = "exists";

    /// Create a new read file task
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

impl TaskDescriptor for ReadFileTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "read-file".to_string(),
            category: NodeCategory::Tool,
            label: "Read File".to_string(),
            description: "Reads content from a file".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_PATH, "Path", PortDataType::String),
                PortMetadata::optional(
                    Self::PORT_PROJECT_ROOT,
                    "Project Root",
                    PortDataType::String,
                ),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_CONTENT, "Content", PortDataType::String),
                PortMetadata::optional(Self::PORT_EXISTS, "Exists", PortDataType::Boolean),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(ReadFileTask::descriptor));

#[async_trait]
impl Task for ReadFileTask {
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
            "ReadFileTask {}: reading file at '{}'",
            self.task_id,
            full_path.display()
        );

        // Check if file exists and read content
        let exists = full_path.exists();
        let content = if exists {
            match fs::read_to_string(&full_path).await {
                Ok(content) => content,
                Err(e) => {
                    return Err(GraphError::TaskExecutionFailed(format!(
                        "Failed to read file '{}': {}",
                        full_path.display(),
                        e
                    )));
                }
            }
        } else {
            String::new()
        };

        // Store outputs in context
        let content_key = ContextKeys::output(&self.task_id, "content");
        context.set(&content_key, content.clone()).await;

        let exists_key = ContextKeys::output(&self.task_id, "exists");
        context.set(&exists_key, exists).await;

        log::debug!(
            "ReadFileTask {}: read {} bytes (exists: {})",
            self.task_id,
            content.len(),
            exists
        );

        Ok(TaskResult::new(Some(content), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_task_id() {
        let task = ReadFileTask::new("my_reader");
        assert_eq!(task.id(), "my_reader");
    }

    #[tokio::test]
    async fn test_read_existing_file() {
        // Create temp file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        {
            let mut file = std::fs::File::create(&file_path).unwrap();
            writeln!(file, "Hello, world!").unwrap();
        }

        let task = ReadFileTask::with_project_root("test_reader", dir.path().to_path_buf());
        let context = Context::new();

        // Set path input
        let path_key = ContextKeys::input("test_reader", "path");
        context.set(&path_key, "test.txt".to_string()).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify content output
        let content_key = ContextKeys::output("test_reader", "content");
        let content: Option<String> = context.get(&content_key).await;
        assert!(content.is_some());
        assert!(content.unwrap().contains("Hello, world!"));

        // Verify exists output
        let exists_key = ContextKeys::output("test_reader", "exists");
        let exists: Option<bool> = context.get(&exists_key).await;
        assert_eq!(exists, Some(true));
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let dir = tempdir().unwrap();
        let task = ReadFileTask::with_project_root("test_reader", dir.path().to_path_buf());
        let context = Context::new();

        // Set path to non-existent file
        let path_key = ContextKeys::input("test_reader", "path");
        context.set(&path_key, "nonexistent.txt".to_string()).await;

        // Run task - should succeed with empty content
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify empty content
        let content_key = ContextKeys::output("test_reader", "content");
        let content: Option<String> = context.get(&content_key).await;
        assert_eq!(content, Some(String::new()));

        // Verify exists is false
        let exists_key = ContextKeys::output("test_reader", "exists");
        let exists: Option<bool> = context.get(&exists_key).await;
        assert_eq!(exists, Some(false));
    }

    #[tokio::test]
    async fn test_missing_path_error() {
        let task = ReadFileTask::new("test_reader");
        let context = Context::new();

        // Run without setting path - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}

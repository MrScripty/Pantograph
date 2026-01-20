//! Component Preview Task
//!
//! Renders a Svelte component on the canvas preview.
//! Emits stream data that tells the frontend to render the specified component.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};

use super::ContextKeys;

/// Component Preview Task
///
/// Renders a Svelte component on the canvas preview.
/// The component path and props are stored in stream data for the frontend.
///
/// # Inputs (from context)
/// - `{task_id}.input.component` (required) - Path to the Svelte component
/// - `{task_id}.input.props` (optional) - Props to pass to the component (JSON)
///
/// # Outputs (to context)
/// - `{task_id}.output.rendered` - Boolean indicating success
///
/// # Streaming
/// - `{task_id}.stream.preview` - Stream event with component info for frontend
#[derive(Clone)]
pub struct ComponentPreviewTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl ComponentPreviewTask {
    /// Create a new component preview task
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

#[async_trait]
impl Task for ComponentPreviewTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: component path
        let component_key = ContextKeys::input(&self.task_id, "component");
        let component_path: String = context.get(&component_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'component' at key '{}'",
                component_key
            ))
        })?;

        // Get optional props
        let props_key = ContextKeys::input(&self.task_id, "props");
        let props: serde_json::Value = context
            .get(&props_key)
            .await
            .unwrap_or(serde_json::Value::Null);

        // Store stream data for frontend to render component
        let stream_key = ContextKeys::stream(&self.task_id, "preview");
        context
            .set(
                &stream_key,
                serde_json::json!({
                    "type": "component_preview",
                    "path": component_path,
                    "props": props
                }),
            )
            .await;

        // Store rendered flag
        let output_key = ContextKeys::output(&self.task_id, "rendered");
        context.set(&output_key, true).await;

        log::debug!(
            "ComponentPreviewTask {}: rendering component '{}'",
            self.task_id,
            component_path
        );

        Ok(TaskResult::new(Some("rendered".to_string()), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = ComponentPreviewTask::new("my_preview");
        assert_eq!(task.id(), "my_preview");
    }

    #[tokio::test]
    async fn test_component_preview() {
        let task = ComponentPreviewTask::new("test_preview");
        let context = Context::new();

        // Set component input
        let component_key = ContextKeys::input("test_preview", "component");
        context
            .set(&component_key, "src/components/MyComponent.svelte".to_string())
            .await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify rendered output
        let output_key = ContextKeys::output("test_preview", "rendered");
        let rendered: Option<bool> = context.get(&output_key).await;
        assert_eq!(rendered, Some(true));

        // Verify stream data
        let stream_key = ContextKeys::stream("test_preview", "preview");
        let stream: Option<serde_json::Value> = context.get(&stream_key).await;
        assert!(stream.is_some());
        let data = stream.unwrap();
        assert_eq!(data["type"], "component_preview");
        assert_eq!(data["path"], "src/components/MyComponent.svelte");
    }

    #[tokio::test]
    async fn test_component_with_props() {
        let task = ComponentPreviewTask::new("test_preview");
        let context = Context::new();

        // Set component input
        let component_key = ContextKeys::input("test_preview", "component");
        context
            .set(&component_key, "MyComponent.svelte".to_string())
            .await;

        // Set props
        let props_key = ContextKeys::input("test_preview", "props");
        context
            .set(
                &props_key,
                serde_json::json!({
                    "title": "Hello",
                    "count": 42
                }),
            )
            .await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify props in stream data
        let stream_key = ContextKeys::stream("test_preview", "preview");
        let stream: Option<serde_json::Value> = context.get(&stream_key).await;
        let data = stream.unwrap();
        assert_eq!(data["props"]["title"], "Hello");
        assert_eq!(data["props"]["count"], 42);
    }

    #[tokio::test]
    async fn test_missing_component_error() {
        let task = ComponentPreviewTask::new("test_preview");
        let context = Context::new();

        // Run without setting component - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}

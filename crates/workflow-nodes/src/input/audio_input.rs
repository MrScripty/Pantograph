//! Audio Input Task
//!
//! Provides audio data (base64 encoded) from a file picker or upstream nodes.
//! Outputs the audio data for downstream processing or playback.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Audio Input Task
///
/// Provides audio data (base64 encoded) from the frontend file picker.
/// The frontend stores the selected file via `_data.audio_data`, and this
/// task passes it through to the output port.
///
/// # Inputs (from context)
/// - `{task_id}.input.audio_data` (required) - Base64-encoded audio data
///
/// # Outputs (to context)
/// - `{task_id}.output.audio` - The base64 audio data
#[derive(Clone)]
pub struct AudioInputTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl AudioInputTask {
    /// Port ID for audio data input (base64 from file picker)
    pub const PORT_AUDIO_DATA: &'static str = "audio_data";
    /// Port ID for audio output
    pub const PORT_AUDIO: &'static str = "audio";

    /// Create a new audio input task
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

impl TaskDescriptor for AudioInputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "audio-input".to_string(),
            category: NodeCategory::Input,
            label: "Audio Input".to_string(),
            description: "Load audio files into the workflow".to_string(),
            inputs: vec![PortMetadata::required(
                Self::PORT_AUDIO_DATA,
                "Audio Data",
                PortDataType::Audio,
            )],
            outputs: vec![PortMetadata::optional(
                Self::PORT_AUDIO,
                "Audio",
                PortDataType::Audio,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(AudioInputTask::descriptor));

#[async_trait]
impl Task for AudioInputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: audio_data (base64 encoded)
        let input_key = ContextKeys::input(&self.task_id, Self::PORT_AUDIO_DATA);
        let audio_data: String = context.get(&input_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'audio_data' at key '{}'",
                input_key
            ))
        })?;

        // Store output in context
        let output_key = ContextKeys::output(&self.task_id, Self::PORT_AUDIO);
        context.set(&output_key, audio_data.clone()).await;

        log::debug!(
            "AudioInputTask {}: passing through {} bytes of audio data",
            self.task_id,
            audio_data.len()
        );

        Ok(TaskResult::new(Some(audio_data), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = AudioInputTask::new("my_audio");
        assert_eq!(task.id(), "my_audio");
    }

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = AudioInputTask::descriptor();
        assert_eq!(meta.node_type, "audio-input");
    }

    #[tokio::test]
    async fn test_audio_passthrough() {
        // Arrange
        let task = AudioInputTask::new("test_audio");
        let context = Context::new();
        let input_key = ContextKeys::input("test_audio", "audio_data");
        context
            .set(&input_key, "UklGRiQAAABXQVZFZm10".to_string())
            .await;

        // Act
        let result = task.run(context.clone()).await.unwrap();

        // Assert
        assert!(matches!(result.next_action, NextAction::Continue));
        let output_key = ContextKeys::output("test_audio", "audio");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some("UklGRiQAAABXQVZFZm10".to_string()));
    }

    #[tokio::test]
    async fn test_missing_audio_error() {
        // Arrange
        let task = AudioInputTask::new("test_audio");
        let context = Context::new();

        // Act
        let result = task.run(context).await;

        // Assert
        assert!(result.is_err());
    }
}

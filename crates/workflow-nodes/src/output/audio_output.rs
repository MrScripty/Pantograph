//! Audio Output Task
//!
//! Displays audio output with playback controls in the workflow.
//! Stores the audio in context and can optionally pass it through for chaining.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Audio Output Task
///
/// Displays audio output with playback controls in the workflow.
/// The audio (base64-encoded WAV/MP3) is stored in context for display
/// and optionally passed through for downstream chaining.
///
/// # Inputs (from context)
/// - `{task_id}.input.audio` (optional) - Base64-encoded audio data
///
/// # Outputs (to context)
/// - `{task_id}.output.audio` - The same audio (for chaining)
///
/// # Streaming
/// - `{task_id}.stream.audio` - Stream event with the audio content
#[derive(Clone)]
pub struct AudioOutputTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl AudioOutputTask {
    /// Port ID for audio input/output
    pub const PORT_AUDIO: &'static str = "audio";
    /// Port ID for streaming input
    pub const PORT_STREAM: &'static str = "stream";

    /// Create a new audio output task
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

impl TaskDescriptor for AudioOutputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "audio-output".to_string(),
            category: NodeCategory::Output,
            label: "Audio Output".to_string(),
            description: "Plays audio output from the workflow".to_string(),
            inputs: vec![
                PortMetadata::optional(Self::PORT_AUDIO, "Audio", PortDataType::Audio),
                PortMetadata::optional(
                    Self::PORT_STREAM,
                    "Audio Stream",
                    PortDataType::AudioStream,
                ),
            ],
            outputs: vec![PortMetadata::optional(
                Self::PORT_AUDIO,
                "Audio",
                PortDataType::Audio,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(AudioOutputTask::descriptor));

#[async_trait]
impl Task for AudioOutputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get optional audio input (base64-encoded)
        let input_key = ContextKeys::input(&self.task_id, Self::PORT_AUDIO);
        let audio: Option<String> = context.get(&input_key).await;

        if let Some(ref audio_data) = audio {
            // Store output in context (for chaining)
            let output_key = ContextKeys::output(&self.task_id, Self::PORT_AUDIO);
            context.set(&output_key, audio_data.clone()).await;

            // Store stream data for frontend display
            let stream_key = ContextKeys::stream(&self.task_id, Self::PORT_AUDIO);
            context
                .set(
                    &stream_key,
                    serde_json::json!({
                        "type": "audio",
                        "content": audio_data
                    }),
                )
                .await;

            log::debug!(
                "AudioOutputTask {}: outputting audio ({} bytes)",
                self.task_id,
                audio_data.len()
            );
        } else {
            log::debug!(
                "AudioOutputTask {}: no audio input (stream-only mode)",
                self.task_id,
            );
        }

        Ok(TaskResult::new(audio, NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = AudioOutputTask::new("my_output");
        assert_eq!(task.id(), "my_output");
    }

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = AudioOutputTask::descriptor();
        assert_eq!(meta.node_type, "audio-output");
    }

    #[tokio::test]
    async fn test_audio_output_stores_in_context() {
        // Arrange
        let task = AudioOutputTask::new("test_output");
        let context = Context::new();
        let input_key = ContextKeys::input("test_output", "audio");
        context
            .set(&input_key, "UklGRiQAAABXQVZFZm10".to_string())
            .await;

        // Act
        let result = task.run(context.clone()).await.unwrap();

        // Assert
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response.as_deref(), Some("UklGRiQAAABXQVZFZm10"));

        let output_key = ContextKeys::output("test_output", "audio");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some("UklGRiQAAABXQVZFZm10".to_string()));

        let stream_key = ContextKeys::stream("test_output", "audio");
        let stream: Option<serde_json::Value> = context.get(&stream_key).await;
        assert!(stream.is_some());
        let stream_data = stream.unwrap();
        assert_eq!(stream_data["type"], "audio");
        assert_eq!(stream_data["content"], "UklGRiQAAABXQVZFZm10");
    }

    #[tokio::test]
    async fn test_missing_audio_ok() {
        // Arrange
        let task = AudioOutputTask::new("test_output");
        let context = Context::new();

        // Act — run without setting audio (stream-only mode)
        let result = task.run(context).await.unwrap();

        // Assert
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response, None);
    }
}

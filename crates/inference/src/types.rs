//! Common types for inference operations

use serde::{Deserialize, Serialize};

/// LLM server status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMStatus {
    pub ready: bool,
    pub mode: String,
    pub url: Option<String>,
}

/// Chat message with multimodal content support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Vec<ContentPart>,
}

/// Content part - text or image
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlData },
}

/// Image URL data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrlData {
    pub url: String,
}

/// Chat completion request (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub choices: Vec<StreamChoice>,
}

/// Streaming choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

/// Delta content in streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
}

/// Server event for streaming
#[derive(Clone, Serialize)]
pub struct StreamEvent {
    pub content: Option<String>,
    pub done: bool,
    pub error: Option<String>,
}

/// Server operating mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerModeInfo {
    /// Current mode type
    pub mode: String,
    /// Whether the server is ready
    pub ready: bool,
    /// URL if connected to external server
    pub url: Option<String>,
    /// Model path if using sidecar
    pub model_path: Option<String>,
    /// Whether in embedding mode (sidecar only)
    pub is_embedding_mode: bool,
}

/// Type identifier for masked prompts in JSON context values
pub const MASKED_PROMPT_TYPE: &str = "masked_prompt";

/// A segment of a masked prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSegment {
    /// The text content of this segment
    pub text: String,
    /// Whether this segment should be regenerated (true) or preserved as anchor (false)
    pub masked: bool,
}

/// A masked prompt consisting of segments, some masked for regeneration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskedPrompt {
    /// Type identifier, always "masked_prompt"
    #[serde(rename = "type")]
    pub prompt_type: String,
    /// The prompt segments
    pub segments: Vec<PromptSegment>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_segment_serde_roundtrip() {
        let segment = PromptSegment {
            text: "Hello world".to_string(),
            masked: true,
        };
        let json = serde_json::to_string(&segment).unwrap();
        let decoded: PromptSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.text, "Hello world");
        assert!(decoded.masked);
    }

    #[test]
    fn test_masked_prompt_serde_roundtrip() {
        let prompt = MaskedPrompt {
            prompt_type: MASKED_PROMPT_TYPE.to_string(),
            segments: vec![
                PromptSegment {
                    text: "The quick ".to_string(),
                    masked: false,
                },
                PromptSegment {
                    text: "brown fox".to_string(),
                    masked: true,
                },
                PromptSegment {
                    text: " jumps over".to_string(),
                    masked: false,
                },
            ],
        };
        let json = serde_json::to_string(&prompt).unwrap();
        let decoded: MaskedPrompt = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.prompt_type, "masked_prompt");
        assert_eq!(decoded.segments.len(), 3);
        assert!(!decoded.segments[0].masked);
        assert!(decoded.segments[1].masked);
        assert!(!decoded.segments[2].masked);
        assert_eq!(decoded.segments[0].text, "The quick ");
        assert_eq!(decoded.segments[1].text, "brown fox");
        assert_eq!(decoded.segments[2].text, " jumps over");
    }
}

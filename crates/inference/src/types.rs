//! Common types for inference operations

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// Base64-encoded image payload used across image-generation requests/results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodedImage {
    /// Base64-encoded image bytes.
    pub data_base64: String,
    /// MIME type describing the encoded image payload.
    pub mime_type: String,
    /// Optional image width in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Optional image height in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// Text-to-image request contract used by diffusion-capable backends.
///
/// The request is append-only by design so later modes (img2img, inpaint)
/// can reuse the same contract with optional `init_image` / `mask_image`
/// fields instead of replacing it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageGenerationRequest {
    /// Backend-specific model identifier or path.
    pub model: String,
    /// Positive prompt describing the desired image.
    pub prompt: String,
    /// Optional negative prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    /// Target image width in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Target image height in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Number of denoising steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_inference_steps: Option<u32>,
    /// Guidance / CFG scale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance_scale: Option<f32>,
    /// Deterministic seed, if supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Optional scheduler identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<String>,
    /// Number of images to produce for the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_images_per_prompt: Option<u32>,
    /// Optional init image reserved for later img2img support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_image: Option<EncodedImage>,
    /// Optional mask image reserved for later inpaint support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_image: Option<EncodedImage>,
    /// Optional denoise strength reserved for later img2img/inpaint support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<f32>,
    /// Backend/model-specific append-only options.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra_options: Value,
}

/// Image-generation response contract returned by diffusion-capable backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageGenerationResult {
    /// Generated image payloads.
    pub images: Vec<EncodedImage>,
    /// Effective seed used by the backend, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_used: Option<u64>,
    /// Optional backend metadata such as scheduler or timings.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
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

    #[test]
    fn test_image_generation_request_serde_roundtrip_preserves_future_fields() {
        let request = ImageGenerationRequest {
            model: "Qwen/Qwen-Image".to_string(),
            prompt: "a red paper lantern in the rain".to_string(),
            negative_prompt: Some("blurry".to_string()),
            width: Some(1024),
            height: Some(1024),
            num_inference_steps: Some(30),
            guidance_scale: Some(4.0),
            seed: Some(42),
            scheduler: Some("flow_match_euler".to_string()),
            num_images_per_prompt: Some(1),
            init_image: None,
            mask_image: None,
            strength: None,
            extra_options: serde_json::json!({
                "true_cfg_scale": 4.0
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        let decoded: ImageGenerationRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.model, "Qwen/Qwen-Image");
        assert_eq!(decoded.seed, Some(42));
        assert_eq!(decoded.extra_options["true_cfg_scale"], serde_json::json!(4.0));
    }

    #[test]
    fn test_image_generation_result_serde_roundtrip() {
        let result = ImageGenerationResult {
            images: vec![EncodedImage {
                data_base64: "aGVsbG8=".to_string(),
                mime_type: "image/png".to_string(),
                width: Some(512),
                height: Some(512),
            }],
            seed_used: Some(42),
            metadata: serde_json::json!({
                "scheduler": "flow_match_euler"
            }),
        };

        let json = serde_json::to_string(&result).unwrap();
        let decoded: ImageGenerationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.seed_used, Some(42));
        assert_eq!(decoded.images.len(), 1);
        assert_eq!(decoded.images[0].mime_type, "image/png");
        assert_eq!(decoded.metadata["scheduler"], serde_json::json!("flow_match_euler"));
    }
}

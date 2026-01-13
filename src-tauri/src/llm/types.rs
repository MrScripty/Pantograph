use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerConfig {
    #[serde(rename = "external")]
    External { url: String },
    #[serde(rename = "sidecar")]
    Sidecar {
        model_path: String,
        mmproj_path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMStatus {
    pub ready: bool,
    pub mode: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Vec<ContentPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrlData {
    pub url: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct StreamEvent {
    pub content: Option<String>,
    pub done: bool,
    pub error: Option<String>,
}

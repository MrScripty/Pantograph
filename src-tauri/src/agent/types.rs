use serde::{Deserialize, Serialize};

/// Bounds of a drawing on the canvas
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawingBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub width: f64,
    pub height: f64,
    pub center_x: f64,
    pub center_y: f64,
}

/// Information about an existing component in the UI tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub bounds: ComponentBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Request from frontend to run the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub prompt: String,
    pub image_base64: String,
    pub drawing_bounds: Option<DrawingBounds>,
    pub component_tree: Vec<ComponentInfo>,
    pub target_element_id: Option<String>,
}

/// Response from the agent to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub file_changes: Vec<FileChange>,
    pub component_updates: Vec<ComponentUpdate>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub action: FileAction,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileAction {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentUpdate {
    pub id: String,
    pub path: String,
    pub position: Position,
    pub size: Size,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// Events streamed to the frontend during agent execution
#[derive(Debug, Clone, Serialize)]
pub struct AgentEvent {
    pub event_type: AgentEventType,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventType {
    ToolCall,
    ToolResult,
    Content,
    ComponentCreated,
    Done,
    Error,
}

/// Tailwind color palette structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailwindColors {
    pub colors: std::collections::HashMap<String, Vec<String>>,
}

/// Template information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub path: String,
}

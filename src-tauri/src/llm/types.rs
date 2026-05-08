use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct StreamEvent {
    pub content: Option<String>,
    pub done: bool,
    pub error: Option<String>,
}

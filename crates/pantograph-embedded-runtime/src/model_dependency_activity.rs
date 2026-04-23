use std::sync::Arc;

use chrono::Utc;
use node_engine::ModelDependencyRequest;
use serde::Serialize;

pub type DependencyActivityEmitter = Arc<dyn Fn(DependencyActivityEvent) + Send + Sync>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DependencyActivityEvent {
    pub timestamp: String,
    pub node_type: String,
    pub model_path: String,
    pub phase: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct DependencyActivityContext {
    node_type: String,
    model_path: String,
}

impl DependencyActivityContext {
    pub(super) fn from_request(request: &ModelDependencyRequest) -> Self {
        Self {
            node_type: request.node_type.trim().to_string(),
            model_path: request.model_path.trim().to_string(),
        }
    }

    pub(super) fn unknown() -> Self {
        Self {
            node_type: "unknown".to_string(),
            model_path: "unknown".to_string(),
        }
    }
}

pub(super) fn emit_activity_with_emitter(
    emitter: Option<&DependencyActivityEmitter>,
    context: &DependencyActivityContext,
    phase: &str,
    message: impl Into<String>,
    binding_id: Option<&str>,
    requirement_name: Option<&str>,
    stream: Option<&str>,
) {
    let Some(emitter) = emitter else {
        return;
    };
    emitter(DependencyActivityEvent {
        timestamp: Utc::now().to_rfc3339(),
        node_type: context.node_type.clone(),
        model_path: context.model_path.clone(),
        phase: phase.to_string(),
        message: message.into(),
        binding_id: binding_id.map(|v| v.to_string()),
        requirement_name: requirement_name.map(|v| v.to_string()),
        stream: stream.map(|v| v.to_string()),
    });
}

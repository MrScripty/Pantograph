use std::sync::Arc;

use serde::Serialize;

pub type DependencyActivityEmitter = Arc<dyn Fn(DependencyActivityEvent) + Send + Sync>;

#[derive(Default)]
pub struct DependencyActivityHub {
    emitter: std::sync::RwLock<Option<DependencyActivityEmitter>>,
}

impl DependencyActivityHub {
    pub fn set_emitter(&self, emitter: DependencyActivityEmitter) {
        if let Ok(mut slot) = self.emitter.write() {
            *slot = Some(emitter);
        }
    }

    pub fn emit(&self, event: DependencyActivityEvent) {
        let emitter = self.emitter.read().ok().and_then(|slot| slot.clone());
        if let Some(emitter) = emitter {
            emitter(event);
        }
    }

    pub fn emitter(self: &Arc<Self>) -> DependencyActivityEmitter {
        let hub = self.clone();
        Arc::new(move |event| hub.emit(event))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DependencyActivityEvent {
    pub timestamp: String,
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_node_id: Option<String>,
    pub phase: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
}

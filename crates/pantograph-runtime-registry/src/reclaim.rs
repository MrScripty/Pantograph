use crate::{RuntimeRegistryStatus, RuntimeRetentionReason};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeReclaimAction {
    None,
    StopProducer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeReclaimDisposition {
    pub runtime_id: String,
    pub action: RuntimeReclaimAction,
    pub reason: RuntimeRetentionReason,
    pub status: RuntimeRegistryStatus,
}

impl RuntimeReclaimDisposition {
    pub fn no_action(
        runtime_id: impl Into<String>,
        reason: RuntimeRetentionReason,
        status: RuntimeRegistryStatus,
    ) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            action: RuntimeReclaimAction::None,
            reason,
            status,
        }
    }

    pub fn stop_producer(runtime_id: impl Into<String>, status: RuntimeRegistryStatus) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            action: RuntimeReclaimAction::StopProducer,
            reason: RuntimeRetentionReason::Evictable,
            status,
        }
    }
}

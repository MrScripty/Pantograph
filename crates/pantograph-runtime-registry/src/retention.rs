use crate::RuntimeRegistryStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRetentionDecision {
    Retain,
    Evict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRetentionReason {
    ActiveReservations,
    KeepAliveReservation,
    PinnedModel,
    Status(RuntimeRegistryStatus),
    Evictable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeRetentionDisposition {
    pub runtime_id: String,
    pub decision: RuntimeRetentionDecision,
    pub reason: RuntimeRetentionReason,
}

impl RuntimeRetentionDisposition {
    pub fn retain(runtime_id: impl Into<String>, reason: RuntimeRetentionReason) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            decision: RuntimeRetentionDecision::Retain,
            reason,
        }
    }

    pub fn evict(runtime_id: impl Into<String>) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            decision: RuntimeRetentionDecision::Evict,
            reason: RuntimeRetentionReason::Evictable,
        }
    }
}

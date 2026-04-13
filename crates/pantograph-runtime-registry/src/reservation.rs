use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeReservationRequest {
    pub runtime_id: String,
    pub workflow_id: String,
    #[serde(default)]
    pub usage_profile: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub pin_runtime: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeReservationLease {
    pub reservation_id: u64,
    pub runtime_id: String,
    pub workflow_id: String,
    #[serde(default)]
    pub usage_profile: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    pub pin_runtime: bool,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeReservationRecord {
    pub reservation_id: u64,
    pub runtime_id: String,
    pub workflow_id: String,
    pub usage_profile: Option<String>,
    pub model_id: Option<String>,
    pub pin_runtime: bool,
    pub created_at_ms: u64,
}

impl RuntimeReservationRecord {
    pub(crate) fn into_lease(self) -> RuntimeReservationLease {
        RuntimeReservationLease {
            reservation_id: self.reservation_id,
            runtime_id: self.runtime_id,
            workflow_id: self.workflow_id,
            usage_profile: self.usage_profile,
            model_id: self.model_id,
            pin_runtime: self.pin_runtime,
            created_at_ms: self.created_at_ms,
        }
    }
}

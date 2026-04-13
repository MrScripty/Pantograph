use serde::{Deserialize, Serialize};

use super::reservation::RuntimeReservationLease;
use super::state::{RuntimeModelResidencyRecord, RuntimeRegistryStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeRegistrySnapshot {
    pub generated_at_ms: u64,
    pub runtimes: Vec<RuntimeRegistryRuntimeSnapshot>,
    pub reservations: Vec<RuntimeReservationLease>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeRegistryRuntimeSnapshot {
    pub runtime_id: String,
    pub display_name: String,
    pub backend_keys: Vec<String>,
    pub status: RuntimeRegistryStatus,
    #[serde(default)]
    pub runtime_instance_id: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    pub last_transition_at_ms: u64,
    pub active_reservation_ids: Vec<u64>,
    pub models: Vec<RuntimeModelResidencyRecord>,
}

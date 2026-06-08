use serde::{Deserialize, Serialize};

use crate::admission::{RuntimeAdmissionBudget, RuntimeReservationResourceClaim};
use crate::reservation::RuntimeReservationLease;
use crate::state::{RuntimeModelResidencyRecord, RuntimeRegistryStatus};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_residency_key: Option<String>,
    pub status: RuntimeRegistryStatus,
    #[serde(default)]
    pub runtime_instance_id: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    pub last_transition_at_ms: u64,
    pub active_reservation_ids: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_reservation_claims: Vec<RuntimeActiveReservationClaim>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_budget: Option<RuntimeAdmissionBudget>,
    pub models: Vec<RuntimeModelResidencyRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeActiveReservationClaim {
    pub reservation_id: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<RuntimeReservationResourceClaim>,
}

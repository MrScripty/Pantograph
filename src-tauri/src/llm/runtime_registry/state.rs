use std::collections::{BTreeMap, BTreeSet};

use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRegistryStatus {
    Stopped,
    Warming,
    Ready,
    Busy,
    Unhealthy,
    Stopping,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeModelResidencyRecord {
    pub model_id: String,
    #[serde(default)]
    pub usage_profile: Option<String>,
    pub pinned: bool,
    pub loaded_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRegistryRecord {
    pub runtime_id: String,
    pub display_name: String,
    pub backend_keys: BTreeSet<String>,
    pub status: RuntimeRegistryStatus,
    pub runtime_instance_id: Option<String>,
    pub last_error: Option<String>,
    pub last_transition_at_ms: u64,
    pub active_reservations: BTreeSet<u64>,
    pub models: BTreeMap<String, RuntimeModelResidencyRecord>,
}

impl RuntimeRegistryRecord {
    pub fn new(runtime_id: &str, display_name: &str, now_ms: u64) -> Self {
        Self {
            runtime_id: canonical_runtime_id(runtime_id),
            display_name: display_name.trim().to_string(),
            backend_keys: BTreeSet::new(),
            status: RuntimeRegistryStatus::Stopped,
            runtime_instance_id: None,
            last_error: None,
            last_transition_at_ms: now_ms,
            active_reservations: BTreeSet::new(),
            models: BTreeMap::new(),
        }
    }

    pub fn set_backend_keys<I>(&mut self, backend_keys: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.backend_keys = backend_keys
            .into_iter()
            .map(|backend_key| canonical_runtime_backend_key(&backend_key))
            .filter(|backend_key| !backend_key.is_empty())
            .collect();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeTransition {
    WarmupStarted { runtime_instance_id: Option<String> },
    Ready { runtime_instance_id: Option<String> },
    Busy { runtime_instance_id: Option<String> },
    Unhealthy { message: String },
    Failed { message: String },
    StopRequested,
    Stopped,
}

impl RuntimeTransition {
    pub fn target_status(&self) -> RuntimeRegistryStatus {
        match self {
            Self::WarmupStarted { .. } => RuntimeRegistryStatus::Warming,
            Self::Ready { .. } => RuntimeRegistryStatus::Ready,
            Self::Busy { .. } => RuntimeRegistryStatus::Busy,
            Self::Unhealthy { .. } => RuntimeRegistryStatus::Unhealthy,
            Self::Failed { .. } => RuntimeRegistryStatus::Failed,
            Self::StopRequested => RuntimeRegistryStatus::Stopping,
            Self::Stopped => RuntimeRegistryStatus::Stopped,
        }
    }

    pub fn can_transition_from(&self, status: RuntimeRegistryStatus) -> bool {
        match (status, self.target_status()) {
            (RuntimeRegistryStatus::Stopped, RuntimeRegistryStatus::Warming)
            | (RuntimeRegistryStatus::Stopped, RuntimeRegistryStatus::Ready)
            | (RuntimeRegistryStatus::Stopped, RuntimeRegistryStatus::Failed)
            | (RuntimeRegistryStatus::Warming, RuntimeRegistryStatus::Ready)
            | (RuntimeRegistryStatus::Warming, RuntimeRegistryStatus::Failed)
            | (RuntimeRegistryStatus::Warming, RuntimeRegistryStatus::Stopping)
            | (RuntimeRegistryStatus::Ready, RuntimeRegistryStatus::Busy)
            | (RuntimeRegistryStatus::Ready, RuntimeRegistryStatus::Unhealthy)
            | (RuntimeRegistryStatus::Ready, RuntimeRegistryStatus::Stopping)
            | (RuntimeRegistryStatus::Busy, RuntimeRegistryStatus::Ready)
            | (RuntimeRegistryStatus::Busy, RuntimeRegistryStatus::Unhealthy)
            | (RuntimeRegistryStatus::Busy, RuntimeRegistryStatus::Stopping)
            | (RuntimeRegistryStatus::Unhealthy, RuntimeRegistryStatus::Warming)
            | (RuntimeRegistryStatus::Unhealthy, RuntimeRegistryStatus::Ready)
            | (RuntimeRegistryStatus::Unhealthy, RuntimeRegistryStatus::Failed)
            | (RuntimeRegistryStatus::Unhealthy, RuntimeRegistryStatus::Stopping)
            | (RuntimeRegistryStatus::Stopping, RuntimeRegistryStatus::Stopped)
            | (RuntimeRegistryStatus::Stopping, RuntimeRegistryStatus::Failed)
            | (RuntimeRegistryStatus::Failed, RuntimeRegistryStatus::Stopped)
            | (RuntimeRegistryStatus::Failed, RuntimeRegistryStatus::Warming) => true,
            (current, target) => current == target,
        }
    }
}

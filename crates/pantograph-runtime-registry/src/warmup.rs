use crate::RuntimeRegistryStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeWarmupDecision {
    StartRuntime,
    ReuseLoadedRuntime,
    WaitForTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeWarmupReason {
    NoLoadedInstance,
    RecoveryRequired,
    LoadedInstanceReady,
    LoadedInstanceBusy,
    WarmupInProgress,
    StopInProgress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeWarmupDisposition {
    pub runtime_id: String,
    pub decision: RuntimeWarmupDecision,
    pub reason: RuntimeWarmupReason,
    pub status: RuntimeRegistryStatus,
    #[serde(default)]
    pub runtime_instance_id: Option<String>,
}

impl RuntimeWarmupDisposition {
    pub fn start(
        runtime_id: impl Into<String>,
        reason: RuntimeWarmupReason,
        status: RuntimeRegistryStatus,
    ) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            decision: RuntimeWarmupDecision::StartRuntime,
            reason,
            status,
            runtime_instance_id: None,
        }
    }

    pub fn reuse(
        runtime_id: impl Into<String>,
        reason: RuntimeWarmupReason,
        status: RuntimeRegistryStatus,
        runtime_instance_id: Option<String>,
    ) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            decision: RuntimeWarmupDecision::ReuseLoadedRuntime,
            reason,
            status,
            runtime_instance_id,
        }
    }

    pub fn wait(
        runtime_id: impl Into<String>,
        reason: RuntimeWarmupReason,
        status: RuntimeRegistryStatus,
        runtime_instance_id: Option<String>,
    ) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            decision: RuntimeWarmupDecision::WaitForTransition,
            reason,
            status,
            runtime_instance_id,
        }
    }
}

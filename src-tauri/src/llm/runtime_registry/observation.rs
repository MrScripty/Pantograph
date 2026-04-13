use pantograph_runtime_identity::canonical_runtime_id;

use super::state::RuntimeRegistryStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeObservation {
    pub runtime_id: String,
    pub display_name: String,
    pub backend_keys: Vec<String>,
    pub model_id: Option<String>,
    pub status: RuntimeRegistryStatus,
    pub runtime_instance_id: Option<String>,
    pub last_error: Option<String>,
}

impl RuntimeObservation {
    fn from_active_runtime(mode_info: &inference::ServerModeInfo) -> Self {
        let snapshot = mode_info
            .active_runtime
            .as_ref()
            .cloned()
            .unwrap_or_default();
        let runtime_id = snapshot
            .runtime_id
            .clone()
            .or_else(|| mode_info.backend_key.clone())
            .or_else(|| mode_info.backend_name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let display_name = mode_info
            .backend_name
            .clone()
            .unwrap_or_else(|| runtime_id.clone());

        Self {
            runtime_id,
            display_name,
            backend_keys: mode_info.backend_key.clone().into_iter().collect(),
            model_id: mode_info.active_model_target.clone(),
            status: observed_status(&snapshot),
            runtime_instance_id: snapshot.runtime_instance_id,
            last_error: snapshot.last_error,
        }
    }

    fn from_embedding_runtime(mode_info: &inference::ServerModeInfo) -> Option<Self> {
        let snapshot = mode_info.embedding_runtime.as_ref()?.clone();
        let runtime_id = snapshot
            .runtime_id
            .clone()
            .unwrap_or_else(|| "llama.cpp.embedding".to_string());

        Some(Self {
            runtime_id,
            display_name: "Dedicated embedding runtime".to_string(),
            backend_keys: mode_info.backend_key.clone().into_iter().collect(),
            model_id: mode_info.embedding_model_target.clone(),
            status: observed_status(&snapshot),
            runtime_instance_id: snapshot.runtime_instance_id,
            last_error: snapshot.last_error,
        })
    }

    pub(super) fn runtime_id(&self) -> String {
        canonical_runtime_id(&self.runtime_id)
    }
}

pub(super) fn observations_from_mode_info(
    mode_info: &inference::ServerModeInfo,
) -> Vec<RuntimeObservation> {
    let mut observations = vec![RuntimeObservation::from_active_runtime(mode_info)];

    if let Some(observation) = RuntimeObservation::from_embedding_runtime(mode_info) {
        observations.push(observation);
    }

    observations
}

fn observed_status(snapshot: &inference::RuntimeLifecycleSnapshot) -> RuntimeRegistryStatus {
    if snapshot.active {
        if snapshot.warmup_started_at_ms.is_some() && snapshot.warmup_completed_at_ms.is_none() {
            return RuntimeRegistryStatus::Warming;
        }

        return RuntimeRegistryStatus::Ready;
    }

    if snapshot.last_error.is_some() {
        return RuntimeRegistryStatus::Failed;
    }

    RuntimeRegistryStatus::Stopped
}

//! Backend-owned runtime-registry translation helpers.
//!
//! This module converts gateway lifecycle facts and producer-specific runtime
//! snapshots into `pantograph_runtime_registry::RuntimeObservation` values so
//! host adapters do not own registry-observation mapping logic.

use pantograph_runtime_identity::{
    canonical_runtime_id, runtime_backend_key_aliases, runtime_display_name,
};
use pantograph_runtime_registry::{
    observed_runtime_status_from_lifecycle, RuntimeObservation, RuntimeRegistry,
    RuntimeRegistryRuntimeSnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRuntimeDescriptor {
    pub runtime_id: String,
    pub display_name: String,
    pub backend_keys: Vec<String>,
    pub runtime_instance_id: Option<String>,
}

pub fn active_runtime_descriptor(mode_info: &inference::ServerModeInfo) -> ActiveRuntimeDescriptor {
    let runtime_id = mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_id.clone())
        .or_else(|| mode_info.backend_key.clone())
        .or_else(|| mode_info.backend_name.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let display_name = mode_info
        .backend_name
        .clone()
        .unwrap_or_else(|| runtime_id.clone());
    let backend_keys = mode_info
        .backend_key
        .clone()
        .into_iter()
        .collect::<Vec<_>>();
    let runtime_instance_id = mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_instance_id.clone());

    ActiveRuntimeDescriptor {
        runtime_id,
        display_name,
        backend_keys,
        runtime_instance_id,
    }
}

pub fn active_runtime_observation(
    mode_info: &inference::ServerModeInfo,
    include_stopped: bool,
) -> Option<RuntimeObservation> {
    let snapshot = mode_info
        .active_runtime
        .as_ref()
        .cloned()
        .unwrap_or_default();
    let descriptor = active_runtime_descriptor(mode_info);
    let status = observed_runtime_status_from_lifecycle(
        snapshot.active,
        snapshot.warmup_started_at_ms,
        snapshot.warmup_completed_at_ms,
        snapshot.last_error.is_some(),
    );

    if !include_stopped
        && matches!(
            status,
            pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
        )
        && snapshot.last_error.is_none()
    {
        return None;
    }

    Some(RuntimeObservation {
        runtime_id: descriptor.runtime_id,
        display_name: descriptor.display_name,
        backend_keys: descriptor.backend_keys,
        model_id: mode_info.active_model_target.clone(),
        status,
        runtime_instance_id: snapshot.runtime_instance_id,
        last_error: snapshot.last_error,
    })
}

pub fn embedding_runtime_observation(
    mode_info: &inference::ServerModeInfo,
) -> Option<RuntimeObservation> {
    let snapshot = mode_info.embedding_runtime.as_ref()?.clone();
    let runtime_id = snapshot
        .runtime_id
        .clone()
        .unwrap_or_else(|| "llama.cpp.embedding".to_string());

    Some(RuntimeObservation {
        runtime_id,
        display_name: "Dedicated embedding runtime".to_string(),
        backend_keys: mode_info.backend_key.clone().into_iter().collect(),
        model_id: mode_info.embedding_model_target.clone(),
        status: observed_runtime_status_from_lifecycle(
            snapshot.active,
            snapshot.warmup_started_at_ms,
            snapshot.warmup_completed_at_ms,
            snapshot.last_error.is_some(),
        ),
        runtime_instance_id: snapshot.runtime_instance_id,
        last_error: snapshot.last_error,
    })
}

pub fn observations_from_mode_info(
    mode_info: &inference::ServerModeInfo,
) -> Vec<RuntimeObservation> {
    let mut observations = Vec::new();

    if let Some(observation) = active_runtime_observation(mode_info, true) {
        observations.push(observation);
    }

    if let Some(observation) = embedding_runtime_observation(mode_info) {
        observations.push(observation);
    }

    observations
}

pub fn reconcile_runtime_registry_mode_info(
    registry: &RuntimeRegistry,
    mode_info: &inference::ServerModeInfo,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    registry.observe_runtimes(observations_from_mode_info(mode_info))
}

pub fn reconcile_active_runtime_mode_info(
    registry: &RuntimeRegistry,
    mode_info: &inference::ServerModeInfo,
    include_stopped: bool,
) -> Option<RuntimeRegistryRuntimeSnapshot> {
    active_runtime_observation(mode_info, include_stopped)
        .map(|observation| registry.observe_runtime(observation))
}

pub fn reconcile_runtime_registry_snapshot_override(
    registry: &RuntimeRegistry,
    snapshot: &inference::RuntimeLifecycleSnapshot,
    model_id: Option<&str>,
) -> Option<RuntimeRegistryRuntimeSnapshot> {
    let runtime_id = snapshot
        .runtime_id
        .as_deref()
        .map(canonical_runtime_id)
        .filter(|runtime_id| !runtime_id.is_empty())?;
    let display_name = runtime_display_name(&runtime_id)
        .unwrap_or(runtime_id.as_str())
        .to_string();
    let backend_keys = runtime_backend_key_aliases(&display_name, &runtime_id);

    Some(registry.observe_runtime(RuntimeObservation {
        runtime_id,
        display_name: display_name.clone(),
        backend_keys,
        model_id: model_id.map(ToOwned::to_owned),
        status: observed_runtime_status_from_lifecycle(
            snapshot.active,
            snapshot.warmup_started_at_ms,
            snapshot.warmup_completed_at_ms,
            snapshot.last_error.is_some(),
        ),
        runtime_instance_id: snapshot.runtime_instance_id.clone(),
        last_error: snapshot.last_error.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_runtime_registry::RuntimeRegistryStatus;

    #[test]
    fn reconcile_mode_info_registers_active_and_embedding_runtimes() {
        let registry = RuntimeRegistry::new();

        let snapshots = reconcile_runtime_registry_mode_info(
            &registry,
            &inference::ServerModeInfo {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                mode: "sidecar_inference".to_string(),
                ready: true,
                url: Some("http://127.0.0.1:11434".to_string()),
                model_path: None,
                is_embedding_mode: false,
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-1".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp.embedding".to_string()),
                    runtime_instance_id: Some("llama-embed-1".to_string()),
                    warmup_started_at_ms: Some(11),
                    warmup_completed_at_ms: None,
                    warmup_duration_ms: None,
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
            },
        );

        assert_eq!(snapshots.len(), 2);
        let active_runtime = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "llama_cpp")
            .expect("active runtime snapshot");
        assert_eq!(active_runtime.status, RuntimeRegistryStatus::Ready);
        assert_eq!(active_runtime.models[0].model_id, "/models/qwen.gguf");

        let embedding_runtime = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(embedding_runtime.status, RuntimeRegistryStatus::Warming);
        assert_eq!(embedding_runtime.models[0].model_id, "/models/embed.gguf");
    }

    #[test]
    fn reconcile_mode_info_stops_unobserved_runtimes_without_reservations() {
        let registry = RuntimeRegistry::new();

        reconcile_runtime_registry_mode_info(
            &registry,
            &inference::ServerModeInfo {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                mode: "sidecar_inference".to_string(),
                ready: true,
                url: None,
                model_path: None,
                is_embedding_mode: false,
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-1".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            },
        );

        let snapshots = reconcile_runtime_registry_mode_info(
            &registry,
            &inference::ServerModeInfo {
                backend_name: Some("ollama".to_string()),
                backend_key: Some("ollama".to_string()),
                mode: "external".to_string(),
                ready: true,
                url: Some("http://127.0.0.1:11434".to_string()),
                model_path: None,
                is_embedding_mode: false,
                active_model_target: Some("llava:13b".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("ollama".to_string()),
                    runtime_instance_id: Some("ollama-1".to_string()),
                    warmup_started_at_ms: Some(30),
                    warmup_completed_at_ms: Some(35),
                    warmup_duration_ms: Some(5),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            },
        );

        assert_eq!(snapshots.len(), 2);
        let llama = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "llama_cpp")
            .expect("llama snapshot");
        assert_eq!(llama.status, RuntimeRegistryStatus::Stopped);
        assert!(llama.models.is_empty());

        let ollama = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "ollama")
            .expect("ollama snapshot");
        assert_eq!(ollama.status, RuntimeRegistryStatus::Ready);
        assert_eq!(ollama.models[0].model_id, "llava:13b");
    }

    #[test]
    fn reconcile_snapshot_override_adds_python_runtime_without_stopping_gateway_runtime() {
        let registry = RuntimeRegistry::new();

        reconcile_runtime_registry_mode_info(
            &registry,
            &inference::ServerModeInfo {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                mode: "sidecar_inference".to_string(),
                ready: true,
                url: Some("http://127.0.0.1:11434".to_string()),
                model_path: None,
                is_embedding_mode: false,
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-1".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            },
        );

        let pytorch = reconcile_runtime_registry_snapshot_override(
            &registry,
            &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("PyTorch".to_string()),
                runtime_instance_id: Some("python-runtime:pytorch:venv_torch".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            },
            Some("/models/demo"),
        )
        .expect("python snapshot should be reconciled");

        assert_eq!(pytorch.runtime_id, "pytorch");
        assert_eq!(pytorch.display_name, "PyTorch (Python sidecar)");
        assert_eq!(pytorch.status, RuntimeRegistryStatus::Ready);
        assert_eq!(pytorch.models[0].model_id, "/models/demo");

        let snapshot = registry.snapshot();
        let llama = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama_cpp")
            .expect("gateway runtime should remain in registry");
        assert_eq!(llama.status, RuntimeRegistryStatus::Ready);

        let pytorch = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "pytorch")
            .expect("python runtime should be present in registry");
        assert!(pytorch.backend_keys.contains(&"pytorch".to_string()));
    }
}

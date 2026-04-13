//! Tauri adapter for the backend-owned runtime registry.
//!
//! This module translates backend lifecycle facts into
//! `pantograph_runtime_registry::RuntimeObservation` values and then delegates
//! all registry state ownership to the backend crate.

use pantograph_runtime_registry::{
    RuntimeObservation, RuntimeRegistryRuntimeSnapshot, RuntimeRegistryStatus,
};

pub use pantograph_runtime_registry::{RuntimeRegistry, SharedRuntimeRegistry};

pub fn reconcile_runtime_registry_mode_info(
    registry: &RuntimeRegistry,
    mode_info: &inference::ServerModeInfo,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    registry.observe_runtimes(observations_from_mode_info(mode_info))
}

fn observations_from_mode_info(mode_info: &inference::ServerModeInfo) -> Vec<RuntimeObservation> {
    let mut observations = vec![active_runtime_observation(mode_info)];

    if let Some(observation) = embedding_runtime_observation(mode_info) {
        observations.push(observation);
    }

    observations
}

fn active_runtime_observation(mode_info: &inference::ServerModeInfo) -> RuntimeObservation {
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

    RuntimeObservation {
        runtime_id,
        display_name,
        backend_keys: mode_info.backend_key.clone().into_iter().collect(),
        model_id: mode_info.active_model_target.clone(),
        status: observed_status(&snapshot),
        runtime_instance_id: snapshot.runtime_instance_id,
        last_error: snapshot.last_error,
    }
}

fn embedding_runtime_observation(
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
        status: observed_status(&snapshot),
        runtime_instance_id: snapshot.runtime_instance_id,
        last_error: snapshot.last_error,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

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
                    lifecycle_decision_reason: Some("started_runtime".to_string()),
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
                    lifecycle_decision_reason: Some("started_embedding_runtime".to_string()),
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
                    lifecycle_decision_reason: Some("started_runtime".to_string()),
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
                    lifecycle_decision_reason: Some("connected_external_runtime".to_string()),
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
}

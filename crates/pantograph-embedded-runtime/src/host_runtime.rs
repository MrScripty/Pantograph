//! Backend-owned host runtime snapshot contract.
//!
//! Hosts may observe richer producer facts than the core inference gateway can
//! represent on its own. This module defines the backend-owned snapshot shape
//! that hosted runtime composition should consume so adapters do not have to
//! pass framework-specific runtime structs across the boundary.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostRuntimeModeSnapshot {
    pub backend_name: Option<String>,
    pub backend_key: Option<String>,
    pub active_model_target: Option<String>,
    pub embedding_model_target: Option<String>,
    pub active_runtime: Option<inference::RuntimeLifecycleSnapshot>,
    pub embedding_runtime: Option<inference::RuntimeLifecycleSnapshot>,
}

impl HostRuntimeModeSnapshot {
    pub fn from_mode_info(mode_info: &inference::ServerModeInfo) -> Self {
        Self {
            backend_name: mode_info.backend_name.clone(),
            backend_key: mode_info.backend_key.clone(),
            active_model_target: mode_info.active_model_target.clone(),
            embedding_model_target: mode_info.embedding_model_target.clone(),
            active_runtime: mode_info.active_runtime.clone(),
            embedding_runtime: mode_info.embedding_runtime.clone(),
        }
    }
}

impl From<&inference::ServerModeInfo> for HostRuntimeModeSnapshot {
    fn from(value: &inference::ServerModeInfo) -> Self {
        Self::from_mode_info(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_runtime_mode_snapshot_copies_runtime_facts_from_mode_info() {
        let snapshot = HostRuntimeModeSnapshot::from_mode_info(&inference::ServerModeInfo {
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
                runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
                warmup_started_at_ms: Some(11),
                warmup_completed_at_ms: Some(19),
                warmup_duration_ms: Some(8),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
        });

        assert_eq!(snapshot.backend_name.as_deref(), Some("llama.cpp"));
        assert_eq!(snapshot.backend_key.as_deref(), Some("llama_cpp"));
        assert_eq!(
            snapshot.active_model_target.as_deref(),
            Some("/models/qwen.gguf")
        );
        assert_eq!(
            snapshot.embedding_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
        assert_eq!(
            snapshot
                .embedding_runtime
                .as_ref()
                .and_then(|runtime| runtime.runtime_id.as_deref()),
            Some("llama.cpp.embedding")
        );
    }
}

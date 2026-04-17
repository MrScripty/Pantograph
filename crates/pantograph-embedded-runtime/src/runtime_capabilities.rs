//! Backend-owned runtime capability helpers.
//!
//! Hosts may contribute runtime capabilities from producer-specific runtime
//! facts, but the capability-shape mapping belongs in backend Rust rather than
//! adapter modules.

use crate::HostRuntimeModeSnapshot;
use pantograph_runtime_identity::{
    backend_key_aliases, canonical_runtime_backend_key, canonical_runtime_id,
    runtime_backend_key_aliases, runtime_display_name,
};
use pantograph_workflow_service::{
    WorkflowRuntimeCapability, WorkflowRuntimeInstallState, WorkflowRuntimeSourceKind,
};

pub fn dedicated_embedding_runtime_capabilities(
    snapshot: Option<inference::RuntimeLifecycleSnapshot>,
) -> Vec<WorkflowRuntimeCapability> {
    let Some(snapshot) = snapshot else {
        return Vec::new();
    };

    vec![WorkflowRuntimeCapability {
        runtime_id: snapshot
            .runtime_id
            .as_deref()
            .map(canonical_runtime_id)
            .unwrap_or_else(|| "llama.cpp.embedding".to_string()),
        display_name: "Dedicated embedding runtime".to_string(),
        install_state: WorkflowRuntimeInstallState::Installed,
        available: snapshot.active,
        configured: snapshot.active,
        can_install: false,
        can_remove: false,
        source_kind: WorkflowRuntimeSourceKind::Host,
        selected: false,
        supports_external_connection: false,
        backend_keys: backend_key_aliases("llama.cpp", "llama_cpp"),
        missing_files: Vec::new(),
        unavailable_reason: snapshot.last_error,
    }]
}

pub fn runtime_capabilities_from_mode_info(
    mode_info: &HostRuntimeModeSnapshot,
) -> Vec<WorkflowRuntimeCapability> {
    let mut capabilities = Vec::new();
    capabilities.extend(dedicated_embedding_runtime_capabilities(
        mode_info.embedding_runtime.clone(),
    ));
    capabilities
}

pub fn python_runtime_capabilities(
    executable_probe: Result<std::path::PathBuf, String>,
    selected_backend_key: &str,
) -> Vec<WorkflowRuntimeCapability> {
    let (available, unavailable_reason) = match executable_probe {
        Ok(_) => (true, None),
        Err(reason) => (false, Some(reason)),
    };
    [
        (
            runtime_display_name("pytorch").unwrap_or("PyTorch (Python sidecar)"),
            "pytorch",
        ),
        (
            runtime_display_name("diffusers").unwrap_or("Diffusers (Python sidecar)"),
            "diffusers",
        ),
        (
            runtime_display_name("onnx-runtime").unwrap_or("ONNX Runtime (Python sidecar)"),
            "onnx-runtime",
        ),
        (
            runtime_display_name("stable_audio").unwrap_or("Stable Audio (Python sidecar)"),
            "stable_audio",
        ),
    ]
    .into_iter()
    .map(|(display_name, runtime_id)| {
        let backend_keys = runtime_backend_key_aliases(display_name, runtime_id);
        WorkflowRuntimeCapability {
            runtime_id: runtime_id.to_string(),
            display_name: display_name.to_string(),
            install_state: if available {
                WorkflowRuntimeInstallState::SystemProvided
            } else {
                WorkflowRuntimeInstallState::Missing
            },
            available,
            configured: available,
            can_install: false,
            can_remove: false,
            source_kind: WorkflowRuntimeSourceKind::System,
            selected: runtime_matches_backend(&backend_keys, selected_backend_key),
            supports_external_connection: false,
            backend_keys,
            missing_files: Vec::new(),
            unavailable_reason: unavailable_reason.clone(),
        }
    })
    .collect()
}

fn runtime_matches_backend(backend_keys: &[String], selected_backend_key: &str) -> bool {
    backend_keys
        .iter()
        .any(|backend_key| canonical_runtime_backend_key(backend_key) == selected_backend_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedicated_embedding_runtime_capability_reports_dedicated_runtime() {
        let capabilities =
            dedicated_embedding_runtime_capabilities(Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                runtime_instance_id: Some("llama-cpp-embedding-9".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }));

        assert_eq!(capabilities.len(), 1);
        let capability = &capabilities[0];
        assert_eq!(capability.runtime_id, "llama.cpp.embedding");
        assert_eq!(capability.display_name, "Dedicated embedding runtime");
        assert_eq!(capability.source_kind, WorkflowRuntimeSourceKind::Host);
        assert!(capability.available);
        assert!(capability.configured);
        assert!(!capability.selected);
        assert!(capability.backend_keys.contains(&"llama_cpp".to_string()));
        assert!(capability.backend_keys.contains(&"llamacpp".to_string()));
    }

    #[test]
    fn dedicated_embedding_runtime_capability_omits_missing_snapshot() {
        assert!(dedicated_embedding_runtime_capabilities(None).is_empty());
    }

    #[test]
    fn runtime_capabilities_from_mode_info_collects_embedding_runtime_capability() {
        let capabilities = runtime_capabilities_from_mode_info(&HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: None,
            embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                runtime_instance_id: Some("llama-cpp-embedding-4".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(15),
                warmup_duration_ms: Some(5),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
        });

        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].runtime_id, "llama.cpp.embedding");
        assert!(capabilities[0].available);
    }

    #[test]
    fn python_runtime_capabilities_report_python_backed_engines() {
        let capabilities = python_runtime_capabilities(
            Ok(std::path::PathBuf::from("/usr/bin/python3")),
            "pytorch",
        );

        assert_eq!(capabilities.len(), 4);

        let pytorch = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "pytorch")
            .expect("pytorch capability");
        assert!(pytorch.available);
        assert!(pytorch.configured);
        assert_eq!(pytorch.source_kind, WorkflowRuntimeSourceKind::System);
        assert!(pytorch.selected);
        assert!(!pytorch.supports_external_connection);
        assert!(pytorch.backend_keys.contains(&"pytorch".to_string()));
        assert!(pytorch.backend_keys.contains(&"torch".to_string()));

        let diffusion = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "diffusers")
            .expect("diffusers capability");
        assert!(diffusion.backend_keys.contains(&"diffusers".to_string()));

        let onnx = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "onnx-runtime")
            .expect("onnx capability");
        assert!(onnx.backend_keys.contains(&"onnx-runtime".to_string()));

        let stable_audio = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "stable_audio")
            .expect("stable audio capability");
        assert!(stable_audio
            .backend_keys
            .contains(&"stable_audio".to_string()));
    }

    #[test]
    fn python_runtime_capabilities_keep_unavailable_reason() {
        let capabilities = python_runtime_capabilities(
            Err("python executable is not configured".to_string()),
            "llama_cpp",
        );

        assert_eq!(capabilities.len(), 4);
        for capability in capabilities {
            assert!(!capability.available);
            assert!(!capability.configured);
            assert_eq!(capability.source_kind, WorkflowRuntimeSourceKind::System);
            assert!(!capability.selected);
            assert_eq!(
                capability.unavailable_reason.as_deref(),
                Some("python executable is not configured")
            );
        }
    }
}

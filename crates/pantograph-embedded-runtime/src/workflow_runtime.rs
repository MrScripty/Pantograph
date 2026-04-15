use std::collections::HashMap;

use crate::HostRuntimeModeSnapshot;
use node_engine::{NodeEngineError, WorkflowExecutor};
use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};
use pantograph_workflow_service::{WorkflowCapabilitiesResponse, WorkflowTraceRuntimeMetrics};

#[derive(Debug, Clone)]
pub struct RuntimeDiagnosticsProjection {
    pub active_runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub runtime_model_target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeEventProjection {
    pub active_runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    pub embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub active_model_target: Option<String>,
    pub embedding_model_target: Option<String>,
}

pub async fn sync_embedding_emit_metadata_flags(
    executor: &mut WorkflowExecutor,
) -> Result<(), NodeEngineError> {
    let snapshot = executor.get_graph_snapshot().await;
    let mut counts = HashMap::<String, u32>::new();
    for edge in &snapshot.edges {
        let key = format!("{}:{}", edge.source, edge.source_handle);
        counts
            .entry(key)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    for node in &snapshot.nodes {
        if node.node_type != "embedding" {
            continue;
        }
        let key = format!("{}:metadata", node.id);
        let emit_metadata = counts.get(&key).copied().unwrap_or(0) > 0;
        let mut data = node.data.clone();
        match data {
            serde_json::Value::Object(ref mut map) => {
                map.insert(
                    "emit_metadata".to_string(),
                    serde_json::json!(emit_metadata),
                );
            }
            _ => {
                data = serde_json::json!({ "emit_metadata": emit_metadata });
            }
        }
        executor.update_node_data(&node.id, data).await?;
    }

    Ok(())
}

pub fn trace_runtime_metrics(
    snapshot: &inference::RuntimeLifecycleSnapshot,
    model_target: Option<&str>,
) -> WorkflowTraceRuntimeMetrics {
    let observed_runtime_ids = observed_runtime_ids(snapshot, &[]);
    let runtime_id = observed_runtime_ids.first().cloned();
    WorkflowTraceRuntimeMetrics {
        runtime_id: runtime_id.clone(),
        observed_runtime_ids,
        runtime_instance_id: snapshot.runtime_instance_id.clone(),
        model_target: model_target.map(ToOwned::to_owned),
        warmup_started_at_ms: snapshot.warmup_started_at_ms,
        warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
        warmup_duration_ms: snapshot.warmup_duration_ms,
        runtime_reused: snapshot.runtime_reused,
        lifecycle_decision_reason: snapshot.normalized_lifecycle_decision_reason(),
    }
}

pub fn normalized_runtime_lifecycle_snapshot(
    snapshot: &inference::RuntimeLifecycleSnapshot,
) -> inference::RuntimeLifecycleSnapshot {
    inference::RuntimeLifecycleSnapshot {
        runtime_id: snapshot
            .runtime_id
            .as_deref()
            .map(canonical_runtime_id)
            .filter(|runtime_id| !runtime_id.is_empty()),
        runtime_instance_id: snapshot.runtime_instance_id.clone(),
        warmup_started_at_ms: snapshot.warmup_started_at_ms,
        warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
        warmup_duration_ms: snapshot.warmup_duration_ms,
        runtime_reused: snapshot.runtime_reused,
        lifecycle_decision_reason: snapshot.normalized_lifecycle_decision_reason(),
        active: snapshot.active,
        last_error: snapshot.last_error.clone(),
    }
}

pub fn trace_runtime_metrics_with_observed_runtime_ids(
    snapshot: &inference::RuntimeLifecycleSnapshot,
    model_target: Option<&str>,
    additional_observed_runtime_ids: &[String],
) -> WorkflowTraceRuntimeMetrics {
    let observed_runtime_ids = observed_runtime_ids(snapshot, additional_observed_runtime_ids);
    let runtime_id = observed_runtime_ids.first().cloned();
    WorkflowTraceRuntimeMetrics {
        runtime_id,
        observed_runtime_ids,
        runtime_instance_id: snapshot.runtime_instance_id.clone(),
        model_target: model_target.map(ToOwned::to_owned),
        warmup_started_at_ms: snapshot.warmup_started_at_ms,
        warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
        warmup_duration_ms: snapshot.warmup_duration_ms,
        runtime_reused: snapshot.runtime_reused,
        lifecycle_decision_reason: snapshot.normalized_lifecycle_decision_reason(),
    }
}

fn observed_runtime_ids(
    snapshot: &inference::RuntimeLifecycleSnapshot,
    additional_observed_runtime_ids: &[String],
) -> Vec<String> {
    let mut observed_runtime_ids = snapshot
        .runtime_id
        .as_deref()
        .map(canonical_runtime_id)
        .filter(|runtime_id| !runtime_id.is_empty())
        .into_iter()
        .collect::<Vec<_>>();
    for runtime_id in additional_observed_runtime_ids {
        let runtime_id = canonical_runtime_id(runtime_id);
        if runtime_id.is_empty() || observed_runtime_ids.contains(&runtime_id) {
            continue;
        }
        observed_runtime_ids.push(runtime_id);
    }
    observed_runtime_ids
}

pub fn resolve_runtime_model_target(
    mode_info: &HostRuntimeModeSnapshot,
    snapshot: &inference::RuntimeLifecycleSnapshot,
) -> Option<String> {
    if snapshot
        .runtime_id
        .as_deref()
        .map(canonical_runtime_id)
        .as_deref()
        == Some("llama.cpp.embedding")
    {
        return mode_info.embedding_model_target.clone();
    }
    mode_info.active_model_target.clone()
}

fn runtime_capability_matches_required_backend(
    capability: &pantograph_workflow_service::WorkflowRuntimeCapability,
    required_backend_key: &str,
) -> bool {
    let required_backend_key = canonical_runtime_backend_key(required_backend_key);
    canonical_runtime_backend_key(&capability.runtime_id) == required_backend_key
        || capability
            .backend_keys
            .iter()
            .any(|backend_key| canonical_runtime_backend_key(backend_key) == required_backend_key)
}

pub fn capability_runtime_lifecycle_snapshot(
    capabilities: Option<&WorkflowCapabilitiesResponse>,
) -> Option<inference::RuntimeLifecycleSnapshot> {
    let capabilities = capabilities?;
    let selected_runtime = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.selected)
        .or_else(|| {
            if capabilities.runtime_requirements.required_backends.len() != 1 {
                return None;
            }

            let required_backend_key = &capabilities.runtime_requirements.required_backends[0];
            capabilities
                .runtime_capabilities
                .iter()
                .filter(|capability| {
                    runtime_capability_matches_required_backend(capability, required_backend_key)
                })
                .max_by(|left, right| {
                    (
                        left.available && left.configured,
                        left.available,
                        left.configured,
                    )
                        .cmp(&(
                            right.available && right.configured,
                            right.available,
                            right.configured,
                        ))
                        .then_with(|| left.runtime_id.cmp(&right.runtime_id))
                })
        })?;
    let lifecycle_decision_reason = if selected_runtime.selected {
        "selected_runtime_reported"
    } else {
        "required_runtime_reported"
    };

    Some(inference::RuntimeLifecycleSnapshot {
        runtime_id: Some(canonical_runtime_id(&selected_runtime.runtime_id))
            .filter(|runtime_id| !runtime_id.is_empty()),
        runtime_instance_id: None,
        warmup_started_at_ms: None,
        warmup_completed_at_ms: None,
        warmup_duration_ms: None,
        runtime_reused: None,
        lifecycle_decision_reason: Some(lifecycle_decision_reason.to_string()),
        active: false,
        last_error: selected_runtime
            .unavailable_reason
            .clone()
            .filter(|reason| !reason.trim().is_empty()),
    })
}

pub fn build_runtime_diagnostics_projection(
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_snapshot: &inference::RuntimeLifecycleSnapshot,
    gateway_mode_info: &HostRuntimeModeSnapshot,
    runtime_model_target_override: Option<&str>,
) -> RuntimeDiagnosticsProjection {
    let projection = build_runtime_event_projection(
        None,
        None,
        None,
        None,
        None,
        runtime_snapshot_override,
        gateway_snapshot,
        None,
        gateway_mode_info,
        runtime_model_target_override,
    );

    RuntimeDiagnosticsProjection {
        active_runtime_snapshot: projection.active_runtime_snapshot,
        trace_runtime_metrics: projection.trace_runtime_metrics,
        runtime_model_target: projection.active_model_target,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_runtime_event_projection(
    stored_active_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_active_model_target: Option<&str>,
    stored_embedding_model_target: Option<&str>,
    stored_trace_runtime_metrics: Option<WorkflowTraceRuntimeMetrics>,
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_snapshot: &inference::RuntimeLifecycleSnapshot,
    embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_mode_info: &HostRuntimeModeSnapshot,
    runtime_model_target_override: Option<&str>,
) -> RuntimeEventProjection {
    let active_runtime_snapshot = runtime_snapshot_override
        .cloned()
        .or_else(|| stored_active_runtime_snapshot.cloned())
        .unwrap_or_else(|| gateway_snapshot.clone());
    let embedding_runtime_snapshot = stored_embedding_runtime_snapshot
        .cloned()
        .or_else(|| embedding_runtime_snapshot.cloned());
    let active_model_target = runtime_model_target_override
        .map(ToOwned::to_owned)
        .or_else(|| stored_active_model_target.map(ToOwned::to_owned))
        .or_else(|| resolve_runtime_model_target(gateway_mode_info, &active_runtime_snapshot));
    let embedding_model_target = stored_embedding_model_target
        .map(ToOwned::to_owned)
        .or_else(|| gateway_mode_info.embedding_model_target.clone());
    let trace_runtime_metrics = stored_trace_runtime_metrics.unwrap_or_else(|| {
        trace_runtime_metrics(&active_runtime_snapshot, active_model_target.as_deref())
    });

    RuntimeEventProjection {
        active_runtime_snapshot,
        embedding_runtime_snapshot,
        trace_runtime_metrics,
        active_model_target,
        embedding_model_target,
    }
}

#[cfg(test)]
mod tests {
    use crate::HostRuntimeModeSnapshot;
    use pantograph_workflow_service::WorkflowCapabilitiesResponse;

    use super::{
        build_runtime_diagnostics_projection, build_runtime_event_projection,
        capability_runtime_lifecycle_snapshot, normalized_runtime_lifecycle_snapshot,
        resolve_runtime_model_target, trace_runtime_metrics,
        trace_runtime_metrics_with_observed_runtime_ids,
    };

    #[test]
    fn trace_runtime_metrics_keeps_canonical_backend_lifecycle_reason() {
        let metrics = trace_runtime_metrics(
            &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("pytorch".to_string()),
                runtime_instance_id: Some("pytorch-1".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("runtime_reused".to_string()),
                active: true,
                last_error: None,
            },
            Some("/models/demo"),
        );

        assert_eq!(
            metrics.lifecycle_decision_reason.as_deref(),
            Some("runtime_reused")
        );
        assert_eq!(metrics.model_target.as_deref(), Some("/models/demo"));
    }

    #[test]
    fn trace_runtime_metrics_normalizes_known_runtime_aliases() {
        let metrics = trace_runtime_metrics(
            &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-cpp-1".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            },
            Some("/models/main.gguf"),
        );

        assert_eq!(metrics.runtime_id.as_deref(), Some("llama_cpp"));
        assert_eq!(metrics.observed_runtime_ids, vec!["llama_cpp".to_string()]);
    }

    #[test]
    fn normalized_runtime_lifecycle_snapshot_canonicalizes_runtime_aliases() {
        let snapshot = normalized_runtime_lifecycle_snapshot(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("pytorch-1".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        });

        assert_eq!(snapshot.runtime_id.as_deref(), Some("pytorch"));
        assert_eq!(
            snapshot.lifecycle_decision_reason.as_deref(),
            Some("runtime_reused")
        );
    }

    #[test]
    fn normalized_runtime_lifecycle_snapshot_infers_backend_owned_default_reason() {
        let snapshot = normalized_runtime_lifecycle_snapshot(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-1".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: None,
            active: true,
            last_error: None,
        });

        assert_eq!(snapshot.runtime_id.as_deref(), Some("llama_cpp"));
        assert_eq!(
            snapshot.lifecycle_decision_reason.as_deref(),
            Some("runtime_ready")
        );
    }

    #[test]
    fn trace_runtime_metrics_with_observed_runtime_ids_preserves_all_runtime_ids() {
        let metrics = trace_runtime_metrics_with_observed_runtime_ids(
            &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("onnxruntime".to_string()),
                runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: false,
                last_error: None,
            },
            Some("/tmp/model.onnx"),
            &[
                "diffusers".to_string(),
                "onnx-runtime".to_string(),
                "diffusers".to_string(),
            ],
        );

        assert_eq!(metrics.runtime_id.as_deref(), Some("onnx-runtime"));
        assert_eq!(
            metrics.observed_runtime_ids,
            vec!["onnx-runtime".to_string(), "diffusers".to_string()]
        );
        assert_eq!(metrics.model_target.as_deref(), Some("/tmp/model.onnx"));
    }

    #[test]
    fn resolve_runtime_model_target_prefers_embedding_target_for_embedding_alias() {
        let mode_info = HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: None,
            embedding_runtime: None,
        };
        let snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama_cpp_embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        };

        assert_eq!(
            resolve_runtime_model_target(&mode_info, &snapshot).as_deref(),
            Some("/models/embed.gguf")
        );
    }

    #[test]
    fn build_runtime_diagnostics_projection_prefers_execution_snapshot_override() {
        let execution_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
            warmup_started_at_ms: Some(100),
            warmup_completed_at_ms: Some(110),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        };
        let restored_gateway_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-restore-9".to_string()),
            warmup_started_at_ms: Some(200),
            warmup_completed_at_ms: Some(240),
            warmup_duration_ms: Some(40),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        };
        let mode_info = HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/restore.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: None,
            embedding_runtime: None,
        };

        let projection = build_runtime_diagnostics_projection(
            Some(&execution_snapshot),
            &restored_gateway_snapshot,
            &mode_info,
            Some("/models/embed.gguf"),
        );

        assert_eq!(
            projection
                .active_runtime_snapshot
                .runtime_instance_id
                .as_deref(),
            Some("llama-cpp-embedding-2")
        );
        assert_eq!(
            projection.runtime_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
        assert_eq!(
            projection.trace_runtime_metrics.runtime_id.as_deref(),
            Some("llama.cpp.embedding")
        );
        assert_eq!(
            projection.trace_runtime_metrics.observed_runtime_ids,
            vec!["llama.cpp.embedding".to_string()]
        );
        assert_eq!(
            projection
                .trace_runtime_metrics
                .lifecycle_decision_reason
                .as_deref(),
            Some("runtime_reused")
        );
    }

    #[test]
    fn build_runtime_event_projection_prefers_stored_runtime_over_gateway_snapshot() {
        let stored_active_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("onnx-runtime".to_string()),
            runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: false,
            last_error: None,
        };
        let stored_embedding_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-3".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        };
        let gateway_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-main-1".to_string()),
            warmup_started_at_ms: Some(1),
            warmup_completed_at_ms: Some(2),
            warmup_duration_ms: Some(1),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        };
        let gateway_mode_info = HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: None,
            embedding_runtime: None,
        };

        let projection = build_runtime_event_projection(
            Some(&stored_active_runtime_snapshot),
            Some(&stored_embedding_runtime_snapshot),
            Some("/models/sidecar.onnx"),
            Some("/models/embed.gguf"),
            None,
            None,
            &gateway_snapshot,
            None,
            &gateway_mode_info,
            None,
        );

        assert_eq!(
            projection.active_runtime_snapshot.runtime_id.as_deref(),
            Some("onnx-runtime")
        );
        assert_eq!(
            projection
                .embedding_runtime_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.runtime_id.as_deref()),
            Some("llama.cpp.embedding")
        );
        assert_eq!(
            projection.active_model_target.as_deref(),
            Some("/models/sidecar.onnx")
        );
        assert_eq!(
            projection.embedding_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
        assert_eq!(
            projection.trace_runtime_metrics.runtime_id.as_deref(),
            Some("onnx-runtime")
        );
    }

    #[test]
    fn build_runtime_event_projection_preserves_live_embedding_snapshot_without_stored_override() {
        let gateway_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-main-2".to_string()),
            warmup_started_at_ms: Some(1),
            warmup_completed_at_ms: Some(3),
            warmup_duration_ms: Some(2),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        };
        let live_embedding_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama_cpp_embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-7".to_string()),
            warmup_started_at_ms: Some(4),
            warmup_completed_at_ms: Some(7),
            warmup_duration_ms: Some(3),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        };
        let gateway_mode_info = HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: None,
            embedding_runtime: None,
        };

        let projection = build_runtime_event_projection(
            None,
            None,
            None,
            None,
            None,
            None,
            &gateway_snapshot,
            Some(&live_embedding_runtime_snapshot),
            &gateway_mode_info,
            None,
        );

        assert_eq!(
            projection.active_model_target.as_deref(),
            Some("/models/main.gguf")
        );
        assert_eq!(
            projection.embedding_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
        assert_eq!(
            projection
                .embedding_runtime_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.runtime_instance_id.as_deref()),
            Some("llama-cpp-embedding-7")
        );
    }

    #[test]
    fn capability_runtime_lifecycle_snapshot_prefers_selected_runtime() {
        let snapshot = capability_runtime_lifecycle_snapshot(Some(&WorkflowCapabilitiesResponse {
            max_input_bindings: 4,
            max_output_targets: 2,
            max_value_bytes: 1000,
            runtime_requirements: pantograph_workflow_service::WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "high".to_string(),
                required_models: vec!["model-a".to_string()],
                required_backends: vec!["pytorch".to_string()],
                required_extensions: Vec::new(),
            },
            models: Vec::new(),
            runtime_capabilities: vec![pantograph_workflow_service::WorkflowRuntimeCapability {
                runtime_id: "PyTorch".to_string(),
                display_name: "PyTorch (Python sidecar)".to_string(),
                install_state:
                    pantograph_workflow_service::WorkflowRuntimeInstallState::SystemProvided,
                available: true,
                configured: true,
                can_install: false,
                can_remove: false,
                source_kind: pantograph_workflow_service::WorkflowRuntimeSourceKind::System,
                selected: true,
                supports_external_connection: false,
                backend_keys: vec!["torch".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        }))
        .expect("selected capability snapshot");

        assert_eq!(snapshot.runtime_id.as_deref(), Some("pytorch"));
        assert_eq!(
            snapshot.lifecycle_decision_reason.as_deref(),
            Some("selected_runtime_reported")
        );
        assert!(!snapshot.active);
    }

    #[test]
    fn capability_runtime_lifecycle_snapshot_matches_required_backend_alias() {
        let snapshot = capability_runtime_lifecycle_snapshot(Some(&WorkflowCapabilitiesResponse {
            max_input_bindings: 4,
            max_output_targets: 2,
            max_value_bytes: 1000,
            runtime_requirements: pantograph_workflow_service::WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "high".to_string(),
                required_models: vec!["model-a".to_string()],
                required_backends: vec!["onnxruntime".to_string()],
                required_extensions: Vec::new(),
            },
            models: Vec::new(),
            runtime_capabilities: vec![pantograph_workflow_service::WorkflowRuntimeCapability {
                runtime_id: "onnx-runtime".to_string(),
                display_name: "ONNX Runtime".to_string(),
                install_state:
                    pantograph_workflow_service::WorkflowRuntimeInstallState::SystemProvided,
                available: true,
                configured: true,
                can_install: false,
                can_remove: false,
                source_kind: pantograph_workflow_service::WorkflowRuntimeSourceKind::System,
                selected: false,
                supports_external_connection: false,
                backend_keys: vec!["ONNX Runtime".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        }))
        .expect("required backend snapshot");

        assert_eq!(snapshot.runtime_id.as_deref(), Some("onnx-runtime"));
        assert_eq!(
            snapshot.lifecycle_decision_reason.as_deref(),
            Some("required_runtime_reported")
        );
    }
}

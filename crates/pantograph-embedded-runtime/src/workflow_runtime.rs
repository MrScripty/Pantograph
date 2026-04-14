use std::collections::HashMap;

use node_engine::{NodeEngineError, WorkflowExecutor};
use pantograph_runtime_identity::canonical_runtime_id;
use pantograph_workflow_service::WorkflowTraceRuntimeMetrics;

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
    let runtime_id = snapshot
        .runtime_id
        .as_deref()
        .map(canonical_runtime_id)
        .filter(|runtime_id| !runtime_id.is_empty());
    WorkflowTraceRuntimeMetrics {
        runtime_id: runtime_id.clone(),
        observed_runtime_ids: runtime_id.into_iter().collect(),
        runtime_instance_id: snapshot.runtime_instance_id.clone(),
        model_target: model_target.map(ToOwned::to_owned),
        warmup_started_at_ms: snapshot.warmup_started_at_ms,
        warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
        warmup_duration_ms: snapshot.warmup_duration_ms,
        runtime_reused: snapshot.runtime_reused,
        lifecycle_decision_reason: snapshot.normalized_lifecycle_decision_reason(),
    }
}

pub fn resolve_runtime_model_target(
    mode_info: &inference::ServerModeInfo,
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

#[cfg(test)]
mod tests {
    use super::{resolve_runtime_model_target, trace_runtime_metrics};

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
    fn resolve_runtime_model_target_prefers_embedding_target_for_embedding_alias() {
        let mode_info = inference::ServerModeInfo {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            mode: "sidecar_inference".to_string(),
            ready: true,
            url: None,
            model_path: None,
            is_embedding_mode: false,
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
}

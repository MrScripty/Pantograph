use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};

use crate::workflow::WorkflowCapabilitiesResponse;

use super::types::WorkflowTraceRuntimeMetrics;

pub(super) fn apply_runtime_snapshot(
    trace: &mut super::store::WorkflowTraceRunState,
    runtime: &WorkflowTraceRuntimeMetrics,
    capabilities: Option<&WorkflowCapabilitiesResponse>,
    error: Option<&str>,
    _captured_at_ms: u64,
) {
    merge_runtime_metrics(&mut trace.runtime, runtime);

    if let Some(capabilities) = capabilities {
        if trace.runtime.runtime_id.is_none() {
            trace.runtime.runtime_id = infer_runtime_id(capabilities);
        }
        if trace.runtime.lifecycle_decision_reason.is_none() {
            trace.runtime.lifecycle_decision_reason =
                Some(runtime_lifecycle_reason(capabilities).to_string());
        }
        return;
    }

    if error.is_some() && trace.runtime.lifecycle_decision_reason.is_none() {
        trace.runtime.lifecycle_decision_reason = Some("capabilities_snapshot_failed".to_string());
    }
}

fn merge_runtime_metrics(
    target: &mut WorkflowTraceRuntimeMetrics,
    source: &WorkflowTraceRuntimeMetrics,
) {
    for runtime_id in &source.observed_runtime_ids {
        push_observed_runtime_id(&mut target.observed_runtime_ids, runtime_id);
    }
    if let Some(runtime_id) = source.runtime_id.clone() {
        if source.observed_runtime_ids.is_empty() {
            push_observed_runtime_id(&mut target.observed_runtime_ids, &runtime_id);
        }
        target.runtime_id = Some(runtime_id);
    }
    if let Some(runtime_instance_id) = source.runtime_instance_id.clone() {
        target.runtime_instance_id = Some(runtime_instance_id);
    }
    if let Some(model_target) = source.model_target.clone() {
        target.model_target = Some(model_target);
    }
    if let Some(warmup_started_at_ms) = source.warmup_started_at_ms {
        target.warmup_started_at_ms = Some(warmup_started_at_ms);
    }
    if let Some(warmup_completed_at_ms) = source.warmup_completed_at_ms {
        target.warmup_completed_at_ms = Some(warmup_completed_at_ms);
    }
    if let Some(warmup_duration_ms) = source.warmup_duration_ms {
        target.warmup_duration_ms = Some(warmup_duration_ms);
    }
    if let Some(runtime_reused) = source.runtime_reused {
        target.runtime_reused = Some(runtime_reused);
    }
    if let Some(lifecycle_decision_reason) = source.lifecycle_decision_reason.clone() {
        target.lifecycle_decision_reason = Some(lifecycle_decision_reason);
    }
}

fn push_observed_runtime_id(observed_runtime_ids: &mut Vec<String>, runtime_id: &str) {
    let runtime_id = runtime_id.trim();
    if runtime_id.is_empty()
        || observed_runtime_ids
            .iter()
            .any(|existing| existing == runtime_id)
    {
        return;
    }

    observed_runtime_ids.push(runtime_id.to_string());
}

pub(crate) fn infer_runtime_id(capabilities: &WorkflowCapabilitiesResponse) -> Option<String> {
    if let Some(selected_runtime) = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.selected)
    {
        return normalize_inferred_runtime_id(&selected_runtime.runtime_id);
    }

    if capabilities.runtime_requirements.required_backends.len() == 1 {
        if let Some(runtime_id) = find_runtime_id_for_required_backend(
            capabilities,
            &capabilities.runtime_requirements.required_backends[0],
        ) {
            return Some(runtime_id);
        }
        return capabilities
            .runtime_requirements
            .required_backends
            .first()
            .and_then(|runtime_id| normalize_inferred_runtime_id(runtime_id));
    }

    if capabilities.runtime_capabilities.len() == 1 {
        return capabilities
            .runtime_capabilities
            .first()
            .and_then(|capability| normalize_inferred_runtime_id(&capability.runtime_id));
    }

    None
}

fn find_runtime_id_for_required_backend(
    capabilities: &WorkflowCapabilitiesResponse,
    required_backend_key: &str,
) -> Option<String> {
    let required_backend_key = canonical_runtime_backend_key(required_backend_key);
    capabilities
        .runtime_capabilities
        .iter()
        .filter(|capability| runtime_capability_matches_backend(capability, &required_backend_key))
        .max_by(|left, right| {
            runtime_capability_match_rank(left)
                .cmp(&runtime_capability_match_rank(right))
                .then_with(|| left.runtime_id.cmp(&right.runtime_id))
        })
        .and_then(|capability| normalize_inferred_runtime_id(&capability.runtime_id))
}

fn normalize_inferred_runtime_id(runtime_id: &str) -> Option<String> {
    let runtime_id = canonical_runtime_id(runtime_id);
    if runtime_id.is_empty() {
        None
    } else {
        Some(runtime_id)
    }
}

fn runtime_capability_matches_backend(
    capability: &crate::workflow::WorkflowRuntimeCapability,
    required_backend_key: &str,
) -> bool {
    canonical_runtime_backend_key(&capability.runtime_id) == required_backend_key
        || capability
            .backend_keys
            .iter()
            .any(|backend_key| canonical_runtime_backend_key(backend_key) == required_backend_key)
}

fn runtime_capability_match_rank(
    capability: &crate::workflow::WorkflowRuntimeCapability,
) -> (bool, bool, bool) {
    (
        capability.selected,
        capability.available && capability.configured,
        capability.available,
    )
}

pub(crate) fn runtime_lifecycle_reason(
    capabilities: &WorkflowCapabilitiesResponse,
) -> &'static str {
    if capabilities
        .runtime_capabilities
        .iter()
        .any(|capability| capability.selected)
    {
        "selected_runtime_reported"
    } else if capabilities.runtime_requirements.required_backends.len() == 1
        && find_runtime_id_for_required_backend(
            capabilities,
            &capabilities.runtime_requirements.required_backends[0],
        )
        .is_some()
    {
        "required_runtime_reported"
    } else if capabilities
        .runtime_capabilities
        .iter()
        .any(|capability| capability.available && capability.configured)
    {
        "configured_runtime_available"
    } else if !capabilities
        .runtime_requirements
        .required_backends
        .is_empty()
    {
        "runtime_requirements_reported"
    } else {
        "capabilities_snapshot_available"
    }
}

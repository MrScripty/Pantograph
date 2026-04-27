use std::collections::HashSet;

use super::{
    WorkflowIdentity, WorkflowInputTarget, WorkflowIoResponse, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowServiceError,
};

pub(super) fn validate_timeout_ms(timeout_ms: Option<u64>) -> Result<(), WorkflowServiceError> {
    if matches!(timeout_ms, Some(0)) {
        return Err(WorkflowServiceError::InvalidRequest(
            "timeout_ms must be greater than zero when provided".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_workflow_id(workflow_id: &str) -> Result<(), WorkflowServiceError> {
    WorkflowIdentity::parse(workflow_id)
        .map(|_| ())
        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))
}

pub(crate) fn validate_workflow_semantic_version(
    workflow_semantic_version: &str,
) -> Result<(), WorkflowServiceError> {
    let mut parts = workflow_semantic_version.split('.');
    let valid = parts.next().is_some_and(is_numeric_semver_part)
        && parts.next().is_some_and(is_numeric_semver_part)
        && parts.next().is_some_and(is_numeric_semver_part)
        && parts.next().is_none();
    if !valid {
        return Err(WorkflowServiceError::InvalidRequest(
            "workflow_semantic_version must use major.minor.patch numeric semantic version"
                .to_string(),
        ));
    }
    Ok(())
}

fn is_numeric_semver_part(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
}

pub(super) fn validate_bindings(
    bindings: &[WorkflowPortBinding],
    field_name: &str,
) -> Result<(), WorkflowServiceError> {
    let mut seen = HashSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        if binding.node_id.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "{}.{}.node_id must be non-empty",
                field_name, index
            )));
        }
        if binding.port_id.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "{}.{}.port_id must be non-empty",
                field_name, index
            )));
        }
        if !seen.insert((binding.node_id.as_str(), binding.port_id.as_str())) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "{} has duplicate binding '{}.{}'",
                field_name, binding.node_id, binding.port_id
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_host_output_bindings(
    bindings: &[WorkflowPortBinding],
    field_name: &str,
) -> Result<(), WorkflowServiceError> {
    let mut seen = HashSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        if binding.node_id.trim().is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "{}.{}.node_id must be non-empty",
                field_name, index
            )));
        }
        if binding.port_id.trim().is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "{}.{}.port_id must be non-empty",
                field_name, index
            )));
        }
        if !seen.insert((binding.node_id.as_str(), binding.port_id.as_str())) {
            return Err(WorkflowServiceError::Internal(format!(
                "{} has duplicate binding '{}.{}'",
                field_name, binding.node_id, binding.port_id
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_output_targets(
    targets: &[WorkflowOutputTarget],
) -> Result<(), WorkflowServiceError> {
    let mut seen = HashSet::new();
    for (index, target) in targets.iter().enumerate() {
        if target.node_id.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "output_targets.{}.node_id must be non-empty",
                index
            )));
        }
        if target.port_id.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "output_targets.{}.port_id must be non-empty",
                index
            )));
        }
        if !seen.insert((target.node_id.as_str(), target.port_id.as_str())) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "output_targets has duplicate target '{}.{}'",
                target.node_id, target.port_id
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_output_targets_against_io(
    targets: &[WorkflowOutputTarget],
    io: &WorkflowIoResponse,
) -> Result<(), WorkflowServiceError> {
    let discovered_outputs = discovered_output_target_set(io);

    for (index, target) in targets.iter().enumerate() {
        let key = (target.node_id.clone(), target.port_id.clone());
        if !discovered_outputs.contains(&key) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "output_targets.{} references non-discoverable output '{}.{}'",
                index, target.node_id, target.port_id
            )));
        }
    }

    Ok(())
}

fn discovered_output_target_set(io: &WorkflowIoResponse) -> HashSet<(String, String)> {
    io.outputs
        .iter()
        .flat_map(|node| {
            node.ports
                .iter()
                .map(|port| (node.node_id.clone(), port.port_id.clone()))
        })
        .collect()
}

pub(super) fn collect_invalid_output_targets(
    targets: &[WorkflowOutputTarget],
    io: &WorkflowIoResponse,
) -> Vec<WorkflowOutputTarget> {
    let discovered_outputs = discovered_output_target_set(io);
    let mut invalid_targets = targets
        .iter()
        .filter(|target| {
            !discovered_outputs.contains(&(target.node_id.clone(), target.port_id.clone()))
        })
        .cloned()
        .collect::<Vec<_>>();
    invalid_targets.sort_by(|a, b| {
        a.node_id
            .cmp(&b.node_id)
            .then_with(|| a.port_id.cmp(&b.port_id))
    });
    invalid_targets
}

pub(super) fn derive_required_external_inputs(io: &WorkflowIoResponse) -> Vec<WorkflowInputTarget> {
    let mut required_inputs = io
        .inputs
        .iter()
        .flat_map(|node| {
            node.ports.iter().filter_map(move |port| {
                if port.required == Some(true) {
                    Some(WorkflowInputTarget {
                        node_id: node.node_id.clone(),
                        port_id: port.port_id.clone(),
                    })
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();
    required_inputs.sort_by(|a, b| {
        a.node_id
            .cmp(&b.node_id)
            .then_with(|| a.port_id.cmp(&b.port_id))
    });
    required_inputs
}

pub(super) fn validate_requested_outputs_produced(
    targets: &[WorkflowOutputTarget],
    outputs: &[WorkflowPortBinding],
) -> Result<(), WorkflowServiceError> {
    let produced = outputs
        .iter()
        .map(|binding| (binding.node_id.as_str(), binding.port_id.as_str()))
        .collect::<HashSet<_>>();

    for target in targets {
        let key = (target.node_id.as_str(), target.port_id.as_str());
        if !produced.contains(&key) {
            return Err(WorkflowServiceError::OutputNotProduced(format!(
                "requested output target '{}.{}' was not produced",
                target.node_id, target.port_id
            )));
        }
    }

    Ok(())
}

pub(super) fn validate_payload_size(
    binding: &WorkflowPortBinding,
    max_value_bytes: usize,
) -> Result<(), WorkflowServiceError> {
    let payload_bytes = serde_json::to_vec(&binding.value)
        .map_err(|e| WorkflowServiceError::InvalidRequest(format!("invalid binding value: {}", e)))?
        .len();

    if payload_bytes > max_value_bytes {
        return Err(WorkflowServiceError::CapabilityViolation(format!(
            "binding '{}.{}' payload size {} exceeds max_value_bytes {}",
            binding.node_id, binding.port_id, payload_bytes, max_value_bytes
        )));
    }

    Ok(())
}

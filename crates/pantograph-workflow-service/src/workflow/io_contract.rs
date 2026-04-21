use std::collections::HashSet;

use crate::capabilities;

use super::{WorkflowIoNode, WorkflowIoPort, WorkflowIoResponse, WorkflowServiceError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowIoDirection {
    Input,
    Output,
}

pub(super) fn validate_workflow_io(io: &WorkflowIoResponse) -> Result<(), WorkflowServiceError> {
    validate_workflow_io_nodes(&io.inputs, "inputs")?;
    validate_workflow_io_nodes(&io.outputs, "outputs")?;
    Ok(())
}

fn validate_workflow_io_nodes(
    nodes: &[WorkflowIoNode],
    field_name: &str,
) -> Result<(), WorkflowServiceError> {
    for (node_index, node) in nodes.iter().enumerate() {
        if node.node_id.trim().is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "{}.{}.node_id must be non-empty",
                field_name, node_index
            )));
        }
        if node.node_type.trim().is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "{}.{}.node_type must be non-empty",
                field_name, node_index
            )));
        }
        if node.ports.is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "{}.{}.ports must contain at least one entry for node '{}'",
                field_name, node_index, node.node_id
            )));
        }
        let mut seen_port_ids = HashSet::new();
        for (port_index, port) in node.ports.iter().enumerate() {
            if port.port_id.trim().is_empty() {
                return Err(WorkflowServiceError::Internal(format!(
                    "{}.{}.ports.{}.port_id must be non-empty",
                    field_name, node_index, port_index
                )));
            }
            if !seen_port_ids.insert(port.port_id.as_str()) {
                return Err(WorkflowServiceError::Internal(format!(
                    "{}.{}.ports has duplicate port_id '{}' for node '{}'",
                    field_name, node_index, port.port_id, node.node_id
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn derive_workflow_io(
    nodes: &[capabilities::StoredGraphNode],
) -> Result<WorkflowIoResponse, WorkflowServiceError> {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for node in nodes {
        let Some(direction) = classify_workflow_io_direction(node)? else {
            continue;
        };
        let entry = build_workflow_io_node(node, direction)?;
        match direction {
            WorkflowIoDirection::Input => inputs.push(entry),
            WorkflowIoDirection::Output => outputs.push(entry),
        }
    }

    inputs.sort_by(|a, b| a.node_id.cmp(&b.node_id));
    outputs.sort_by(|a, b| a.node_id.cmp(&b.node_id));

    Ok(WorkflowIoResponse { inputs, outputs })
}

fn classify_workflow_io_direction(
    node: &capabilities::StoredGraphNode,
) -> Result<Option<WorkflowIoDirection>, WorkflowServiceError> {
    let category = extract_nested_trimmed_str(node.data(), &["definition", "category"])
        .map(|v| v.to_ascii_lowercase());
    let Some(direction) = (match category.as_deref() {
        Some("input") => Some(WorkflowIoDirection::Input),
        Some("output") => Some(WorkflowIoDirection::Output),
        _ => None,
    }) else {
        return Ok(None);
    };

    let origin = extract_nested_trimmed_str(node.data(), &["definition", "io_binding_origin"])
        .map(|v| v.to_ascii_lowercase())
        .ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "workflow I/O schema error: node '{}' missing definition.io_binding_origin",
                node.id()
            ))
        })?;

    match origin.as_str() {
        "client_session" => Ok(Some(direction)),
        "integrated" => Ok(None),
        _ => Err(WorkflowServiceError::InvalidRequest(format!(
            "workflow I/O schema error: node '{}' has invalid definition.io_binding_origin '{}'",
            node.id(),
            origin
        ))),
    }
}

fn build_workflow_io_node(
    node: &capabilities::StoredGraphNode,
    direction: WorkflowIoDirection,
) -> Result<WorkflowIoNode, WorkflowServiceError> {
    let name = extract_nested_trimmed_str(node.data(), &["name"])
        .or_else(|| extract_nested_trimmed_str(node.data(), &["label"]))
        .or_else(|| extract_nested_trimmed_str(node.data(), &["definition", "label"]));
    let description = extract_nested_trimmed_str(node.data(), &["description"])
        .or_else(|| extract_nested_trimmed_str(node.data(), &["definition", "description"]));
    let ports = derive_workflow_io_ports(node, direction)?;

    Ok(WorkflowIoNode {
        node_id: node.id().to_string(),
        node_type: node.node_type().to_string(),
        name,
        description,
        ports,
    })
}

fn derive_workflow_io_ports(
    node: &capabilities::StoredGraphNode,
    direction: WorkflowIoDirection,
) -> Result<Vec<WorkflowIoPort>, WorkflowServiceError> {
    let key = match direction {
        WorkflowIoDirection::Input => "inputs",
        WorkflowIoDirection::Output => "outputs",
    };

    let mut ports = ports_from_definition(node, key)?;
    ports.sort_by(|a, b| a.port_id.cmp(&b.port_id));
    Ok(ports)
}

fn ports_from_definition(
    node: &capabilities::StoredGraphNode,
    key: &str,
) -> Result<Vec<WorkflowIoPort>, WorkflowServiceError> {
    let entries = node
        .data()
        .get("definition")
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "workflow I/O schema error: node '{}' missing definition.{}",
                node.id(),
                key
            ))
        })?;
    if entries.is_empty() {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "workflow I/O schema error: node '{}' has empty definition.{}",
            node.id(),
            key
        )));
    }

    let mut seen_port_ids = HashSet::new();
    let mut ports = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let port_id = entry
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "workflow I/O schema error: node '{}' {}.{} has invalid id",
                    node.id(),
                    key,
                    index
                ))
            })?
            .to_string();
        if !seen_port_ids.insert(port_id.clone()) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "workflow I/O schema error: node '{}' {} has duplicate port id '{}'",
                node.id(),
                key,
                port_id
            )));
        }

        let name = entry
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                entry
                    .get("label")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            });

        let description = entry
            .get("description")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        let data_type = entry
            .get("data_type")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        ports.push(WorkflowIoPort {
            port_id,
            name,
            description,
            data_type,
            required: entry.get("required").and_then(serde_json::Value::as_bool),
            multiple: entry.get("multiple").and_then(serde_json::Value::as_bool),
        })
    }

    Ok(ports)
}

fn extract_nested_trimmed_str(data: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut cursor = data;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::workflow::WorkflowServiceError;

pub const DEFAULT_MAX_BATCH_SIZE: usize = 128;
pub const DEFAULT_MAX_TEXT_LENGTH: usize = 32_768;

#[derive(Debug, Deserialize)]
pub struct StoredWorkflowFile {
    metadata: StoredWorkflowMetadata,
    graph: StoredWorkflowGraph,
}

impl StoredWorkflowFile {
    pub fn nodes(&self) -> &[StoredGraphNode] {
        &self.graph.nodes
    }
}

#[derive(Debug, Deserialize)]
struct StoredWorkflowMetadata {
    name: String,
}

#[derive(Debug, Deserialize)]
struct StoredWorkflowGraph {
    #[serde(default)]
    nodes: Vec<StoredGraphNode>,
    #[serde(default)]
    edges: Vec<StoredGraphEdge>,
}

#[derive(Debug, Deserialize)]
pub struct StoredGraphNode {
    id: String,
    node_type: String,
    #[serde(default)]
    data: serde_json::Value,
    #[serde(default)]
    position: StoredPosition,
}

#[derive(Debug, Deserialize, Default)]
struct StoredPosition {
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
}

#[derive(Debug, Deserialize)]
struct StoredGraphEdge {
    id: String,
    source: String,
    source_handle: String,
    target: String,
    target_handle: String,
}

pub fn default_workflow_roots(manifest_dir: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(current) = std::env::current_dir() {
        extend_ancestor_workflow_roots(&current, &mut roots);
    }
    extend_ancestor_workflow_roots(manifest_dir, &mut roots);

    roots
}

pub fn load_and_validate_workflow(
    workflow_id: &str,
    roots: &[PathBuf],
) -> Result<StoredWorkflowFile, WorkflowServiceError> {
    let workflow_path = find_workflow_file(workflow_id, roots).ok_or_else(|| {
        WorkflowServiceError::WorkflowNotFound(format!("workflow '{}' not found", workflow_id))
    })?;

    let raw = std::fs::read_to_string(&workflow_path)
        .map_err(|e| WorkflowServiceError::WorkflowNotFound(e.to_string()))?;
    let stored: StoredWorkflowFile = serde_json::from_str(&raw).map_err(|e| {
        WorkflowServiceError::CapabilityViolation(format!(
            "workflow '{}' has invalid file structure: {}",
            workflow_id, e
        ))
    })?;

    let graph = node_engine::WorkflowGraph {
        id: workflow_id.to_string(),
        name: stored.metadata.name.clone(),
        nodes: stored
            .graph
            .nodes
            .iter()
            .map(|n| node_engine::GraphNode {
                id: n.id.clone(),
                node_type: n.node_type.clone(),
                data: n.data.clone(),
                position: (n.position.x, n.position.y),
            })
            .collect(),
        edges: stored
            .graph
            .edges
            .iter()
            .map(|e| node_engine::GraphEdge {
                id: e.id.clone(),
                source: e.source.clone(),
                source_handle: e.source_handle.clone(),
                target: e.target.clone(),
                target_handle: e.target_handle.clone(),
            })
            .collect(),
        groups: Vec::new(),
    };

    let validation_errors = node_engine::validation::validate_workflow(&graph, None);
    if validation_errors.is_empty() {
        return Ok(stored);
    }

    let error_text = validation_errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    Err(WorkflowServiceError::CapabilityViolation(format!(
        "workflow '{}' failed graph validation: {}",
        workflow_id, error_text
    )))
}

pub fn extract_required_models(nodes: &[StoredGraphNode]) -> Vec<String> {
    let mut out = HashSet::new();
    for node in nodes {
        extract_model_ids_from_value(&node.data, &mut out);
    }
    let mut models = out.into_iter().collect::<Vec<_>>();
    models.sort();
    models
}

pub fn extract_required_backends(nodes: &[StoredGraphNode]) -> Vec<String> {
    let mut out = HashSet::new();
    for node in nodes {
        extract_backend_keys_from_value(&node.data, &mut out);
    }
    let mut backends = out.into_iter().collect::<Vec<_>>();
    backends.sort();
    backends
}

pub fn extract_required_extensions(nodes: &[StoredGraphNode], has_models: bool) -> Vec<String> {
    let mut out = vec!["inference_gateway".to_string()];
    if has_models {
        out.push("pumas_api".to_string());
    }
    if nodes.iter().any(|n| n.node_type == "dependency-environment") {
        out.push("model_dependency_resolver".to_string());
    }
    out.sort();
    out.dedup();
    out
}

pub fn estimate_memory_requirements(
    required_models: &[String],
    model_metadata: &HashMap<String, serde_json::Value>,
) -> (Option<u64>, Option<u64>, Option<u64>, Option<u64>, String) {
    if required_models.is_empty() {
        return (Some(0), Some(0), Some(0), Some(0), "exact_no_models".to_string());
    }

    const MB: u64 = 1024 * 1024;
    let mut sizes_mb = Vec::new();
    let mut found_count = 0_usize;

    for model_id in required_models {
        let Some(metadata) = model_metadata.get(model_id) else {
            continue;
        };
        if let Some(size_bytes) = extract_model_size_bytes(metadata) {
            let size_mb = (size_bytes.saturating_add(MB - 1)) / MB;
            sizes_mb.push(size_mb.max(1));
            found_count += 1;
        }
    }

    if sizes_mb.is_empty() {
        return (None, None, None, None, "unknown".to_string());
    }

    let peak = sizes_mb.iter().sum::<u64>();
    let min = sizes_mb.into_iter().max().unwrap_or(0);
    let confidence = if found_count == required_models.len() {
        "estimated_from_model_sizes"
    } else {
        "partial_model_sizes"
    };

    (
        Some(peak),
        Some(peak),
        Some(min),
        Some(min),
        confidence.to_string(),
    )
}

pub fn extract_model_size_bytes(metadata: &serde_json::Value) -> Option<u64> {
    metadata
        .get("size_bytes")
        .and_then(|v| v.as_u64())
        .or_else(|| metadata.get("sizeBytes").and_then(|v| v.as_u64()))
        .or_else(|| metadata.get("size").and_then(|v| v.as_u64()))
}

fn find_workflow_file(workflow_id: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    let stem = sanitize_workflow_stem(workflow_id)?;
    let filename = format!("{stem}.json");

    roots
        .iter()
        .map(|root| root.join(&filename))
        .find(|path| path.is_file())
}

fn extend_ancestor_workflow_roots(start: &Path, out: &mut Vec<PathBuf>) {
    for ancestor in start.ancestors() {
        let candidate = ancestor.join(".pantograph").join("workflows");
        if !out.iter().any(|existing| existing == &candidate) {
            out.push(candidate);
        }
    }
}

fn sanitize_workflow_stem(workflow_id: &str) -> Option<String> {
    let trimmed = workflow_id.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ')
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn extract_model_ids_from_value(value: &serde_json::Value, out: &mut HashSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if key.eq_ignore_ascii_case("model_id")
                    || key.eq_ignore_ascii_case("modelId")
                    || key.eq_ignore_ascii_case("dependency_requirements_id")
                    || key.eq_ignore_ascii_case("dependencyRequirementsId")
                {
                    if let Some(raw) = child.as_str() {
                        let trimmed = raw.trim();
                        if !trimmed.is_empty() && trimmed != "default" {
                            out.insert(trimmed.to_string());
                        }
                    }
                }
                extract_model_ids_from_value(child, out);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                extract_model_ids_from_value(child, out);
            }
        }
        _ => {}
    }
}

fn extract_backend_keys_from_value(value: &serde_json::Value, out: &mut HashSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if key.eq_ignore_ascii_case("backend_key") || key.eq_ignore_ascii_case("backendKey")
                {
                    if let Some(raw) = child.as_str() {
                        let trimmed = raw.trim();
                        if !trimmed.is_empty() {
                            out.insert(trimmed.to_string());
                        }
                    }
                }
                extract_backend_keys_from_value(child, out);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                extract_backend_keys_from_value(child, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_estimate_is_unknown_when_no_sizes_exist() {
        let required = vec!["a".to_string()];
        let metadata = HashMap::new();
        let (_, _, _, _, confidence) = estimate_memory_requirements(&required, &metadata);
        assert_eq!(confidence, "unknown");
    }

    #[test]
    fn memory_estimate_uses_model_sizes_when_present() {
        let required = vec!["a".to_string(), "b".to_string()];
        let mut metadata = HashMap::new();
        metadata.insert(
            "a".to_string(),
            serde_json::json!({ "size_bytes": 1024_u64 * 1024_u64 * 2_u64 }),
        );
        metadata.insert(
            "b".to_string(),
            serde_json::json!({ "sizeBytes": 1024_u64 * 1024_u64 }),
        );

        let (peak_vram, peak_ram, min_vram, min_ram, confidence) =
            estimate_memory_requirements(&required, &metadata);
        assert_eq!(peak_vram, Some(3));
        assert_eq!(peak_ram, Some(3));
        assert_eq!(min_vram, Some(2));
        assert_eq!(min_ram, Some(2));
        assert_eq!(confidence, "estimated_from_model_sizes");
    }
}

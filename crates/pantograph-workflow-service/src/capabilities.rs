use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use pantograph_runtime_identity::canonical_runtime_backend_key;
use serde::Deserialize;

use crate::workflow::WorkflowServiceError;

pub const DEFAULT_MAX_INPUT_BINDINGS: usize = 128;
pub const DEFAULT_MAX_OUTPUT_TARGETS: usize = 128;
pub const DEFAULT_MAX_VALUE_BYTES: usize = 32_768;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelUsage {
    pub model_id: String,
    pub node_ids: Vec<String>,
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct StoredWorkflowFile {
    metadata: StoredWorkflowMetadata,
    graph: StoredWorkflowGraph,
}

impl StoredWorkflowFile {
    pub fn nodes(&self) -> &[StoredGraphNode] {
        &self.graph.nodes
    }

    pub(crate) fn edges(&self) -> &[StoredGraphEdge] {
        &self.graph.edges
    }

    pub fn to_workflow_graph(&self, workflow_id: &str) -> node_engine::WorkflowGraph {
        node_engine::WorkflowGraph {
            id: workflow_id.to_string(),
            name: self.metadata.name.clone(),
            nodes: self
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
            edges: self
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
        }
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

impl StoredGraphNode {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn node_type(&self) -> &str {
        &self.node_type
    }

    pub fn data(&self) -> &serde_json::Value {
        &self.data
    }
}

#[derive(Debug, Deserialize, Default)]
struct StoredPosition {
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StoredGraphEdge {
    id: String,
    source: String,
    source_handle: String,
    target: String,
    target_handle: String,
}

const FNV64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV64_PRIME: u64 = 0x100000001b3;

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

    let graph = stored.to_workflow_graph(workflow_id);

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

pub fn workflow_graph_fingerprint(
    workflow_id: &str,
    roots: &[PathBuf],
) -> Result<String, WorkflowServiceError> {
    let stored = load_and_validate_workflow(workflow_id, roots)?;
    Ok(compute_graph_fingerprint(stored.nodes(), stored.edges()))
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

pub fn extract_model_usages(nodes: &[StoredGraphNode]) -> Vec<ModelUsage> {
    let mut usage_by_model: HashMap<String, (HashSet<String>, HashSet<String>)> = HashMap::new();

    for node in nodes {
        let mut model_ids = HashSet::new();
        extract_model_ids_from_value(node.data(), &mut model_ids);
        if model_ids.is_empty() {
            continue;
        }

        let roles = derive_roles(node.node_type(), node.data());
        for model_id in model_ids {
            let entry = usage_by_model
                .entry(model_id)
                .or_insert_with(|| (HashSet::new(), HashSet::new()));
            entry.0.insert(node.id().to_string());
            entry.1.extend(roles.iter().cloned());
        }
    }

    let mut models = usage_by_model
        .into_iter()
        .map(|(model_id, (node_ids, roles))| {
            let mut node_ids = node_ids.into_iter().collect::<Vec<_>>();
            let mut roles = roles.into_iter().collect::<Vec<_>>();
            node_ids.sort();
            roles.sort();
            ModelUsage {
                model_id,
                node_ids,
                roles,
            }
        })
        .collect::<Vec<_>>();
    models.sort_by(|a, b| a.model_id.cmp(&b.model_id));
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
    if nodes
        .iter()
        .any(|n| n.node_type == "dependency-environment")
    {
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
        return (
            Some(0),
            Some(0),
            Some(0),
            Some(0),
            "exact_no_models".to_string(),
        );
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

fn compute_graph_fingerprint(nodes: &[StoredGraphNode], edges: &[StoredGraphEdge]) -> String {
    let mut node_rows = nodes
        .iter()
        .map(|node| format!("{}|{}", node.id, node.node_type))
        .collect::<Vec<_>>();
    node_rows.sort();

    let mut edge_rows = edges
        .iter()
        .map(|edge| {
            format!(
                "{}|{}|{}|{}",
                edge.source, edge.source_handle, edge.target, edge.target_handle
            )
        })
        .collect::<Vec<_>>();
    edge_rows.sort();

    let mut digest = FNV64_OFFSET_BASIS;
    digest = fnv1a64_update(digest, b"v1");
    for row in node_rows {
        digest = fnv1a64_update(digest, row.as_bytes());
        digest = fnv1a64_update(digest, b"\n");
    }
    digest = fnv1a64_update(digest, b"--");
    for row in edge_rows {
        digest = fnv1a64_update(digest, row.as_bytes());
        digest = fnv1a64_update(digest, b"\n");
    }

    format!("{:016x}", digest)
}

fn fnv1a64_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV64_PRIME);
    }
    hash
}

pub fn extract_model_size_bytes(metadata: &serde_json::Value) -> Option<u64> {
    metadata
        .get("size_bytes")
        .and_then(|v| v.as_u64())
        .or_else(|| metadata.get("sizeBytes").and_then(|v| v.as_u64()))
        .or_else(|| metadata.get("size").and_then(|v| v.as_u64()))
}

pub fn select_preferred_hash(hashes: &HashMap<String, String>) -> Option<String> {
    for preferred in ["sha256", "blake3"] {
        if let Some((_, value)) = hashes
            .iter()
            .find(|(key, value)| key.eq_ignore_ascii_case(preferred) && !value.trim().is_empty())
        {
            let trimmed = value.trim();
            if trimmed.contains(':') {
                return Some(trimmed.to_string());
            }
            return Some(format!("{preferred}:{trimmed}"));
        }
    }
    None
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
                        let canonical_backend_key = canonical_runtime_backend_key(raw);
                        if !canonical_backend_key.is_empty() {
                            out.insert(canonical_backend_key);
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

fn derive_roles(node_type: &str, data: &serde_json::Value) -> HashSet<String> {
    let mut roles = HashSet::new();
    let node_type = node_type.to_ascii_lowercase();
    if node_type.contains("embedding") {
        roles.insert("embedding".to_string());
    }
    if node_type.contains("inference") {
        roles.insert("inference".to_string());
    }
    if node_type.contains("puma-lib") || node_type.contains("model-provider") {
        roles.insert("model_source".to_string());
    }
    if value_contains_embedding_hint(data) {
        roles.insert("embedding".to_string());
    }
    roles
}

fn value_contains_embedding_hint(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, child)| {
            key.eq_ignore_ascii_case("embedding")
                || key.eq_ignore_ascii_case("embeddings")
                || value_contains_embedding_hint(child)
        }),
        serde_json::Value::Array(values) => values.iter().any(value_contains_embedding_hint),
        _ => false,
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

    #[test]
    fn extract_model_usages_tracks_nodes_and_roles() {
        let nodes = vec![
            StoredGraphNode {
                id: "n1".to_string(),
                node_type: "embedding-inference".to_string(),
                data: serde_json::json!({"model_id": "m1"}),
                position: StoredPosition::default(),
            },
            StoredGraphNode {
                id: "n2".to_string(),
                node_type: "llm-inference".to_string(),
                data: serde_json::json!({"settings": {"model_id": "m1"}}),
                position: StoredPosition::default(),
            },
        ];

        let usages = extract_model_usages(&nodes);
        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].model_id, "m1");
        assert_eq!(usages[0].node_ids, vec!["n1".to_string(), "n2".to_string()]);
        assert_eq!(
            usages[0].roles,
            vec!["embedding".to_string(), "inference".to_string()]
        );
    }

    #[test]
    fn select_preferred_hash_prefers_sha256() {
        let hashes = HashMap::from([
            ("blake3".to_string(), "bbb".to_string()),
            ("sha256".to_string(), "aaa".to_string()),
        ]);
        assert_eq!(
            select_preferred_hash(&hashes).as_deref(),
            Some("sha256:aaa")
        );
    }

    #[test]
    fn extract_required_backends_normalizes_known_aliases() {
        let nodes = vec![
            StoredGraphNode {
                id: "n1".to_string(),
                node_type: "llm-inference".to_string(),
                data: serde_json::json!({"backend_key": "llama.cpp"}),
                position: StoredPosition::default(),
            },
            StoredGraphNode {
                id: "n2".to_string(),
                node_type: "llm-inference".to_string(),
                data: serde_json::json!({"settings": {"backendKey": "PyTorch"}}),
                position: StoredPosition::default(),
            },
            StoredGraphNode {
                id: "n3".to_string(),
                node_type: "llm-inference".to_string(),
                data: serde_json::json!({"backend_key": "llamacpp"}),
                position: StoredPosition::default(),
            },
        ];

        assert_eq!(
            extract_required_backends(&nodes),
            vec!["llama_cpp".to_string(), "pytorch".to_string()]
        );
    }
}

//! Headless workflow API adapter for Tauri transport.
//!
//! This module maps Tauri command invocations to host-agnostic service logic in
//! `pantograph-workflow-service`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_workflow_service::{
    RuntimeSignature, WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse, WorkflowHost,
    WorkflowHostCapabilities, WorkflowRunRequest, WorkflowRunResponse, WorkflowRuntimeRequirements,
    WorkflowService, WorkflowServiceError,
};
use tauri::State;

use crate::llm::SharedGateway;

use super::commands::SharedExtensions;

const DEFAULT_MAX_BATCH_SIZE: usize = 128;
const DEFAULT_MAX_TEXT_LENGTH: usize = 32_768;

pub async fn workflow_run(
    request: WorkflowRunRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
) -> Result<WorkflowRunResponse, String> {
    let host = TauriWorkflowHost::new(gateway.inner().clone(), extensions.inner().clone());
    WorkflowService::new()
        .workflow_run(&host, request)
        .await
        .map_err(|e| e.to_string())
}

pub async fn workflow_get_capabilities(
    request: WorkflowCapabilitiesRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
) -> Result<WorkflowCapabilitiesResponse, String> {
    let host = TauriWorkflowHost::new(gateway.inner().clone(), extensions.inner().clone());
    WorkflowService::new()
        .workflow_get_capabilities(&host, request)
        .await
        .map_err(|e| e.to_string())
}

struct TauriWorkflowHost {
    gateway: SharedGateway,
    extensions: SharedExtensions,
}

impl TauriWorkflowHost {
    fn new(gateway: SharedGateway, extensions: SharedExtensions) -> Self {
        Self {
            gateway,
            extensions,
        }
    }

    async fn pumas_api(&self) -> Option<Arc<pumas_library::PumasApi>> {
        let ext = self.extensions.read().await;
        ext.get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
            .cloned()
    }

    async fn compute_runtime_requirements(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowRuntimeRequirements, WorkflowServiceError> {
        let stored = load_and_validate_workflow(workflow_id)?;
        let mut required_models = extract_required_models(&stored.graph.nodes);
        let mut required_backends = extract_required_backends(&stored.graph.nodes);
        let required_extensions =
            extract_required_extensions(&stored.graph.nodes, !required_models.is_empty());

        if required_backends.is_empty() {
            required_backends.push(self.gateway.current_backend_name().await);
        }

        let (
            estimated_peak_vram_mb,
            estimated_peak_ram_mb,
            estimated_min_vram_mb,
            estimated_min_ram_mb,
            estimation_confidence,
        ) = self.estimate_memory_requirements(&required_models).await;

        required_models.sort();
        required_models.dedup();
        required_backends.sort();
        required_backends.dedup();

        Ok(WorkflowRuntimeRequirements {
            estimated_peak_vram_mb,
            estimated_peak_ram_mb,
            estimated_min_vram_mb,
            estimated_min_ram_mb,
            estimation_confidence,
            required_models,
            required_backends,
            required_extensions,
        })
    }

    async fn estimate_memory_requirements(
        &self,
        required_models: &[String],
    ) -> (Option<u64>, Option<u64>, Option<u64>, Option<u64>, String) {
        if required_models.is_empty() {
            return (Some(0), Some(0), Some(0), Some(0), "exact_no_models".to_string());
        }

        let Some(api) = self.pumas_api().await else {
            return (None, None, None, None, "unknown".to_string());
        };

        const MB: u64 = 1024 * 1024;
        let mut sizes_mb = Vec::new();
        let mut found_count = 0_usize;
        for model_id in required_models {
            let Ok(Some(model)) = api.get_model(model_id).await else {
                continue;
            };
            if let Some(size_bytes) = extract_model_size_bytes(&model.metadata) {
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
}

#[derive(Debug, serde::Deserialize)]
struct StoredWorkflowFile {
    metadata: StoredWorkflowMetadata,
    graph: StoredWorkflowGraph,
}

#[derive(Debug, serde::Deserialize)]
struct StoredWorkflowMetadata {
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct StoredWorkflowGraph {
    #[serde(default)]
    nodes: Vec<StoredGraphNode>,
    #[serde(default)]
    edges: Vec<StoredGraphEdge>,
}

#[derive(Debug, serde::Deserialize)]
struct StoredGraphNode {
    id: String,
    node_type: String,
    #[serde(default)]
    data: serde_json::Value,
    #[serde(default)]
    position: StoredPosition,
}

#[derive(Debug, serde::Deserialize, Default)]
struct StoredPosition {
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
}

#[derive(Debug, serde::Deserialize)]
struct StoredGraphEdge {
    id: String,
    source: String,
    source_handle: String,
    target: String,
    target_handle: String,
}

fn load_and_validate_workflow(workflow_id: &str) -> Result<StoredWorkflowFile, WorkflowServiceError> {
    let workflow_path = find_workflow_file(workflow_id).ok_or_else(|| {
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
    if !validation_errors.is_empty() {
        let error_text = validation_errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(WorkflowServiceError::CapabilityViolation(format!(
            "workflow '{}' failed graph validation: {}",
            workflow_id, error_text
        )));
    }

    Ok(stored)
}

fn find_workflow_file(workflow_id: &str) -> Option<PathBuf> {
    let stem = sanitize_workflow_stem(workflow_id)?;
    let filename = format!("{stem}.json");

    workflow_roots()
        .into_iter()
        .map(|root| root.join(&filename))
        .find(|path| path.is_file())
}

fn workflow_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(current) = std::env::current_dir() {
        extend_ancestor_workflow_roots(&current, &mut roots);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    extend_ancestor_workflow_roots(&manifest_dir, &mut roots);

    roots
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

fn extract_required_models(nodes: &[StoredGraphNode]) -> Vec<String> {
    let mut out = HashSet::new();
    for node in nodes {
        extract_model_ids_from_value(&node.data, &mut out);
    }
    let mut models = out.into_iter().collect::<Vec<_>>();
    models.sort();
    models
}

fn extract_required_backends(nodes: &[StoredGraphNode]) -> Vec<String> {
    let mut out = HashSet::new();
    for node in nodes {
        extract_backend_keys_from_value(&node.data, &mut out);
    }
    let mut backends = out.into_iter().collect::<Vec<_>>();
    backends.sort();
    backends
}

fn extract_required_extensions(nodes: &[StoredGraphNode], has_models: bool) -> Vec<String> {
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

fn extract_model_size_bytes(metadata: &serde_json::Value) -> Option<u64> {
    metadata
        .get("size_bytes")
        .and_then(|v| v.as_u64())
        .or_else(|| metadata.get("sizeBytes").and_then(|v| v.as_u64()))
        .or_else(|| metadata.get("size").and_then(|v| v.as_u64()))
}

#[async_trait]
impl WorkflowHost for TauriWorkflowHost {
    async fn validate_workflow(
        &self,
        workflow_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        load_and_validate_workflow(workflow_id).map(|_| ())
    }

    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(WorkflowHostCapabilities {
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
            max_text_length: DEFAULT_MAX_TEXT_LENGTH,
            runtime_requirements: self.compute_runtime_requirements(workflow_id).await?,
        })
    }

    async fn run_object(
        &self,
        _workflow_id: &str,
        text: &str,
        model_id: Option<&str>,
    ) -> Result<(Vec<f32>, Option<usize>), WorkflowServiceError> {
        let inner = self.gateway.inner_arc();

        if !inner.is_ready().await {
            return Err(WorkflowServiceError::RuntimeNotReady(
                "inference gateway is not ready".to_string(),
            ));
        }

        let capabilities = inner.capabilities().await;
        if !capabilities.embeddings {
            return Err(WorkflowServiceError::CapabilityViolation(
                "active backend does not support embeddings".to_string(),
            ));
        }

        let selected_model = model_id
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("default");

        let results = inner
            .embeddings(vec![text.to_string()], selected_model)
            .await
            .map_err(|err| match err {
                inference::GatewayError::Backend(inference::backend::BackendError::NotReady) => {
                    WorkflowServiceError::RuntimeNotReady("backend is not ready".to_string())
                }
                other => WorkflowServiceError::Internal(other.to_string()),
            })?;

        let first = results.into_iter().next().ok_or_else(|| {
            WorkflowServiceError::Internal("no embedding vector returned".to_string())
        })?;

        Ok((first.vector, Some(first.token_count)))
    }

    async fn resolve_runtime_signature(
        &self,
        _workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<RuntimeSignature, WorkflowServiceError> {
        let backend = self.gateway.current_backend_name().await;
        let model_id = model_id
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("default")
            .to_string();

        Ok(RuntimeSignature {
            model_id,
            model_revision_or_hash: None,
            backend,
            vector_dimensions,
        })
    }
}

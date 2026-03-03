//! UniFFI bindings for Pantograph workflow engine.
//!
//! This crate provides cross-language bindings for the Pantograph node-engine,
//! enabling native access from Python, C#, Swift, Kotlin, Go, and Ruby.
//!
//! # Architecture
//!
//! Types with `serde_json::Value` or `(f64, f64)` fields are wrapped in
//! FFI-safe records. Complex graphs are marshaled as JSON strings at the
//! boundary for maximum flexibility.
//!
//! # Usage
//!
//! ```bash
//! # Build the cdylib
//! cargo build -p pantograph-uniffi --release
//!
//! # Generate Python bindings
//! pantograph-uniffi-bindgen generate --library --language python \
//!     --out-dir ./bindings/python target/release/libpantograph_uniffi.so
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use node_engine::{
    Context, EventSink, OrchestrationGraph, OrchestrationStore, TaskExecutor, WorkflowEvent,
    WorkflowExecutor, WorkflowGraph,
};
use pantograph_workflow_service::{
    WorkflowRunRequest, WorkflowHost, WorkflowHostCapabilities, WorkflowService,
    WorkflowServiceError, WorkflowCapabilitiesRequest, RuntimeSignature,
    WorkflowRuntimeRequirements,
};
use tokio::sync::RwLock;

// UniFFI scaffolding
uniffi::setup_scaffolding!();

// ============================================================================
// Error types
// ============================================================================

/// FFI-friendly error type mapping from NodeEngineError.
#[derive(Debug, Clone, uniffi::Error, thiserror::Error)]
pub enum FfiError {
    #[error("Graph execution error: {message}")]
    GraphFlow { message: String },

    #[error("Missing input: {message}")]
    MissingInput { message: String },

    #[error("Invalid input type: {message}")]
    InvalidInputType { message: String },

    #[error("Execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("Context not found: {message}")]
    ContextNotFound { message: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("Compression error: {message}")]
    Compression { message: String },

    #[error("Cancelled")]
    Cancelled,

    #[error("Gateway error: {message}")]
    Gateway { message: String },

    #[error("RAG error: {message}")]
    Rag { message: String },

    #[error("IO error: {message}")]
    Io { message: String },

    #[error("{message}")]
    Other { message: String },
}

impl From<node_engine::NodeEngineError> for FfiError {
    fn from(err: node_engine::NodeEngineError) -> Self {
        use node_engine::NodeEngineError;
        match err {
            NodeEngineError::GraphFlow(msg) => FfiError::GraphFlow { message: msg },
            NodeEngineError::MissingInput(msg) => FfiError::MissingInput { message: msg },
            NodeEngineError::InvalidInputType { port, expected } => FfiError::InvalidInputType {
                message: format!("{}: expected {}", port, expected),
            },
            NodeEngineError::ExecutionFailed(msg) => FfiError::ExecutionFailed { message: msg },
            NodeEngineError::ContextNotFound(msg) => FfiError::ContextNotFound { message: msg },
            NodeEngineError::Serialization(err) => FfiError::Serialization {
                message: err.to_string(),
            },
            NodeEngineError::Compression(msg) => FfiError::Compression { message: msg },
            NodeEngineError::Cancelled => FfiError::Cancelled,
            NodeEngineError::Gateway(msg) => FfiError::Gateway { message: msg },
            NodeEngineError::Rag(msg) => FfiError::Rag { message: msg },
            NodeEngineError::Io(err) => FfiError::Io {
                message: err.to_string(),
            },
        }
    }
}

pub type FfiResult<T> = Result<T, FfiError>;

// ============================================================================
// FFI Wrapper Records
// ============================================================================

/// FFI-safe representation of a graph node.
#[derive(uniffi::Record)]
pub struct FfiGraphNode {
    pub id: String,
    pub node_type: String,
    pub position_x: f64,
    pub position_y: f64,
    /// Node data as JSON string (from serde_json::Value)
    pub data_json: String,
}

/// FFI-safe representation of a graph edge.
#[derive(uniffi::Record)]
pub struct FfiGraphEdge {
    pub id: String,
    pub source: String,
    pub source_handle: String,
    pub target: String,
    pub target_handle: String,
}

/// FFI-safe representation of a workflow graph.
#[derive(uniffi::Record)]
pub struct FfiWorkflowGraph {
    pub id: String,
    pub name: String,
    pub nodes: Vec<FfiGraphNode>,
    pub edges: Vec<FfiGraphEdge>,
}

impl From<WorkflowGraph> for FfiWorkflowGraph {
    fn from(g: WorkflowGraph) -> Self {
        Self {
            id: g.id.clone(),
            name: g.name.clone(),
            nodes: g
                .nodes
                .iter()
                .map(|n| FfiGraphNode {
                    id: n.id.clone(),
                    node_type: n.node_type.clone(),
                    position_x: n.position.0,
                    position_y: n.position.1,
                    data_json: n.data.to_string(),
                })
                .collect(),
            edges: g
                .edges
                .iter()
                .map(|e| FfiGraphEdge {
                    id: e.id.clone(),
                    source: e.source.clone(),
                    source_handle: e.source_handle.clone(),
                    target: e.target.clone(),
                    target_handle: e.target_handle.clone(),
                })
                .collect(),
        }
    }
}

/// FFI-safe cache statistics.
#[derive(uniffi::Record)]
pub struct FfiCacheStats {
    pub cached_nodes: u64,
    pub total_versions: u64,
    pub global_version: u64,
}

/// FFI-safe orchestration metadata.
#[derive(uniffi::Record)]
pub struct FfiOrchestrationMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub node_count: u64,
}

/// FFI-safe workflow event.
#[derive(uniffi::Record)]
pub struct FfiWorkflowEvent {
    /// Event type identifier
    pub event_type: String,
    /// Full event data as JSON
    pub event_json: String,
}

// ============================================================================
// Simple TaskExecutor for UniFFI (synchronous JSON-based)
// ============================================================================

/// A no-op TaskExecutor for use when the host language handles execution
/// through the graph snapshot mechanism rather than callbacks.
struct NoopTaskExecutor;

#[async_trait::async_trait]
impl TaskExecutor for NoopTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        _inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &node_engine::ExecutorExtensions,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        Err(node_engine::NodeEngineError::ExecutionFailed(format!(
            "No executor configured for task '{}'",
            task_id
        )))
    }
}

// ============================================================================
// Free functions
// ============================================================================

/// Get the version of the pantograph-uniffi bindings.
#[uniffi::export]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Validate a workflow graph JSON string, returning error messages.
#[uniffi::export]
pub fn validate_workflow_json(graph_json: String) -> Result<Vec<String>, FfiError> {
    let graph: WorkflowGraph =
        serde_json::from_str(&graph_json).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })?;
    let errors = node_engine::validation::validate_workflow(&graph, None);
    Ok(errors.iter().map(|e| e.to_string()).collect())
}

/// Validate an orchestration graph JSON string, returning error messages.
#[uniffi::export]
pub fn validate_orchestration_json(graph_json: String) -> Result<Vec<String>, FfiError> {
    let graph: OrchestrationGraph =
        serde_json::from_str(&graph_json).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })?;
    let errors = node_engine::validation::validate_orchestration(&graph);
    Ok(errors.iter().map(|e| e.to_string()).collect())
}

const DEFAULT_MAX_BATCH_SIZE: usize = 128;
const DEFAULT_MAX_TEXT_LENGTH: usize = 32_768;

struct UniffiWorkflowHost {
    base_url: String,
    pumas_api: Option<Arc<pumas_library::PumasApi>>,
    http_client: reqwest::Client,
    resolved_model_id: std::sync::Mutex<Option<String>>,
}

impl UniffiWorkflowHost {
    fn new(base_url: String, pumas_api: Option<Arc<pumas_library::PumasApi>>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            pumas_api,
            http_client: reqwest::Client::new(),
            resolved_model_id: std::sync::Mutex::new(None),
        }
    }

    async fn resolve_model_revision_or_hash(
        &self,
        model_id: &str,
    ) -> Result<Option<String>, WorkflowServiceError> {
        let Some(api) = &self.pumas_api else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?
            .ok_or_else(|| {
                WorkflowServiceError::RuntimeSignatureUnavailable(format!(
                    "model '{}' not found in model library",
                    model_id
                ))
            })?;

        select_model_hash(&model.hashes)
            .map(Some)
            .ok_or_else(|| {
                WorkflowServiceError::RuntimeSignatureUnavailable(format!(
                    "model '{}' is missing sha256/blake3 hash metadata",
                    model_id
                ))
            })
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
            required_backends.push("openai-compatible".to_string());
        }

        let (
            estimated_peak_vram_mb,
            estimated_peak_ram_mb,
            estimated_min_vram_mb,
            estimated_min_ram_mb,
            estimation_confidence,
        ) = estimate_memory_requirements(self.pumas_api.as_ref(), &required_models).await;

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
}

#[async_trait::async_trait]
impl WorkflowHost for UniffiWorkflowHost {
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
        let model = model_id
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("default");

        let url = format!("{}/v1/embeddings", self.base_url);
        let body = serde_json::json!({
            "input": [text],
            "model": model,
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        if !response.status().is_success() {
            return Err(WorkflowServiceError::Internal(format!(
                "embedding api error {}",
                response.status()
            )));
        }

        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|e| WorkflowServiceError::Internal(e.to_string()))?;

        let (embedding, token_count, response_model_id) = parse_embedding_payload(&payload)?;
        if let Some(model_id) = response_model_id {
            if let Ok(mut guard) = self.resolved_model_id.lock() {
                *guard = Some(model_id);
            }
        }

        Ok((embedding, token_count))
    }

    async fn resolve_runtime_signature(
        &self,
        _workflow_id: &str,
        model_id: Option<&str>,
        vector_dimensions: usize,
    ) -> Result<RuntimeSignature, WorkflowServiceError> {
        let resolved_model_id = model_id
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                self.resolved_model_id
                    .lock()
                    .ok()
                    .and_then(|guard| guard.clone())
            })
            .unwrap_or_else(|| "default".to_string());
        let model_revision_or_hash = self.resolve_model_revision_or_hash(&resolved_model_id).await?;

        Ok(RuntimeSignature {
            model_id: resolved_model_id,
            model_revision_or_hash,
            backend: "openai-compatible".to_string(),
            vector_dimensions,
        })
    }
}

fn parse_embedding_payload(
    payload: &serde_json::Value,
) -> Result<(Vec<f32>, Option<usize>, Option<String>), WorkflowServiceError> {
    let embedding_values = payload
        .get("data")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("embedding"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| WorkflowServiceError::Internal("missing embedding vector".to_string()))?;

    let mut embedding = Vec::with_capacity(embedding_values.len());
    for (index, value) in embedding_values.iter().enumerate() {
        let number = value.as_f64().ok_or_else(|| {
            WorkflowServiceError::Internal(format!(
                "invalid embedding value at index {}",
                index
            ))
        })?;
        embedding.push(number as f32);
    }

    let token_count = payload
        .get("usage")
        .and_then(|v| v.get("prompt_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let response_model_id = payload
        .get("model")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);

    Ok((embedding, token_count, response_model_id))
}

fn select_model_hash(hashes: &std::collections::HashMap<String, String>) -> Option<String> {
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

fn extract_required_models(nodes: &[StoredGraphNode]) -> Vec<String> {
    let mut out = std::collections::HashSet::new();
    for node in nodes {
        extract_model_ids_from_value(&node.data, &mut out);
    }
    let mut models = out.into_iter().collect::<Vec<_>>();
    models.sort();
    models
}

fn extract_required_backends(nodes: &[StoredGraphNode]) -> Vec<String> {
    let mut out = std::collections::HashSet::new();
    for node in nodes {
        extract_backend_keys_from_value(&node.data, &mut out);
    }
    let mut backends = out.into_iter().collect::<Vec<_>>();
    backends.sort();
    backends
}

fn extract_required_extensions(nodes: &[StoredGraphNode], has_models: bool) -> Vec<String> {
    let mut out = vec!["http_client".to_string()];
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

fn extract_model_ids_from_value(
    value: &serde_json::Value,
    out: &mut std::collections::HashSet<String>,
) {
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

fn extract_backend_keys_from_value(
    value: &serde_json::Value,
    out: &mut std::collections::HashSet<String>,
) {
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

async fn estimate_memory_requirements(
    pumas_api: Option<&Arc<pumas_library::PumasApi>>,
    required_models: &[String],
) -> (Option<u64>, Option<u64>, Option<u64>, Option<u64>, String) {
    if required_models.is_empty() {
        return (Some(0), Some(0), Some(0), Some(0), "exact_no_models".to_string());
    }

    let Some(api) = pumas_api else {
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

fn extract_model_size_bytes(metadata: &serde_json::Value) -> Option<u64> {
    metadata
        .get("size_bytes")
        .and_then(|v| v.as_u64())
        .or_else(|| metadata.get("sizeBytes").and_then(|v| v.as_u64()))
        .or_else(|| metadata.get("size").and_then(|v| v.as_u64()))
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

fn map_workflow_service_error(err: WorkflowServiceError) -> FfiError {
    match err {
        WorkflowServiceError::InvalidRequest(message)
        | WorkflowServiceError::CapabilityViolation(message)
        | WorkflowServiceError::WorkflowNotFound(message)
        | WorkflowServiceError::RuntimeNotReady(message)
        | WorkflowServiceError::RuntimeSignatureUnavailable(message)
        | WorkflowServiceError::Internal(message) => FfiError::Other { message },
    }
}

/// Execute headless workflow contract (`workflow_run`) and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn workflow_run(
    base_url: String,
    request_json: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<String, FfiError> {
    let request: WorkflowRunRequest =
        serde_json::from_str(&request_json).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })?;

    let host = UniffiWorkflowHost::new(
        base_url,
        pumas_api.as_ref().map(|api| api.api_arc()),
    );
    let response = WorkflowService::new()
        .workflow_run(&host, request)
        .await
        .map_err(map_workflow_service_error)?;

    serde_json::to_string(&response).map_err(|e| FfiError::Serialization {
        message: e.to_string(),
    })
}

/// Execute headless workflow capabilities contract and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn workflow_get_capabilities(
    base_url: String,
    request_json: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<String, FfiError> {
    let request: WorkflowCapabilitiesRequest =
        serde_json::from_str(&request_json).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })?;

    let host = UniffiWorkflowHost::new(
        base_url,
        pumas_api.as_ref().map(|api| api.api_arc()),
    );
    let response = WorkflowService::new()
        .workflow_get_capabilities(&host, request)
        .await
        .map_err(map_workflow_service_error)?;

    serde_json::to_string(&response).map_err(|e| FfiError::Serialization {
        message: e.to_string(),
    })
}

// ============================================================================
// FfiWorkflowEngine - Main workflow engine object
// ============================================================================

/// The main Pantograph workflow engine handle.
///
/// Wraps a `WorkflowExecutor` for graph CRUD, demand-driven execution,
/// and event collection.
///
/// # Example (Python)
///
/// ```python
/// engine = FfiWorkflowEngine("wf-1", "My Workflow")
/// engine.add_node("n1", "text-input", 0.0, 0.0, "{}")
/// engine.add_node("n2", "text-output", 200.0, 0.0, "{}")
/// engine.add_edge("n1", "text", "n2", "text")
/// graph = engine.get_graph()
/// ```
#[derive(uniffi::Object)]
pub struct FfiWorkflowEngine {
    executor: Arc<RwLock<WorkflowExecutor>>,
    task_executor: Arc<dyn TaskExecutor>,
    event_buffer: Arc<RwLock<Vec<FfiWorkflowEvent>>>,
}

/// Callback EventSink that buffers events for polling.
struct BufferedEventSink {
    buffer: Arc<RwLock<Vec<FfiWorkflowEvent>>>,
}

impl EventSink for BufferedEventSink {
    fn send(&self, event: WorkflowEvent) -> std::result::Result<(), node_engine::EventError> {
        let event_type = format!("{:?}", event)
            .split('(')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        let event_json = serde_json::to_string(&event).map_err(|e| node_engine::EventError {
            message: e.to_string(),
        })?;

        // Use try_write to avoid blocking in sync context
        if let Ok(mut buf) = self.buffer.try_write() {
            buf.push(FfiWorkflowEvent {
                event_type,
                event_json,
            });
        }
        Ok(())
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl FfiWorkflowEngine {
    /// Create a new workflow engine with an empty graph.
    #[uniffi::constructor]
    pub fn new(id: String, name: String) -> Arc<Self> {
        let graph = WorkflowGraph::new(&id, &name);
        let event_buffer = Arc::new(RwLock::new(Vec::new()));
        let event_sink: Arc<dyn EventSink> = Arc::new(BufferedEventSink {
            buffer: event_buffer.clone(),
        });
        let executor = WorkflowExecutor::new("uniffi-execution", graph, event_sink);

        Arc::new(Self {
            executor: Arc::new(RwLock::new(executor)),
            task_executor: Arc::new(NoopTaskExecutor),
            event_buffer,
        })
    }

    /// Create from a JSON-serialized workflow graph.
    #[uniffi::constructor]
    pub fn from_json(graph_json: String) -> Result<Arc<Self>, FfiError> {
        let graph: WorkflowGraph =
            serde_json::from_str(&graph_json).map_err(|e| FfiError::Serialization {
                message: e.to_string(),
            })?;
        let event_buffer = Arc::new(RwLock::new(Vec::new()));
        let event_sink: Arc<dyn EventSink> = Arc::new(BufferedEventSink {
            buffer: event_buffer.clone(),
        });
        let executor = WorkflowExecutor::new("uniffi-execution", graph, event_sink);

        Ok(Arc::new(Self {
            executor: Arc::new(RwLock::new(executor)),
            task_executor: Arc::new(NoopTaskExecutor),
            event_buffer,
        }))
    }

    // ============================
    // Graph CRUD
    // ============================

    /// Add a node to the graph.
    pub async fn add_node(
        &self,
        id: String,
        node_type: String,
        x: f64,
        y: f64,
        data_json: String,
    ) -> Result<(), FfiError> {
        let data: serde_json::Value =
            serde_json::from_str(&data_json).unwrap_or(serde_json::Value::Null);

        let node = node_engine::GraphNode {
            id,
            node_type,
            position: (x, y),
            data,
        };

        let exec = self.executor.read().await;
        exec.add_node(node).await;
        Ok(())
    }

    /// Add an edge to the graph.
    pub async fn add_edge(
        &self,
        source: String,
        source_handle: String,
        target: String,
        target_handle: String,
    ) -> Result<(), FfiError> {
        let edge_id = format!(
            "e-{}-{}-{}-{}",
            source, source_handle, target, target_handle
        );
        let edge = node_engine::GraphEdge {
            id: edge_id,
            source,
            source_handle,
            target,
            target_handle,
        };

        let exec = self.executor.read().await;
        exec.add_edge(edge).await;
        Ok(())
    }

    /// Remove an edge by ID.
    pub async fn remove_edge(&self, edge_id: String) -> Result<(), FfiError> {
        let exec = self.executor.read().await;
        exec.remove_edge(&edge_id).await;
        Ok(())
    }

    /// Update a node's data.
    pub async fn update_node_data(
        &self,
        node_id: String,
        data_json: String,
    ) -> Result<(), FfiError> {
        let data: serde_json::Value =
            serde_json::from_str(&data_json).unwrap_or(serde_json::Value::Null);

        let exec = self.executor.read().await;
        exec.update_node_data(&node_id, data)
            .await
            .map_err(FfiError::from)
    }

    // ============================
    // Query
    // ============================

    /// Get the current graph state.
    pub async fn get_graph(&self) -> FfiWorkflowGraph {
        let exec = self.executor.read().await;
        let snapshot = exec.get_graph_snapshot().await;
        FfiWorkflowGraph::from(snapshot)
    }

    /// Export the graph as a JSON string.
    pub async fn export_graph_json(&self) -> Result<String, FfiError> {
        let exec = self.executor.read().await;
        let snapshot = exec.get_graph_snapshot().await;
        serde_json::to_string(&snapshot).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    /// Get cache statistics.
    pub async fn cache_stats(&self) -> FfiCacheStats {
        let exec = self.executor.read().await;
        let stats = exec.cache_stats().await;
        FfiCacheStats {
            cached_nodes: stats.cached_nodes as u64,
            total_versions: stats.total_versions as u64,
            global_version: stats.global_version,
        }
    }

    // ============================
    // Execution
    // ============================

    /// Mark a node as modified (invalidates caches).
    pub async fn mark_modified(&self, node_id: String) {
        let exec = self.executor.read().await;
        exec.mark_modified(&node_id).await;
    }

    // ============================
    // Events
    // ============================

    /// Drain all buffered events since last call.
    pub async fn drain_events(&self) -> Vec<FfiWorkflowEvent> {
        let mut buffer = self.event_buffer.write().await;
        std::mem::take(&mut *buffer)
    }
}

// ============================================================================
// FfiOrchestrationStore - Orchestration graph storage
// ============================================================================

/// Persistent orchestration graph store.
///
/// Manages orchestration graphs in memory with optional file persistence.
#[derive(uniffi::Object)]
pub struct FfiOrchestrationStore {
    store: Arc<RwLock<OrchestrationStore>>,
}

#[uniffi::export(async_runtime = "tokio")]
impl FfiOrchestrationStore {
    /// Create a new in-memory store.
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Arc::new(RwLock::new(OrchestrationStore::new())),
        })
    }

    /// Create a store with file persistence.
    #[uniffi::constructor]
    pub fn with_persistence(path: String) -> Arc<Self> {
        Arc::new(Self {
            store: Arc::new(RwLock::new(OrchestrationStore::with_persistence(path))),
        })
    }

    /// List all orchestration graph metadata.
    pub async fn list_graphs(&self) -> Vec<FfiOrchestrationMetadata> {
        let guard = self.store.read().await;
        guard
            .list_graphs()
            .into_iter()
            .map(|m| FfiOrchestrationMetadata {
                id: m.id,
                name: m.name,
                description: m.description,
                node_count: m.node_count as u64,
            })
            .collect()
    }

    /// Insert an orchestration graph (as JSON).
    pub async fn insert_graph(&self, graph_json: String) -> Result<(), FfiError> {
        let graph: OrchestrationGraph =
            serde_json::from_str(&graph_json).map_err(|e| FfiError::Serialization {
                message: e.to_string(),
            })?;
        let mut guard = self.store.write().await;
        guard.insert_graph(graph).map_err(FfiError::from)
    }

    /// Get an orchestration graph by ID (as JSON).
    pub async fn get_graph(&self, graph_id: String) -> Option<String> {
        let guard = self.store.read().await;
        guard
            .get_graph(&graph_id)
            .and_then(|g| serde_json::to_string(g).ok())
    }

    /// Remove an orchestration graph by ID.
    pub async fn remove_graph(&self, graph_id: String) -> Result<(), FfiError> {
        let mut guard = self.store.write().await;
        guard.remove_graph(&graph_id).map_err(FfiError::from)?;
        Ok(())
    }
}

// ============================================================================
// FfiPumasApi - Model Library API
// ============================================================================

/// Pumas model library API for model management, HuggingFace search,
/// downloads, and imports.
#[derive(uniffi::Object)]
pub struct FfiPumasApi {
    api: Arc<pumas_library::PumasApi>,
}

#[uniffi::export(async_runtime = "tokio")]
impl FfiPumasApi {
    /// Create a new PumasApi instance.
    ///
    /// `launcher_root` is the root directory for the pumas library.
    #[uniffi::constructor]
    pub async fn new(launcher_root: String) -> Result<Arc<Self>, FfiError> {
        let api = pumas_library::PumasApi::builder(&launcher_root)
            .auto_create_dirs(true)
            .with_hf_client(true)
            .with_process_manager(false)
            .build()
            .await
            .map_err(|e| FfiError::Other {
                message: format!("PumasApi init error: {}", e),
            })?;

        Ok(Arc::new(Self { api: Arc::new(api) }))
    }

    // --- Local library ---

    /// List all models in the local library. Returns JSON array of ModelRecord.
    pub async fn list_models(&self) -> Result<String, FfiError> {
        let models = self.api.list_models().await.map_err(|e| FfiError::Other {
            message: e.to_string(),
        })?;
        serde_json::to_string(&models).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    /// Search the local model library. Returns JSON SearchResult.
    pub async fn search_models(
        &self,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<String, FfiError> {
        let result = self
            .api
            .search_models(&query, limit as usize, offset as usize)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        serde_json::to_string(&result).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    /// Get a single model by ID. Returns JSON ModelRecord or None.
    pub async fn get_model(&self, model_id: String) -> Result<Option<String>, FfiError> {
        let model = self
            .api
            .get_model(&model_id)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        match model {
            Some(m) => {
                let json = serde_json::to_string(&m).map_err(|e| FfiError::Serialization {
                    message: e.to_string(),
                })?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    }

    // --- HuggingFace ---

    /// Search HuggingFace for models. Returns JSON array of HuggingFaceModel.
    pub async fn search_hf(
        &self,
        query: String,
        kind: Option<String>,
        limit: u32,
    ) -> Result<String, FfiError> {
        let models = self
            .api
            .search_hf_models(&query, kind.as_deref(), limit as usize)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        serde_json::to_string(&models).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    /// Get file tree for a HuggingFace repo. Returns JSON RepoFileTree.
    pub async fn get_repo_files(&self, repo_id: String) -> Result<String, FfiError> {
        let tree = self
            .api
            .get_hf_repo_files(&repo_id)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        serde_json::to_string(&tree).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    // --- Download ---

    /// Start a model download. `request_json` is a JSON DownloadRequest.
    /// Returns the download ID.
    pub async fn start_download(&self, request_json: String) -> Result<String, FfiError> {
        let request: pumas_library::model_library::DownloadRequest =
            serde_json::from_str(&request_json).map_err(|e| FfiError::Serialization {
                message: e.to_string(),
            })?;
        self.api
            .start_hf_download(&request)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })
    }

    /// Get download progress. Returns JSON ModelDownloadProgress or None.
    pub async fn get_download_progress(
        &self,
        download_id: String,
    ) -> Result<Option<String>, FfiError> {
        let progress = self.api.get_hf_download_progress(&download_id).await;
        match progress {
            Some(p) => {
                let json = serde_json::to_string(&p).map_err(|e| FfiError::Serialization {
                    message: e.to_string(),
                })?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    }

    /// Cancel a download. Returns true if cancelled.
    pub async fn cancel_download(&self, download_id: String) -> Result<bool, FfiError> {
        self.api
            .cancel_hf_download(&download_id)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })
    }

    // --- Import ---

    /// Import a model. `spec_json` is a JSON ModelImportSpec.
    /// Returns JSON ModelImportResult.
    pub async fn import_model(&self, spec_json: String) -> Result<String, FfiError> {
        let spec: pumas_library::model_library::ModelImportSpec = serde_json::from_str(&spec_json)
            .map_err(|e| FfiError::Serialization {
                message: e.to_string(),
            })?;
        let result = self
            .api
            .import_model(&spec)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        serde_json::to_string(&result).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    // --- System ---

    /// Get disk space info. Returns JSON DiskSpaceResponse.
    pub async fn get_disk_space(&self) -> Result<String, FfiError> {
        let info = self
            .api
            .get_disk_space()
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        serde_json::to_string(&info).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    /// Check if Ollama is running.
    pub async fn is_ollama_running(&self) -> bool {
        self.api.is_ollama_running().await
    }
}

impl FfiPumasApi {
    fn api_arc(&self) -> Arc<pumas_library::PumasApi> {
        self.api.clone()
    }
}

/// Inject PumasApi into a workflow engine's extensions.
#[uniffi::export(async_runtime = "tokio")]
impl FfiWorkflowEngine {
    /// Set a PumasApi on this engine for model resolution in workflow nodes.
    pub async fn set_pumas_api(&self, api: Arc<FfiPumasApi>) {
        let mut exec = self.executor.write().await;
        exec.extensions_mut()
            .set(node_engine::extension_keys::PUMAS_API, api.api_arc());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    static CWD_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_ffi_error_conversion() {
        let err = node_engine::NodeEngineError::ExecutionFailed("test".to_string());
        let ffi_err: FfiError = err.into();
        assert!(matches!(ffi_err, FfiError::ExecutionFailed { .. }));
    }

    #[test]
    fn test_ffi_error_cancelled() {
        let err = node_engine::NodeEngineError::Cancelled;
        let ffi_err: FfiError = err.into();
        assert!(matches!(ffi_err, FfiError::Cancelled));
    }

    #[test]
    fn test_ffi_graph_conversion() {
        let graph = WorkflowGraph::new("test", "Test Graph");
        let ffi = FfiWorkflowGraph::from(graph);
        assert_eq!(ffi.id, "test");
        assert_eq!(ffi.name, "Test Graph");
        assert!(ffi.nodes.is_empty());
        assert!(ffi.edges.is_empty());
    }

    #[test]
    fn test_validate_empty_workflow() {
        let graph = WorkflowGraph::new("test", "Test");
        let json = serde_json::to_string(&graph).unwrap();
        let errors = validate_workflow_json(json).unwrap();
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_workflow_engine_new() {
        let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
        let graph = engine.get_graph().await;
        assert_eq!(graph.id, "wf-1");
        assert_eq!(graph.name, "Test");
    }

    #[tokio::test]
    async fn test_workflow_engine_add_node() {
        let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
        engine
            .add_node(
                "n1".to_string(),
                "text-input".to_string(),
                0.0,
                0.0,
                "{}".to_string(),
            )
            .await
            .unwrap();

        let graph = engine.get_graph().await;
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].id, "n1");
    }

    #[tokio::test]
    async fn test_workflow_engine_export_json() {
        let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
        let json = engine.export_graph_json().await.unwrap();
        assert!(json.contains("wf-1"));
    }

    #[tokio::test]
    async fn test_orchestration_store() {
        let store = FfiOrchestrationStore::new();
        let list = store.list_graphs().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_drain_events_empty() {
        let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
        let events = engine.drain_events().await;
        assert!(events.is_empty());
    }

    fn create_temp_workflow_root(workflow_id: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pantograph-uniffi-tests-{suffix}"));
        let workflows_dir = root.join(".pantograph").join("workflows");
        std::fs::create_dir_all(&workflows_dir).expect("create workflows dir");

        let workflow_json = serde_json::json!({
            "version": "1.0",
            "metadata": {
                "name": "Test Workflow",
                "created": "2026-01-01T00:00:00Z",
                "modified": "2026-01-01T00:00:00Z"
            },
            "graph": {
                "nodes": [],
                "edges": []
            }
        });
        let file_path = workflows_dir.join(format!("{}.json", workflow_id));
        std::fs::write(
            file_path,
            serde_json::to_vec(&workflow_json).expect("serialize workflow"),
        )
        .expect("write workflow");
        root
    }

    fn spawn_single_embedding_server(
        status_code: u16,
        body: serde_json::Value,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("local addr");
        let body_text = body.to_string();
        let reason = if status_code == 200 { "OK" } else { "ERROR" };

        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("set timeout");
            let mut request_buf = [0_u8; 8192];
            let _ = stream.read(&mut request_buf);

            let response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status_code,
                reason,
                body_text.len(),
                body_text
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });

        (format!("http://{}", addr), handle)
    }

    #[test]
    #[ignore = "requires local TCP bind permissions in test environment"]
    fn test_workflow_run_contract_success() {
        let _guard = CWD_LOCK.lock().expect("lock cwd");
        let workflow_id = "wf_contract_success";
        let root = create_temp_workflow_root(workflow_id);
        let original_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&root).expect("set cwd");

        let payload = serde_json::json!({
            "data": [{ "embedding": [0.1, 0.2, 0.3] }],
            "usage": { "prompt_tokens": 4 },
            "model": "model-from-server"
        });
        let (base_url, server_thread) = spawn_single_embedding_server(200, payload);

        let request_json = serde_json::json!({
            "workflow_id": workflow_id,
            "objects": [{ "object_id": "obj-1", "text": "hello world" }]
        })
        .to_string();

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let response_json = runtime
            .block_on(workflow_run(base_url, request_json, None))
            .expect("workflow_run");
        let response: pantograph_workflow_service::WorkflowRunResponse =
            serde_json::from_str(&response_json).expect("parse response");

        server_thread.join().expect("join server");
        std::env::set_current_dir(original_cwd).expect("restore cwd");
        let _ = std::fs::remove_dir_all(root);

        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].object_id, "obj-1");
        assert_eq!(response.model_signature.model_id, "model-from-server");
    }

    #[test]
    fn test_workflow_get_capabilities_contract_success() {
        let _guard = CWD_LOCK.lock().expect("lock cwd");
        let workflow_id = "wf_contract_caps";
        let root = create_temp_workflow_root(workflow_id);
        let original_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&root).expect("set cwd");

        let request_json = serde_json::json!({
            "workflow_id": workflow_id
        })
        .to_string();

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let response_json = runtime
            .block_on(workflow_get_capabilities(
                "http://127.0.0.1:9".to_string(),
                request_json,
                None,
            ))
            .expect("capabilities");
        let response: pantograph_workflow_service::WorkflowCapabilitiesResponse =
            serde_json::from_str(&response_json).expect("parse capabilities");

        std::env::set_current_dir(original_cwd).expect("restore cwd");
        let _ = std::fs::remove_dir_all(root);

        assert_eq!(response.max_batch_size, DEFAULT_MAX_BATCH_SIZE);
        assert_eq!(response.max_text_length, DEFAULT_MAX_TEXT_LENGTH);
        assert_eq!(response.runtime_requirements.required_models.len(), 0);
        assert_eq!(response.runtime_requirements.estimated_peak_ram_mb, Some(0));
    }

    #[test]
    fn test_parse_embedding_payload_rejects_non_numeric() {
        let payload = serde_json::json!({
            "data": [{ "embedding": [0.1, "oops", 0.3] }]
        });
        let err = parse_embedding_payload(&payload).expect_err("must reject malformed vector");
        assert!(err.to_string().contains("invalid embedding value"));
    }
}

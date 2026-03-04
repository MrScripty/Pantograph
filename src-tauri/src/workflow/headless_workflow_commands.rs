//! Headless workflow API adapter for Tauri transport.
//!
//! This module maps Tauri command invocations to host-agnostic service logic in
//! `pantograph-workflow-service`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_workflow_service::{
    capabilities, WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse, WorkflowHost,
    WorkflowHostModelDescriptor, WorkflowIoRequest, WorkflowIoResponse, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowPreflightRequest, WorkflowPreflightResponse, WorkflowRunRequest,
    WorkflowRunResponse, WorkflowService, WorkflowServiceError, WorkflowSessionCloseRequest,
    WorkflowSessionCloseResponse, WorkflowSessionCreateRequest, WorkflowSessionCreateResponse,
    WorkflowSessionRunRequest,
};
use tauri::State;
use uuid::Uuid;

use crate::agent::rag::SharedRagManager;
use crate::llm::SharedGateway;

use super::commands::SharedExtensions;
use super::task_executor::TauriTaskExecutor;
pub type SharedWorkflowService = Arc<WorkflowService>;

const DEFAULT_MAX_INPUT_BINDINGS: usize = 128;
const DEFAULT_MAX_OUTPUT_TARGETS: usize = 128;
const DEFAULT_MAX_VALUE_BYTES: usize = 32_768;

fn workflow_error_json(error: WorkflowServiceError) -> String {
    error.to_envelope_json()
}

#[derive(Clone, Default)]
struct RuntimeExtensionsSnapshot {
    pumas_api: Option<Arc<pumas_library::PumasApi>>,
    kv_cache_store: Option<Arc<inference::kv_cache::KvCacheStore>>,
    dependency_resolver: Option<Arc<dyn node_engine::ModelDependencyResolver>>,
}

async fn snapshot_runtime_extensions(extensions: &SharedExtensions) -> RuntimeExtensionsSnapshot {
    let shared = extensions.read().await;
    RuntimeExtensionsSnapshot {
        pumas_api: shared
            .get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
            .cloned(),
        kv_cache_store: shared
            .get::<Arc<inference::kv_cache::KvCacheStore>>(
                node_engine::extension_keys::KV_CACHE_STORE,
            )
            .cloned(),
        dependency_resolver: shared
            .get::<Arc<dyn node_engine::ModelDependencyResolver>>(
                node_engine::extension_keys::MODEL_DEPENDENCY_RESOLVER,
            )
            .cloned(),
    }
}

fn apply_runtime_extensions(
    executor: &mut node_engine::WorkflowExecutor,
    snapshot: &RuntimeExtensionsSnapshot,
) {
    if let Some(api) = &snapshot.pumas_api {
        executor
            .extensions_mut()
            .set(node_engine::extension_keys::PUMAS_API, api.clone());
    }
    if let Some(store) = &snapshot.kv_cache_store {
        executor
            .extensions_mut()
            .set(node_engine::extension_keys::KV_CACHE_STORE, store.clone());
    }
    if let Some(resolver) = &snapshot.dependency_resolver {
        executor.extensions_mut().set(
            node_engine::extension_keys::MODEL_DEPENDENCY_RESOLVER,
            resolver.clone(),
        );
    }
}

pub async fn workflow_run(
    request: WorkflowRunRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunResponse, String> {
    let host = TauriWorkflowHost::with_rag_manager(
        gateway.inner().clone(),
        extensions.inner().clone(),
        rag_manager.inner().clone(),
    );
    workflow_service
        .workflow_run(&host, request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_capabilities(
    request: WorkflowCapabilitiesRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowCapabilitiesResponse, String> {
    let host = TauriWorkflowHost::new(gateway.inner().clone(), extensions.inner().clone());
    workflow_service
        .workflow_get_capabilities(&host, request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_io(
    request: WorkflowIoRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowIoResponse, String> {
    let host = TauriWorkflowHost::new(gateway.inner().clone(), extensions.inner().clone());
    workflow_service
        .workflow_get_io(&host, request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_preflight(
    request: WorkflowPreflightRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowPreflightResponse, String> {
    let host = TauriWorkflowHost::new(gateway.inner().clone(), extensions.inner().clone());
    workflow_service
        .workflow_preflight(&host, request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_create_session(
    request: WorkflowSessionCreateRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionCreateResponse, String> {
    let host = TauriWorkflowHost::new(gateway.inner().clone(), extensions.inner().clone());
    workflow_service
        .create_workflow_session(&host, request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_run_session(
    request: WorkflowSessionRunRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunResponse, String> {
    let host = TauriWorkflowHost::with_rag_manager(
        gateway.inner().clone(),
        extensions.inner().clone(),
        rag_manager.inner().clone(),
    );
    workflow_service
        .run_workflow_session(&host, request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_close_session(
    request: WorkflowSessionCloseRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionCloseResponse, String> {
    workflow_service
        .close_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

struct TauriWorkflowHost {
    gateway: SharedGateway,
    extensions: SharedExtensions,
    rag_manager: Option<SharedRagManager>,
}

impl TauriWorkflowHost {
    fn new(gateway: SharedGateway, extensions: SharedExtensions) -> Self {
        Self {
            gateway,
            extensions,
            rag_manager: None,
        }
    }

    fn with_rag_manager(
        gateway: SharedGateway,
        extensions: SharedExtensions,
        rag_manager: SharedRagManager,
    ) -> Self {
        Self {
            gateway,
            extensions,
            rag_manager: Some(rag_manager),
        }
    }

    async fn pumas_api(&self) -> Option<Arc<pumas_library::PumasApi>> {
        let ext = self.extensions.read().await;
        ext.get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
            .cloned()
    }

    fn apply_input_bindings(
        graph: &mut node_engine::WorkflowGraph,
        inputs: &[WorkflowPortBinding],
    ) -> Result<(), WorkflowServiceError> {
        for binding in inputs {
            let node = graph
                .nodes
                .iter_mut()
                .find(|node| node.id == binding.node_id)
                .ok_or_else(|| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "input binding references unknown node_id '{}'",
                        binding.node_id
                    ))
                })?;

            if node.data.is_null() {
                node.data = serde_json::json!({});
            }

            let map = node.data.as_object_mut().ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "input node '{}' has non-object data payload",
                    binding.node_id
                ))
            })?;
            map.insert(binding.port_id.clone(), binding.value.clone());
        }

        Ok(())
    }

    fn resolve_output_node_ids(
        graph: &node_engine::WorkflowGraph,
        output_targets: Option<&[WorkflowOutputTarget]>,
    ) -> Result<Vec<String>, WorkflowServiceError> {
        if let Some(targets) = output_targets {
            let known_nodes = graph
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<HashSet<_>>();
            let mut dedup = HashSet::new();
            let mut node_ids = Vec::new();

            for target in targets {
                if !known_nodes.contains(target.node_id.as_str()) {
                    return Err(WorkflowServiceError::InvalidRequest(format!(
                        "output target references unknown node_id '{}'",
                        target.node_id
                    )));
                }
                if dedup.insert(target.node_id.clone()) {
                    node_ids.push(target.node_id.clone());
                }
            }
            return Ok(node_ids);
        }

        let output_node_ids = graph
            .nodes
            .iter()
            .filter(|node| node.node_type.ends_with("-output"))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if output_node_ids.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow has no output nodes; add explicit `*-output` nodes or provide output_targets"
                    .to_string(),
            ));
        }

        Ok(output_node_ids)
    }

    fn collect_run_outputs(
        node_outputs: &HashMap<String, HashMap<String, serde_json::Value>>,
        output_node_ids: &[String],
        output_targets: Option<&[WorkflowOutputTarget]>,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if let Some(targets) = output_targets {
            let mut outputs = Vec::with_capacity(targets.len());
            for target in targets {
                let value = node_outputs
                    .get(&target.node_id)
                    .and_then(|ports| ports.get(&target.port_id))
                    .cloned()
                    .ok_or_else(|| {
                        WorkflowServiceError::Internal(format!(
                            "workflow output '{}.{}' was not produced",
                            target.node_id, target.port_id
                        ))
                    })?;

                outputs.push(WorkflowPortBinding {
                    node_id: target.node_id.clone(),
                    port_id: target.port_id.clone(),
                    value,
                });
            }
            return Ok(outputs);
        }

        let mut outputs = Vec::new();
        for node_id in output_node_ids {
            let Some(ports) = node_outputs.get(node_id) else {
                continue;
            };

            let mut keys = ports.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for port_id in keys {
                if let Some(value) = ports.get(&port_id) {
                    outputs.push(WorkflowPortBinding {
                        node_id: node_id.clone(),
                        port_id,
                        value: value.clone(),
                    });
                }
            }
        }

        Ok(outputs)
    }
}

#[async_trait]
impl WorkflowHost for TauriWorkflowHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        capabilities::default_workflow_roots(Path::new(env!("CARGO_MANIFEST_DIR")))
    }

    fn max_input_bindings(&self) -> usize {
        DEFAULT_MAX_INPUT_BINDINGS
    }

    fn max_output_targets(&self) -> usize {
        DEFAULT_MAX_OUTPUT_TARGETS
    }

    fn max_value_bytes(&self) -> usize {
        DEFAULT_MAX_VALUE_BYTES
    }

    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok(self.gateway.current_backend_name().await)
    }

    async fn model_metadata(
        &self,
        model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        let Some(api) = self.pumas_api().await else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| m.metadata))
    }

    async fn model_descriptor(
        &self,
        model_id: &str,
    ) -> Result<Option<WorkflowHostModelDescriptor>, WorkflowServiceError> {
        let Some(api) = self.pumas_api().await else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| WorkflowHostModelDescriptor {
            model_type: Some(m.model_type.trim().to_string()).filter(|v| !v.is_empty()),
            hashes: m.hashes,
        }))
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: pantograph_workflow_service::WorkflowRunOptions,
        run_handle: pantograph_workflow_service::WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if run_handle.is_cancelled() {
            return Err(WorkflowServiceError::RuntimeTimeout(
                "workflow run cancelled before execution started".to_string(),
            ));
        }

        let rag_manager = self.rag_manager.clone().ok_or_else(|| {
            WorkflowServiceError::Internal(
                "workflow execution host requires rag manager state".to_string(),
            )
        })?;

        let stored = capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots())?;
        let mut graph = stored.to_workflow_graph(workflow_id);
        Self::apply_input_bindings(&mut graph, inputs)?;

        let output_node_ids = Self::resolve_output_node_ids(&graph, output_targets)?;
        let runtime_ext = snapshot_runtime_extensions(&self.extensions).await;

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let project_root = Path::new(manifest_dir)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let execution_id = Uuid::new_v4().to_string();
        let core = Arc::new(
            node_engine::CoreTaskExecutor::new()
                .with_project_root(project_root)
                .with_gateway(self.gateway.inner_arc())
                .with_execution_id(execution_id.clone()),
        );
        let host = Arc::new(TauriTaskExecutor::new(rag_manager));
        let task_executor = node_engine::CompositeTaskExecutor::new(
            Some(host as Arc<dyn node_engine::TaskExecutor>),
            core,
        );

        let mut executor = node_engine::WorkflowExecutor::new(
            execution_id,
            graph,
            Arc::new(node_engine::NullEventSink),
        );
        apply_runtime_extensions(&mut executor, &runtime_ext);

        let mut node_outputs = HashMap::new();
        for node_id in &output_node_ids {
            if run_handle.is_cancelled() {
                return Err(WorkflowServiceError::RuntimeTimeout(
                    "workflow run cancelled during execution".to_string(),
                ));
            }
            let outputs = executor
                .demand(node_id, &task_executor)
                .await
                .map_err(|e| WorkflowServiceError::Internal(e.to_string()))?;
            node_outputs.insert(node_id.clone(), outputs);
        }

        Self::collect_run_outputs(&node_outputs, &output_node_ids, output_targets)
    }
}

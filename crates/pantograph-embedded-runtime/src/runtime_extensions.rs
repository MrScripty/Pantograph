use std::sync::Arc;

use node_engine::{EventSink, ExecutorExtensions, WorkflowExecutor};
use tokio::sync::RwLock;
use workflow_nodes::setup::{PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};

use crate::SharedWorkflowService;

pub type SharedExtensions = Arc<RwLock<ExecutorExtensions>>;

#[derive(Clone, Default)]
pub struct RuntimeExtensionsSnapshot {
    pub pumas_api: Option<Arc<pumas_library::PumasApi>>,
    pub pumas_selector_access: Option<Arc<PumasSelectorAccess>>,
    pub kv_cache_store: Option<Arc<inference::kv_cache::KvCacheStore>>,
    pub workflow_service: Option<SharedWorkflowService>,
}

impl RuntimeExtensionsSnapshot {
    pub async fn from_shared(shared: &SharedExtensions) -> Self {
        let guard = shared.read().await;
        Self::from_extensions(&guard)
    }

    pub async fn from_shared_with_workflow_service(
        shared: &SharedExtensions,
        workflow_service: SharedWorkflowService,
    ) -> Self {
        let mut snapshot = Self::from_shared(shared).await;
        snapshot.workflow_service = Some(workflow_service);
        snapshot
    }

    pub fn from_extensions(shared: &ExecutorExtensions) -> Self {
        Self {
            pumas_api: shared
                .get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
                .cloned(),
            pumas_selector_access: shared
                .get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
                .cloned(),
            kv_cache_store: shared
                .get::<Arc<inference::kv_cache::KvCacheStore>>(
                    node_engine::extension_keys::KV_CACHE_STORE,
                )
                .cloned(),
            workflow_service: shared
                .get::<SharedWorkflowService>(
                    crate::task_executor::runtime_extension_keys::WORKFLOW_SERVICE,
                )
                .cloned(),
        }
    }
}

pub fn apply_runtime_extensions(
    executor: &mut WorkflowExecutor,
    snapshot: &RuntimeExtensionsSnapshot,
) {
    apply_runtime_extensions_for_execution(executor, snapshot, None, None, None, None);
}

pub fn apply_runtime_extensions_for_execution(
    executor: &mut WorkflowExecutor,
    snapshot: &RuntimeExtensionsSnapshot,
    event_sink: Option<Arc<dyn EventSink>>,
    execution_id: Option<String>,
    python_runtime_execution_recorder: Option<
        Arc<crate::task_executor::PythonRuntimeExecutionRecorder>,
    >,
    inference_lifecycle_sink: Option<Arc<dyn inference::InferenceRequestLifecycleEventSink>>,
) {
    if let Some(api) = &snapshot.pumas_api {
        executor
            .extensions_mut()
            .set(node_engine::extension_keys::PUMAS_API, api.clone());
    }
    if let Some(selector_access) = &snapshot.pumas_selector_access {
        executor
            .extensions_mut()
            .set(PUMAS_SELECTOR_ACCESS, selector_access.clone());
    }
    if let Some(store) = &snapshot.kv_cache_store {
        executor
            .extensions_mut()
            .set(node_engine::extension_keys::KV_CACHE_STORE, store.clone());
    }
    if let Some(workflow_service) = &snapshot.workflow_service {
        executor.extensions_mut().set(
            crate::task_executor::runtime_extension_keys::WORKFLOW_SERVICE,
            workflow_service.clone(),
        );
    }
    if let Some(event_sink) = event_sink {
        executor.extensions_mut().set(
            crate::task_executor::runtime_extension_keys::EVENT_SINK,
            event_sink,
        );
    }
    if let Some(execution_id) = execution_id {
        executor.extensions_mut().set(
            crate::task_executor::runtime_extension_keys::EXECUTION_ID,
            execution_id,
        );
    }
    if let Some(recorder) = python_runtime_execution_recorder {
        executor.extensions_mut().set(
            crate::task_executor::runtime_extension_keys::PYTHON_RUNTIME_EXECUTION_RECORDER,
            recorder,
        );
    }
    if let Some(sink) = inference_lifecycle_sink {
        executor
            .extensions_mut()
            .set(node_engine::extension_keys::INFERENCE_LIFECYCLE_SINK, sink);
    }
}

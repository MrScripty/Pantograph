use std::sync::Arc;

use node_engine::{EventSink, ExecutorExtensions, WorkflowExecutor};
use tokio::sync::RwLock;

pub type SharedExtensions = Arc<RwLock<ExecutorExtensions>>;

#[derive(Clone, Default)]
pub struct RuntimeExtensionsSnapshot {
    pub pumas_api: Option<Arc<pumas_library::PumasApi>>,
    pub kv_cache_store: Option<Arc<inference::kv_cache::KvCacheStore>>,
    pub dependency_resolver: Option<Arc<dyn node_engine::ModelDependencyResolver>>,
}

impl RuntimeExtensionsSnapshot {
    pub async fn from_shared(shared: &SharedExtensions) -> Self {
        let guard = shared.read().await;
        Self::from_extensions(&guard)
    }

    pub fn from_extensions(shared: &ExecutorExtensions) -> Self {
        Self {
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
}

pub fn apply_runtime_extensions(
    executor: &mut WorkflowExecutor,
    snapshot: &RuntimeExtensionsSnapshot,
) {
    apply_runtime_extensions_for_execution(executor, snapshot, None, None, None);
}

pub fn apply_runtime_extensions_for_execution(
    executor: &mut WorkflowExecutor,
    snapshot: &RuntimeExtensionsSnapshot,
    event_sink: Option<Arc<dyn EventSink>>,
    execution_id: Option<String>,
    python_runtime_execution_recorder: Option<
        Arc<crate::task_executor::PythonRuntimeExecutionRecorder>,
    >,
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
}

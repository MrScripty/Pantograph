use std::sync::Arc;

use node_engine::{OrchestrationStore, TaskExecutor, WorkflowExecutor};
use rustler::Resource;

/// Wrapper for WorkflowExecutor shared via ResourceArc.
pub struct WorkflowExecutorResource {
    pub executor: Arc<tokio::sync::RwLock<WorkflowExecutor>>,
    pub task_executor: Arc<dyn TaskExecutor>,
    pub runtime: Arc<tokio::runtime::Runtime>,
}
impl Resource for WorkflowExecutorResource {}

/// Wrapper for OrchestrationStore shared via ResourceArc.
pub struct OrchestrationStoreResource {
    pub store: Arc<tokio::sync::RwLock<OrchestrationStore>>,
}
impl Resource for OrchestrationStoreResource {}

/// Wrapper for NodeRegistry shared via ResourceArc.
pub struct NodeRegistryResource {
    pub registry: Arc<tokio::sync::RwLock<node_engine::NodeRegistry>>,
}
impl Resource for NodeRegistryResource {}

/// Wrapper for PumasApi shared via ResourceArc.
pub struct PumasApiResource {
    pub api: Arc<pumas_library::PumasApi>,
    pub runtime: Arc<tokio::runtime::Runtime>,
}
impl Resource for PumasApiResource {}

/// Wrapper for ExecutorExtensions shared via ResourceArc.
///
/// Extensions hold optional runtime dependencies (e.g. PumasApi) that
/// port options providers need. Initialized via `extensions_setup`.
pub struct ExtensionsResource {
    pub extensions: Arc<tokio::sync::RwLock<node_engine::ExecutorExtensions>>,
    pub runtime: Arc<tokio::runtime::Runtime>,
}
impl Resource for ExtensionsResource {}

/// Wrapper for InferenceGateway shared via ResourceArc.
///
/// The gateway manages the llama.cpp server lifecycle and should outlive
/// individual executors so the model stays loaded across demand cycles.
/// Create once at app startup, pass to every executor via
/// `executor_new_with_inference`.
pub struct InferenceGatewayResource {
    pub gateway: Arc<inference::InferenceGateway>,
    pub runtime: Arc<tokio::runtime::Runtime>,
}
impl Resource for InferenceGatewayResource {}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use pantograph_runtime_registry::SharedRuntimeRegistry;
use pantograph_workflow_service::{WorkflowRuntimeCapability, WorkflowService};
#[cfg(test)]
use pantograph_workflow_service::{
    WorkflowSchedulerDiagnosticsProvider, WorkflowSchedulerRuntimeDiagnosticsRequest,
    WorkflowSchedulerRuntimeRegistryDiagnostics,
};
mod embedded_data_graph_execution;
mod embedded_edit_session_execution;
mod embedded_runtime_lifecycle;
mod embedded_workflow_graph_api;
mod embedded_workflow_host;
mod embedded_workflow_host_helpers;
mod embedded_workflow_service_api;
pub mod embedding_workflow;
pub mod host_runtime;
pub mod managed_runtime_manager;
pub mod model_dependencies;
mod node_execution;
mod node_execution_diagnostics;
pub mod python_runtime;
mod python_runtime_execution;
pub mod rag;
pub mod runtime_capabilities;
mod runtime_config;
mod runtime_extensions;
pub mod runtime_health;
pub mod runtime_recovery;
pub mod runtime_registry;
mod runtime_registry_controller;
mod runtime_registry_errors;
mod runtime_registry_lifecycle;
mod runtime_registry_observations;
pub mod task_executor;
pub mod technical_fit;
mod workflow_execution_session_execution;
pub mod workflow_runtime;
mod workflow_scheduler_diagnostics;

pub use embedded_edit_session_execution::EditSessionGraphExecutionOutcome;
pub use host_runtime::HostRuntimeModeSnapshot;
pub use managed_runtime_manager::{
    cancel_managed_runtime_manager_job, inspect_managed_runtime_manager_runtime,
    install_managed_runtime_manager_runtime, list_managed_runtime_manager_runtimes,
    pause_managed_runtime_manager_job, refresh_managed_runtime_manager_catalog_views,
    remove_managed_runtime_manager_runtime, select_managed_runtime_manager_version,
    set_default_managed_runtime_manager_version_view, ManagedRuntimeManagerProgress,
    ManagedRuntimeManagerRuntimeView,
};
pub use model_dependencies::{SharedModelDependencyResolver, TauriModelDependencyResolver};
pub use node_execution::{
    CacheCapability, DiagnosticsCapability, ExternalToolCapability, ManagedCapabilityKind,
    ManagedCapabilityRoute, ModelExecutionCapability, NodeCancellationToken, NodeExecutionContext,
    NodeExecutionContextInput, NodeExecutionError, NodeExecutionGuarantee,
    NodeExecutionGuaranteeEvidence, NodeExecutionInput, NodeExecutionOutput, NodeExecutionResult,
    NodeLineageContext, NodeManagedCapabilities, NodeOutputSummary, NodeProgressEvent,
    NodeProgressHandle, ResourceAccessCapability,
};
pub use node_execution_diagnostics::{
    adapt_node_engine_diagnostic_event, NodeExecutionDiagnosticEvent,
    NodeExecutionDiagnosticEventKind, NodeExecutionDiagnosticsRecorder,
};
pub use python_runtime::{
    ProcessPythonRuntimeAdapter, PythonNodeExecutionRequest, PythonRuntimeAdapter,
    PythonStreamHandler,
};
pub use rag::{RagBackend, RagDocument};
#[cfg(feature = "standalone")]
pub use runtime_config::StandaloneRuntimeConfig;
pub use runtime_config::{EmbeddedRuntimeConfig, EmbeddedRuntimeError};
pub use runtime_extensions::{
    apply_runtime_extensions, apply_runtime_extensions_for_execution, RuntimeExtensionsSnapshot,
    SharedExtensions,
};
pub use task_executor::{runtime_extension_keys, TauriTaskExecutor as PantographTaskExecutor};
pub(crate) use workflow_scheduler_diagnostics::EmbeddedWorkflowSchedulerDiagnosticsProvider;

pub type SharedWorkflowService = Arc<WorkflowService>;

const RUNTIME_WARMUP_POLL_INTERVAL_MS: u64 = 25;

#[cfg(not(test))]
const RUNTIME_WARMUP_WAIT_TIMEOUT_MS: u64 = 5_000;

#[cfg(test)]
const RUNTIME_WARMUP_WAIT_TIMEOUT_MS: u64 = 250;

pub struct EmbeddedRuntime {
    config: EmbeddedRuntimeConfig,
    gateway: Arc<inference::InferenceGateway>,
    extensions: SharedExtensions,
    workflow_service: SharedWorkflowService,
    runtime_registry: Option<SharedRuntimeRegistry>,
    session_runtime_reservations: Arc<Mutex<HashMap<String, u64>>>,
    session_executions:
        Arc<workflow_execution_session_execution::WorkflowExecutionSessionExecutionStore>,
    rag_backend: Option<Arc<dyn RagBackend>>,
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
    additional_runtime_capabilities: Vec<WorkflowRuntimeCapability>,
}

pub(crate) struct EmbeddedWorkflowHost {
    app_data_dir: PathBuf,
    project_root: PathBuf,
    workflow_roots: Vec<PathBuf>,
    gateway: Arc<inference::InferenceGateway>,
    extensions: SharedExtensions,
    runtime_registry: Option<SharedRuntimeRegistry>,
    session_runtime_reservations: Arc<Mutex<HashMap<String, u64>>>,
    session_executions:
        Arc<workflow_execution_session_execution::WorkflowExecutionSessionExecutionStore>,
    rag_backend: Option<Arc<dyn RagBackend>>,
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
    additional_runtime_capabilities: Vec<WorkflowRuntimeCapability>,
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

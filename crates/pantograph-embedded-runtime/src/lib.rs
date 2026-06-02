use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use pantograph_runtime_registry::SharedRuntimeRegistry;
use pantograph_workflow_service::{
    WorkflowRuntimeCapability, WorkflowService, WorkflowSessionRuntimeLoadProof,
};
#[cfg(test)]
use pantograph_workflow_service::{
    WorkflowSchedulerDiagnosticsProvider, WorkflowSchedulerRuntimeDiagnosticsRequest,
    WorkflowSchedulerRuntimeRegistryDiagnostics,
};
mod dependency_environment_probe_selector;
mod dependency_environment_probe_snapshot;
mod dependency_inventory;
#[cfg(any(test, feature = "standalone"))]
mod dependency_inventory_device_toolchain;
#[cfg(any(test, feature = "standalone"))]
mod dependency_inventory_device_toolchain_source;
mod dependency_inventory_dispatch;
#[cfg(any(test, feature = "standalone"))]
mod dependency_inventory_managed_runtime;
mod dependency_inventory_python;
#[cfg(any(test, feature = "standalone"))]
mod dependency_inventory_runtime_feature;
#[cfg(any(test, feature = "standalone"))]
mod dependency_inventory_runtime_feature_source;
#[cfg(any(test, feature = "standalone"))]
mod dependency_inventory_system_package;
#[cfg(any(test, feature = "standalone"))]
mod dependency_inventory_system_package_source;
#[cfg(test)]
mod dependency_inventory_tests;
pub mod dependency_readiness;
mod dependency_readiness_lifecycle;
#[cfg(test)]
mod dependency_readiness_lifecycle_tests;
mod embedded_data_graph_execution;
mod embedded_edit_session_execution;
mod embedded_runtime_lifecycle;
mod embedded_workflow_graph_api;
mod embedded_workflow_host;
mod embedded_workflow_host_helpers;
mod embedded_workflow_service_api;
pub mod embedding_workflow;
pub mod host_runtime;
mod inference_interface_facts_provider;
mod inference_resource_estimator;
pub mod managed_runtime_manager;
mod media_base64;
mod model_dependency_activity;
mod node_execution;
mod node_execution_diagnostics;
mod node_execution_ledger;
mod node_io_artifacts;
pub mod package_readiness_provider;
#[allow(dead_code)]
mod pumas_dispatch_package_facts;
pub mod python_package_readiness_probe;
pub mod python_runtime;
mod python_runtime_env_resolution;
mod python_runtime_execution;
pub mod rag;
#[allow(dead_code)]
mod reservation_lifecycle;
pub mod runtime_capabilities;
mod runtime_config;
#[allow(dead_code)]
mod runtime_dispatch_candidate_provider;
#[allow(dead_code)]
mod runtime_dispatch_capability_facts;
#[allow(dead_code)]
mod runtime_dispatch_resource_facts;
#[allow(dead_code)]
mod runtime_dispatch_source_snapshot;
mod runtime_extensions;
pub mod runtime_health;
#[allow(dead_code)]
mod runtime_host_execution_port;
#[allow(dead_code)]
mod runtime_host_image_execution;
mod runtime_host_load_target;
#[allow(dead_code)]
mod runtime_host_media_artifact_sink;
#[allow(dead_code)]
mod runtime_host_package_facts;
pub mod runtime_recovery;
pub mod runtime_registry;
mod runtime_registry_controller;
mod runtime_registry_errors;
mod runtime_registry_lifecycle;
mod runtime_registry_observations;
pub mod task_executor;
pub mod technical_fit;
mod workflow_event_identity;
mod workflow_execution_session_execution;
pub mod workflow_runtime;
mod workflow_scheduler_diagnostics;
#[allow(dead_code)]
mod workflow_service_composition;

pub use dependency_readiness_lifecycle::{
    EmbeddedDependencyReadinessSnapshotProducer, EmbeddedDependencyReadinessSnapshotProducerConfig,
    EmbeddedDependencyReadinessSnapshotProducerHandle,
};
pub use embedded_edit_session_execution::EditSessionGraphExecutionOutcome;
pub use host_runtime::HostRuntimeModeSnapshot;
pub use managed_runtime_manager::{
    cancel_managed_runtime_manager_job, inspect_managed_runtime_manager_runtime,
    install_managed_runtime_manager_runtime, list_managed_runtime_manager_runtimes,
    pause_managed_runtime_manager_job, refresh_managed_runtime_manager_catalog_views,
    remove_managed_runtime_manager_runtime, remove_managed_runtime_manager_runtime_version,
    select_managed_runtime_manager_version, set_default_managed_runtime_manager_version_view,
    ManagedRuntimeManagerProgress, ManagedRuntimeManagerRuntimeView,
};
pub use model_dependency_activity::{DependencyActivityEvent, DependencyActivityHub};
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
pub use node_execution_ledger::{
    inference_diagnostic_event_ledger_append_request,
    inference_lifecycle_event_ledger_append_request, InferenceLifecycleLedgerRecorder,
    InferenceLifecycleWorkflowLedgerSink, ManagedModelUsageSubmission,
    NodeExecutionWorkflowLedgerSink, RuntimeLedgerSubmissionError, SubmittedModelUsageEvent,
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
pub use workflow_service_composition::{
    EmbeddedHostedStartupCompositionInput, EmbeddedHostedStartupCompositionOutput,
    EmbeddedHostedStartupPumasSelectorSource, EmbeddedWorkflowServiceComposition,
};

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
    dependency_readiness_snapshot_producer:
        Option<dependency_readiness_lifecycle::EmbeddedDependencyReadinessSnapshotProducerHandle>,
    session_runtime_reservations: Arc<Mutex<HashMap<String, u64>>>,
    session_runtime_load_proofs: Arc<Mutex<HashMap<String, WorkflowSessionRuntimeLoadProof>>>,
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
    workflow_service: SharedWorkflowService,
    runtime_registry: Option<SharedRuntimeRegistry>,
    session_runtime_reservations: Arc<Mutex<HashMap<String, u64>>>,
    session_runtime_load_proofs: Arc<Mutex<HashMap<String, WorkflowSessionRuntimeLoadProof>>>,
    session_executions:
        Arc<workflow_execution_session_execution::WorkflowExecutionSessionExecutionStore>,
    rag_backend: Option<Arc<dyn RagBackend>>,
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
    additional_runtime_capabilities: Vec<WorkflowRuntimeCapability>,
    node_event_sink: Option<Arc<dyn node_engine::EventSink>>,
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

#[cfg(test)]
mod package_readiness_provider_tests;

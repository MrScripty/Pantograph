#[cfg(test)]
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::time::Duration;

#[cfg(test)]
use crate::capabilities;
use crate::graph::GraphSessionStore;
#[cfg(test)]
use crate::graph::WorkflowGraphSessionStateView;
use crate::scheduler::WorkflowSessionStore;
#[cfg(test)]
use crate::technical_fit::WorkflowTechnicalFitOverride;
#[cfg(test)]
use crate::technical_fit::{WorkflowTechnicalFitDecision, WorkflowTechnicalFitRequest};

mod attribution_api;
mod contracts;
mod graph_api;
mod host;
mod io_contract;
mod preflight_api;
mod runtime_preflight;
mod service_config;
mod session_execution_api;
mod session_lifecycle_api;
mod session_queue_api;
mod session_runtime;
mod validation;
mod workflow_run_api;

pub use self::attribution_api::{WorkflowAttributedRunRequest, WorkflowAttributedRunResponse};
pub use self::contracts::*;
pub use self::host::{
    WorkflowHost, WorkflowSchedulerDiagnosticsProvider, WorkflowSchedulerRuntimeDiagnosticsRequest,
};
pub(crate) use self::runtime_preflight::runtime_issue_for_capability;
pub use self::runtime_preflight::{evaluate_runtime_preflight, format_runtime_not_ready_message};
pub(crate) use self::validation::validate_workflow_id;

pub use pantograph_runtime_attribution::{
    AttributionRepository, BucketSelection, ClientRegistrationRequest, ClientRegistrationResponse,
    ClientSessionOpenRequest, ClientSessionOpenResponse, ClientSessionRecord,
    ClientSessionResumeRequest, CredentialProofRequest, CredentialSecret, SqliteAttributionStore,
    WorkflowRunAttribution, WorkflowRunRecord,
};

#[cfg(test)]
use crate::graph::WorkflowSessionKind;
#[cfg(test)]
use crate::scheduler::unix_timestamp_ms;

pub(crate) use crate::scheduler::scheduler_snapshot_trace_execution_id;
pub use crate::scheduler::{
    select_runtime_unload_candidate_by_affinity, WorkflowSchedulerAdmissionOutcome,
    WorkflowSchedulerDecisionReason, WorkflowSchedulerRuntimeRegistryDiagnostics,
    WorkflowSchedulerRuntimeWarmupDecision, WorkflowSchedulerRuntimeWarmupReason,
    WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse,
    WorkflowSessionInspectionRequest, WorkflowSessionInspectionResponse,
    WorkflowSessionKeepAliveRequest, WorkflowSessionKeepAliveResponse,
    WorkflowSessionQueueCancelRequest, WorkflowSessionQueueCancelResponse,
    WorkflowSessionQueueItem, WorkflowSessionQueueItemStatus, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueListResponse, WorkflowSessionQueueReprioritizeRequest,
    WorkflowSessionQueueReprioritizeResponse, WorkflowSessionRetentionHint,
    WorkflowSessionRuntimeSelectionTarget, WorkflowSessionRuntimeUnloadCandidate,
    WorkflowSessionStaleCleanupRequest, WorkflowSessionStaleCleanupResponse,
    WorkflowSessionStaleCleanupWorker, WorkflowSessionStaleCleanupWorkerConfig,
    WorkflowSessionState, WorkflowSessionStatusRequest, WorkflowSessionStatusResponse,
    WorkflowSessionSummary, WorkflowSessionUnloadReason,
};

/// Service entrypoint for workflow API operations.
#[derive(Clone)]
pub struct WorkflowService {
    session_store: Arc<Mutex<WorkflowSessionStore>>,
    graph_session_store: Arc<GraphSessionStore>,
    attribution_store: Option<Arc<Mutex<SqliteAttributionStore>>>,
    scheduler_diagnostics_provider:
        Arc<Mutex<Option<Arc<dyn WorkflowSchedulerDiagnosticsProvider>>>>,
}

#[cfg(test)]
mod tests;

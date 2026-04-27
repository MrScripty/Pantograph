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
use crate::scheduler::WorkflowExecutionSessionStore;
#[cfg(test)]
use crate::technical_fit::WorkflowTechnicalFitOverride;
#[cfg(test)]
use crate::technical_fit::{WorkflowTechnicalFitDecision, WorkflowTechnicalFitRequest};

mod attribution_api;
mod contracts;
mod diagnostics_api;
mod graph_api;
mod host;
mod identity;
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

pub use self::contracts::*;
pub use self::diagnostics_api::{
    WorkflowDiagnosticsUsageQueryRequest, WorkflowDiagnosticsUsageQueryResponse,
    WorkflowDiagnosticsUsageSummary, WorkflowSchedulerTimelineQueryRequest,
    WorkflowSchedulerTimelineQueryResponse,
};
pub use self::host::{
    WorkflowHost, WorkflowSchedulerDiagnosticsProvider, WorkflowSchedulerRuntimeDiagnosticsRequest,
};
pub use self::identity::{WorkflowIdentity, WorkflowIdentityError};
pub(crate) use self::runtime_preflight::runtime_issue_for_capability;
pub use self::runtime_preflight::{evaluate_runtime_preflight, format_runtime_not_ready_message};
pub(crate) use self::validation::validate_workflow_id;

pub use pantograph_diagnostics_ledger::{
    ProjectionStateRecord, SchedulerTimelineProjectionRecord, SqliteDiagnosticsLedger,
    WorkflowTimingExpectation, WorkflowTimingExpectationComparison,
};
pub use pantograph_runtime_attribution::{
    AttributionRepository, BucketCreateRequest, BucketDeleteRequest, BucketRecord, BucketSelection,
    ClientRegistrationRequest, ClientRegistrationResponse, ClientSessionOpenRequest,
    ClientSessionOpenResponse, ClientSessionRecord, ClientSessionResumeRequest,
    CredentialProofRequest, CredentialSecret, SqliteAttributionStore,
    WorkflowPresentationRevisionRecord, WorkflowPresentationRevisionResolveRequest,
    WorkflowRunAttribution, WorkflowRunRecord, WorkflowRunSnapshotRecord,
    WorkflowRunSnapshotRequest, WorkflowRunVersionProjection, WorkflowVersionRecord,
    WorkflowVersionResolveRequest,
};

#[cfg(test)]
use crate::graph::WorkflowExecutionSessionKind;
#[cfg(test)]
use crate::scheduler::unix_timestamp_ms;

pub(crate) use crate::scheduler::scheduler_snapshot_workflow_run_id;
pub use crate::scheduler::{
    select_runtime_unload_candidate_by_affinity, WorkflowExecutionSessionInspectionRequest,
    WorkflowExecutionSessionInspectionResponse, WorkflowExecutionSessionKeepAliveRequest,
    WorkflowExecutionSessionKeepAliveResponse, WorkflowExecutionSessionQueueCancelRequest,
    WorkflowExecutionSessionQueueCancelResponse, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionQueueItemStatus, WorkflowExecutionSessionQueueListRequest,
    WorkflowExecutionSessionQueueListResponse, WorkflowExecutionSessionQueueReprioritizeRequest,
    WorkflowExecutionSessionQueueReprioritizeResponse, WorkflowExecutionSessionRetentionHint,
    WorkflowExecutionSessionRuntimeSelectionTarget, WorkflowExecutionSessionRuntimeUnloadCandidate,
    WorkflowExecutionSessionStaleCleanupRequest, WorkflowExecutionSessionStaleCleanupResponse,
    WorkflowExecutionSessionStaleCleanupWorker, WorkflowExecutionSessionStaleCleanupWorkerConfig,
    WorkflowExecutionSessionState, WorkflowExecutionSessionStatusRequest,
    WorkflowExecutionSessionStatusResponse, WorkflowExecutionSessionSummary,
    WorkflowExecutionSessionUnloadReason, WorkflowSchedulerAdmissionOutcome,
    WorkflowSchedulerDecisionReason, WorkflowSchedulerRuntimeRegistryDiagnostics,
    WorkflowSchedulerRuntimeWarmupDecision, WorkflowSchedulerRuntimeWarmupReason,
    WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse,
};

/// Service entrypoint for workflow API operations.
#[derive(Clone)]
pub struct WorkflowService {
    session_store: Arc<Mutex<WorkflowExecutionSessionStore>>,
    graph_session_store: Arc<GraphSessionStore>,
    attribution_store: Option<Arc<Mutex<SqliteAttributionStore>>>,
    diagnostics_ledger: Option<Arc<Mutex<SqliteDiagnosticsLedger>>>,
    scheduler_diagnostics_provider:
        Arc<Mutex<Option<Arc<dyn WorkflowSchedulerDiagnosticsProvider>>>>,
}

#[cfg(test)]
mod tests;

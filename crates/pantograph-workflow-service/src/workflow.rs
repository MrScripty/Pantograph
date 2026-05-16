#[cfg(test)]
use async_trait::async_trait;
use std::path::PathBuf;
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

mod artifact_api;
mod artifact_contracts;
mod artifact_output_conversion;
mod artifact_settings_api;
mod artifact_store;
mod attribution_api;
mod contracts;
#[allow(dead_code)]
mod diagnostic_errors;
mod diagnostics_api;
mod execution_plan;
mod execution_plan_admission;
mod execution_plan_model_ref;
mod execution_plan_selected_facts;
mod graph_api;
mod host;
mod identity;
mod io_contract;
mod local_network_api;
mod media_capability_contracts;
mod preflight_api;
mod runtime_preflight;
mod service_config;
mod session_execution_api;
mod session_io_artifacts;
mod session_lifecycle_api;
mod session_queue_api;
mod session_runtime;
mod session_runtime_load_lifecycle;
mod validation;
mod workflow_run_api;

pub use self::artifact_contracts::*;
pub use self::artifact_store::{
    ArtifactBodyRead, ArtifactStore, ArtifactStoreError, ArtifactStoreStats,
    ArtifactStreamChunkWriteRequest, ArtifactStreamFinalizeRequest, ArtifactStreamOpenRequest,
    ArtifactWriteRequest,
};
pub use self::contracts::*;
pub use self::diagnostics_api::{
    ResolvedNodeIoDirection, ResolvedNodeIoProvenanceKind, ResolvedNodeIoRecord,
    ResolvedNodeIoResolution, WorkflowDiagnosticEventRecordResponse,
    WorkflowDiagnosticsProjectionAdvance, WorkflowDiagnosticsProjectionFailure,
    WorkflowDiagnosticsProjectionInvalidation, WorkflowDiagnosticsProjectionKind,
    WorkflowDiagnosticsProjectionRefreshReason, WorkflowDiagnosticsProjectionRefreshRequest,
    WorkflowDiagnosticsProjectionRefreshResponse, WorkflowDiagnosticsProjectionRefreshSink,
    WorkflowDiagnosticsUsageQueryRequest, WorkflowDiagnosticsUsageQueryResponse,
    WorkflowDiagnosticsUsageSummary, WorkflowIoArtifactQueryRequest,
    WorkflowIoArtifactQueryResponse, WorkflowLibraryAssetAccessRecordRequest,
    WorkflowLibraryAssetAccessRecordResponse, WorkflowLibraryUsageQueryRequest,
    WorkflowLibraryUsageQueryResponse, WorkflowNodeStatusQueryRequest,
    WorkflowNodeStatusQueryResponse, WorkflowProjectionRebuildRequest,
    WorkflowProjectionRebuildResponse, WorkflowRetentionCleanupRequest,
    WorkflowRetentionCleanupResponse, WorkflowRetentionPolicyQueryRequest,
    WorkflowRetentionPolicyQueryResponse, WorkflowRetentionPolicyUpdateRequest,
    WorkflowRetentionPolicyUpdateResponse, WorkflowRunDetailQueryRequest,
    WorkflowRunDetailQueryResponse, WorkflowRunInspectionQueryRequest,
    WorkflowRunInspectionQueryResponse, WorkflowRunListQueryRequest, WorkflowRunListQueryResponse,
    WorkflowSchedulerEstimateQueryRequest, WorkflowSchedulerEstimateQueryResponse,
    WorkflowSchedulerEstimateRecord, WorkflowSchedulerTimelineQueryRequest,
    WorkflowSchedulerTimelineQueryResponse,
};
pub use self::execution_plan::{
    WorkflowExecutionPlan, WorkflowExecutionPlanDiagnostic, WorkflowExecutionPlanDiagnosticCode,
    WorkflowExecutionPlanDiagnosticSeverity, WorkflowExecutionPlanError,
    WorkflowExecutionPlanNodeDecision, WORKFLOW_EXECUTION_PLAN_MAX_DIAGNOSTICS,
    WORKFLOW_EXECUTION_PLAN_MAX_NODE_DECISIONS, WORKFLOW_EXECUTION_PLAN_MAX_POLICY_TRACE_IDS,
    WORKFLOW_EXECUTION_PLAN_SCHEMA_VERSION,
};
pub(crate) use self::execution_plan_admission::build_workflow_execution_plan_from_admission;
pub use self::execution_plan_model_ref::{
    WorkflowExecutionPlanModelRef, WorkflowExecutionPlanModelRefError,
};
pub use self::execution_plan_selected_facts::{
    WorkflowExecutionPlanBackendKey, WorkflowExecutionPlanDeviceId, WorkflowExecutionPlanRuntimeId,
    WorkflowExecutionPlanRuntimeVariantId, WorkflowExecutionPlanSelectedFactError,
};
pub use self::host::{
    WorkflowHost, WorkflowSchedulerDiagnosticsProvider, WorkflowSchedulerRuntimeDiagnosticsRequest,
};
pub use self::identity::{WorkflowIdentity, WorkflowIdentityError};
pub use self::media_capability_contracts::*;
pub(crate) use self::runtime_preflight::runtime_issue_for_capability;
pub use self::runtime_preflight::{evaluate_runtime_preflight, format_runtime_not_ready_message};
pub(crate) use self::validation::validate_workflow_id;

pub use pantograph_diagnostics_ledger::{
    IoArtifactProjectionRecord, IoArtifactRetentionState, IoArtifactRetentionSummaryRecord,
    LibraryAssetCacheStatus, LibraryAssetOperation, LibraryUsageProjectionRecord,
    ProjectionStateRecord, RunDetailProjectionRecord, RunListFacetRecord, RunListProjectionRecord,
    RunListProjectionStatus, SchedulerModelCacheState, SchedulerTimelineProjectionRecord,
    SqliteDiagnosticsLedger, WorkflowTimingExpectation, WorkflowTimingExpectationComparison,
};
pub use pantograph_runtime_attribution::{
    AttributionRepository, BucketCreateRequest, BucketDeleteRequest, BucketRecord, BucketSelection,
    ClientRegistrationRequest, ClientRegistrationResponse, ClientSessionOpenRequest,
    ClientSessionOpenResponse, ClientSessionRecord, ClientSessionResumeRequest,
    CredentialProofRequest, CredentialSecret, SqliteAttributionStore,
    WorkflowPresentationRevisionRecord, WorkflowPresentationRevisionResolveRequest,
    WorkflowRunAttribution, WorkflowRunAttributionContext, WorkflowRunAttributionResolveRequest,
    WorkflowRunRecord, WorkflowRunSnapshotRecord, WorkflowRunSnapshotRequest,
    WorkflowRunVersionProjection, WorkflowVersionRecord, WorkflowVersionResolveRequest,
};

#[cfg(test)]
use crate::graph::WorkflowExecutionSessionKind;
#[cfg(test)]
use crate::scheduler::unix_timestamp_ms;

pub(crate) use crate::scheduler::scheduler_snapshot_workflow_run_id;
pub use crate::scheduler::{
    select_runtime_unload_candidate_by_affinity, WorkflowAdminQueueCancelRequest,
    WorkflowAdminQueueCancelResponse, WorkflowAdminQueuePushFrontRequest,
    WorkflowAdminQueuePushFrontResponse, WorkflowAdminQueueReprioritizeRequest,
    WorkflowAdminQueueReprioritizeResponse, WorkflowExecutionSessionAttributionContext,
    WorkflowExecutionSessionInspectionRequest, WorkflowExecutionSessionInspectionResponse,
    WorkflowExecutionSessionKeepAliveRequest, WorkflowExecutionSessionKeepAliveResponse,
    WorkflowExecutionSessionQueueCancelRequest, WorkflowExecutionSessionQueueCancelResponse,
    WorkflowExecutionSessionQueueItem, WorkflowExecutionSessionQueueItemStatus,
    WorkflowExecutionSessionQueueListRequest, WorkflowExecutionSessionQueueListResponse,
    WorkflowExecutionSessionQueuePushFrontRequest, WorkflowExecutionSessionQueuePushFrontResponse,
    WorkflowExecutionSessionQueueReprioritizeRequest,
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
    artifact_store: Option<Arc<Mutex<ArtifactStore>>>,
    artifact_format_settings: Arc<Mutex<ArtifactFormatSettings>>,
    artifact_format_settings_path: Option<Arc<PathBuf>>,
    artifact_format_dependency_versions: Arc<Mutex<ArtifactFormatDependencyVersions>>,
    attribution_store: Option<Arc<Mutex<SqliteAttributionStore>>>,
    diagnostics_ledger: Option<Arc<Mutex<SqliteDiagnosticsLedger>>>,
    diagnostics_projection_refresh_sink:
        Arc<Mutex<Option<Arc<dyn WorkflowDiagnosticsProjectionRefreshSink>>>>,
    media_conversion_executor:
        Arc<Mutex<Option<Arc<dyn pantograph_media_conversion::MediaConversionExecutor>>>>,
    scheduler_diagnostics_provider:
        Arc<Mutex<Option<Arc<dyn WorkflowSchedulerDiagnosticsProvider>>>>,
}

#[cfg(test)]
mod tests;

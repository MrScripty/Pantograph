//! Durable model and license usage diagnostics ledger.
//!
//! This crate persists model/license usage records separately from transient
//! runtime trace diagnostics.

mod error;
mod event;
mod records;
mod repository;
mod schema;
mod sqlite;
mod timing;
mod util;

pub use error::DiagnosticsLedgerError;
pub use event::{
    sanitize_diagnostic_error_text, DiagnosticErrorLocation, DiagnosticErrorOccurredPayload,
    DiagnosticErrorRecoverability, DiagnosticErrorScopeKind, DiagnosticErrorSeverity,
    DiagnosticEventAppendRequest, DiagnosticEventKind, DiagnosticEventPayload,
    DiagnosticEventPrivacyClass, DiagnosticEventRecord, DiagnosticEventRetentionClass,
    DiagnosticEventSourceComponent, InferenceCompatibilityIssueDiagnosticSummary,
    InferenceCompatibilityReportDiagnosticSummary, InferenceExecutionDiagnosticObservedPayload,
    InferenceOptionDiagnosticSummary, InferenceOptionSupportCounts,
    InferenceUsageDiagnosticSummary, IoArtifactAccessMode, IoArtifactConversionDependency,
    IoArtifactConversionStatus, IoArtifactFormatMetadata, IoArtifactLifecycleState,
    IoArtifactObservedPayload, IoArtifactPayloadKind, IoArtifactProjectionQuery,
    IoArtifactProjectionRecord, IoArtifactRetentionState, IoArtifactRetentionSummaryQuery,
    IoArtifactRetentionSummaryRecord, IoArtifactRole, LibraryAssetAccessedPayload,
    LibraryAssetCacheStatus, LibraryAssetOperation, LibraryUsageProjectionQuery,
    LibraryUsageProjectionRecord, NodeExecutionProjectionStatus, NodeExecutionStatusPayload,
    NodeStatusProjectionQuery, NodeStatusProjectionRecord, ProjectionStateRecord,
    ProjectionStateUpdate, ProjectionStatus, RetentionArtifactStateChangedPayload,
    RetentionPolicyActorScope, RetentionPolicyChangedPayload, RunDetailProjectionQuery,
    RunDetailProjectionRecord, RunListFacetKind, RunListFacetRecord, RunListProjectionQuery,
    RunListProjectionRecord, RunListProjectionStatus, RunSnapshotAcceptedPayload,
    RunSnapshotNodeVersionPayload, RunStartedPayload, RunTerminalPayload, RunTerminalStatus,
    RuntimeCapabilityObservedPayload, SchedulerEstimateBlockingCondition,
    SchedulerEstimateProducedPayload, SchedulerModelCacheState,
    SchedulerModelLifecycleChangedPayload, SchedulerModelLifecycleTransition,
    SchedulerQueueControlAction, SchedulerQueueControlActorScope, SchedulerQueueControlOutcome,
    SchedulerQueueControlPayload, SchedulerQueuePlacementPayload,
    SchedulerReservationChangedPayload, SchedulerReservationResourceKind,
    SchedulerReservationTransition, SchedulerRunAdmittedPayload, SchedulerRunDelayedPayload,
    SchedulerTimelineProjectionQuery, SchedulerTimelineProjectionRecord,
    DIAGNOSTIC_EVENT_SCHEMA_VERSION, IO_ARTIFACT_PROJECTION_NAME, IO_ARTIFACT_PROJECTION_VERSION,
    LIBRARY_USAGE_PROJECTION_NAME, LIBRARY_USAGE_PROJECTION_VERSION,
    MAX_DIAGNOSTIC_ERROR_CAUSE_COUNT, MAX_DIAGNOSTIC_ERROR_CAUSE_LEN,
    MAX_DIAGNOSTIC_ERROR_TEXT_LEN, MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES,
    MAX_INFERENCE_COMPATIBILITY_ISSUES, MAX_INFERENCE_OPTION_DIAGNOSTICS,
    NODE_STATUS_PROJECTION_NAME, NODE_STATUS_PROJECTION_VERSION, RUN_DETAIL_PROJECTION_NAME,
    RUN_DETAIL_PROJECTION_VERSION, RUN_LIST_PROJECTION_NAME, RUN_LIST_PROJECTION_VERSION,
    SCHEDULER_TIMELINE_PROJECTION_NAME, SCHEDULER_TIMELINE_PROJECTION_VERSION,
};
pub use records::{
    ApplyArtifactRetentionPolicyCommand, ApplyArtifactRetentionPolicyResult, DiagnosticsProjection,
    DiagnosticsQuery, DiagnosticsRetentionCleanupTrigger, DiagnosticsRetentionCompressionBehavior,
    DiagnosticsRetentionMediaBehavior, DiagnosticsRetentionPayloadMode, DiagnosticsRetentionPolicy,
    DiagnosticsRetentionPolicySettings, DiagnosticsRetentionScopePolicy, ExecutionGuaranteeLevel,
    LicenseSnapshot, ModelIdentity, ModelLicenseUsageEvent, ModelOutputMeasurement,
    OutputMeasurementUnavailableReason, OutputModality, PruneUsageEventsCommand,
    PruneUsageEventsResult, RetentionClass, UpdateRetentionPolicyCommand, UsageEventStatus,
    UsageLineage, DEFAULT_STANDARD_RETENTION_DAYS, MAX_RETENTION_DAYS, MILLIS_PER_DAY,
};
pub use repository::DiagnosticsLedgerRepository;
pub use sqlite::SqliteDiagnosticsLedger;
pub use timing::{
    PruneTimingObservationsCommand, PruneTimingObservationsResult, WorkflowRunSummaryProjection,
    WorkflowRunSummaryQuery, WorkflowRunSummaryRecord, WorkflowRunSummaryStatus,
    WorkflowTimingExpectation, WorkflowTimingExpectationComparison, WorkflowTimingExpectationQuery,
    WorkflowTimingObservation, WorkflowTimingObservationScope, WorkflowTimingObservationStatus,
    MIN_TIMING_EXPECTATION_SAMPLE_COUNT,
};

#[cfg(test)]
mod tests;

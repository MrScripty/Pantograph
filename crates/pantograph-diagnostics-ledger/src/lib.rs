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
    DiagnosticEventAppendRequest, DiagnosticEventKind, DiagnosticEventPayload,
    DiagnosticEventPrivacyClass, DiagnosticEventRecord, DiagnosticEventRetentionClass,
    DiagnosticEventSourceComponent, IoArtifactObservedPayload, IoArtifactProjectionQuery,
    IoArtifactProjectionRecord, IoArtifactRetentionState, IoArtifactRetentionSummaryQuery,
    IoArtifactRetentionSummaryRecord, LibraryAssetAccessedPayload, LibraryUsageProjectionQuery,
    LibraryUsageProjectionRecord, NodeExecutionProjectionStatus, NodeExecutionStatusPayload,
    NodeStatusProjectionQuery, NodeStatusProjectionRecord, ProjectionStateRecord,
    ProjectionStateUpdate, ProjectionStatus, RetentionArtifactStateChangedPayload,
    RetentionPolicyChangedPayload, RunDetailProjectionQuery, RunDetailProjectionRecord,
    RunListFacetKind, RunListFacetRecord, RunListProjectionQuery, RunListProjectionRecord,
    RunListProjectionStatus, RunSnapshotAcceptedPayload, RunSnapshotNodeVersionPayload,
    RunStartedPayload, RunTerminalPayload, RunTerminalStatus, RuntimeCapabilityObservedPayload,
    SchedulerEstimateProducedPayload, SchedulerModelLifecycleChangedPayload,
    SchedulerModelLifecycleTransition, SchedulerQueuePlacementPayload, SchedulerRunDelayedPayload,
    SchedulerTimelineProjectionQuery, SchedulerTimelineProjectionRecord,
    DIAGNOSTIC_EVENT_SCHEMA_VERSION, IO_ARTIFACT_PROJECTION_NAME, IO_ARTIFACT_PROJECTION_VERSION,
    LIBRARY_USAGE_PROJECTION_NAME, LIBRARY_USAGE_PROJECTION_VERSION,
    MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES, NODE_STATUS_PROJECTION_NAME,
    NODE_STATUS_PROJECTION_VERSION, RUN_DETAIL_PROJECTION_NAME, RUN_DETAIL_PROJECTION_VERSION,
    RUN_LIST_PROJECTION_NAME, RUN_LIST_PROJECTION_VERSION, SCHEDULER_TIMELINE_PROJECTION_NAME,
    SCHEDULER_TIMELINE_PROJECTION_VERSION,
};
pub use records::{
    DiagnosticsProjection, DiagnosticsQuery, DiagnosticsRetentionPolicy, ExecutionGuaranteeLevel,
    LicenseSnapshot, ModelIdentity, ModelLicenseUsageEvent, ModelOutputMeasurement,
    OutputMeasurementUnavailableReason, OutputModality, PruneUsageEventsCommand,
    PruneUsageEventsResult, RetentionClass, UpdateRetentionPolicyCommand, UsageEventStatus,
    UsageLineage, DEFAULT_STANDARD_RETENTION_DAYS, MAX_RETENTION_DAYS,
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

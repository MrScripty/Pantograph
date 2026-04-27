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
    DiagnosticEventSourceComponent, IoArtifactObservedPayload, LibraryAssetAccessedPayload,
    ProjectionStateRecord, ProjectionStateUpdate, ProjectionStatus, RetentionPolicyChangedPayload,
    RunListProjectionQuery, RunListProjectionRecord, RunListProjectionStatus,
    RunSnapshotAcceptedPayload, RunStartedPayload, RunTerminalPayload, RunTerminalStatus,
    RuntimeCapabilityObservedPayload, SchedulerEstimateProducedPayload,
    SchedulerQueuePlacementPayload, SchedulerTimelineProjectionQuery,
    SchedulerTimelineProjectionRecord, DIAGNOSTIC_EVENT_SCHEMA_VERSION,
    MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES, RUN_LIST_PROJECTION_NAME, RUN_LIST_PROJECTION_VERSION,
    SCHEDULER_TIMELINE_PROJECTION_NAME, SCHEDULER_TIMELINE_PROJECTION_VERSION,
};
pub use records::{
    DiagnosticsProjection, DiagnosticsQuery, DiagnosticsRetentionPolicy, ExecutionGuaranteeLevel,
    LicenseSnapshot, ModelIdentity, ModelLicenseUsageEvent, ModelOutputMeasurement,
    OutputMeasurementUnavailableReason, OutputModality, PruneUsageEventsCommand,
    PruneUsageEventsResult, RetentionClass, UsageEventStatus, UsageLineage,
    DEFAULT_STANDARD_RETENTION_DAYS,
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

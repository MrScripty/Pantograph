//! Durable model and license usage diagnostics ledger.
//!
//! This crate persists model/license usage records separately from transient
//! runtime trace diagnostics.

mod error;
mod records;
mod repository;
mod schema;
mod sqlite;
mod timing;
mod util;

pub use error::DiagnosticsLedgerError;
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

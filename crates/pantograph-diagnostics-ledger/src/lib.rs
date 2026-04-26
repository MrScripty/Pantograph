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
    DEFAULT_STANDARD_RETENTION_DAYS, DiagnosticsProjection, DiagnosticsQuery,
    DiagnosticsRetentionPolicy, ExecutionGuaranteeLevel, LicenseSnapshot, ModelIdentity,
    ModelLicenseUsageEvent, ModelOutputMeasurement, OutputMeasurementUnavailableReason,
    OutputModality, PruneUsageEventsCommand, PruneUsageEventsResult, RetentionClass,
    UsageEventStatus, UsageLineage,
};
pub use repository::DiagnosticsLedgerRepository;
pub use sqlite::SqliteDiagnosticsLedger;
pub use timing::{
    MIN_TIMING_EXPECTATION_SAMPLE_COUNT, PruneTimingObservationsCommand,
    PruneTimingObservationsResult, WorkflowRunSummaryProjection, WorkflowRunSummaryQuery,
    WorkflowRunSummaryRecord, WorkflowRunSummaryStatus, WorkflowTimingExpectation,
    WorkflowTimingExpectationComparison, WorkflowTimingExpectationQuery, WorkflowTimingObservation,
    WorkflowTimingObservationScope, WorkflowTimingObservationStatus,
};

#[cfg(test)]
mod tests;

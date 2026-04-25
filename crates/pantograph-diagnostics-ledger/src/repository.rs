use crate::{
    DiagnosticsLedgerError, DiagnosticsProjection, DiagnosticsQuery, DiagnosticsRetentionPolicy,
    ModelLicenseUsageEvent, PruneTimingObservationsCommand, PruneTimingObservationsResult,
    PruneUsageEventsCommand, PruneUsageEventsResult, WorkflowTimingExpectation,
    WorkflowTimingExpectationQuery, WorkflowTimingObservation,
};

pub trait DiagnosticsLedgerRepository {
    fn record_usage_event(
        &mut self,
        event: ModelLicenseUsageEvent,
    ) -> Result<(), DiagnosticsLedgerError>;

    fn query_usage_events(
        &self,
        query: DiagnosticsQuery,
    ) -> Result<DiagnosticsProjection, DiagnosticsLedgerError>;

    fn retention_policy(&self) -> Result<DiagnosticsRetentionPolicy, DiagnosticsLedgerError>;

    fn prune_usage_events(
        &mut self,
        command: PruneUsageEventsCommand,
    ) -> Result<PruneUsageEventsResult, DiagnosticsLedgerError>;

    fn record_timing_observation(
        &mut self,
        observation: WorkflowTimingObservation,
    ) -> Result<(), DiagnosticsLedgerError>;

    fn timing_expectation(
        &self,
        query: WorkflowTimingExpectationQuery,
    ) -> Result<WorkflowTimingExpectation, DiagnosticsLedgerError>;

    fn prune_timing_observations(
        &mut self,
        command: PruneTimingObservationsCommand,
    ) -> Result<PruneTimingObservationsResult, DiagnosticsLedgerError>;
}

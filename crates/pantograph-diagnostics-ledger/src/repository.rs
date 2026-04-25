use crate::{
    DiagnosticsLedgerError, DiagnosticsProjection, DiagnosticsQuery, DiagnosticsRetentionPolicy,
    ModelLicenseUsageEvent, PruneUsageEventsCommand, PruneUsageEventsResult,
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
}

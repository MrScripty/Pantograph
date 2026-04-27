use crate::{
    DiagnosticEventAppendRequest, DiagnosticEventRecord, DiagnosticsLedgerError,
    DiagnosticsProjection, DiagnosticsQuery, DiagnosticsRetentionPolicy, ModelLicenseUsageEvent,
    ProjectionStateRecord, ProjectionStateUpdate, PruneTimingObservationsCommand,
    PruneTimingObservationsResult, PruneUsageEventsCommand, PruneUsageEventsResult,
    SchedulerTimelineProjectionQuery, SchedulerTimelineProjectionRecord,
    WorkflowRunSummaryProjection, WorkflowRunSummaryQuery, WorkflowRunSummaryRecord,
    WorkflowTimingExpectation, WorkflowTimingExpectationQuery, WorkflowTimingObservation,
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

    fn append_diagnostic_event(
        &mut self,
        request: DiagnosticEventAppendRequest,
    ) -> Result<DiagnosticEventRecord, DiagnosticsLedgerError>;

    fn diagnostic_events_after(
        &self,
        last_event_seq: i64,
        limit: u32,
    ) -> Result<Vec<DiagnosticEventRecord>, DiagnosticsLedgerError>;

    fn projection_state(
        &self,
        projection_name: &str,
    ) -> Result<Option<ProjectionStateRecord>, DiagnosticsLedgerError>;

    fn upsert_projection_state(
        &mut self,
        update: ProjectionStateUpdate,
    ) -> Result<ProjectionStateRecord, DiagnosticsLedgerError>;

    fn drain_scheduler_timeline_projection(
        &mut self,
        limit: u32,
    ) -> Result<ProjectionStateRecord, DiagnosticsLedgerError>;

    fn query_scheduler_timeline_projection(
        &self,
        query: SchedulerTimelineProjectionQuery,
    ) -> Result<Vec<SchedulerTimelineProjectionRecord>, DiagnosticsLedgerError>;

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

    fn upsert_workflow_run_summary(
        &mut self,
        record: WorkflowRunSummaryRecord,
    ) -> Result<(), DiagnosticsLedgerError>;

    fn query_workflow_run_summaries(
        &self,
        query: WorkflowRunSummaryQuery,
    ) -> Result<WorkflowRunSummaryProjection, DiagnosticsLedgerError>;
}

use pantograph_diagnostics_ledger::SchedulerModelLifecycleTransition;
use pantograph_runtime_attribution::WorkflowRunSnapshotRecord;

use super::session_execution_api::SchedulerModelLifecycleEventRequest;
use super::{WorkflowExecutionSessionSummary, WorkflowService, WorkflowServiceError};

#[derive(Clone, Copy)]
pub(super) struct WorkflowRuntimeLoadLifecycleContext<'a> {
    pub(super) session: &'a WorkflowExecutionSessionSummary,
    pub(super) snapshot: Option<&'a WorkflowRunSnapshotRecord>,
    pub(super) workflow_run_id: &'a str,
    pub(super) workflow_semantic_version: &'a str,
    pub(super) selected_runtime_id: Option<&'a str>,
    pub(super) required_backends: &'a [String],
    pub(super) required_models: &'a [String],
}

pub(super) enum WorkflowRuntimeLoadLifecycleEvent<'a> {
    Requested,
    DependencyResolved {
        duration_ms: u64,
    },
    Completed {
        duration_ms: u64,
    },
    Failed {
        duration_ms: u64,
        error: &'a str,
        canonical_error_event_id: Option<&'a str>,
    },
}

impl WorkflowService {
    pub(super) fn record_runtime_load_lifecycle_event_if_configured(
        &self,
        context: WorkflowRuntimeLoadLifecycleContext<'_>,
        event: WorkflowRuntimeLoadLifecycleEvent<'_>,
    ) -> Result<(), WorkflowServiceError> {
        let (transition, reason, duration_ms, error, canonical_error_event_id) = match event {
            WorkflowRuntimeLoadLifecycleEvent::Requested => (
                SchedulerModelLifecycleTransition::LoadRequested,
                "runtime admission requested required models",
                None,
                None,
                None,
            ),
            WorkflowRuntimeLoadLifecycleEvent::DependencyResolved { duration_ms } => (
                SchedulerModelLifecycleTransition::LoadDependencyResolved,
                "runtime admission resolved required model dependencies",
                Some(duration_ms),
                None,
                None,
            ),
            WorkflowRuntimeLoadLifecycleEvent::Completed { duration_ms } => (
                SchedulerModelLifecycleTransition::LoadCompleted,
                "runtime admission proved requested model active",
                Some(duration_ms),
                None,
                None,
            ),
            WorkflowRuntimeLoadLifecycleEvent::Failed {
                duration_ms,
                error,
                canonical_error_event_id,
            } => (
                SchedulerModelLifecycleTransition::LoadFailed,
                "runtime admission failed to load required models",
                Some(duration_ms),
                Some(error),
                canonical_error_event_id,
            ),
        };

        self.record_scheduler_model_lifecycle_events_if_configured(
            SchedulerModelLifecycleEventRequest {
                session: context.session,
                snapshot: context.snapshot,
                workflow_run_id: context.workflow_run_id,
                workflow_semantic_version: context.workflow_semantic_version,
                selected_runtime_id: context.selected_runtime_id,
                required_backends: context.required_backends,
                required_models: context.required_models,
                transition,
                reason: Some(reason),
                duration_ms,
                error,
                canonical_error_event_id,
            },
        )
    }
}

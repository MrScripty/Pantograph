use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::session_types::WorkflowExecutionSessionKind;
use crate::workflow::{
    WorkflowExecutionSessionQueueItem, WorkflowExecutionSessionQueueItemStatus,
    WorkflowExecutionSessionState, WorkflowExecutionSessionSummary,
    WorkflowSchedulerAdmissionOutcome,
};

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

/// Focused runtime/lifecycle state for one graph edit session.
#[derive(Debug, Clone)]
pub(crate) struct GraphEditSessionRuntime {
    active_execution_id: Option<String>,
    active_execution_started_at_ms: Option<u64>,
    run_count: u64,
    last_accessed: Instant,
}

impl GraphEditSessionRuntime {
    pub(crate) fn new() -> Self {
        Self {
            active_execution_id: None,
            active_execution_started_at_ms: None,
            run_count: 0,
            last_accessed: Instant::now(),
        }
    }

    pub(crate) fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    pub(crate) fn is_stale(&self, timeout: Duration) -> bool {
        self.last_accessed.elapsed() > timeout
    }

    pub(crate) fn session_summary(&self, session_id: &str) -> WorkflowExecutionSessionSummary {
        WorkflowExecutionSessionSummary {
            session_id: session_id.to_string(),
            workflow_id: session_id.to_string(),
            session_kind: WorkflowExecutionSessionKind::Edit,
            usage_profile: None,
            keep_alive: false,
            state: if self.active_execution_id.is_some() {
                WorkflowExecutionSessionState::Running
            } else {
                WorkflowExecutionSessionState::IdleLoaded
            },
            queued_runs: usize::from(self.active_execution_id.is_some()),
            run_count: self.run_count,
        }
    }

    pub(crate) fn queue_items(&self) -> Vec<WorkflowExecutionSessionQueueItem> {
        self.active_execution_id
            .as_ref()
            .map(|execution_id| {
                let started_at_ms = self.active_execution_started_at_ms;
                WorkflowExecutionSessionQueueItem {
                    queue_id: execution_id.clone(),
                    run_id: Some(execution_id.clone()),
                    enqueued_at_ms: started_at_ms,
                    dequeued_at_ms: started_at_ms,
                    priority: 0,
                    queue_position: Some(0),
                    scheduler_admission_outcome: Some(WorkflowSchedulerAdmissionOutcome::Admitted),
                    scheduler_decision_reason: None,
                    status: WorkflowExecutionSessionQueueItemStatus::Running,
                }
            })
            .into_iter()
            .collect()
    }

    pub(crate) fn mark_running(&mut self, session_id: &str) {
        self.touch();
        if self.active_execution_id.as_deref() != Some(session_id)
            || self.active_execution_started_at_ms.is_none()
        {
            self.active_execution_started_at_ms = Some(unix_timestamp_ms());
        }
        self.active_execution_id = Some(session_id.to_string());
    }

    pub(crate) fn finish_run(&mut self) {
        self.touch();
        if self.active_execution_id.take().is_some() {
            self.active_execution_started_at_ms = None;
            self.run_count = self.run_count.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GraphEditSessionRuntime;
    use crate::workflow::{WorkflowExecutionSessionQueueItemStatus, WorkflowExecutionSessionState};

    #[test]
    fn session_summary_defaults_to_idle_loaded() {
        let runtime = GraphEditSessionRuntime::new();
        let summary = runtime.session_summary("session-1");

        assert_eq!(summary.session_id, "session-1");
        assert_eq!(summary.workflow_id, "session-1");
        assert_eq!(summary.state, WorkflowExecutionSessionState::IdleLoaded);
        assert_eq!(summary.queued_runs, 0);
        assert_eq!(summary.run_count, 0);
    }

    #[test]
    fn mark_running_populates_running_queue_item() {
        let mut runtime = GraphEditSessionRuntime::new();
        runtime.mark_running("session-1");

        let summary = runtime.session_summary("session-1");
        let queue_items = runtime.queue_items();

        assert_eq!(summary.state, WorkflowExecutionSessionState::Running);
        assert_eq!(summary.queued_runs, 1);
        assert_eq!(queue_items.len(), 1);
        assert_eq!(queue_items[0].queue_id, "session-1");
        assert_eq!(
            queue_items[0].status,
            WorkflowExecutionSessionQueueItemStatus::Running
        );
        assert!(queue_items[0].enqueued_at_ms.is_some());
    }

    #[test]
    fn finish_run_clears_active_execution_and_increments_count() {
        let mut runtime = GraphEditSessionRuntime::new();
        runtime.mark_running("session-1");
        runtime.finish_run();

        let summary = runtime.session_summary("session-1");

        assert_eq!(summary.state, WorkflowExecutionSessionState::IdleLoaded);
        assert_eq!(summary.queued_runs, 0);
        assert_eq!(summary.run_count, 1);
        assert!(runtime.queue_items().is_empty());
    }
}

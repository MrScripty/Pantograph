use crate::workflow::{
    WorkflowExecutionSessionQueueItem, WorkflowExecutionSessionQueueItemStatus,
    WorkflowExecutionSessionState, WorkflowExecutionSessionSummary,
    WorkflowSchedulerAdmissionOutcome, WorkflowSchedulerDecisionReason,
};
use crate::WorkflowSchedulerSnapshotDiagnostics;

pub(super) fn apply_scheduler_snapshot(
    trace: &mut super::store::WorkflowTraceRunState,
    workflow_run_id: &str,
    session_id: &str,
    session: Option<&WorkflowExecutionSessionSummary>,
    items: &[WorkflowExecutionSessionQueueItem],
    diagnostics: Option<&WorkflowSchedulerSnapshotDiagnostics>,
    error: Option<&str>,
) {
    if trace.session_id.is_none() {
        trace.session_id = Some(session_id.to_string());
    }

    if error.is_some() {
        trace.queue.scheduler_admission_outcome = None;
        trace.queue.scheduler_decision_reason = Some(
            WorkflowSchedulerDecisionReason::SchedulerSnapshotFailed
                .as_str()
                .to_string(),
        );
        trace.queue.scheduler_snapshot_diagnostics = None;
        return;
    }

    let matched_item = matched_queue_item(workflow_run_id, items);
    let pending_visible = matched_item
        .map(|item| item.status == WorkflowExecutionSessionQueueItemStatus::Pending)
        .unwrap_or_else(|| {
            session
                .map(|summary| summary.queued_runs > 0)
                .unwrap_or(false)
                || items
                    .iter()
                    .any(|item| item.status == WorkflowExecutionSessionQueueItemStatus::Pending)
        });
    let running_visible = matched_item
        .map(|item| item.status == WorkflowExecutionSessionQueueItemStatus::Running)
        .unwrap_or_else(|| {
            matches!(
                session.map(|summary| summary.state),
                Some(WorkflowExecutionSessionState::Running)
            ) || items
                .iter()
                .any(|item| item.status == WorkflowExecutionSessionQueueItemStatus::Running)
        });

    if pending_visible {
        if let Some(enqueued_at_ms) = matched_item.and_then(|item| item.enqueued_at_ms) {
            trace.queue.enqueued_at_ms.get_or_insert(enqueued_at_ms);
        }
        if !matches!(
            trace.status,
            super::types::WorkflowTraceStatus::Completed
                | super::types::WorkflowTraceStatus::Failed
                | super::types::WorkflowTraceStatus::Cancelled
        ) && !running_visible
        {
            trace.status = super::types::WorkflowTraceStatus::Queued;
        }
    }

    if running_visible {
        if let Some(enqueued_at_ms) = matched_item.and_then(|item| item.enqueued_at_ms) {
            trace.queue.enqueued_at_ms.get_or_insert(enqueued_at_ms);
        }
        if let Some(dequeued_at_ms) = matched_item.and_then(|item| item.dequeued_at_ms) {
            trace.queue.dequeued_at_ms.get_or_insert(dequeued_at_ms);
        }
        if !matches!(
            trace.status,
            super::types::WorkflowTraceStatus::Completed
                | super::types::WorkflowTraceStatus::Failed
                | super::types::WorkflowTraceStatus::Cancelled
                | super::types::WorkflowTraceStatus::Waiting
        ) {
            trace.status = super::types::WorkflowTraceStatus::Running;
        }
    }

    trace.queue.queue_wait_ms = match (trace.queue.enqueued_at_ms, trace.queue.dequeued_at_ms) {
        (Some(enqueued_at_ms), Some(dequeued_at_ms)) => {
            Some(dequeued_at_ms.saturating_sub(enqueued_at_ms))
        }
        _ => None,
    };
    trace.queue.scheduler_admission_outcome =
        scheduler_admission_outcome(workflow_run_id, session, items);
    trace.queue.scheduler_decision_reason =
        scheduler_decision_reason(workflow_run_id, session, items);
    trace.queue.scheduler_snapshot_diagnostics = diagnostics.cloned();
}

fn scheduler_admission_outcome(
    workflow_run_id: &str,
    session: Option<&WorkflowExecutionSessionSummary>,
    items: &[WorkflowExecutionSessionQueueItem],
) -> Option<String> {
    let matched_item = matched_queue_item(workflow_run_id, items);
    let outcome = if let Some(item) = matched_item {
        item.scheduler_admission_outcome.or(match item.status {
            WorkflowExecutionSessionQueueItemStatus::Pending => {
                Some(WorkflowSchedulerAdmissionOutcome::Queued)
            }
            WorkflowExecutionSessionQueueItemStatus::Running => {
                Some(WorkflowSchedulerAdmissionOutcome::Admitted)
            }
        })
    } else {
        let pending_visible = session
            .map(|summary| summary.queued_runs > 0)
            .unwrap_or(false)
            || items
                .iter()
                .any(|item| item.status == WorkflowExecutionSessionQueueItemStatus::Pending);
        let running_visible = matches!(
            session.map(|summary| summary.state),
            Some(WorkflowExecutionSessionState::Running)
        ) || items
            .iter()
            .any(|item| item.status == WorkflowExecutionSessionQueueItemStatus::Running);

        if running_visible {
            Some(WorkflowSchedulerAdmissionOutcome::Admitted)
        } else if pending_visible {
            Some(WorkflowSchedulerAdmissionOutcome::Queued)
        } else {
            None
        }
    }?;

    Some(outcome.as_str().to_string())
}

fn scheduler_decision_reason(
    workflow_run_id: &str,
    session: Option<&WorkflowExecutionSessionSummary>,
    items: &[WorkflowExecutionSessionQueueItem],
) -> Option<String> {
    let matched_item = matched_queue_item(workflow_run_id, items);
    let reason = if let Some(item) = matched_item {
        item.scheduler_decision_reason.or(match item.status {
            WorkflowExecutionSessionQueueItemStatus::Pending => {
                Some(WorkflowSchedulerDecisionReason::MatchedPendingItem)
            }
            WorkflowExecutionSessionQueueItemStatus::Running => {
                Some(WorkflowSchedulerDecisionReason::MatchedRunningItem)
            }
        })
    } else {
        let pending_visible = session
            .map(|summary| summary.queued_runs > 0)
            .unwrap_or(false)
            || items
                .iter()
                .any(|item| item.status == WorkflowExecutionSessionQueueItemStatus::Pending);
        let running_visible = matches!(
            session.map(|summary| summary.state),
            Some(WorkflowExecutionSessionState::Running)
        ) || items
            .iter()
            .any(|item| item.status == WorkflowExecutionSessionQueueItemStatus::Running);

        if running_visible && pending_visible {
            Some(WorkflowSchedulerDecisionReason::SessionRunningWithBacklog)
        } else if running_visible {
            Some(WorkflowSchedulerDecisionReason::SessionRunning)
        } else if pending_visible {
            Some(WorkflowSchedulerDecisionReason::SessionQueued)
        } else {
            match session.map(|summary| summary.state) {
                Some(WorkflowExecutionSessionState::IdleLoaded) => {
                    Some(WorkflowSchedulerDecisionReason::IdleLoaded)
                }
                Some(WorkflowExecutionSessionState::IdleUnloaded) => {
                    Some(WorkflowSchedulerDecisionReason::IdleUnloaded)
                }
                Some(WorkflowExecutionSessionState::Running) | None => None,
            }
        }
    }?;

    Some(reason.as_str().to_string())
}

fn matched_queue_item<'a>(
    workflow_run_id: &str,
    items: &'a [WorkflowExecutionSessionQueueItem],
) -> Option<&'a WorkflowExecutionSessionQueueItem> {
    items
        .iter()
        .find(|item| item.workflow_run_id == workflow_run_id)
}

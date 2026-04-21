use crate::WorkflowSchedulerSnapshotDiagnostics;
use crate::workflow::{
    WorkflowSchedulerAdmissionOutcome, WorkflowSchedulerDecisionReason, WorkflowSessionQueueItem,
    WorkflowSessionQueueItemStatus, WorkflowSessionState, WorkflowSessionSummary,
};

pub(super) fn apply_scheduler_snapshot(
    trace: &mut super::store::WorkflowTraceRunState,
    execution_id: &str,
    session_id: &str,
    session: Option<&WorkflowSessionSummary>,
    items: &[WorkflowSessionQueueItem],
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

    let matched_item = matched_queue_item(execution_id, items);
    let pending_visible = matched_item
        .map(|item| item.status == WorkflowSessionQueueItemStatus::Pending)
        .unwrap_or_else(|| {
            session
                .map(|summary| summary.queued_runs > 0)
                .unwrap_or(false)
                || items
                    .iter()
                    .any(|item| item.status == WorkflowSessionQueueItemStatus::Pending)
        });
    let running_visible = matched_item
        .map(|item| item.status == WorkflowSessionQueueItemStatus::Running)
        .unwrap_or_else(|| {
            matches!(
                session.map(|summary| summary.state),
                Some(WorkflowSessionState::Running)
            ) || items
                .iter()
                .any(|item| item.status == WorkflowSessionQueueItemStatus::Running)
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
        scheduler_admission_outcome(execution_id, session, items);
    trace.queue.scheduler_decision_reason = scheduler_decision_reason(execution_id, session, items);
    trace.queue.scheduler_snapshot_diagnostics = diagnostics.cloned();
}

fn scheduler_admission_outcome(
    execution_id: &str,
    session: Option<&WorkflowSessionSummary>,
    items: &[WorkflowSessionQueueItem],
) -> Option<String> {
    let matched_item = matched_queue_item(execution_id, items);
    let outcome = if let Some(item) = matched_item {
        item.scheduler_admission_outcome.or(match item.status {
            WorkflowSessionQueueItemStatus::Pending => {
                Some(WorkflowSchedulerAdmissionOutcome::Queued)
            }
            WorkflowSessionQueueItemStatus::Running => {
                Some(WorkflowSchedulerAdmissionOutcome::Admitted)
            }
        })
    } else {
        let pending_visible = session
            .map(|summary| summary.queued_runs > 0)
            .unwrap_or(false)
            || items
                .iter()
                .any(|item| item.status == WorkflowSessionQueueItemStatus::Pending);
        let running_visible = matches!(
            session.map(|summary| summary.state),
            Some(WorkflowSessionState::Running)
        ) || items
            .iter()
            .any(|item| item.status == WorkflowSessionQueueItemStatus::Running);

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
    execution_id: &str,
    session: Option<&WorkflowSessionSummary>,
    items: &[WorkflowSessionQueueItem],
) -> Option<String> {
    let matched_item = matched_queue_item(execution_id, items);
    let reason = if let Some(item) = matched_item {
        item.scheduler_decision_reason.or(match item.status {
            WorkflowSessionQueueItemStatus::Pending => {
                Some(WorkflowSchedulerDecisionReason::MatchedPendingItem)
            }
            WorkflowSessionQueueItemStatus::Running => {
                Some(WorkflowSchedulerDecisionReason::MatchedRunningItem)
            }
        })
    } else {
        let pending_visible = session
            .map(|summary| summary.queued_runs > 0)
            .unwrap_or(false)
            || items
                .iter()
                .any(|item| item.status == WorkflowSessionQueueItemStatus::Pending);
        let running_visible = matches!(
            session.map(|summary| summary.state),
            Some(WorkflowSessionState::Running)
        ) || items
            .iter()
            .any(|item| item.status == WorkflowSessionQueueItemStatus::Running);

        if running_visible && pending_visible {
            Some(WorkflowSchedulerDecisionReason::SessionRunningWithBacklog)
        } else if running_visible {
            Some(WorkflowSchedulerDecisionReason::SessionRunning)
        } else if pending_visible {
            Some(WorkflowSchedulerDecisionReason::SessionQueued)
        } else {
            match session.map(|summary| summary.state) {
                Some(WorkflowSessionState::IdleLoaded) => {
                    Some(WorkflowSchedulerDecisionReason::IdleLoaded)
                }
                Some(WorkflowSessionState::IdleUnloaded) => {
                    Some(WorkflowSchedulerDecisionReason::IdleUnloaded)
                }
                Some(WorkflowSessionState::Running) | None => None,
            }
        }
    }?;

    Some(reason.as_str().to_string())
}

fn matched_queue_item<'a>(
    execution_id: &str,
    items: &'a [WorkflowSessionQueueItem],
) -> Option<&'a WorkflowSessionQueueItem> {
    items
        .iter()
        .find(|item| item.run_id.as_deref() == Some(execution_id))
        .or_else(|| items.iter().find(|item| item.queue_id == execution_id))
}

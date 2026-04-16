use crate::workflow::{
    WorkflowSessionQueueItem, WorkflowSessionQueueItemStatus, WorkflowSessionState,
    WorkflowSessionSummary,
};

pub(super) fn apply_scheduler_snapshot(
    trace: &mut super::store::WorkflowTraceRunState,
    execution_id: &str,
    session_id: &str,
    session: Option<&WorkflowSessionSummary>,
    items: &[WorkflowSessionQueueItem],
    error: Option<&str>,
    captured_at_ms: u64,
) {
    if trace.session_id.is_none() {
        trace.session_id = Some(session_id.to_string());
    }

    if error.is_some() {
        trace.queue.scheduler_decision_reason = Some("scheduler_snapshot_failed".to_string());
        return;
    }

    let matched_item = matched_queue_item(execution_id, session_id, items);
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
        } else {
            trace.queue.enqueued_at_ms.get_or_insert(captured_at_ms);
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
        } else {
            trace.queue.dequeued_at_ms.get_or_insert(captured_at_ms);
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
    trace.queue.scheduler_decision_reason =
        scheduler_decision_reason(execution_id, session_id, session, items);
}

fn scheduler_decision_reason(
    execution_id: &str,
    session_id: &str,
    session: Option<&WorkflowSessionSummary>,
    items: &[WorkflowSessionQueueItem],
) -> Option<String> {
    let matched_item = matched_queue_item(execution_id, session_id, items);
    let reason = if let Some(item) = matched_item {
        match item.status {
            WorkflowSessionQueueItemStatus::Pending => Some("matched_pending_item"),
            WorkflowSessionQueueItemStatus::Running => Some("matched_running_item"),
        }
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
            Some("session_running_with_backlog")
        } else if running_visible {
            Some("session_running")
        } else if pending_visible {
            Some("session_queued")
        } else {
            match session.map(|summary| summary.state) {
                Some(WorkflowSessionState::IdleLoaded) => Some("idle_loaded"),
                Some(WorkflowSessionState::IdleUnloaded) => Some("idle_unloaded"),
                Some(WorkflowSessionState::Running) | None => None,
            }
        }
    }?;

    Some(reason.to_string())
}

fn matched_queue_item<'a>(
    execution_id: &str,
    session_id: &str,
    items: &'a [WorkflowSessionQueueItem],
) -> Option<&'a WorkflowSessionQueueItem> {
    items
        .iter()
        .find(|item| item.run_id.as_deref() == Some(execution_id))
        .or_else(|| items.iter().find(|item| item.queue_id == execution_id))
        .or_else(|| {
            items
                .iter()
                .find(|item| item.run_id.as_deref() == Some(session_id))
        })
        .or_else(|| items.iter().find(|item| item.queue_id == session_id))
}

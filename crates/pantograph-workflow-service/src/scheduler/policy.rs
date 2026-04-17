use crate::workflow::WorkflowServiceError;

use super::store::WorkflowSessionQueuedRun;
use super::WorkflowSchedulerDecisionReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkflowSchedulerQueuePlacement {
    pub(crate) index: usize,
    pub(crate) reason: WorkflowSchedulerDecisionReason,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct PriorityThenFifoSchedulerPolicy;

impl PriorityThenFifoSchedulerPolicy {
    pub(crate) fn placement_for_enqueue(
        &self,
        queue: &[WorkflowSessionQueuedRun],
        queued: &WorkflowSessionQueuedRun,
    ) -> WorkflowSchedulerQueuePlacement {
        let mut matched_tie_break = false;
        let index = queue
            .iter()
            .position(|existing| {
                let higher_priority = queued.priority > existing.priority;
                let earlier_same_priority = queued.priority == existing.priority
                    && queued.enqueued_tick < existing.enqueued_tick;
                if earlier_same_priority {
                    matched_tie_break = true;
                }
                higher_priority || earlier_same_priority
            })
            .unwrap_or(queue.len());

        let reason = if queue.is_empty() || index == 0 && !matched_tie_break {
            WorkflowSchedulerDecisionReason::HighestPriorityFirst
        } else if matched_tie_break {
            WorkflowSchedulerDecisionReason::FifoPriorityTieBreak
        } else {
            WorkflowSchedulerDecisionReason::WaitingForHigherPriority
        };

        WorkflowSchedulerQueuePlacement { index, reason }
    }

    pub(crate) fn admission_reason(
        &self,
        queue: &[WorkflowSessionQueuedRun],
        queue_id: &str,
    ) -> Result<Option<WorkflowSchedulerDecisionReason>, WorkflowServiceError> {
        let Some(front) = queue.first() else {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in queue",
                queue_id
            )));
        };

        if front.queue_id == queue_id {
            return Ok(Some(WorkflowSchedulerDecisionReason::AdmittedForExecution));
        }

        if queue.iter().any(|item| item.queue_id == queue_id) {
            return Ok(None);
        }

        Err(WorkflowServiceError::QueueItemNotFound(format!(
            "queue item '{}' not found in queue",
            queue_id
        )))
    }
}

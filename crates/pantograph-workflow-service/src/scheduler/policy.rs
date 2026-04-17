use std::cmp::Ordering;

use crate::workflow::WorkflowServiceError;

use super::store::WorkflowSessionQueuedRun;
use super::WorkflowSchedulerDecisionReason;

const STARVATION_BYPASS_THRESHOLD: u32 = 2;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct PriorityThenFifoSchedulerPolicy;

impl PriorityThenFifoSchedulerPolicy {
    pub(crate) fn placement_index_for_enqueue(
        &self,
        queue: &[WorkflowSessionQueuedRun],
        queued: &WorkflowSessionQueuedRun,
    ) -> usize {
        queue
            .iter()
            .position(|existing| self.compare_runs(queued, existing) == Ordering::Less)
            .unwrap_or(queue.len())
    }

    pub(crate) fn refresh_queue(&self, queue: &mut [WorkflowSessionQueuedRun]) {
        queue.sort_by(|left, right| self.compare_runs(left, right));

        for index in 0..queue.len() {
            let reason = self.reason_for_queue_position(queue, index);
            queue[index].scheduler_decision_reason = reason;
        }
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

    fn compare_runs(
        &self,
        left: &WorkflowSessionQueuedRun,
        right: &WorkflowSessionQueuedRun,
    ) -> Ordering {
        self.effective_priority(right)
            .cmp(&self.effective_priority(left))
            .then_with(|| left.enqueued_tick.cmp(&right.enqueued_tick))
            .then_with(|| left.queue_id.cmp(&right.queue_id))
    }

    fn effective_priority(&self, queued: &WorkflowSessionQueuedRun) -> i32 {
        queued
            .priority
            .saturating_add(self.starvation_priority_boost(queued))
    }

    fn starvation_priority_boost(&self, queued: &WorkflowSessionQueuedRun) -> i32 {
        (queued.starvation_bypass_count / STARVATION_BYPASS_THRESHOLD).min(i32::MAX as u32) as i32
    }

    fn reason_for_queue_position(
        &self,
        queue: &[WorkflowSessionQueuedRun],
        index: usize,
    ) -> WorkflowSchedulerDecisionReason {
        let item = &queue[index];
        if index == 0 {
            let promoted_over_higher_base_priority = self.starvation_priority_boost(item) > 0
                && queue
                    .iter()
                    .skip(1)
                    .any(|other| other.priority > item.priority);
            if promoted_over_higher_base_priority {
                WorkflowSchedulerDecisionReason::StarvationProtection
            } else {
                WorkflowSchedulerDecisionReason::HighestPriorityFirst
            }
        } else if queue[..index].iter().any(|ahead| {
            self.effective_priority(ahead) == self.effective_priority(item)
                && ahead.enqueued_tick < item.enqueued_tick
        }) {
            WorkflowSchedulerDecisionReason::FifoPriorityTieBreak
        } else {
            WorkflowSchedulerDecisionReason::WaitingForHigherPriority
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::workflow::{WorkflowOutputTarget, WorkflowPortBinding};

    use super::*;

    fn queued_run(
        queue_id: &str,
        priority: i32,
        enqueued_tick: u64,
        starvation_bypass_count: u32,
    ) -> WorkflowSessionQueuedRun {
        WorkflowSessionQueuedRun {
            queue_id: queue_id.to_string(),
            run_id: Some(queue_id.to_string()),
            enqueued_at_ms: 0,
            inputs: Vec::<WorkflowPortBinding>::new(),
            output_targets: Some(Vec::<WorkflowOutputTarget>::new()),
            override_selection: None,
            timeout_ms: None,
            priority,
            scheduler_decision_reason: WorkflowSchedulerDecisionReason::WaitingForHigherPriority,
            enqueued_tick,
            starvation_bypass_count,
        }
    }

    #[test]
    fn refresh_queue_promotes_starved_run_over_newer_higher_priority_items() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let mut queue = vec![
            queued_run("high-1", 2, 2, 0),
            queued_run("high-2", 2, 3, 0),
            queued_run("starved", 0, 1, 4),
        ];

        policy.refresh_queue(&mut queue);

        assert_eq!(queue[0].queue_id, "starved");
        assert_eq!(
            queue[0].scheduler_decision_reason,
            WorkflowSchedulerDecisionReason::StarvationProtection
        );
        assert_eq!(queue[1].queue_id, "high-1");
        assert_eq!(
            queue[1].scheduler_decision_reason,
            WorkflowSchedulerDecisionReason::FifoPriorityTieBreak
        );
        assert_eq!(queue[2].queue_id, "high-2");
    }
}

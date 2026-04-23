use super::WorkflowSchedulerDecisionReason;

use super::store::WORKFLOW_SESSION_QUEUE_POLL_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WorkflowSchedulerAdmissionEta {
    pub(super) wait_ms: u64,
    pub(super) not_before_ms: u64,
}

pub(super) fn next_admission_eta(
    active_run_blocks_admission: bool,
    next_admission_queue_id: Option<&str>,
    next_admission_reason: Option<WorkflowSchedulerDecisionReason>,
    now_ms: u64,
) -> Option<WorkflowSchedulerAdmissionEta> {
    if active_run_blocks_admission || next_admission_queue_id.is_none() {
        return None;
    }

    let wait_ms = match next_admission_reason {
        Some(WorkflowSchedulerDecisionReason::WaitingForRuntimeAdmission) => {
            WORKFLOW_SESSION_QUEUE_POLL_MS
        }
        Some(WorkflowSchedulerDecisionReason::WaitingForRuntimeCapacity) => return None,
        _ => 0,
    };

    Some(WorkflowSchedulerAdmissionEta {
        wait_ms,
        not_before_ms: now_ms.saturating_add(wait_ms),
    })
}

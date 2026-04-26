use crate::workflow::{WorkflowSchedulerRuntimeDiagnosticsRequest, WorkflowServiceError};

use super::super::store_admission::next_admission_eta;
use super::super::{
    PriorityThenFifoSchedulerPolicy, WorkflowSchedulerDecisionReason,
    WorkflowSchedulerRuntimeCapacityPressure, WorkflowSchedulerSnapshotDiagnostics,
};
use super::{WorkflowExecutionSessionStore, unix_timestamp_ms};

impl WorkflowExecutionSessionStore {
    pub(crate) fn scheduler_snapshot_diagnostics(
        &self,
        session_id: &str,
    ) -> Result<WorkflowSchedulerSnapshotDiagnostics, WorkflowServiceError> {
        let now_ms = unix_timestamp_ms();
        let state = self.active.get(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        let reclaimable_loaded_session_count = self.runtime_unload_candidates(session_id).len();
        let loaded_session_count = self.loaded_session_count();
        let active_run_blocks_admission = state.active_run.is_some();

        let mut admission_input = Self::admission_input_from_state(state);
        admission_input.has_active_run = false;
        let predicted_admission =
            PriorityThenFifoSchedulerPolicy.predicted_admission_decision(&admission_input);
        let next_admission_workflow_run_id = predicted_admission
            .as_ref()
            .and_then(|decision| decision.admitted_workflow_run_id.clone());
        let next_admission_bypassed_workflow_run_id = match (
            state
                .queue
                .first()
                .map(|queued| queued.workflow_run_id.as_str()),
            next_admission_workflow_run_id.as_deref(),
        ) {
            (Some(workflow_run_head_id), Some(next_workflow_run_id))
                if workflow_run_head_id != next_workflow_run_id =>
            {
                Some(workflow_run_head_id.to_string())
            }
            _ => None,
        };
        let next_admission_reason = next_admission_workflow_run_id
            .as_deref()
            .and_then(|queue_id| {
                state
                    .queue
                    .iter()
                    .find(|queued| queued.workflow_run_id == queue_id)
                    .map(|queued| queued.scheduler_decision_reason)
            })
            .filter(|reason| {
                matches!(
                    reason,
                    WorkflowSchedulerDecisionReason::WaitingForRuntimeCapacity
                        | WorkflowSchedulerDecisionReason::WaitingForRuntimeAdmission
                )
            })
            .or_else(|| {
                predicted_admission
                    .as_ref()
                    .and_then(|decision| decision.reason)
            });

        let runtime_capacity_pressure = if loaded_session_count < self.max_loaded_sessions {
            WorkflowSchedulerRuntimeCapacityPressure::Available
        } else if reclaimable_loaded_session_count > 0 {
            WorkflowSchedulerRuntimeCapacityPressure::RebalanceRequired
        } else {
            WorkflowSchedulerRuntimeCapacityPressure::Saturated
        };
        let next_admission_eta = next_admission_eta(
            active_run_blocks_admission,
            next_admission_workflow_run_id.as_deref(),
            next_admission_reason,
            now_ms,
        );

        Ok(WorkflowSchedulerSnapshotDiagnostics {
            loaded_session_count,
            max_loaded_sessions: self.max_loaded_sessions,
            reclaimable_loaded_session_count,
            runtime_capacity_pressure,
            active_run_blocks_admission,
            next_admission_workflow_run_id,
            next_admission_bypassed_workflow_run_id,
            next_admission_after_runs: predicted_admission
                .as_ref()
                .map(|_| usize::from(active_run_blocks_admission)),
            next_admission_wait_ms: next_admission_eta.map(|eta| eta.wait_ms),
            next_admission_not_before_ms: next_admission_eta.map(|eta| eta.not_before_ms),
            next_admission_reason,
            runtime_registry: None,
        })
    }

    pub(crate) fn scheduler_runtime_diagnostics_request(
        &self,
        session_id: &str,
    ) -> Result<WorkflowSchedulerRuntimeDiagnosticsRequest, WorkflowServiceError> {
        let state = self.active.get(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        let diagnostics = self.scheduler_snapshot_diagnostics(session_id)?;
        Ok(WorkflowSchedulerRuntimeDiagnosticsRequest {
            session_id: session_id.to_string(),
            workflow_id: state.workflow_id.clone(),
            usage_profile: state.usage_profile.clone(),
            keep_alive: state.keep_alive,
            runtime_loaded: state.runtime_loaded,
            next_admission_workflow_run_id: diagnostics.next_admission_workflow_run_id,
            reclaim_candidates: self.runtime_unload_candidates(session_id),
        })
    }
}

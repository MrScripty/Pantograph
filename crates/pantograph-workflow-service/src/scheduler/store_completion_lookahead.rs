use super::super::task_orchestrator::WorkflowReservationCleanupAcknowledgement;
use super::{
    WorkflowCompletionCleanupGate, WorkflowExecutionSessionStore, WorkflowSchedulerTaskAttemptId,
};
use crate::workflow::{
    bounded_serialized, bounded_task, equivalent_environment, WorkflowCompletionSuccessorSnapshot,
    WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass, WorkflowServiceError,
};
use pantograph_dependency_planning::DependencyReadinessProofEnvelope;
use pantograph_scheduler::{SchedulerTaskStateKind, SchedulerTaskStateRecord};

impl WorkflowExecutionSessionStore {
    pub(crate) fn active_run_runtime_task_snapshot(
        &self,
        session: &str,
        run_id: &str,
        task_id: &str,
    ) -> Result<(WorkflowSchedulerTask, SchedulerTaskStateRecord), WorkflowServiceError> {
        let run = self
            .active
            .get(session)
            .and_then(|s| s.active_run.as_ref())
            .filter(|r| r.workflow_run_id == run_id)
            .ok_or_else(|| {
                WorkflowServiceError::Internal(format!(
                    "active workflow run '{run_id}' has no scheduler task state"
                ))
            })?;
        let task = run
            .scheduler_task_graph
            .as_ref()
            .and_then(|g| g.tasks.iter().find(|t| t.task_id.as_str() == task_id))
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "runtime scheduler task '{task_id}' is not in active workflow run '{run_id}'"
                ))
            })?;
        let record = run.scheduler_task_records.get(task_id).ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "runtime scheduler task '{task_id}' has no active task-state record"
            ))
        })?;
        Ok((task.clone(), record.clone()))
    }
    pub(crate) fn completion_successor_snapshot(
        &self,
        session: &str,
        run_id: &str,
        first: &WorkflowSchedulerTask,
        ready: &SchedulerTaskStateRecord,
        proof: &DependencyReadinessProofEnvelope,
    ) -> Option<WorkflowCompletionSuccessorSnapshot> {
        let run = self.active.get(session)?.active_run.as_ref()?;
        if run.workflow_run_id != run_id
            || !bounded_task(first)
            || ready.state.kind() != SchedulerTaskStateKind::Ready
        {
            return None;
        }
        if ready.state.execution_intent()?.runtime_task_intent()
            != first.schedulable_intent.as_ref()
            || !bounded_serialized(ready)
        {
            return None;
        }
        let graph = run.scheduler_task_graph.as_ref()?;
        if graph.tasks.len() > 128 || run.scheduler_task_records.len() > 128 {
            return None;
        }
        let mut pending = graph.tasks.iter().filter(|t| {
            t.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
                && run
                    .scheduler_task_records
                    .get(t.task_id.as_str())
                    .is_none_or(|r| r.state.kind() != SchedulerTaskStateKind::Completed)
        });
        let a = pending.next()?;
        let b = pending.next()?;
        if pending.next().is_some() {
            return None;
        }
        let next = if a.task_id == first.task_id {
            b
        } else if b.task_id == first.task_id {
            a
        } else {
            return None;
        };
        if !bounded_task(next) || !equivalent_environment(first, next, proof) {
            return None;
        }
        let record = run.scheduler_task_records.get(next.task_id.as_str())?;
        if !matches!(&record.state, pantograph_scheduler::SchedulerTaskState::AwaitingInputs { diagnostics }
            if diagnostics.len() <= 32 && diagnostics.iter().all(|d| d.message.len() <= 1024 && d.hint.as_ref().is_none_or(|s| s.len() <= 1024)))
            || !bounded_serialized(record)
            || record.state.kind() != SchedulerTaskStateKind::AwaitingInputs
            || !next.dependency_task_ids.contains(&first.task_id)
            || !next.dependency_task_ids.iter().all(|id| {
                id == &first.task_id
                    || run
                        .scheduler_task_records
                        .get(id.as_str())
                        .is_some_and(|r| r.state.kind() == SchedulerTaskStateKind::Completed)
            })
        {
            return None;
        }
        let predecessor_inputs: Vec<_> = next
            .input_bindings
            .iter()
            .filter(|b| b.source_task_id == first.task_id)
            .cloned()
            .collect();
        if predecessor_inputs.is_empty() {
            return None;
        }
        let mut known = next.clone();
        known
            .input_bindings
            .retain(|b| b.source_task_id != first.task_id);
        let known_inputs = self.active_run_completion_inputs(session, run_id, &known)?;
        Some(WorkflowCompletionSuccessorSnapshot {
            predecessor_task_id: first.task_id.to_string(),
            predecessor_state_version: ready.state_version,
            task: next.clone(),
            record: record.clone(),
            known_inputs,
            predecessor_inputs,
            shared_environment: proof.preflight_result.environment_ref.clone()?,
        })
    }
    pub(crate) fn validate_completion_successor_snapshot(
        &self,
        session: &str,
        run: &str,
        first: &WorkflowSchedulerTask,
        ready: &SchedulerTaskStateRecord,
        proof: &DependencyReadinessProofEnvelope,
        expected: &WorkflowCompletionSuccessorSnapshot,
    ) -> Result<(), WorkflowServiceError> {
        if self
            .completion_successor_snapshot(session, run, first, ready, proof)
            .as_ref()
            == Some(expected)
        {
            Ok(())
        } else {
            Err(WorkflowServiceError::InvalidRequest(
                "owned completion successor/cohort/input snapshot changed before start".into(),
            ))
        }
    }
    pub(crate) fn install_completion_cleanup_gate(
        &mut self,
        session: &str,
        run_id: &str,
        pair: &WorkflowCompletionSuccessorSnapshot,
        attempt: &WorkflowSchedulerTaskAttemptId,
    ) {
        // Called only after successful start, under the same store guard.
        let run = self
            .active
            .get_mut(session)
            .and_then(|s| s.active_run.as_mut())
            .expect("start owns active run");
        assert_eq!(run.workflow_run_id, run_id);
        run.completion_cleanup_gate = Some(WorkflowCompletionCleanupGate {
            first_task_id: pair.predecessor_task_id.clone(),
            first_attempt_id: attempt.clone(),
            successor_task_id: pair.task.task_id.to_string(),
            reservation_lease_id: None,
        });
    }
    pub(crate) fn completion_cleanup_pending(
        &self,
        session: &str,
        run_id: &str,
        task_id: &str,
    ) -> bool {
        self.active
            .get(session)
            .and_then(|s| s.active_run.as_ref())
            .filter(|r| r.workflow_run_id == run_id)
            .and_then(|r| r.completion_cleanup_gate.as_ref())
            .is_some_and(|gate| gate.successor_task_id == task_id)
    }
    pub(crate) fn acknowledge_completion_cleanup(
        &mut self,
        session: &str,
        run_id: &str,
        task_id: &str,
        attempt: &WorkflowSchedulerTaskAttemptId,
        ack: &WorkflowReservationCleanupAcknowledgement,
    ) {
        let Some(run) = self
            .active
            .get_mut(session)
            .and_then(|s| s.active_run.as_mut())
            .filter(|r| r.workflow_run_id == run_id)
        else {
            return;
        };
        let matches = ack.cleanup_release && ack.workflow_run_id == run_id && ack.task_id == task_id && run.completion_cleanup_gate.as_ref().is_some_and(|gate| gate.first_task_id == task_id
            && gate.first_attempt_id == *attempt && gate.reservation_lease_id.as_ref() == Some(&ack.as_ref().reservation_lease_id)
            && matches!(ack.as_ref().state, pantograph_runtime_host_contracts::ReservationLifecycleApplicationState::Applied));
        if matches {
            run.completion_cleanup_gate = None;
        }
    }
}

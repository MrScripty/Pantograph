use super::WorkflowExecutionSessionStore;
use crate::workflow::{
    bounded_proof, bounded_serialized, bounded_task, equivalent_environment,
    WorkflowSchedulerTaskExecutionClass, WorkflowSerialReadyMember, WorkflowSerialReadyPair,
    WorkflowServiceError,
};
use pantograph_scheduler::SchedulerTaskStateKind;

impl WorkflowExecutionSessionStore {
    pub(crate) fn serial_ready_member(
        &self,
        session: &str,
        run_id: &str,
        task_id: &str,
    ) -> Option<WorkflowSerialReadyMember> {
        let run = self.active.get(session)?.active_run.as_ref()?;
        if run.workflow_run_id != run_id {
            return None;
        }
        let graph = run.scheduler_task_graph.as_ref()?;
        if graph.tasks.len() > 128 || run.scheduler_task_records.len() > 128 {
            return None;
        }
        let task = graph.tasks.iter().find(|t| t.task_id.as_str() == task_id)?;
        let record = run.scheduler_task_records.get(task_id)?;
        let proof = run.runtime_dispatch_readiness_proofs.get(task_id)?;
        if task.execution_class != WorkflowSchedulerTaskExecutionClass::RuntimeInference
            || record.state.kind() != SchedulerTaskStateKind::Ready
            || !bounded_task(task)
            || !bounded_proof(proof)
            || !bounded_serialized(record)
            || !equivalent_environment(task, task, proof)
            || record.state.execution_intent()?.runtime_task_intent()
                != task.schedulable_intent.as_ref()
            || self.completion_cleanup_pending(session, run_id, task_id)
        {
            return None;
        }
        let inputs = self.active_run_completion_inputs(session, run_id, task)?;
        Some(WorkflowSerialReadyMember {
            task: task.clone(),
            record: record.clone(),
            proof: proof.clone(),
            inputs,
        })
    }
    pub(crate) fn serial_ready_pair(
        &self,
        session: &str,
        run_id: &str,
    ) -> Option<WorkflowSerialReadyPair> {
        let run = self.active.get(session)?.active_run.as_ref()?;
        if run.workflow_run_id != run_id {
            return None;
        }
        let graph = run.scheduler_task_graph.as_ref()?;
        if graph.tasks.len() > 128 || run.scheduler_task_records.len() != graph.tasks.len() {
            return None;
        }
        let mut ready = graph.tasks.iter().filter(|t| {
            t.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
                && run
                    .scheduler_task_records
                    .get(t.task_id.as_str())
                    .is_some_and(|r| r.state.kind() == SchedulerTaskStateKind::Ready)
        });
        // Authoritative first two, with no skip over unsupported first tasks.
        let a = ready.next()?;
        let b = ready.next()?;
        let first = self.serial_ready_member(session, run_id, a.task_id.as_str())?;
        let second = self.serial_ready_member(session, run_id, b.task_id.as_str())?;
        if first.proof.preflight_result.environment_ref.is_none()
            || first.proof.preflight_result.environment_ref
                != second.proof.preflight_result.environment_ref
            || !equivalent_environment(&first.task, &second.task, &first.proof)
            || !equivalent_environment(&second.task, &first.task, &second.proof)
        {
            return None;
        }
        let population = graph
            .tasks
            .iter()
            .map(|t| {
                let r = run.scheduler_task_records.get(t.task_id.as_str())?;
                Some((
                    t.task_id.to_string(),
                    t.execution_class,
                    r.state.kind(),
                    r.state_version,
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(WorkflowSerialReadyPair {
            members: [first, second],
            population,
        })
    }
    pub(crate) fn validate_serial_ready_pair(
        &self,
        session: &str,
        run: &str,
        expected: &WorkflowSerialReadyPair,
    ) -> Result<(), WorkflowServiceError> {
        if self.serial_ready_pair(session, run).as_ref() == Some(expected) {
            Ok(())
        } else {
            Err(WorkflowServiceError::InvalidRequest(
                "serial Ready population/proofs/inputs changed before Start".into(),
            ))
        }
    }
}

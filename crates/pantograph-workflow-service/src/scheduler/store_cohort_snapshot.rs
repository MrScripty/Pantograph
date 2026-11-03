use super::WorkflowExecutionSessionStore;
use crate::workflow::{
    bounded_proof, bounded_serialized, bounded_task, cohort_fingerprint, equivalent_environment,
    WorkflowCohortKnownInputIdentity, WorkflowCohortMember, WorkflowCohortRefusal,
    WorkflowFrozenCohortSnapshot, WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass,
    WorkflowSchedulerTaskResultStatus, WORKFLOW_COHORT_MAX_GRAPH_TASKS,
    WORKFLOW_COHORT_MAX_SNAPSHOT_BYTES,
};
use pantograph_scheduler::{
    SchedulerTaskState, SchedulerTaskStateKind, SCHEDULER_COHORT_MAX_TASKS,
};

impl WorkflowExecutionSessionStore {
    /// Separate from the pair API. Capture the complete unfinished runtime
    /// population in one run; unsupported work is a refusal, never omission.
    pub(crate) fn frozen_cohort_snapshot(
        &self,
        session: &str,
        run_id: &str,
        first_task_id: &str,
    ) -> Result<WorkflowFrozenCohortSnapshot, WorkflowCohortRefusal> {
        use WorkflowCohortRefusal::*;
        // Bound caller-controlled keys before HashMap hashing or tree lookup.
        if session.len() > 256 || run_id.len() > 128 || first_task_id.len() > 128 {
            return Err(InvalidIdentity);
        }
        let state = self.active.get(session).ok_or(NoActiveRun)?;
        let run = state
            .active_run
            .as_ref()
            .filter(|r| r.workflow_run_id == run_id)
            .ok_or(NoActiveRun)?;
        let graph = run.scheduler_task_graph.as_ref().ok_or(NoActiveRun)?;
        if graph.tasks.is_empty()
            || graph.tasks.len() > WORKFLOW_COHORT_MAX_GRAPH_TASKS
            || run.scheduler_task_records.len() > WORKFLOW_COHORT_MAX_GRAPH_TASKS
            || run.scheduler_task_results.len() > WORKFLOW_COHORT_MAX_GRAPH_TASKS
            || run.runtime_dispatch_readiness_proofs.len() > WORKFLOW_COHORT_MAX_GRAPH_TASKS
        {
            return Err(PopulationLimit);
        }
        if run.scheduler_task_records.len() != graph.tasks.len() {
            return Err(UnknownRecord);
        }
        if graph.workflow_id.as_str() != state.workflow_id
            || graph.workflow_run_id.as_str() != run_id
        {
            return Err(InvalidIdentity);
        }
        // Check collection sizes/strings before bounded streaming serialization.
        // In particular, do not clone graph or result populations to find members.
        let mut pending = Vec::with_capacity(SCHEDULER_COHORT_MAX_TASKS);
        for (index, task) in graph.tasks.iter().enumerate() {
            if !bounded_graph_task(task) {
                return Err(UnsupportedTask);
            }
            if task.workflow_id != graph.workflow_id
                || task.workflow_run_id != graph.workflow_run_id
                || graph.tasks[..index]
                    .iter()
                    .any(|other| other.task_id == task.task_id)
            {
                return Err(InvalidIdentity);
            }
            let record = run
                .scheduler_task_records
                .get(task.task_id.as_str())
                .ok_or(UnknownRecord)?;
            if record.workflow_id != task.workflow_id
                || record.workflow_run_id != task.workflow_run_id
                || record.node_id != task.node_id
                || record.task_id != task.task_id
            {
                return Err(InvalidIdentity);
            }
            // Compare runtime payloads to the already shape-bounded graph task
            // before serialization/validation can walk arbitrary record strings.
            if let Some(execution) = record.state.execution_intent() {
                if (task.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
                    && execution.runtime_task_intent() != task.schedulable_intent.as_ref())
                    || (task.execution_class
                        != WorkflowSchedulerTaskExecutionClass::RuntimeInference
                        && execution.runtime_task_intent().is_some())
                {
                    return Err(UnsupportedTask);
                }
            }
            if !bounded_record(record) || record.validate().is_err() {
                return Err(UnsupportedState);
            }
            if task.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
                && record.state.kind() != SchedulerTaskStateKind::Completed
            {
                if pending.len() == SCHEDULER_COHORT_MAX_TASKS {
                    return Err(PopulationLimit);
                }
                if !matches!(
                    record.state.kind(),
                    SchedulerTaskStateKind::Ready
                        | SchedulerTaskStateKind::AwaitingInputs
                        | SchedulerTaskStateKind::WaitingDependencyReadiness
                ) {
                    return Err(UnsupportedState);
                }
                pending.push(task);
            } else if record.state.kind() == SchedulerTaskStateKind::Running {
                // Serialized observation cannot silently omit already-running work.
                return Err(UnsupportedState);
            }
        }
        let first = pending
            .iter()
            .find(|t| t.task_id.as_str() == first_task_id)
            .copied()
            .ok_or(FirstNotAdmitted)?;
        let ready = run
            .scheduler_task_records
            .get(first_task_id)
            .ok_or(UnknownRecord)?;
        let proof = run
            .runtime_dispatch_readiness_proofs
            .get(first_task_id)
            .ok_or(FirstNotAdmitted)?;
        if ready.state.kind() != SchedulerTaskStateKind::Ready
            || !bounded_proof(proof)
            || proof.validate().is_err()
            || proof.preflight_result.readiness_state
                != pantograph_dependency_planning::DependencyEnvironmentReadinessState::Ready
            || proof.preflight_result.environment_ref.is_none()
            || proof.execution_context.workflow_id.as_str() != first.workflow_id.as_str()
            || proof.execution_context.workflow_run_id.as_str() != first.workflow_run_id.as_str()
            || proof.execution_context.scheduler_task_id.as_str() != first.task_id.as_str()
            || proof.execution_context.node_id.as_str() != first.node_id.as_str()
            || !equivalent_environment(first, first, proof)
        {
            return Err(FirstNotAdmitted);
        }
        if run.completion_cleanup_gate.is_some() {
            return Err(CleanupPending);
        }
        let mut remaining = WORKFLOW_COHORT_MAX_SNAPSHOT_BYTES;
        if run
            .runtime_dispatch_readiness_proofs
            .values()
            .any(|p| !bounded_proof(p))
        {
            return Err(FirstNotAdmitted);
        }
        let population_fingerprint = cohort_fingerprint(
            &(
                graph,
                &run.scheduler_task_records,
                &run.runtime_dispatch_readiness_proofs,
            ),
            &mut remaining,
        )?;
        cohort_fingerprint(proof, &mut remaining)?;
        pending.sort_by(|a, b| a.task_id.cmp(&b.task_id));
        let mut members = Vec::with_capacity(pending.len());
        for task in &pending {
            for (index, dependency) in task.dependency_task_ids.iter().enumerate() {
                if dependency == &task.task_id
                    || task.dependency_task_ids[..index].contains(dependency)
                {
                    return Err(InvalidDependencies);
                }
                let dependency_record = run
                    .scheduler_task_records
                    .get(dependency.as_str())
                    .ok_or(UnknownRecord)?;
                if !pending.iter().any(|t| t.task_id == *dependency)
                    && dependency_record.state.kind() != SchedulerTaskStateKind::Completed
                {
                    return Err(ExternalUnfinishedPrerequisite);
                }
                if task.task_id == first.task_id
                    && dependency_record.state.kind() != SchedulerTaskStateKind::Completed
                {
                    return Err(FirstNotAdmitted);
                }
            }
            let mut known = (*task).clone();
            known.input_bindings.clear();
            let mut symbolic_inputs = Vec::new();
            let mut known_input_identities = Vec::new();
            for (index, binding) in task.input_bindings.iter().enumerate() {
                if !task.dependency_task_ids.contains(&binding.source_task_id)
                    || task.input_bindings[..index]
                        .iter()
                        .any(|b| b.target_port_id == binding.target_port_id)
                {
                    return Err(InvalidDependencies);
                }
                let source_task = graph
                    .tasks
                    .iter()
                    .find(|t| t.task_id == binding.source_task_id)
                    .ok_or(InvalidDependencies)?;
                if source_task.node_id != binding.source_node_id {
                    return Err(InvalidDependencies);
                }
                if pending.iter().any(|t| t.task_id == binding.source_task_id) {
                    symbolic_inputs.push(binding.clone());
                    continue;
                }
                let source_record = run
                    .scheduler_task_records
                    .get(binding.source_task_id.as_str())
                    .ok_or(UnknownRecord)?;
                let result = run
                    .scheduler_task_results
                    .get(binding.source_task_id.as_str())
                    .ok_or(UnknownInput)?;
                if source_record.state.kind() != SchedulerTaskStateKind::Completed
                    || result.status != WorkflowSchedulerTaskResultStatus::Completed
                    || result.workflow_id != source_task.workflow_id.as_str()
                    || result.workflow_run_id != run_id
                    || result.node_id != source_task.node_id.as_str()
                    || result.task_id != source_task.task_id.as_str()
                    || result.outputs.len() > 32
                    || result.outputs.iter().any(|o| o.port_id.len() > 128)
                {
                    return Err(UnknownInput);
                }
                let mut outputs = result
                    .outputs
                    .iter()
                    .filter(|o| o.port_id == binding.source_port_id);
                let output = outputs.next().ok_or(UnknownInput)?;
                if outputs.next().is_some() || !bounded_input_value(&output.value) {
                    return Err(UnknownInput);
                }
                let value_fingerprint = cohort_fingerprint(&output.value, &mut remaining)?;
                known_input_identities.push(WorkflowCohortKnownInputIdentity {
                    binding: binding.clone(),
                    source_state_version: source_record.state_version,
                    value_fingerprint,
                });
                known.input_bindings.push(binding.clone());
            }
            if task.task_id == first.task_id && !symbolic_inputs.is_empty() {
                return Err(FirstNotAdmitted);
            }
            let known_inputs = self
                .active_run_completion_inputs(session, run_id, &known)
                .ok_or(UnknownInput)?;
            members.push(WorkflowCohortMember {
                task: (*task).clone(),
                record: run.scheduler_task_records[task.task_id.as_str()].clone(),
                known_inputs,
                known_input_identities,
                symbolic_inputs,
            });
        }
        // Bounded topological closure. An independent future task is valid; a
        // cycle or prerequisite outside the represented/satisfied population is not.
        let mut completed = Vec::with_capacity(members.len());
        while completed.len() < members.len() {
            let Some(next) = members.iter().find(|m| {
                !completed.contains(&&m.task.task_id)
                    && m.task.dependency_task_ids.iter().all(|id| {
                        !members.iter().any(|m| m.task.task_id == *id) || completed.contains(&id)
                    })
            }) else {
                return Err(InvalidDependencies);
            };
            completed.push(&next.task.task_id);
        }
        let mut snapshot = WorkflowFrozenCohortSnapshot {
            session_id: session.to_owned(),
            workflow_run_id: run_id.to_owned(),
            first_task_id: first.task_id.clone(),
            first_readiness_proof: proof.clone(),
            population_fingerprint,
            members,
            identity: String::new(),
        };
        snapshot.identity = cohort_fingerprint(&snapshot, &mut remaining)?;
        // Count the final identity too, so serialization cannot exceed the budget.
        remaining
            .checked_sub(snapshot.identity.len())
            .ok_or(SnapshotByteLimit)?;
        Ok(snapshot)
    }

    pub(crate) fn validate_frozen_cohort_snapshot(
        &self,
        expected: &WorkflowFrozenCohortSnapshot,
    ) -> Result<(), WorkflowCohortRefusal> {
        let current = self.frozen_cohort_snapshot(
            &expected.session_id,
            &expected.workflow_run_id,
            expected.first_task_id.as_str(),
        )?;
        if &current == expected {
            Ok(())
        } else {
            Err(WorkflowCohortRefusal::SnapshotChanged)
        }
    }
}

fn bounded_record(record: &pantograph_scheduler::SchedulerTaskStateRecord) -> bool {
    let diagnostics = match &record.state {
        SchedulerTaskState::AwaitingInputs { diagnostics }
        | SchedulerTaskState::InputUnavailable { diagnostics }
        | SchedulerTaskState::Invalid { diagnostics }
        | SchedulerTaskState::PausedDeferred { diagnostics, .. }
        | SchedulerTaskState::RetryableFailed { diagnostics, .. }
        | SchedulerTaskState::TerminalFailed { diagnostics } => Some(diagnostics),
        _ => None,
    };
    diagnostics.is_none_or(|ds| {
        ds.len() <= 32
            && ds
                .iter()
                .all(|d| d.message.len() <= 1024 && d.hint.as_ref().is_none_or(|s| s.len() <= 1024))
    }) && bounded_serialized(record)
}
fn bounded_graph_task(task: &WorkflowSchedulerTask) -> bool {
    if ![
        task.workflow_id.as_str(),
        task.workflow_run_id.as_str(),
        task.task_id.as_str(),
        task.node_id.as_str(),
    ]
    .into_iter()
    .all(|s| s.len() <= 256)
        || task
            .dependency_task_ids
            .iter()
            .take(17)
            .any(|id| id.as_str().len() > 256)
    {
        return false;
    }
    if task.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference {
        return bounded_task(task)
            && task.schedulable_intent.as_ref().is_some_and(|i| {
                i.task_id == task.task_id
                    && i.node_id == task.node_id
                    && i.workflow_id == task.workflow_id
                    && i.workflow_run_id == task.workflow_run_id
                    && i.validate().is_ok()
            });
    }
    use crate::workflow::{
        WorkflowSchedulerNonRuntimeTaskTemplate as N, WorkflowSchedulerSourceInputTemplate as S,
    };
    task.node_type.len() <= 128
        && task.dependency_task_ids.len() <= 16
        && task.input_bindings.len() <= 16
        && task
            .input_bindings
            .iter()
            .all(|b| b.source_port_id.len() <= 128 && b.target_port_id.len() <= 128)
        && task.diagnostics.is_empty()
        && task.schedulable_intent.is_none()
        && task.schedulable_intent_template.is_none()
        && task.runtime_source_context.is_none()
        && task
            .non_runtime_task_template
            .as_ref()
            .is_none_or(|t| match t {
                N::JsonFilter { path } => path.len() <= 1024,
                _ => true,
            })
        && task
            .source_input_task_template
            .as_ref()
            .is_none_or(|t| match t {
                S::Text { port_id }
                | S::Boolean { port_id }
                | S::Integer { port_id }
                | S::Selection { port_id } => port_id.len() <= 128,
            })
        && bounded_serialized(task)
}
fn bounded_input_value(value: &crate::workflow::WorkflowSchedulerTaskResultValue) -> bool {
    use crate::workflow::WorkflowSchedulerTaskResultValue as V;
    match value {
        V::String(s) => s.len() <= 64 * 1024,
        V::MediaArtifactRef(r) => {
            r.artifact_id.len() <= 1024 && r.media_type.as_ref().is_none_or(|s| s.len() <= 128)
        }
        V::PumasModelRef(m) => {
            m.model_id.len() <= 128
                && m.revision.as_ref().is_none_or(|s| s.len() <= 128)
                && m.selected_artifact_id
                    .as_ref()
                    .is_none_or(|s| s.len() <= 128)
                && m.selected_artifact_path.is_none()
                && m.migration_diagnostics.is_empty()
        }
        V::Bool(_) | V::I64(_) | V::U64(_) | V::Json(serde_json::Value::Number(_)) => true,
        _ => false,
    }
}

use super::WorkflowExecutionSessionStore;
use crate::workflow::{
    bounded_proof, bounded_serialized, bounded_task, equivalent_environment,
    WorkflowSchedulerTaskExecutionClass, WorkflowSerialReadyMember, WorkflowSerialReadyPair,
    WorkflowServiceError,
};
use pantograph_scheduler::SchedulerTaskStateKind;

impl WorkflowExecutionSessionStore {
    pub(crate) fn validate_serial_service_admission(
        &self,
        session: &str,
        run: &str,
        admission: &crate::workflow::WorkflowSerialServiceAdmission,
    ) -> Result<(), WorkflowServiceError> {
        if self.serial_service_identity(session, run)? == admission.identity
            && self
                .serial_service_ready_member(session, run, admission.member.task.task_id.as_str())
                .as_ref()
                == Some(&admission.member)
        {
            Ok(())
        } else {
            Err(WorkflowServiceError::InvalidRequest(
                "fair serial Ready representative changed before Start".into(),
            ))
        }
    }
    pub(crate) fn serial_service_identity(
        &self,
        session: &str,
        run: &str,
    ) -> Result<
        (
            super::super::service_opportunities::ServiceOpportunityKey,
            i32,
        ),
        WorkflowServiceError,
    > {
        let state = self.active.get(session).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound("serial service session not found".into())
        })?;
        let active = state
            .active_run
            .as_ref()
            .filter(|active| active.workflow_run_id == run)
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest("serial service active run changed".into())
            })?;
        // Only attributed session creation populates this context, after backend
        // credential/session/bucket ownership checks. Session IDs are not units.
        let attribution = state.attribution.as_ref().ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(
                "fair serial service requires authenticated client/bucket session attribution"
                    .into(),
            )
        })?;
        Ok((
            super::super::service_opportunities::ServiceOpportunityKey {
                client_id: attribution.client_id.clone(),
                bucket_id: attribution.bucket_id.clone(),
            },
            active.priority,
        ))
    }

    pub(crate) fn serial_service_ready_member(
        &self,
        session: &str,
        run: &str,
        task: &str,
    ) -> Option<WorkflowSerialReadyMember> {
        let active = self.active.get(session)?.active_run.as_ref()?;
        let graph = active.scheduler_task_graph.as_ref()?;
        // Check before walking/cloning the graph. Never skip the first Ready task.
        if graph.tasks.len() > 128 || active.scheduler_task_records.len() > 128 {
            return None;
        }
        let first = graph.tasks.iter().find(|candidate| {
            candidate.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
                && active
                    .scheduler_task_records
                    .get(candidate.task_id.as_str())
                    .is_some_and(|record| record.state.kind() == SchedulerTaskStateKind::Ready)
        })?;
        (first.task_id.as_str() == task).then_some(())?;
        self.serial_ready_member(session, run, task)
    }

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
    /// Bounded immutable key-planning inputs, never a reservation or Ready grant.
    /// Unsupported members make the whole window fall back; none are skipped.
    pub(crate) fn serial_ready_window(
        &self,
        session: &str,
        run_id: &str,
        anchor: &str,
    ) -> Option<Vec<WorkflowSerialReadyMember>> {
        use crate::workflow::{
            WorkflowSchedulerTaskResultStatus, WorkflowSchedulerTaskResultValue,
        };
        use pantograph_runtime_host_contracts::{
            RuntimeHostExecutionInput, RuntimeHostExecutionInputValue,
        };

        if session.len() > 128 || run_id.len() > 128 || anchor.len() > 128 {
            return None;
        }
        let run = self.active.get(session)?.active_run.as_ref()?;
        if run.workflow_run_id.len() > 128 || run.workflow_run_id != run_id {
            return None;
        }
        let graph = run.scheduler_task_graph.as_ref()?;
        if graph.tasks.len() > 128 || run.scheduler_task_records.len() != graph.tasks.len() {
            return None;
        }
        // Hash/equality lookup keys must also be bounded for unselected tasks.
        if graph
            .tasks
            .iter()
            .any(|task| task.task_id.as_str().len() > 128)
        {
            return None;
        }
        let mut selected = Vec::with_capacity(8);
        for task in &graph.tasks {
            let record = run.scheduler_task_records.get(task.task_id.as_str())?;
            if task.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
                && record.state.kind() == SchedulerTaskStateKind::Ready
            {
                selected.push((task, record));
                if selected.len() == 8 {
                    break;
                }
            }
        }
        if selected.first()?.0.task_id.as_str() != anchor {
            return None;
        }

        // Only borrowed references until every selected member's raw metadata and
        // actual text pass. In particular, do not compare unbounded intents.
        let mut remaining_text = 64 * 1024usize;
        let mut checked = Vec::with_capacity(selected.len());
        for (task, record) in selected {
            let proof = run
                .runtime_dispatch_readiness_proofs
                .get(task.task_id.as_str())?;
            if !window_metadata_bounded(task, record, proof)
                || self.completion_cleanup_pending(session, run_id, task.task_id.as_str())
            {
                return None;
            }
            let [binding] = task.input_bindings.as_slice() else {
                return None;
            };
            let result = run
                .scheduler_task_results
                .get(binding.source_task_id.as_str())?;
            if result.status != WorkflowSchedulerTaskResultStatus::Completed
                || result.outputs.len() > 32
            {
                return None;
            }
            let output = result
                .outputs
                .iter()
                .find(|output| output.port_id == binding.source_port_id)?;
            let WorkflowSchedulerTaskResultValue::String(text) = &output.value else {
                return None;
            };
            if !window_raw_bounded(task, record, proof, text) {
                return None;
            }
            remaining_text = remaining_text.checked_sub(text.len())?;
            checked.push((task, record, proof, text));
        }
        // These are bounded structural checks, not execution admission. The
        // serialized limit also bounds the complete retained snapshot per member.
        for (task, record, proof, text) in &checked {
            if !bounded_serialized(&(task, record, proof, ("text", text)))
                || !equivalent_environment(task, task, proof)
                || record.state.execution_intent()?.runtime_task_intent()
                    != task.schedulable_intent.as_ref()
            {
                return None;
            }
        }
        Some(
            checked
                .into_iter()
                .map(|(task, record, proof, text)| WorkflowSerialReadyMember {
                    task: task.clone(),
                    record: record.clone(),
                    proof: proof.clone(),
                    inputs: vec![RuntimeHostExecutionInput {
                        port_id: "text".into(),
                        value: RuntimeHostExecutionInputValue::String(text.clone()),
                    }],
                })
                .collect(),
        )
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

impl WorkflowSerialReadyMember {
    /// Complete raw bounds before cache equality; deliberately performs no JSON
    /// encoding or workload-key construction on cache hits.
    pub(crate) fn grouping_bounded(&self) -> bool {
        use pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue;
        let [input] = self.inputs.as_slice() else {
            return false;
        };
        let RuntimeHostExecutionInputValue::String(text) = &input.value else {
            return false;
        };
        input.port_id == "text" && window_raw_bounded(&self.task, &self.record, &self.proof, text)
    }
}

fn window_raw_bounded(
    task: &crate::workflow::WorkflowSchedulerTask,
    record: &pantograph_scheduler::SchedulerTaskStateRecord,
    proof: &pantograph_dependency_planning::DependencyReadinessProofEnvelope,
    text: &str,
) -> bool {
    if text.is_empty()
        || text.len() > 64 * 1024
        || record.state.kind() != SchedulerTaskStateKind::Ready
        || !window_metadata_bounded(task, record, proof)
    {
        return false;
    }
    // Every dynamic string in the complete cloned/equality snapshot, including
    // repeated identity fields, counts toward this raw-byte budget.
    let intent = task.schedulable_intent.as_ref().unwrap();
    let template = task.schedulable_intent_template.as_ref().unwrap();
    let record_intent = record
        .state
        .execution_intent()
        .unwrap()
        .runtime_task_intent()
        .unwrap();
    let source = &template.dependency_readiness_source;
    let ctx = &proof.execution_context;
    let identity = &proof.preflight_result.identity_key;
    let mut bytes = text.len() + "text".len();
    let mut add = |s: &str| {
        bytes += s.len();
    };
    for s in [
        task.workflow_id.as_str(),
        task.workflow_run_id.as_str(),
        task.node_id.as_str(),
        task.task_id.as_str(),
        task.node_type.as_str(),
        record.workflow_id.as_str(),
        record.workflow_run_id.as_str(),
        record.node_id.as_str(),
        record.task_id.as_str(),
        record.last_transition_id.as_str(),
        template.task_type.as_str(),
        source.graph_revision.as_str(),
        source.descriptor_fingerprint.as_str(),
        source.dependency_requirements_id.as_str(),
        source.dependency_override_fingerprint.as_str(),
        ctx.workflow_id.as_str(),
        ctx.workflow_run_id.as_str(),
        ctx.scheduler_task_id.as_str(),
        ctx.node_id.as_str(),
        ctx.graph_revision.as_str(),
        ctx.descriptor_fingerprint.as_str(),
        ctx.dependency_requirements_id.as_str(),
        ctx.correlation_id.as_str(),
        identity.task_id.as_str(),
        proof.readiness_proof_id.as_str(),
    ] {
        add(s);
    }
    for intent in [intent, record_intent] {
        for s in [
            intent.workflow_id.as_str(),
            intent.workflow_run_id.as_str(),
            intent.node_id.as_str(),
            intent.task_id.as_str(),
            intent.task_type.as_str(),
        ] {
            add(s);
        }
        if let Some(s) = &intent.fairness_key {
            add(s.as_str());
        }
        add(intent
            .constraints
            .requested_runtime_id
            .as_ref()
            .unwrap()
            .as_str());
        add(intent
            .constraints
            .requested_device_id
            .as_ref()
            .unwrap()
            .as_str());
    }
    for model in [
        &intent.model_ref,
        &record_intent.model_ref,
        &identity.model_ref,
    ] {
        add(&model.model_id);
        if let Some(s) = &model.revision {
            add(s);
        }
        if let Some(s) = &model.selected_artifact_id {
            add(s);
        }
    }
    for id in &task.dependency_task_ids {
        add(id.as_str());
    }
    for binding in &task.input_bindings {
        for s in [
            binding.source_node_id.as_str(),
            binding.source_task_id.as_str(),
            &binding.source_port_id,
            &binding.target_port_id,
        ] {
            add(s);
        }
    }
    if let Some(s) = &task.inference_descriptor_fingerprint {
        add(s.as_str());
    }
    if let Some(c) = &task.runtime_source_context {
        add(&c.operation_type);
        add(&c.context_shape_key);
        add(&c.cancellation_mode);
    }
    add(template
        .constraints
        .requested_runtime_id
        .as_ref()
        .unwrap()
        .as_str());
    add(template
        .constraints
        .requested_device_id
        .as_ref()
        .unwrap()
        .as_str());
    for ids in [
        &source.selected_binding_ids,
        &ctx.selected_binding_ids,
        &identity.selected_binding_ids,
    ] {
        for id in ids {
            add(id.as_str());
        }
    }
    if let Some(s) = &source.validation_session_id {
        add(s.as_str());
    }
    if let Some(s) = &source.validation_snapshot_id {
        add(s.as_str());
    }
    if let Some(s) = &ctx.validation_session_id {
        add(s.as_str());
    }
    if let Some(s) = &ctx.validation_snapshot_id {
        add(s.as_str());
    }
    if let Some(s) = &ctx.dependency_override_fingerprint {
        add(s.as_str());
    }
    if let Some(s) = &identity.task_type {
        add(s.as_str());
    }
    if let Some(s) = &identity.scheduler_intent.requested_runtime_id {
        add(s.as_str());
    }
    if let Some(s) = &identity.scheduler_intent.requested_device_id {
        add(s.as_str());
    }
    if let Some(c) = &identity.platform_context {
        add(c.platform_key.as_str());
    }
    if let Some(s) = &proof.preflight_result.dependency_requirements_id {
        add(s.as_str());
    }
    if let Some(e) = &proof.preflight_result.environment_ref {
        add(e.environment_id.as_str());
        if let Some(s) = &e.manifest_id {
            add(s.as_str());
        }
    }
    // Addition cannot overflow: raw preflight fixes every count and scalar cap.
    bytes <= 64 * 1024
}

// C2a deliberately accepts less than the generic scheduler. Every variable-size
// field of the cloned task, Ready record and proof is either bounded here or
// required absent/empty before any equality, serialization or metadata clone.
fn window_model_bounded(model: &pantograph_dependency_planning::PumasModelRef) -> bool {
    model.model_id.len() <= 128
        && model.revision.as_ref().is_none_or(|s| s.len() <= 128)
        && model
            .selected_artifact_id
            .as_ref()
            .is_none_or(|s| s.len() <= 128)
        && model.selected_artifact_path.is_none()
        && model.migration_diagnostics.is_empty()
}

fn window_intent_bounded(intent: &pantograph_scheduler::SchedulableTaskIntent) -> bool {
    [
        intent.workflow_id.as_str(),
        intent.workflow_run_id.as_str(),
        intent.node_id.as_str(),
        intent.task_id.as_str(),
        intent.task_type.as_str(),
    ]
    .iter()
    .all(|s| s.len() <= 128)
        && intent
            .fairness_key
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && intent.task_type.as_str() == "embedding"
        && window_model_bounded(&intent.model_ref)
        && intent
            .constraints
            .requested_runtime_id
            .as_ref()
            .is_some_and(|s| s.as_str() == "candle")
        && intent
            .constraints
            .requested_device_id
            .as_ref()
            .is_some_and(|s| s.as_str() == "cpu")
        && intent.trait_settings.is_empty()
        && intent.dependency_override_patches.is_empty()
        && intent.estimate_hints.len() <= 8
}

fn window_metadata_bounded(
    task: &crate::workflow::WorkflowSchedulerTask,
    record: &pantograph_scheduler::SchedulerTaskStateRecord,
    proof: &pantograph_dependency_planning::DependencyReadinessProofEnvelope,
) -> bool {
    let (Some(intent), Some(template), Some(record_intent)) = (
        task.schedulable_intent.as_ref(),
        task.schedulable_intent_template.as_ref(),
        record
            .state
            .execution_intent()
            .and_then(|execution| execution.runtime_task_intent()),
    ) else {
        return false;
    };
    let source = &template.dependency_readiness_source;
    let ctx = &proof.execution_context;
    let identity = &proof.preflight_result.identity_key;
    let metadata_strings = [
        task.workflow_id.as_str(),
        task.workflow_run_id.as_str(),
        task.node_id.as_str(),
        task.task_id.as_str(),
        task.node_type.as_str(),
        record.workflow_id.as_str(),
        record.workflow_run_id.as_str(),
        record.node_id.as_str(),
        record.task_id.as_str(),
        record.last_transition_id.as_str(),
        template.task_type.as_str(),
        source.graph_revision.as_str(),
        source.descriptor_fingerprint.as_str(),
        source.dependency_requirements_id.as_str(),
        source.dependency_override_fingerprint.as_str(),
        ctx.workflow_id.as_str(),
        ctx.workflow_run_id.as_str(),
        ctx.scheduler_task_id.as_str(),
        ctx.node_id.as_str(),
        ctx.graph_revision.as_str(),
        ctx.descriptor_fingerprint.as_str(),
        ctx.dependency_requirements_id.as_str(),
        ctx.correlation_id.as_str(),
        identity.task_id.as_str(),
        proof.readiness_proof_id.as_str(),
    ];
    task.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
        && metadata_strings.iter().all(|s| s.len() <= 128)
        && window_intent_bounded(intent)
        && window_intent_bounded(record_intent)
        && task.dependency_task_ids.len() <= 16
        && task
            .dependency_task_ids
            .iter()
            .all(|s| s.as_str().len() <= 128)
        && task.input_bindings.len() == 1
        && task.input_bindings.iter().all(|b| {
            b.source_node_id.as_str().len() <= 128
                && b.source_task_id.as_str().len() <= 128
                && b.source_port_id.len() <= 128
                && b.target_port_id == "text"
        })
        && task.diagnostics.is_empty()
        && task.non_runtime_task_template.is_none()
        && task.source_input_task_template.is_none()
        && task
            .inference_descriptor_fingerprint
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && task.runtime_source_context.as_ref().is_none_or(|c| {
            c.operation_type.len() <= 128
                && c.context_shape_key.len() <= 128
                && c.cancellation_mode.len() <= 128
        })
        && template.task_type.as_str() == "embedding"
        && template
            .constraints
            .requested_runtime_id
            .as_ref()
            .is_some_and(|s| s.as_str() == "candle")
        && template
            .constraints
            .requested_device_id
            .as_ref()
            .is_some_and(|s| s.as_str() == "cpu")
        && template.trait_settings.is_empty()
        && template.dependency_override_patches.is_empty()
        && template.estimate_hints.len() <= 8
        && source.selected_binding_ids.len() <= 8
        && source
            .selected_binding_ids
            .iter()
            .all(|s| s.as_str().len() <= 128)
        && source
            .validation_session_id
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && source
            .validation_snapshot_id
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && ctx.selected_binding_ids.len() <= 8
        && ctx
            .selected_binding_ids
            .iter()
            .all(|s| s.as_str().len() <= 128)
        && ctx
            .validation_session_id
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && ctx
            .validation_snapshot_id
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && ctx
            .dependency_override_fingerprint
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && identity.selected_binding_ids.len() <= 8
        && identity
            .selected_binding_ids
            .iter()
            .all(|s| s.as_str().len() <= 128)
        && window_model_bounded(&identity.model_ref)
        && identity
            .task_type
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && identity
            .scheduler_intent
            .requested_runtime_id
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && identity
            .scheduler_intent
            .requested_device_id
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && identity
            .platform_context
            .as_ref()
            .is_none_or(|c| c.platform_key.as_str().len() <= 128)
        && proof
            .preflight_result
            .dependency_requirements_id
            .as_ref()
            .is_none_or(|s| s.as_str().len() <= 128)
        && proof
            .preflight_result
            .environment_ref
            .as_ref()
            .is_none_or(|e| {
                e.environment_id.as_str().len() <= 128
                    && e.manifest_id
                        .as_ref()
                        .is_none_or(|s| s.as_str().len() <= 128)
            })
        && proof.preflight_result.diagnostics.is_empty()
}

#[cfg(test)]
#[path = "store_serial_ready_tests.rs"]
mod tests;

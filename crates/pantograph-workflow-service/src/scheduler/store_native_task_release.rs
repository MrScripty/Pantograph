//! Historical exact task-release latch; not full-envelope admission authority.
use super::*;
use crate::scheduler::store::{
    WorkflowSchedulerTaskAttemptId, WorkflowSchedulerTaskReservationBinding,
};
use crate::workflow::native_task_release::WorkflowNativeTaskReleaseObserver;
use crate::workflow::WorkflowSchedulerTaskResult;
use inference::gateway::CandleCpuVerifiedTaskRelease;
use pantograph_runtime_host_contracts::{
    RuntimeHostBatchExecutionRequest, RuntimeHostExecutionInputValue,
};
use pantograph_runtime_registry::RuntimeRegistry;
#[cfg(any(test, feature = "test-support"))]
use pantograph_scheduler::{SchedulerDispatchCandidateId, SchedulerReservationLeaseId};
use pantograph_scheduler::{
    SchedulerProtectedEpisode, SchedulerProtectedOutcome, SchedulerProtectedPhase,
    SchedulerSerialAttemptIdentity, SchedulerTaskStateRecord, SchedulerTaskStateTransition,
};
use std::sync::Mutex;

pub(crate) struct WorkflowNativeReleaseBinding {
    episode: SchedulerProtectedEpisode,
    intent: ValidatedSchedulableTaskIntent,
    tags: [String; 7],
    reservation_id: u64,
    input: [u8; 32],
    dispatch: [u8; 32],
    gateway: Arc<inference::InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    dispatch_checked: bool,
    acknowledged: bool,
    produced_output: Option<[u8; 32]>,
    pub(in crate::scheduler::store) output_accepted: bool,
}
impl std::fmt::Debug for WorkflowNativeReleaseBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowNativeReleaseBinding")
            .field("episode", &self.episode)
            .field("reservation_id", &self.reservation_id)
            .field("dispatch_checked", &self.dispatch_checked)
            .field("acknowledged", &self.acknowledged)
            .finish()
    }
}
impl WorkflowNativeReleaseBinding {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        episode: SchedulerProtectedEpisode,
        intent: ValidatedSchedulableTaskIntent,
        identity: SchedulerSerialAttemptIdentity<'_>,
        input: [u8; 32],
        dispatch: [u8; 32],
        gateway: Arc<inference::InferenceGateway>,
        registry: Arc<RuntimeRegistry>,
    ) -> Result<Self, WorkflowServiceError> {
        if !crate::workflow::native_task_release::identity_is_bounded(identity) {
            return Err(invalid("native identity byte limit"));
        }
        let tags = [
            identity.workflow_id,
            identity.workflow_run_id,
            identity.node_id,
            identity.task_id,
            identity.attempt_id,
            identity.execution_request_id,
            identity.candidate_id,
        ];
        let reservation_id = identity
            .reservation_lease_id
            .strip_prefix("runtime-registry.")
            .and_then(|n| n.parse::<u64>().ok())
            .ok_or_else(|| invalid("native lease identity"))?;
        let i = intent.as_ref();
        if episode.phase != SchedulerProtectedPhase::Running
            || !bounded_intent(&intent)
            || !tags.iter().all(|s| bounded(s, 128))
            || identity.reservation_lease_id != format!("runtime-registry.{reservation_id}")
            || episode.target.workflow_id != i.workflow_id
            || episode.target.workflow_run_id != i.workflow_run_id
            || episode.target.node_id != i.node_id
            || episode.target.task_id != i.task_id
            || Some(&episode.target.fairness_key) != i.fairness_key.as_ref()
            || tags[..4]
                != [
                    i.workflow_id.as_str(),
                    i.workflow_run_id.as_str(),
                    i.node_id.as_str(),
                    i.task_id.as_str(),
                ]
            || i.task_type.as_str() != "embedding"
            || !i.trait_settings.is_empty()
            || i.constraints
                .requested_runtime_id
                .as_ref()
                .map(|s| s.as_str())
                != Some("candle")
            || i.constraints
                .requested_device_id
                .as_ref()
                .map(|s| s.as_str())
                != Some("cpu")
        {
            return Err(invalid("native running attempt binding refused"));
        }
        Ok(Self {
            episode,
            intent,
            tags: tags.map(str::to_owned),
            reservation_id,
            input,
            dispatch,
            gateway,
            registry,
            dispatch_checked: false,
            acknowledged: false,
            produced_output: None,
            output_accepted: false,
        })
    }
    fn matches_identity(&self, id: SchedulerSerialAttemptIdentity<'_>) -> bool {
        self.tags.iter().map(String::as_str).eq([
            id.workflow_id,
            id.workflow_run_id,
            id.node_id,
            id.task_id,
            id.attempt_id,
            id.execution_request_id,
            id.candidate_id,
        ]) && id.reservation_lease_id == format!("runtime-registry.{}", self.reservation_id)
    }

    /// Preflight inside the result transaction, before any task mutation. The
    /// caller latches acceptance only after its exact Running -> Completed
    /// transition, attempt removal and result insertion all succeed.
    pub(in crate::scheduler::store) fn validate_output_commit(
        &self,
        attempt: &WorkflowSchedulerTaskAttemptId,
        reservation: Option<&WorkflowSchedulerTaskReservationBinding>,
        current: &SchedulerTaskStateRecord,
        transition: &SchedulerTaskStateTransition,
        result: &WorkflowSchedulerTaskResult,
    ) -> Result<bool, WorkflowServiceError> {
        use crate::workflow::{
            WorkflowSchedulerTaskResultStatus, WorkflowSchedulerTaskResultValue,
        };
        if result.status != WorkflowSchedulerTaskResultStatus::Completed {
            // Preserve established failure/cancellation terminal mutations.
            return Ok(false);
        }
        if result.workflow_run_id != self.tags[1] || result.task_id != self.tags[3] {
            return Ok(false);
        }
        let reservation =
            reservation.ok_or_else(|| invalid("native output reservation missing"))?;
        if !self.dispatch_checked
            || self.output_accepted
            || result.workflow_id != self.tags[0]
            || result.node_id != self.tags[2]
            || attempt.as_str() != self.tags[4]
            || reservation.task_id.as_str() != self.tags[3]
            || reservation.candidate_id.as_ref().map(|c| c.as_str()) != Some(self.tags[6].as_str())
            || reservation.reservation_lease_id.as_str()
                != format!("runtime-registry.{}", self.reservation_id)
            || current.state.task_intent() != Some(self.intent.as_ref())
            || transition.next_state.task_intent() != Some(self.intent.as_ref())
            || result.status != WorkflowSchedulerTaskResultStatus::Completed
        {
            return Err(invalid("native task-output commit identity refused"));
        }
        // The accepted scope is the embedding vector only. Metadata, usage,
        // artifacts, downstream nodes and client delivery remain separate.
        let mut outputs = result.outputs.iter().filter(|o| o.port_id == "embedding");
        let Some(output) = outputs.next() else {
            return Err(invalid("native embedding output missing"));
        };
        if outputs.next().is_some() {
            return Err(invalid("duplicate native embedding output"));
        }
        let WorkflowSchedulerTaskResultValue::Json(value) = &output.value else {
            return Err(invalid("native embedding output type"));
        };
        let vector = value
            .as_array()
            .ok_or_else(|| invalid("native embedding vector type"))?;
        if !(1..=4096).contains(&vector.len()) {
            return Err(invalid("native embedding vector bounds"));
        }
        let fingerprint = inference::gateway::candle_cpu_vector_fingerprint(
            vector.iter().map(|v| v.as_f64().unwrap_or(f64::NAN)),
        )
        .ok_or_else(|| invalid("native embedding vector is not finite numeric output"))?;
        if self.produced_output != Some(fingerprint) {
            return Err(invalid(
                "native embedding differs from sealed producer output",
            ));
        }
        Ok(true)
    }
    pub(in crate::scheduler::store) fn preflight_output_result_bounds(
        &self,
        result: &WorkflowSchedulerTaskResult,
        transition: &SchedulerTaskStateTransition,
    ) -> Result<(), WorkflowServiceError> {
        if result.workflow_run_id == self.tags[1]
            && result.task_id == self.tags[3]
            && result.status == crate::workflow::WorkflowSchedulerTaskResultStatus::Completed
            && !inference::gateway::CandleCpuWarmAttemptRequest::metadata_is_bounded(&(
                result, transition,
            ))
        {
            return Err(invalid("native task-output raw result budget"));
        }
        Ok(())
    }
}
pub(crate) struct WorkflowNativeReleaseSink(pub(crate) Arc<Mutex<WorkflowExecutionSessionStore>>);
impl WorkflowExecutionSessionStore {
    pub(in crate::scheduler::store) fn preflight_native_output_result(
        &self,
        result: &WorkflowSchedulerTaskResult,
        transition: &SchedulerTaskStateTransition,
    ) -> Result<(), WorkflowServiceError> {
        if let Some(p) = &self.progress_commit {
            if let Some(b) = &p.native_release {
                if result.workflow_run_id == b.tags[1]
                    && result.task_id == b.tags[3]
                    && result.status
                        == crate::workflow::WorkflowSchedulerTaskResultStatus::Completed
                    && p.decision.as_ref().and_then(|d| d.active()) != Some(&b.episode)
                {
                    return Err(invalid("native task-output episode no longer current"));
                }
                b.preflight_output_result_bounds(result, transition)?;
            }
        }
        Ok(())
    }
    pub(crate) fn bind_native_task_release(
        &mut self,
        binding: WorkflowNativeReleaseBinding,
    ) -> Result<(), WorkflowServiceError> {
        // Freeze against a fresh full owner capture. This is not a native
        // provider, and installation cannot upgrade controlled facts to one.
        let facts = self.advance_queue_progress(SchedulerProtectedEvent::Observe)?;
        let p = self.progress_commit.as_mut().expect("installed");
        if p.native_release.is_some()
            || p.decision.as_ref().and_then(|d| d.active()) != Some(&binding.episode)
            || !facts.opportunities.iter().any(|o| {
                o.intent == binding.intent
                    && o.generation == binding.episode.target.generation
                    && o.episode_contract == binding.episode.target.episode_contract_identity
            })
        {
            return Err(invalid(
                "native binding differs from current owner opportunity",
            ));
        }
        p.native_release = Some(binding);
        Ok(())
    }
    pub(super) fn check_native_finish(
        &self,
        event: &SchedulerProtectedEvent,
    ) -> Result<bool, WorkflowServiceError> {
        let Some(b) = self
            .progress_commit
            .as_ref()
            .and_then(|p| p.native_release.as_ref())
        else {
            return Ok(false);
        };
        if let SchedulerProtectedEvent::Finished {
            turn_id,
            attempt,
            physical_release_acknowledged,
            outcome,
        } = event
        {
            if *turn_id != b.episode.turn_id
                || *attempt != b.episode.attempt
                || !*physical_release_acknowledged
                || !b.acknowledged
            {
                return Err(invalid(
                    "native task release not acknowledged for this running attempt",
                ));
            }
            if *outcome == SchedulerProtectedOutcome::AcceptedOutput && !b.output_accepted {
                return Err(invalid(
                    "native task output was not accepted by the result owner",
                ));
            }
            return Ok(true);
        }
        Ok(false)
    }
    fn native_binding_mut(
        &mut self,
    ) -> Result<&mut WorkflowNativeReleaseBinding, WorkflowServiceError> {
        let p = self
            .progress_commit
            .as_mut()
            .ok_or_else(|| invalid("native owner not installed"))?;
        let b = p
            .native_release
            .as_mut()
            .ok_or_else(|| invalid("native attempt not bound"))?;
        if p.decision.as_ref().and_then(|d| d.active()) != Some(&b.episode) {
            return Err(invalid("native attempt no longer current"));
        }
        Ok(b)
    }
    pub(crate) fn native_task_release_acknowledged(&self) -> bool {
        self.progress_commit
            .as_ref()
            .and_then(|p| p.native_release.as_ref())
            .is_some_and(|b| b.acknowledged)
    }
}
impl WorkflowNativeTaskReleaseObserver for WorkflowNativeReleaseSink {
    fn observe_task_output(
        &self,
        proof: &inference::gateway::CandleCpuVerifiedTaskOutput<'_>,
    ) -> Result<(), WorkflowServiceError> {
        // Historical production is latched without fresh provider capture and
        // without taking native/registry locks. It is never result acceptance.
        let mut store = self
            .0
            .lock()
            .map_err(|_| invalid("native owner lock poisoned"))?;
        let b = store.native_binding_mut()?;
        let id = proof.identity();
        let wire = format!("runtime-registry.{}", id.reservation_lease_id);
        let serial = SchedulerSerialAttemptIdentity {
            workflow_id: id.workflow_id,
            workflow_run_id: id.workflow_run_id,
            node_id: id.node_id,
            task_id: id.task_id,
            attempt_id: id.attempt_id,
            execution_request_id: id.execution_request_id,
            candidate_id: id.candidate_id,
            reservation_lease_id: &wire,
        };
        let expected = &b.intent.as_ref().model_ref;
        let actual = proof.model_ref();
        if !b.dispatch_checked
            || !b.matches_identity(serial)
            || proof.input_fingerprint() != b.input
            || !proof.matches_authority(&b.gateway, &b.registry)
            || actual.model_id != expected.model_id
            || actual.revision != expected.revision
            || actual.selected_artifact_id != expected.selected_artifact_id
            || actual.selected_artifact_path != expected.selected_artifact_path
            || !actual.migration_diagnostics.is_empty()
            || b.produced_output
                .is_some_and(|old| old != proof.vector_fingerprint())
        {
            return Err(invalid("native sealed output binding refused"));
        }
        b.produced_output = Some(proof.vector_fingerprint());
        Ok(())
    }
    fn check_dispatch(
        &self,
        request: &RuntimeHostBatchExecutionRequest,
        id: &SchedulerSerialAttemptIdentity<'_>,
    ) -> Result<(), WorkflowServiceError> {
        if !inference::gateway::CandleCpuWarmAttemptRequest::metadata_is_bounded(request)
            || request.members.len() != 1
        {
            return Err(invalid("native dispatch metadata bounds"));
        }
        let mut store = self
            .0
            .lock()
            .map_err(|_| invalid("native owner lock poisoned"))?;
        let b = store.native_binding_mut()?;
        let m = &request.members[0];
        let [input] = m.materialized_inputs.as_slice() else {
            return Err(invalid("native singleton input"));
        };
        let RuntimeHostExecutionInputValue::String(text) = &input.value else {
            return Err(invalid("native text input"));
        };
        let selected = m
            .handoff
            .dispatch_decision
            .as_ref()
            .ok_or_else(|| invalid("native selected dispatch missing"))?;
        let selected_bytes =
            serde_json::to_vec(selected).map_err(|_| invalid("native dispatch serialization"))?;
        if *blake3::hash(&selected_bytes).as_bytes() != b.dispatch {
            return Err(invalid("native selected dispatch changed"));
        }
        if b.acknowledged
            || !b.matches_identity(*id)
            || m.handoff.task_intent != *b.intent.as_ref()
            || m.execution_request_id != b.tags[5]
            || input.port_id != "text"
            || text.is_empty()
            || text.len() > 4096
            || *blake3::hash(text.as_bytes()).as_bytes() != b.input
            || m.handoff
                .dispatch_decision
                .as_ref()
                .is_none_or(|d| d.reservation_lease_id.as_str() != id.reservation_lease_id)
        {
            return Err(invalid(
                "native full intent/input/incarnation changed before dispatch",
            ));
        }
        b.dispatch_checked = true;
        Ok(())
    }
    fn acknowledge_task_release(
        &self,
        proof: &CandleCpuVerifiedTaskRelease,
    ) -> Result<(), WorkflowServiceError> {
        // Deliberately no provider capture or registry/native access under this
        // lock: fresh evidence may be unavailable after an irreversible release.
        let mut store = self
            .0
            .lock()
            .map_err(|_| invalid("native owner lock poisoned"))?;
        let b = store.native_binding_mut()?;
        let id = proof.identity();
        let wire = format!("runtime-registry.{}", id.reservation_lease_id);
        let serial_id = SchedulerSerialAttemptIdentity {
            workflow_id: id.workflow_id,
            workflow_run_id: id.workflow_run_id,
            node_id: id.node_id,
            task_id: id.task_id,
            attempt_id: id.attempt_id,
            execution_request_id: id.execution_request_id,
            candidate_id: id.candidate_id,
            reservation_lease_id: &wire,
        };
        let expected = &b.intent.as_ref().model_ref;
        let actual = proof.model_ref();
        if !b.dispatch_checked
            || !b.matches_identity(serial_id)
            || proof.input_fingerprint() != b.input
            || !proof.matches_authority(&b.gateway, &b.registry)
            || actual.model_id != expected.model_id
            || actual.revision != expected.revision
            || actual.selected_artifact_id != expected.selected_artifact_id
            || actual.selected_artifact_path != expected.selected_artifact_path
            || !actual.migration_diagnostics.is_empty()
        {
            return Err(invalid("native sealed task release binding refused"));
        }
        b.acknowledged = true; // Exact duplicate is idempotent; cannot rebind.
        Ok(())
    }
}

#[cfg(any(test, feature = "test-support"))]
impl WorkflowExecutionSessionStore {
    pub(crate) fn native_fixture_output_accepted(&self) -> bool {
        self.progress_commit
            .as_ref()
            .and_then(|p| p.native_release.as_ref())
            .is_some_and(|b| b.output_accepted)
    }
    pub(crate) fn commit_native_fixture_task_output(
        &mut self,
        result: WorkflowSchedulerTaskResult,
        completion_attempt: Option<&str>,
    ) -> Result<(), WorkflowServiceError> {
        use crate::workflow::{
            WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
            WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
        };
        let b = self.native_binding_mut()?;
        let intent = b.intent.as_ref().clone();
        let attempt = WorkflowSchedulerTaskAttemptId::parse(b.tags[4].clone())?;
        let candidate = SchedulerDispatchCandidateId::parse(&b.tags[6])
            .map_err(|_| invalid("fixture candidate"))?;
        let lease =
            SchedulerReservationLeaseId::parse(format!("runtime-registry.{}", b.reservation_id))
                .map_err(|_| invalid("fixture lease"))?;
        let session = self
            .active
            .iter()
            .find(|(_, s)| {
                s.active_run
                    .as_ref()
                    .is_some_and(|r| r.workflow_run_id == intent.workflow_run_id.as_str())
            })
            .map(|(id, _)| id.clone())
            .ok_or_else(|| invalid("fixture run"))?;
        let transition = |state, previous, tag: &str| SchedulerTaskStateTransition {
            contract_version: pantograph_scheduler::SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            transition_id: tag.parse().unwrap(),
            workflow_id: intent.workflow_id.clone(),
            workflow_run_id: intent.workflow_run_id.clone(),
            node_id: intent.node_id.clone(),
            task_id: intent.task_id.clone(),
            expected_previous_state: Some(previous),
            next_state: state,
        };
        let execution = pantograph_scheduler::SchedulerTaskExecutionIntent::runtime(intent.clone());
        let missing = !self.active[&session]
            .active_run
            .as_ref()
            .unwrap()
            .scheduler_task_records
            .contains_key(intent.task_id.as_str());
        if missing {
            let graph = WorkflowSchedulerTaskGraph {
                schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
                workflow_id: intent.workflow_id.clone(),
                workflow_run_id: intent.workflow_run_id.clone(),
                tasks: vec![WorkflowSchedulerTask {
                    workflow_id: intent.workflow_id.clone(),
                    workflow_run_id: intent.workflow_run_id.clone(),
                    node_id: intent.node_id.clone(),
                    task_id: intent.task_id.clone(),
                    node_type: "embedding-inference".into(),
                    execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
                    dependency_task_ids: vec![],
                    input_bindings: vec![],
                    schedulable_intent: Some(intent.clone()),
                    schedulable_intent_template: None,
                    non_runtime_task_template: None,
                    source_input_task_template: None,
                    inference_descriptor_fingerprint: None,
                    runtime_source_context: None,
                    diagnostics: vec![],
                }],
            };
            let record = SchedulerTaskStateRecord {
                contract_version: pantograph_scheduler::SCHEDULER_TASK_STATE_CONTRACT_VERSION,
                workflow_id: intent.workflow_id.clone(),
                workflow_run_id: intent.workflow_run_id.clone(),
                node_id: intent.node_id.clone(),
                task_id: intent.task_id.clone(),
                state: pantograph_scheduler::SchedulerTaskState::Ready {
                    execution_intent: execution.clone(),
                },
                state_version: 1,
                last_transition_id: "fixture.ready".parse().unwrap(),
            };
            self.set_active_run_scheduler_task_state(
                &session,
                intent.workflow_run_id.as_str(),
                graph,
                vec![record],
            )?;
            let _started = self.start_active_run_scheduler_task_attempt(
                &session,
                intent.workflow_run_id.as_str(),
                attempt.clone(),
                transition(
                    pantograph_scheduler::SchedulerTaskState::Running {
                        execution_intent: execution.clone(),
                    },
                    pantograph_scheduler::SchedulerTaskStateKind::Ready,
                    "fixture.running",
                ),
            )?;
            self.bind_active_run_scheduler_task_reservation(
                &session,
                intent.workflow_run_id.as_str(),
                &intent.task_id,
                &attempt,
                lease,
                Some(candidate),
            )?;
        }
        let completion_attempt = completion_attempt
            .map(WorkflowSchedulerTaskAttemptId::parse)
            .transpose()?
            .unwrap_or(attempt);
        let terminal = if result.status
            == crate::workflow::WorkflowSchedulerTaskResultStatus::Completed
        {
            pantograph_scheduler::SchedulerTaskState::Completed {
                execution_intent: execution,
            }
        } else {
            pantograph_scheduler::SchedulerTaskState::TerminalFailed {
                diagnostics: vec![pantograph_scheduler::SchedulerTaskStateDiagnostic {
                    severity: pantograph_scheduler::SchedulerTaskStateDiagnosticSeverity::Error,
                    code: pantograph_scheduler::SchedulerTaskStateDiagnosticCode::TerminalFailure,
                    message: "controlled native task failed".into(),
                    hint: None,
                }],
            }
        };
        self.complete_active_run_scheduler_task(
            &session,
            intent.workflow_run_id.as_str(),
            &completion_attempt,
            transition(
                terminal,
                pantograph_scheduler::SchedulerTaskStateKind::Running,
                "fixture.completed",
            ),
            result,
        )?;
        Ok(())
    }
    pub(crate) fn is_empty_for_native_fixture(&self) -> bool {
        self.active.is_empty() && self.progress_commit.is_none()
    }
    pub(crate) fn native_fixture_episode(&self) -> Option<SchedulerProtectedEpisode> {
        self.progress_commit
            .as_ref()
            .and_then(|p| p.decision.as_ref())
            .and_then(|d| d.active())
            .cloned()
    }
}

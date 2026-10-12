//! Constructor-owned, opt-in serial CPU service ordering. Observations cover
//! actual host execution/drain and matched release, not preparation/load plans.
use super::{WorkflowSchedulerTask, WorkflowService, WorkflowServiceError};
use async_trait::async_trait;
use pantograph_runtime_host_contracts::*;
use pantograph_scheduler::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WorkflowSerialReadyMember {
    pub(crate) task: WorkflowSchedulerTask,
    pub(crate) record: SchedulerTaskStateRecord,
    pub(crate) proof: pantograph_dependency_planning::DependencyReadinessProofEnvelope,
    pub(crate) inputs: Vec<RuntimeHostExecutionInput>,
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WorkflowSerialReadyPair {
    pub(crate) members: [WorkflowSerialReadyMember; 2],
    pub(crate) population: Vec<(
        String,
        super::WorkflowSchedulerTaskExecutionClass,
        SchedulerTaskStateKind,
        u64,
    )>,
}

#[derive(Clone, Copy)]
pub struct WorkflowSerialReadyConfig {
    minimum_samples: usize,
    maximum_age: Duration,
}
impl WorkflowSerialReadyConfig {
    pub fn new(
        minimum_samples: usize,
        maximum_age: Duration,
    ) -> Result<Self, WorkflowServiceError> {
        if !(3..=8).contains(&minimum_samples)
            || maximum_age.is_zero()
            || maximum_age > Duration::from_secs(3600)
        {
            return Err(WorkflowServiceError::InvalidRequest(
                "serial Ready config requires 3–8 samples and age in (0, 1h]".into(),
            ));
        }
        Ok(Self {
            minimum_samples,
            maximum_age,
        })
    }
}
impl Default for WorkflowSerialReadyConfig {
    fn default() -> Self {
        Self {
            minimum_samples: 3,
            maximum_age: Duration::from_secs(60),
        }
    }
}

pub(crate) struct SerialReadyMode {
    pub(crate) port: Arc<dyn SerialRuntimeHostBatchExecutionPort>,
    admission: SchedulerSerialAdmission,
    semaphore: Arc<tokio::sync::Semaphore>,
    active: Mutex<Option<Arc<Mutex<SerialReadyContext>>>>,
    observations: Mutex<VecDeque<Observation>>,
    config: WorkflowSerialReadyConfig,
}
struct Observation {
    key: [u8; 32],
    owner: SerialRuntimeHostCpuOwnerEvidence,
    at: Instant,
    cost_ns: u64,
    #[cfg(any(test, feature = "test-support"))]
    task_id: String,
}
/// A bounded proposal, never execution or cleanup authority.
#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SerialReadyReason {
    OwnerUnavailable,
    IncomparableOwners,
    UnsupportedInputs,
    HistoryUnavailable,
    InvalidClock,
    InsufficientFreshSamples,
    OwnerChanged,
    Ranked,
}
pub(crate) struct SerialReadyDecision {
    pub(crate) position: usize,
    pub(crate) owner: Option<SerialRuntimeHostCpuOwnerEvidence>,
    pub(crate) reason: SerialReadyReason,
    pub(crate) counts: [usize; 2],
    pub(crate) maxima_ns: [u64; 2],
    pub(crate) oldest_age_ns: [u64; 2],
}
impl SerialReadyDecision {
    fn fifo(reason: SerialReadyReason) -> Self {
        Self {
            position: 0,
            owner: None,
            reason,
            counts: [0; 2],
            maxima_ns: [0; 2],
            oldest_age_ns: [0; 2],
        }
    }
}
pub(crate) struct SerialReadyContext {
    preparation: Option<SchedulerSerialPreparation>,
    dispatch: Option<SchedulerSerialDispatch>,
    drained: Option<SchedulerSerialDrainedDispatch>,
    cleanup: Option<Box<dyn SerialRuntimeHostCleanupReceipt>>,
    claimed: bool,
    cleanup_acknowledged: bool,
    successful: bool,
    cancellation: Option<RuntimeHostExecutionCancellationHandle>,
    pub(crate) task: Option<WorkflowSchedulerTask>,
    member: Option<WorkflowSerialReadyMember>,
    pub(crate) owner: Option<SerialRuntimeHostCpuOwnerEvidence>,
    key: Option<[u8; 32]>,
    started: Option<Instant>,
    attempt_id: String,
    candidate_id: String,
    request_id: String,
}
pub(crate) struct SerialReadyScope {
    mode: Arc<SerialReadyMode>,
    context: Arc<Mutex<SerialReadyContext>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}
impl SerialReadyMode {
    pub(crate) fn new(
        port: Arc<dyn SerialRuntimeHostBatchExecutionPort>,
        config: WorkflowSerialReadyConfig,
    ) -> Arc<Self> {
        Arc::new(Self {
            port,
            admission: SchedulerSerialAdmission::new(),
            semaphore: Arc::new(tokio::sync::Semaphore::new(1)),
            active: Mutex::new(None),
            observations: Mutex::new(VecDeque::new()),
            config,
        })
    }
    pub(crate) async fn enter(self: &Arc<Self>) -> Result<SerialReadyScope, WorkflowServiceError> {
        let permit = self.semaphore.clone().acquire_owned().await.map_err(|_| {
            WorkflowServiceError::InvalidRequest("serial execution owner is poisoned".into())
        })?;
        let preparation = self.admission.try_prepare().map_err(|_| {
            WorkflowServiceError::InvalidRequest(
                "serial execution owner is poisoned or busy".into(),
            )
        })?;
        let context = Arc::new(Mutex::new(SerialReadyContext {
            preparation: Some(preparation),
            dispatch: None,
            drained: None,
            cleanup: None,
            claimed: false,
            cleanup_acknowledged: false,
            successful: false,
            cancellation: None,
            task: None,
            member: None,
            owner: None,
            key: None,
            started: None,
            attempt_id: String::new(),
            candidate_id: String::new(),
            request_id: String::new(),
        }));
        *self.active.lock().map_err(lock_error)? = Some(context.clone());
        Ok(SerialReadyScope {
            mode: self.clone(),
            context,
            _permit: permit,
        })
    }
    fn context(&self) -> Result<Arc<Mutex<SerialReadyContext>>, String> {
        self.active
            .lock()
            .map_err(|_| "serial context lock poisoned".to_string())?
            .clone()
            .ok_or_else(|| "serial execution requires owned admission".into())
    }
    pub(crate) fn owner_for(
        &self,
        task: &WorkflowSchedulerTask,
    ) -> Option<SerialRuntimeHostCpuOwnerEvidence> {
        let intent = task.schedulable_intent.as_ref()?;
        if intent.task_type.as_str() != "embedding"
            || !intent.trait_settings.is_empty()
            || intent.constraints.requested_runtime_id.as_ref()?.as_str() != "candle"
            || intent.constraints.requested_device_id.as_ref()?.as_str() != "cpu"
        {
            return None;
        }
        let evidence = self.port.resident_serial_cpu_owner(intent)?;
        (evidence.is_current()
            && evidence.snapshot.loaded_instance != [0; 16]
            && (1..=4).contains(&evidence.snapshot.cpu_threads))
        .then_some(evidence)
    }
    pub(crate) fn select(&self, pair: &super::WorkflowSerialReadyPair) -> SerialReadyDecision {
        let mut decision = SerialReadyDecision::fifo(SerialReadyReason::OwnerUnavailable);
        let (Some(first), Some(second)) = (
            self.owner_for(&pair.members[0].task),
            self.owner_for(&pair.members[1].task),
        ) else {
            return decision;
        };
        if first.snapshot != second.snapshot || !Arc::ptr_eq(&first.generation, &second.generation)
        {
            decision.reason = SerialReadyReason::IncomparableOwners;
            return decision;
        }
        let (Some(first_key), Some(second_key)) = (
            observation_key(&pair.members[0]),
            observation_key(&pair.members[1]),
        ) else {
            decision.reason = SerialReadyReason::UnsupportedInputs;
            return decision;
        };
        let keys = [first_key, second_key];
        let now = Instant::now();
        let Ok(samples) = self.observations.lock() else {
            decision.reason = SerialReadyReason::HistoryUnavailable;
            return decision;
        };
        for sample in samples.iter() {
            let Some(age) = now.checked_duration_since(sample.at) else {
                return SerialReadyDecision::fifo(SerialReadyReason::InvalidClock);
            };
            if sample.owner.snapshot != first.snapshot
                || !Arc::ptr_eq(&sample.owner.generation, &first.generation)
                || age > self.config.maximum_age
            {
                continue;
            }
            for (i, key) in keys.iter().enumerate() {
                if sample.key == *key {
                    decision.counts[i] += 1;
                    decision.maxima_ns[i] = decision.maxima_ns[i].max(sample.cost_ns);
                    decision.oldest_age_ns[i] =
                        decision.oldest_age_ns[i].max(age.as_nanos().min(u64::MAX as u128) as u64);
                }
            }
        }
        if decision
            .counts
            .iter()
            .any(|c| *c < self.config.minimum_samples)
        {
            decision.reason = SerialReadyReason::InsufficientFreshSamples;
        } else if !first.is_current() {
            decision.reason = SerialReadyReason::OwnerChanged;
        } else {
            decision.reason = SerialReadyReason::Ranked;
            decision.position = usize::from(decision.maxima_ns[1] < decision.maxima_ns[0]);
            decision.owner = Some(first);
        }
        decision
    }
    fn migrate_verified_reuse(
        &self,
        previous: &SerialRuntimeHostCpuOwnerEvidence,
        current: &SerialRuntimeHostCpuOwnerEvidence,
    ) {
        // Only after consumed actual receipt cleanup and a fresh private-owner
        // query. Preserve freshness/counts; never carry across arbitrary reload.
        if !current.is_current() {
            return;
        }
        if let Ok(mut samples) = self.observations.lock() {
            for sample in samples.iter_mut() {
                if sample.owner.snapshot == previous.snapshot
                    && Arc::ptr_eq(&sample.owner.generation, &previous.generation)
                {
                    sample.owner = current.clone();
                }
            }
        }
    }
    fn record(
        &self,
        key: [u8; 32],
        owner: SerialRuntimeHostCpuOwnerEvidence,
        started: Instant,
        _task: &WorkflowSchedulerTask,
    ) {
        let now = Instant::now();
        let Some(elapsed) = now.checked_duration_since(started) else {
            return;
        };
        let Ok(cost_ns) = u64::try_from(elapsed.as_nanos()) else {
            return;
        };
        let Ok(mut samples) = self.observations.lock() else {
            return;
        };
        if samples.iter().filter(|s| s.key == key).count() >= 8 {
            if let Some(position) = samples.iter().position(|s| s.key == key) {
                samples.remove(position);
            }
        }
        if samples.len() >= 64 * 8 {
            samples.pop_front();
        }
        // Keep a bounded 64-key population as well as at most eight samples/key.
        if !samples.iter().any(|s| s.key == key) {
            let mut unique = Vec::with_capacity(64);
            for sample in samples.iter() {
                if !unique.contains(&sample.key) {
                    unique.push(sample.key);
                }
            }
            if unique.len() >= 64 {
                if let Some(oldest) = unique.first() {
                    samples.retain(|s| &s.key != oldest);
                }
            }
        }
        samples.push_back(Observation {
            key,
            owner,
            at: now,
            cost_ns,
            #[cfg(any(test, feature = "test-support"))]
            task_id: _task.task_id.to_string(),
        });
    }
}
impl SerialReadyScope {
    pub(crate) fn claim(&self) -> Result<(), WorkflowServiceError> {
        let mut context = self.context.lock().map_err(lock_error)?;
        context.dispatch = Some(
            context
                .preparation
                .take()
                .ok_or_else(|| {
                    WorkflowServiceError::Internal("serial preparation already promoted".into())
                })?
                .begin_dispatch(),
        );
        context.claimed = true;
        Ok(())
    }
    pub(crate) fn capture(
        &self,
        member: &super::WorkflowSerialReadyMember,
        owner: Option<SerialRuntimeHostCpuOwnerEvidence>,
    ) -> Result<(), WorkflowServiceError> {
        let mut context = self.context.lock().map_err(lock_error)?;
        context.task = Some(member.task.clone());
        context.member = Some(member.clone());
        context.key = observation_key(member);
        context.owner = owner;
        Ok(())
    }
    pub(crate) fn started(
        &self,
        started: &super::session_scheduler_runner::WorkflowStartedRuntimeDispatchAttempt,
    ) -> Result<(), WorkflowServiceError> {
        let mut context = self.context.lock().map_err(lock_error)?;
        context.attempt_id = started
            .started_runtime_task
            .attempt_id()
            .as_str()
            .to_owned();
        context.candidate_id = started
            .selected_dispatch
            .candidate_id()
            .ok_or_else(|| {
                WorkflowServiceError::Internal("serial selected candidate missing".into())
            })?
            .to_string();
        Ok(())
    }
}
impl Drop for SerialReadyScope {
    fn drop(&mut self) {
        let failed = self
            .context
            .lock()
            .map_or(true, |c| c.claimed && !c.cleanup_acknowledged);
        if failed {
            self.mode.semaphore.close();
        }
        if let Ok(mut active) = self.mode.active.lock() {
            active.take();
        }
    }
}

pub(crate) struct SerialReadyPortAdapter(pub(crate) Arc<SerialReadyMode>);
#[async_trait]
impl RuntimeHostBatchExecutionPort for SerialReadyPortAdapter {
    async fn execute_runtime_host_batch_request(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        let context = self.0.context().map_err(execution_error)?;
        if request.members.len() != 1 {
            return Err(execution_error(
                "serial owner requires singleton membership",
            ));
        }
        let member = &request.members[0];
        let (dispatch, expected, attempt, candidate) = {
            let mut c = context
                .lock()
                .map_err(|_| execution_error("serial context lock poisoned"))?;
            if let Some(snapshot) = c.member.as_ref() {
                if member.handoff.workflow_id != snapshot.task.workflow_id
                    || member.handoff.workflow_run_id != snapshot.task.workflow_run_id
                    || member.handoff.node_id != snapshot.task.node_id
                    || member.handoff.task_id != snapshot.task.task_id
                    || Some(&member.handoff.task_intent)
                        != snapshot.task.schedulable_intent.as_ref()
                    || member.handoff.readiness_proof != snapshot.proof
                {
                    return Err(execution_error(
                        "serial admitted handoff changed before owner execution",
                    ));
                }
            }
            if let (Some(key), Some(snapshot)) = (c.key, c.member.as_ref()) {
                if observation_key_with_inputs(snapshot, &member.materialized_inputs) != Some(key) {
                    return Err(execution_error(
                        "serial exact input changed before owner execution",
                    ));
                }
            }
            c.request_id = member.execution_request_id.clone();
            c.started = Some(Instant::now());
            c.cancellation = Some(cancellation.clone());
            (
                c.dispatch
                    .take()
                    .ok_or_else(|| execution_error("serial event ownership missing"))?,
                c.owner.as_ref().map(|o| o.snapshot),
                c.attempt_id.clone(),
                c.candidate_id.clone(),
            )
        };
        let identity = SchedulerSerialAttemptIdentity {
            workflow_id: member.handoff.workflow_id.as_str(),
            workflow_run_id: member.handoff.workflow_run_id.as_str(),
            node_id: member.handoff.node_id.as_str(),
            task_id: member.handoff.task_id.as_str(),
            attempt_id: &attempt,
            execution_request_id: &member.execution_request_id,
            candidate_id: &candidate,
            reservation_lease_id: member
                .handoff
                .dispatch_decision
                .as_ref()
                .ok_or_else(|| execution_error("serial dispatch decision missing"))?
                .reservation_lease_id
                .as_str(),
        };
        let bound = match expected {
            Some(owner) => dispatch.bind_attempt(identity, owner),
            None => dispatch.bind_unranked_attempt(identity),
        }
        .map_err(|e| execution_error(format!("serial binding refused: {e:?}")))?;
        let execution = self
            .0
            .port
            .execute_serial_singleton_with_cleanup(request, cancellation.clone(), bound)
            .await?;
        let SerialRuntimeHostDrainedExecution {
            response,
            drained,
            owner: actual_owner,
            cleanup,
        } = execution;
        let predecessor = cleanup.as_ref().map(|receipt| receipt.previous_owner());
        if let Some(receipt) = cleanup.as_ref() {
            let old = predecessor.as_ref().expect("present receipt predecessor");
            let new = receipt.current_owner();
            if old.snapshot.loaded_instance != new.snapshot.loaded_instance
                || old.snapshot.loaded_profile != new.snapshot.loaded_profile
                || old.snapshot.effective_settings != new.snapshot.effective_settings
                || old.snapshot.cpu_threads != new.snapshot.cpu_threads
                || old.snapshot.generation.checked_add(2) != Some(new.snapshot.generation)
                || !Arc::ptr_eq(&old.generation, &new.generation)
                || !actual_owner.as_ref().is_some_and(|actual| {
                    actual.snapshot == new.snapshot
                        && Arc::ptr_eq(&actual.generation, &new.generation)
                })
            {
                return Err(execution_error(
                    "linear warm receipt collection evidence refused",
                ));
            }
        }
        let mut context = context
            .lock()
            .map_err(|_| execution_error("serial context lock poisoned"))?;
        if let Some(expected) = context.owner.as_ref() {
            if !actual_owner.as_ref().is_some_and(|actual| {
                let observed = predecessor.as_ref().unwrap_or(actual);
                observed.snapshot == expected.snapshot
                    && Arc::ptr_eq(&observed.generation, &expected.generation)
            }) {
                return Err(execution_error(
                    "ranked actual owner evidence changed after drain",
                ));
            }
        }
        context.owner = actual_owner;
        context.successful = cancellation.snapshot().state
            == RuntimeHostExecutionCancellationState::Running
            && response.state == RuntimeHostBatchExecutionState::Completed
            && response.members.len() == 1
            && response.members[0].state == RuntimeHostBatchExecutionMemberState::Completed;
        context.drained = Some(drained);
        context.cleanup = cleanup;
        Ok(response)
    }
}
#[async_trait]
impl ReservationLifecyclePort for SerialReadyPortAdapter {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        let context = self.0.context().ok();
        let Some(context) = context else {
            return self.0.port.apply_reservation_lifecycle(event).await;
        };
        let data = {
            let mut c = context
                .lock()
                .map_err(|_| cleanup_error("serial context lock poisoned"))?;
            if c.claimed
                && c.dispatch.is_none()
                && c.drained.is_none()
                && is_release_outcome(&event.outcome)
            {
                return Err(cleanup_error(
                    "serial terminal cleanup requires actual drain proof",
                ));
            }
            c.drained.take().map(|drained| {
                (
                    drained,
                    c.cleanup.take(),
                    c.owner.clone(),
                    c.attempt_id.clone(),
                    c.request_id.clone(),
                    c.key,
                    c.started,
                    c.task.clone(),
                    c.successful,
                )
            })
        };
        let Some((drained, cleanup, expected, attempt, request_id, key, started, task, successful)) =
            data
        else {
            return self.0.port.apply_reservation_lifecycle(event).await;
        };
        let releases_reservation = is_release_outcome(&event.outcome);
        let identity = SchedulerSerialAttemptIdentity {
            workflow_id: event.workflow_id.as_str(),
            workflow_run_id: event.workflow_run_id.as_str(),
            node_id: event.node_id.as_str(),
            task_id: event.task_id.as_str(),
            attempt_id: &attempt,
            execution_request_id: &request_id,
            candidate_id: event
                .candidate_id
                .as_ref()
                .map_or("", SchedulerDispatchCandidateId::as_str),
            reservation_lease_id: event.reservation_lease_id.as_str(),
        };
        let pending = drained
            .expect_cleanup(SchedulerSerialCleanupEvent {
                identity,
                lifecycle_event_id: &event.lifecycle_event_id,
                releases_reservation,
            })
            .map_err(|e| cleanup_error(format!("serial cleanup binding refused: {e:?}")))?;
        let completed_release = event.outcome == ReservationLifecycleOutcome::RuntimeHostCompleted
            || (cleanup.is_some()
                && successful
                && event.outcome == ReservationLifecycleOutcome::RetryDeferred);
        let predecessor = cleanup.as_ref().map(|receipt| receipt.previous_owner());
        let application = match cleanup {
            Some(receipt) => receipt.apply(event).await?,
            None => {
                self.0
                    .port
                    .apply_serial_cleanup(event, expected.as_ref().map(|o| o.snapshot))
                    .await?
            }
        };
        application
            .validate()
            .map_err(|e| cleanup_error(e.to_string()))?;
        let state = match application.state {
            ReservationLifecycleApplicationState::Applied => SchedulerSerialCleanupState::Applied,
            ReservationLifecycleApplicationState::AlreadyApplied => {
                SchedulerSerialCleanupState::AlreadyApplied
            }
            _ => SchedulerSerialCleanupState::Failed,
        };
        pending
            .acknowledge_cleanup(
                &application.lifecycle_event_id,
                application.reservation_lease_id.as_str(),
                state,
            )
            .map_err(|e| cleanup_error(format!("serial cleanup refused: {e:?}")))?;
        context
            .lock()
            .map_err(|_| cleanup_error("serial context lock poisoned"))?
            .cleanup_acknowledged = true;
        if let (true, Some(key), Some(expected), Some(started), Some(task)) = (
            successful && completed_release,
            key,
            expected,
            started,
            task,
        ) {
            let owner_matches = expected.is_current()
                && self.0.owner_for(&task).is_some_and(|o| {
                    o.snapshot == expected.snapshot
                        && Arc::ptr_eq(&o.generation, &expected.generation)
                });
            let running = context.lock().ok().is_some_and(|c| {
                c.cancellation.as_ref().is_some_and(|h| {
                    h.snapshot().state == RuntimeHostExecutionCancellationState::Running
                })
            });
            if owner_matches && running {
                if let Some(previous) = predecessor.as_ref() {
                    self.0.migrate_verified_reuse(previous, &expected);
                }
                self.0.record(key, expected, started, &task);
            }
        }
        Ok(application)
    }
}
fn is_release_outcome(outcome: &ReservationLifecycleOutcome) -> bool {
    matches!(
        outcome,
        ReservationLifecycleOutcome::RuntimeHostCompleted
            | ReservationLifecycleOutcome::RuntimeHostFailed
            | ReservationLifecycleOutcome::RuntimeHostDispatchRejected
            | ReservationLifecycleOutcome::WorkflowCancelled
            | ReservationLifecycleOutcome::RetryDeferred
    )
}
fn observation_key(member: &super::WorkflowSerialReadyMember) -> Option<[u8; 32]> {
    observation_key_with_inputs(member, &member.inputs)
}
fn observation_key_with_inputs(
    member: &super::WorkflowSerialReadyMember,
    inputs: &[RuntimeHostExecutionInput],
) -> Option<[u8; 32]> {
    let intent = member.task.schedulable_intent.as_ref()?;
    let [input] = inputs else {
        return None;
    };
    let RuntimeHostExecutionInputValue::String(text) = &input.value else {
        return None;
    };
    if input.port_id != "text" || text.is_empty() || text.len() > 64 * 1024 {
        return None;
    }
    let proof = &member.proof;
    let bytes = serde_json::to_vec(&(
        &intent.model_ref,
        &intent.task_type,
        &intent.constraints,
        &intent.trait_settings,
        &proof.preflight_result.environment_ref,
        &proof.execution_context.descriptor_fingerprint,
        &proof.execution_context.dependency_requirements_id,
        &proof.execution_context.selected_binding_ids,
        &proof.execution_context.dependency_override_fingerprint,
        input,
    ))
    .ok()?;
    Some(*blake3::hash(&bytes).as_bytes())
}
fn lock_error<T>(_: std::sync::PoisonError<T>) -> WorkflowServiceError {
    WorkflowServiceError::Internal("serial mode lock poisoned".into())
}
fn execution_error(message: impl Into<String>) -> RuntimeHostExecutionPortError {
    RuntimeHostExecutionPortError::ExecutionFailed {
        message: message.into(),
    }
}
fn cleanup_error(message: impl Into<String>) -> ReservationLifecyclePortError {
    ReservationLifecyclePortError::Failed {
        message: message.into(),
    }
}

impl WorkflowService {
    /// Test-only acceptance evidence from the actual cleanup-qualified ring.
    #[cfg(any(test, feature = "test-support"))]
    pub fn serial_ready_observations_for_test(&self) -> Vec<(u64, usize)> {
        let Some(mode) = &self.serial_ready_mode else {
            return Vec::new();
        };
        let samples = mode.observations.lock().expect("serial observations lock");
        let mut counts = std::collections::BTreeMap::new();
        for sample in samples.iter() {
            *counts.entry(sample.owner.snapshot.generation).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }
    /// Read-only, bounded raw observations for an independent qualification oracle.
    /// Tuples contain task identity, exact workload digest, duration ns and age ns.
    /// Does not inject history or expose native owner/generation capabilities.
    #[cfg(any(test, feature = "test-support"))]
    pub fn serial_ready_samples_for_test(&self) -> Vec<(String, [u8; 32], u64, u64)> {
        let Some(mode) = &self.serial_ready_mode else {
            return Vec::new();
        };
        let now = Instant::now();
        mode.observations
            .lock()
            .expect("serial observations lock")
            .iter()
            .map(|s| {
                (
                    s.task_id.clone(),
                    s.key,
                    s.cost_ns,
                    now.saturating_duration_since(s.at)
                        .as_nanos()
                        .min(u64::MAX as u128) as u64,
                )
            })
            .collect()
    }
    /// Dedicated construction, before any store/repository escapes. The supplied
    /// port owns ALL serial fallback/ranked execution, drain and cleanup.
    /// Workflow deadlines are unsupported: run/resume reject `Some(timeout_ms)`.
    /// Dropping a caller wait does not stop native execution or release custody.
    pub fn new_serial_ready_cpu(
        port: Arc<dyn SerialRuntimeHostBatchExecutionPort>,
        config: WorkflowSerialReadyConfig,
    ) -> Self {
        let mut service = Self::new();
        let mode = SerialReadyMode::new(port, config);
        let adapter = Arc::new(SerialReadyPortAdapter(mode.clone()));
        service.scheduler_task_orchestrator = service
            .scheduler_task_orchestrator
            .with_runtime_host_batch_dispatcher(SchedulerRuntimeHostBatchDispatcher::new(
                adapter.clone(),
            ))
            .with_reservation_lifecycle_port(adapter);
        service.runtime_dispatch_assignment_repository = Arc::new(Mutex::new(super::runtime_dispatch_assignment::InMemoryWorkflowRuntimeDispatchAssignmentRepository::new_serial_singleton()));
        service.serial_ready_mode = Some(mode);
        service
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Assertion-only helper for the worker fixture: keep clock corruption and
    /// private history access out of production and the test-support API.
    pub(in crate::workflow) fn assert_invalid_clock_clears_partial_ranking(
        mode: &SerialReadyMode,
        pair: &WorkflowSerialReadyPair,
    ) {
        let ranked = mode.select(pair);
        assert_eq!(ranked.reason, SerialReadyReason::Ranked);
        assert_eq!(ranked.position, 1);
        assert_eq!(ranked.counts, [3, 3]);
        assert!(ranked.maxima_ns[0] > ranked.maxima_ns[1]);
        assert!(ranked.owner.is_some());

        // All six valid samples are visited before this impossible future
        // observation, so returning a partially accumulated proposal would fail.
        let previous_len = {
            let mut samples = mode.observations.lock().unwrap();
            let previous_len = samples.len();
            let first = samples.front().unwrap();
            let future = Observation {
                key: first.key,
                owner: first.owner.clone(),
                at: Instant::now()
                    .checked_add(Duration::from_secs(3600))
                    .unwrap(),
                cost_ns: first.cost_ns,
                task_id: first.task_id.clone(),
            };
            samples.push_back(future);
            previous_len
        };
        let fallback = mode.select(pair);
        assert_eq!(fallback.reason, SerialReadyReason::InvalidClock);
        assert_eq!(fallback.position, 0);
        assert!(fallback.owner.is_none());
        assert_eq!(fallback.counts, [0, 0]);
        assert_eq!(fallback.maxima_ns, [0, 0]);
        assert_eq!(fallback.oldest_age_ns, [0, 0]);
        {
            let mut samples = mode.observations.lock().unwrap();
            assert_eq!(samples.len(), previous_len + 1);
            samples.pop_back();
        }
        let recovered = mode.select(pair);
        assert_eq!(recovered.reason, SerialReadyReason::Ranked);
        assert_eq!(recovered.position, ranked.position);
        assert_eq!(recovered.counts, ranked.counts);
        assert_eq!(recovered.maxima_ns, ranked.maxima_ns);
        assert_eq!(
            recovered.owner.unwrap().snapshot,
            ranked.owner.unwrap().snapshot
        );
    }
}

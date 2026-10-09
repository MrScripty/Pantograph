//! Controlled finite facts for qualification only, around the actual service
//! store. This helper cannot establish a production episode or native capacity.
use super::*;
use crate::scheduler::{
    WorkflowExecutionSessionStore, WorkflowNativeReleaseBinding, WorkflowNativeReleaseSink,
};
use crate::workflow::{WorkflowExecutionSessionRunRequest, WorkflowService};
use pantograph_runtime_registry::RuntimeRegistry;
use pantograph_scheduler::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Debug)]
struct Clock;
impl crate::scheduler::WorkflowQueueProgressClock for Clock {
    fn now_us(&self) -> Result<u64, WorkflowServiceError> {
        Ok(0)
    }
}
#[derive(Debug)]
struct ControlledFacts {
    intent: ValidatedSchedulableTaskIntent,
    absent: AtomicBool,
    stale: AtomicBool,
}
impl crate::scheduler::WorkflowQueueProgressProvider for ControlledFacts {
    fn capture(
        &self,
        view: &crate::scheduler::WorkflowQueueProgressView,
    ) -> Result<crate::scheduler::WorkflowQueueProgressFacts, WorkflowServiceError> {
        let absent = self.absent.load(Ordering::SeqCst);
        Ok(crate::scheduler::WorkflowQueueProgressFacts {
            epoch: if self.stale.load(Ordering::SeqCst) {
                u64::MAX
            } else {
                view.epoch
            },
            valid_until_us: 1,
            owner: "controlled.native-release".into(),
            admitted_apps: vec![self.intent.as_ref().fairness_key.clone().unwrap()],
            population_complete: true,
            opportunities: if absent {
                vec![]
            } else {
                vec![crate::scheduler::WorkflowQueueProgressOpportunity {
                    intent: self.intent.clone(),
                    generation: 1,
                    since_us: 0,
                    eligibility: SchedulerProtectedEligibility::Ready,
                    feasibility: SchedulerProtectedFeasibility::IndividuallyFeasible,
                    episode_contract: "controlled.full-envelope.v1".into(),
                }]
            },
            bindings: view
                .runs
                .iter()
                .filter(|r| !r.active)
                .map(|r| crate::scheduler::WorkflowQueueProgressBinding {
                    session_id: r.session_id.clone(),
                    intent: self.intent.clone(),
                    generation: 1,
                    relation: SchedulerProtectedRelation::Conflicting,
                    whole_run_admission_covered: true,
                    request_fingerprint: r.request_fingerprint.clone().unwrap(),
                })
                .collect(),
            target: view.protected_target.clone(),
            blockers_complete: true,
            blockers: vec![],
        })
    }
}
pub struct WorkflowControlledNativeRelease {
    store: Arc<Mutex<WorkflowExecutionSessionStore>>,
    facts: Arc<ControlledFacts>,
    episode: SchedulerProtectedEpisode,
}
impl WorkflowService {
    /// Qualification-only: prime a controlled protected Running target and
    /// freeze its dispatch identity BEFORE the real native port is invoked.
    /// This is not a physical Started observation or production enable path.
    pub fn controlled_native_task_release(
        &self,
        request: &RuntimeHostBatchExecutionRequest,
        id: SchedulerSerialAttemptIdentity<'_>,
        gateway: Arc<inference::InferenceGateway>,
        registry: Arc<RuntimeRegistry>,
    ) -> Result<
        (
            WorkflowControlledNativeRelease,
            Arc<dyn WorkflowNativeTaskReleaseObserver>,
        ),
        WorkflowServiceError,
    > {
        let invalid = || {
            WorkflowServiceError::InvalidRequest("controlled native release fixture refused".into())
        };
        if !identity_is_bounded(id)
            || request.members.len() != 1
            || !inference::gateway::CandleCpuWarmAttemptRequest::metadata_is_bounded(request)
        {
            return Err(invalid());
        }
        let m = &request.members[0];
        let intent: ValidatedSchedulableTaskIntent = m
            .handoff
            .task_intent
            .clone()
            .try_into()
            .map_err(|_| invalid())?;
        let [input] = m.materialized_inputs.as_slice() else {
            return Err(invalid());
        };
        let pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue::String(text) =
            &input.value
        else {
            return Err(invalid());
        };
        if input.port_id != "text" || text.is_empty() || text.len() > 4096 {
            return Err(invalid());
        }
        let selected = m.handoff.dispatch_decision.as_ref().ok_or_else(invalid)?;
        let dispatch =
            *blake3::hash(&serde_json::to_vec(selected).map_err(|_| invalid())?).as_bytes();
        if intent.as_ref().fairness_key.is_none() {
            return Err(invalid());
        }
        let facts = Arc::new(ControlledFacts {
            intent: intent.clone(),
            absent: AtomicBool::new(false),
            stale: AtomicBool::new(false),
        });
        let mut store = self.session_store.lock().map_err(|_| invalid())?;
        if !store.is_empty_for_native_fixture() {
            return Err(invalid());
        }
        let session =
            store.create_session(id.workflow_id.into(), None, None, vec![], vec![], false)?;
        store.enqueue_run_with_id(
            &session,
            &WorkflowExecutionSessionRunRequest {
                session_id: session.clone(),
                workflow_semantic_version: "controlled.v1".into(),
                inputs: vec![],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
            id.workflow_run_id.into(),
        )?;
        store.mark_runtime_loaded(&session, true)?;
        store.enable_queue_progress(
            "controlled.native-release",
            0,
            2,
            4096,
            facts.clone(),
            Arc::new(Clock),
        )?;
        store.observe_queue_progress_event(SchedulerProtectedEvent::Observe)?;
        store
            .begin_queued_run(&session, id.workflow_run_id)?
            .ok_or_else(invalid)?;
        let mut episode = store.native_fixture_episode().ok_or_else(invalid)?;
        store.observe_queue_progress_event(SchedulerProtectedEvent::Started {
            turn_id: episode.turn_id,
            attempt: episode.attempt,
        })?;
        episode.phase = SchedulerProtectedPhase::Running;
        let binding = WorkflowNativeReleaseBinding::new(
            episode.clone(),
            intent,
            id,
            *blake3::hash(text.as_bytes()).as_bytes(),
            dispatch,
            gateway,
            registry,
        )?;
        store.bind_native_task_release(binding)?;
        drop(store);
        Ok((
            WorkflowControlledNativeRelease {
                store: self.session_store.clone(),
                facts,
                episode,
            },
            Arc::new(WorkflowNativeReleaseSink(self.session_store.clone())),
        ))
    }
}
impl WorkflowControlledNativeRelease {
    pub fn output_accepted(&self) -> bool {
        self.store.lock().unwrap().native_fixture_output_accepted()
    }
    /// Controlled task metadata, actual store Ready -> Running -> Completed
    /// transaction. This never creates a native producer proof or release ACK.
    pub fn accept_task_output(
        &self,
        result: crate::workflow::WorkflowSchedulerTaskResult,
    ) -> Result<(), WorkflowServiceError> {
        self.store
            .lock()
            .unwrap()
            .commit_native_fixture_task_output(result, None)
    }
    pub fn accept_task_output_for_attempt(
        &self,
        result: crate::workflow::WorkflowSchedulerTaskResult,
        attempt: &str,
    ) -> Result<(), WorkflowServiceError> {
        self.store
            .lock()
            .unwrap()
            .commit_native_fixture_task_output(result, Some(attempt))
    }
    pub fn acknowledged(&self) -> bool {
        self.store
            .lock()
            .unwrap()
            .native_task_release_acknowledged()
    }
    pub fn protected(&self) -> bool {
        self.store
            .lock()
            .unwrap()
            .native_fixture_episode()
            .is_some()
    }
    pub fn set_stale_evidence(&self, stale: bool) {
        self.facts.stale.store(stale, Ordering::SeqCst);
    }
    /// Explicit controlled result-owner outcome. Native cleanup never calls it.
    pub fn finish(&self, outcome: SchedulerProtectedOutcome) -> Result<(), WorkflowServiceError> {
        self.facts.absent.store(
            outcome != SchedulerProtectedOutcome::Failed,
            Ordering::SeqCst,
        );
        self.store
            .lock()
            .unwrap()
            .observe_queue_progress_event(SchedulerProtectedEvent::Finished {
                turn_id: self.episode.turn_id,
                attempt: self.episode.attempt,
                outcome,
                physical_release_acknowledged: true,
            })
    }
}

impl WorkflowControlledNativeRelease {
    /// Qualification-only retry event and binding in the SAME persistent store.
    /// Call after an explicit Failed finish; this is not a physical start fact.
    pub fn retry(
        &mut self,
        request: &RuntimeHostBatchExecutionRequest,
        id: SchedulerSerialAttemptIdentity<'_>,
        gateway: Arc<inference::InferenceGateway>,
        registry: Arc<RuntimeRegistry>,
    ) -> Result<(), WorkflowServiceError> {
        let invalid = || {
            WorkflowServiceError::InvalidRequest("controlled native retry fixture refused".into())
        };
        if !identity_is_bounded(id)
            || request.members.len() != 1
            || !inference::gateway::CandleCpuWarmAttemptRequest::metadata_is_bounded(request)
        {
            return Err(invalid());
        }
        let m = &request.members[0];
        if m.handoff.task_intent != *self.facts.intent.as_ref() {
            return Err(invalid());
        }
        let [input] = m.materialized_inputs.as_slice() else {
            return Err(invalid());
        };
        let pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue::String(text) =
            &input.value
        else {
            return Err(invalid());
        };
        if input.port_id != "text" || text.is_empty() || text.len() > 4096 {
            return Err(invalid());
        }
        let selected = m.handoff.dispatch_decision.as_ref().ok_or_else(invalid)?;
        let dispatch =
            *blake3::hash(&serde_json::to_vec(selected).map_err(|_| invalid())?).as_bytes();
        let mut store = self.store.lock().map_err(|_| invalid())?;
        let mut episode = store.native_fixture_episode().ok_or_else(invalid)?;
        if episode.phase != SchedulerProtectedPhase::Draining
            || episode.turn_id != self.episode.turn_id
            || episode.attempt != self.episode.attempt + 1
        {
            return Err(invalid());
        }
        store.observe_queue_progress_event(SchedulerProtectedEvent::Started {
            turn_id: episode.turn_id,
            attempt: episode.attempt,
        })?;
        episode.phase = SchedulerProtectedPhase::Running;
        store.bind_native_task_release(WorkflowNativeReleaseBinding::new(
            episode.clone(),
            self.facts.intent.clone(),
            id,
            *blake3::hash(text.as_bytes()).as_bytes(),
            dispatch,
            gateway,
            registry,
        )?)?;
        self.episode = episode;
        Ok(())
    }
}

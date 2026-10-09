#![cfg_attr(not(test), allow(dead_code))]
//! Owner bridge for real queue dequeue commitment, not native model admission.
#[cfg(feature = "native-task-release")]
#[path = "store_native_task_release.rs"]
mod native_task_release;
#[cfg(feature = "native-task-release")]
pub(crate) use native_task_release::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{WorkflowExecutionSessionStore, WORKFLOW_SESSION_QUEUE_POLL_MS};
use crate::workflow::WorkflowServiceError;
use pantograph_scheduler::{
    SchedulerFairnessKey, SchedulerProgressProtection, SchedulerProtectedBlockingContinuation,
    SchedulerProtectedDecision, SchedulerProtectedEligibility, SchedulerProtectedEvent,
    SchedulerProtectedFeasibility, SchedulerProtectedOpportunity, SchedulerProtectedPermission,
    SchedulerProtectedRelation, SchedulerProtectedSnapshot, SchedulerProtectedTarget,
    ValidatedSchedulableTaskIntent, SCHEDULER_PROTECTION_MAX_APPS,
    SCHEDULER_PROTECTION_MAX_OPPORTUNITIES, SCHEDULER_PROTECTION_MAX_WORK,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowQueueProgressRun {
    pub session_id: String,
    pub workflow_id: String,
    pub workflow_run_id: String,
    pub active: bool,
    pub request_fingerprint: Option<String>,
}
#[derive(Debug)]
pub(crate) struct WorkflowQueueProgressView {
    pub epoch: u64,
    pub now_us: u64,
    pub runs: Vec<WorkflowQueueProgressRun>,
    pub protected_target: Option<SchedulerProtectedTarget>,
}
#[derive(Debug, Clone)]
pub(crate) struct WorkflowQueueProgressOpportunity {
    pub intent: ValidatedSchedulableTaskIntent,
    pub generation: u64,
    pub since_us: u64,
    pub eligibility: SchedulerProtectedEligibility,
    pub feasibility: SchedulerProtectedFeasibility,
    pub episode_contract: String,
}
#[derive(Debug, Clone)]
pub(crate) struct WorkflowQueueProgressBlocker {
    pub intent: ValidatedSchedulableTaskIntent,
    pub generation: u64,
    pub envelope: String,
    pub full_growth_covered: bool,
}
/// One exact binding for every pending run. Required continuation permission
/// can never authorize a new queued workflow: it is a new admission.
#[derive(Debug, Clone)]
pub(crate) struct WorkflowQueueProgressBinding {
    pub session_id: String,
    pub intent: ValidatedSchedulableTaskIntent,
    pub generation: u64,
    pub relation: SchedulerProtectedRelation,
    pub whole_run_admission_covered: bool,
    pub request_fingerprint: String,
}
#[derive(Debug, Clone)]
pub(crate) struct WorkflowQueueProgressFacts {
    pub epoch: u64,
    pub valid_until_us: u64,
    pub owner: String,
    pub admitted_apps: Vec<SchedulerFairnessKey>,
    pub population_complete: bool,
    pub opportunities: Vec<WorkflowQueueProgressOpportunity>,
    pub bindings: Vec<WorkflowQueueProgressBinding>,
    pub target: Option<SchedulerProtectedTarget>,
    pub blockers_complete: bool,
    pub blockers: Vec<WorkflowQueueProgressBlocker>,
}
/// Trusted physical/runtime owner, synchronous and bounded, under the store
/// mutex. Must not reenter the store or await. No native implementation exists.
/// Queue metadata proves identity/presence only, never feasibility/envelopes.
pub(crate) trait WorkflowQueueProgressProvider: std::fmt::Debug + Send + Sync {
    fn capture(
        &self,
        view: &WorkflowQueueProgressView,
    ) -> Result<WorkflowQueueProgressFacts, WorkflowServiceError>;
}
/// Also bounded, nonblocking and non-reentrant under the store mutex.
pub(crate) trait WorkflowQueueProgressClock: std::fmt::Debug + Send + Sync {
    fn now_us(&self) -> Result<u64, WorkflowServiceError>;
}
#[derive(Debug)]
pub(crate) struct WorkflowQueueMonotonicClock(Instant);
impl WorkflowQueueMonotonicClock {
    pub(crate) fn new() -> Self {
        Self(Instant::now())
    }
}
impl WorkflowQueueProgressClock for WorkflowQueueMonotonicClock {
    fn now_us(&self) -> Result<u64, WorkflowServiceError> {
        u64::try_from(self.0.elapsed().as_micros()).map_err(|_| invalid("clock overflow"))
    }
}
#[derive(Debug)]
pub(crate) struct WorkflowQueueProgressCommit {
    controller: SchedulerProgressProtection,
    provider: Arc<dyn WorkflowQueueProgressProvider>,
    clock: Arc<dyn WorkflowQueueProgressClock>,
    decision: Option<SchedulerProtectedDecision>,
    budget: usize,
    observed_at_us: u64,
    valid_until_us: u64,
    evidence_refused: bool,
    #[cfg(feature = "native-task-release")]
    pub(super) native_release: Option<WorkflowNativeReleaseBinding>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WorkflowQueueProgressChoice {
    Ordinary,
    Protected,
    Hold,
}
fn invalid(message: &str) -> WorkflowServiceError {
    WorkflowServiceError::InvalidRequest(format!("protected queue commitment: {message}"))
}
fn bounded(s: &str, n: usize) -> bool {
    s.len() <= n && !s.trim().is_empty()
}
fn bounded_intent(i: &ValidatedSchedulableTaskIntent) -> bool {
    let i = i.as_ref();
    i.model_ref.model_id.len() <= 128
        && i.model_ref.revision.as_ref().is_none_or(|s| s.len() <= 128)
        && i.model_ref
            .selected_artifact_id
            .as_ref()
            .is_none_or(|s| s.len() <= 128)
        && i.model_ref.selected_artifact_path.is_none()
        && i.model_ref.migration_diagnostics.is_empty()
        && i.trait_settings.len() <= 32
        && i.estimate_hints.len() <= 32
        && i.dependency_override_patches.is_empty()
        && crate::workflow::bounded_serialized(i)
}
fn bounded_json(v: &serde_json::Value, depth: usize, left: &mut usize) -> bool {
    if depth > 16 || *left == 0 {
        return false;
    }
    *left -= 1;
    match v {
        serde_json::Value::String(s) => s.len() <= 4096,
        serde_json::Value::Array(a) => {
            a.len() <= *left && a.iter().all(|v| bounded_json(v, depth + 1, left))
        }
        serde_json::Value::Object(o) => {
            o.len() <= *left
                && o.iter()
                    .all(|(k, v)| k.len() <= 128 && bounded_json(v, depth + 1, left))
        }
        _ => true,
    }
}
fn request_fingerprint(
    session: &super::WorkflowExecutionSessionRecord,
    q: &super::WorkflowExecutionSessionQueuedRun,
) -> Result<String, WorkflowServiceError> {
    let mut left = 256;
    if q.workflow_semantic_version.len() > 128
        || q.inputs.len() > 32
        || q.inputs.iter().any(|i| {
            i.node_id.len() > 128 || i.port_id.len() > 128 || !bounded_json(&i.value, 0, &mut left)
        })
        || q.output_targets.as_ref().is_some_and(|o| {
            o.len() > 32
                || o.iter()
                    .any(|t| t.node_id.len() > 128 || t.port_id.len() > 128)
        })
        || q.override_selection.as_ref().is_some_and(|o| {
            [
                &o.runtime_id,
                &o.runtime_variant_id,
                &o.model_id,
                &o.backend_key,
            ]
            .into_iter()
            .any(|s| s.as_ref().is_some_and(|s| s.len() > 128))
        })
    {
        return Err(invalid("request payload limit"));
    }
    let identity = (
        &session.workflow_id,
        &q.workflow_run_id,
        &q.workflow_semantic_version,
        &q.inputs,
        &q.output_targets,
        &q.override_selection,
        q.timeout_ms,
    );
    if !crate::workflow::bounded_serialized(&identity) {
        return Err(invalid("request byte limit"));
    }
    let bytes = serde_json::to_vec(&identity).map_err(|_| invalid("request serialization"))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}
impl WorkflowExecutionSessionStore {
    /// Bound metadata before even the legacy capacity/ranking paths clone it.
    pub(crate) fn queue_progress_preflight(&mut self) -> bool {
        if self.progress_commit.is_none() {
            return true;
        }
        if self.tick == u64::MAX || self.active.len() > SCHEDULER_PROTECTION_MAX_APPS {
            self.progress_commit
                .as_mut()
                .expect("installed")
                .evidence_refused = true;
            return false;
        }
        let mut total = 0;
        let valid = self.active.iter().all(|(id, s)| {
            total += s.queue.len() + usize::from(s.active_run.is_some());
            total <= SCHEDULER_PROTECTION_MAX_OPPORTUNITIES
                && bounded(id, 256)
                && bounded(&s.workflow_id, 128)
                && s.usage_profile.as_ref().is_none_or(|s| s.len() <= 128)
                && s.required_backends.len() <= 32
                && s.required_models.len() <= 32
                && s.required_backends
                    .iter()
                    .chain(&s.required_models)
                    .all(|s| s.len() <= 128)
        });
        if !valid {
            self.progress_commit
                .as_mut()
                .expect("installed")
                .evidence_refused = true;
        }
        valid
    }
    /// Install once for one explicitly finite cohort. Replans, boosts and
    /// arrivals cannot recreate or replace its controller/time origin.
    pub(crate) fn enable_queue_progress(
        &mut self,
        owner: &str,
        gap_us: u64,
        attempts: u32,
        budget: usize,
        provider: Arc<dyn WorkflowQueueProgressProvider>,
        clock: Arc<dyn WorkflowQueueProgressClock>,
    ) -> Result<(), WorkflowServiceError> {
        if self.progress_commit.is_some() || budget > SCHEDULER_PROTECTION_MAX_WORK {
            return Err(invalid("already installed or invalid work budget"));
        }
        let controller = SchedulerProgressProtection::new(owner, clock.now_us()?, gap_us, attempts)
            .map_err(|_| invalid("invalid controller contract"))?;
        self.progress_commit = Some(WorkflowQueueProgressCommit {
            controller,
            provider,
            clock,
            decision: None,
            budget,
            observed_at_us: 0,
            valid_until_us: 0,
            evidence_refused: false,
            #[cfg(feature = "native-task-release")]
            native_release: None,
        });
        Ok(())
    }
    fn progress_view(
        &self,
        now_us: u64,
    ) -> Result<WorkflowQueueProgressView, WorkflowServiceError> {
        if self.tick == u64::MAX || self.active.len() > SCHEDULER_PROTECTION_MAX_APPS {
            return Err(invalid("session population limit"));
        }
        let mut runs = Vec::new();
        for (id, session) in &self.active {
            if !bounded(id, 256)
                || !bounded(&session.workflow_id, 128)
                || session.queue.len() > SCHEDULER_PROTECTION_MAX_OPPORTUNITIES
            {
                return Err(invalid("identity or queue population limit"));
            }
            for (run_id, active) in session
                .queue
                .iter()
                .map(|r| (&r.workflow_run_id, false))
                .chain(
                    session
                        .active_run
                        .iter()
                        .map(|r| (&r.workflow_run_id, true)),
                )
            {
                if runs.len() == SCHEDULER_PROTECTION_MAX_OPPORTUNITIES
                    || !bounded(run_id, 128)
                    || runs
                        .iter()
                        .any(|r: &WorkflowQueueProgressRun| r.workflow_run_id == *run_id)
                {
                    return Err(invalid("run identity or cohort population limit"));
                }
                runs.push(WorkflowQueueProgressRun {
                    session_id: id.clone(),
                    workflow_id: session.workflow_id.clone(),
                    workflow_run_id: run_id.clone(),
                    active,
                    request_fingerprint: if active {
                        None
                    } else {
                        Some(request_fingerprint(
                            session,
                            session
                                .queue
                                .iter()
                                .find(|q| q.workflow_run_id == *run_id)
                                .expect("current run"),
                        )?)
                    },
                });
            }
        }
        runs.sort_by(|a, b| {
            (&a.session_id, &a.workflow_run_id).cmp(&(&b.session_id, &b.workflow_run_id))
        });
        Ok(WorkflowQueueProgressView {
            epoch: self.tick,
            now_us,
            runs,
            protected_target: self
                .progress_commit
                .as_ref()
                .and_then(|p| p.decision.as_ref())
                .and_then(|d| d.active())
                .map(|a| a.target.clone()),
        })
    }
    fn advance_queue_progress(
        &mut self,
        event: SchedulerProtectedEvent,
    ) -> Result<WorkflowQueueProgressFacts, WorkflowServiceError> {
        let p = self
            .progress_commit
            .as_ref()
            .ok_or_else(|| invalid("not installed"))?;
        let mut view = self.progress_view(p.clock.now_us()?)?;
        let facts = p.provider.capture(&view)?;
        // Include capture/planner time in G; an old observation cannot stop time.
        let after_capture = p.clock.now_us()?;
        if after_capture < view.now_us {
            return Err(invalid("clock regression during capture"));
        }
        view.now_us = after_capture;
        if facts.epoch != view.epoch
            || view.now_us > facts.valid_until_us
            || !facts.population_complete
            || facts.bindings.len() != view.runs.iter().filter(|r| !r.active).count()
            || facts.bindings.len() > SCHEDULER_PROTECTION_MAX_OPPORTUNITIES
            || facts.opportunities.len() > SCHEDULER_PROTECTION_MAX_OPPORTUNITIES
            || facts.blockers.len() > SCHEDULER_PROTECTION_MAX_OPPORTUNITIES
            || facts.admitted_apps.len() > SCHEDULER_PROTECTION_MAX_APPS
        {
            return Err(invalid("stale or incomplete bounded owner population"));
        }
        if facts
            .opportunities
            .iter()
            .any(|o| !bounded_intent(&o.intent))
            || facts.bindings.iter().any(|b| !bounded_intent(&b.intent))
            || facts.blockers.iter().any(|b| !bounded_intent(&b.intent))
        {
            return Err(invalid("intent population or payload limit"));
        }
        if facts.opportunities.iter().any(|o| {
            !view.runs.iter().any(|r| {
                r.workflow_id == o.intent.as_ref().workflow_id.as_str()
                    && r.workflow_run_id == o.intent.as_ref().workflow_run_id.as_str()
            })
        }) {
            return Err(invalid("opportunity outside owner queue/run domain"));
        }
        for (index, b) in facts.bindings.iter().enumerate() {
            let i = b.intent.as_ref();
            if !bounded(&b.session_id, 256)
                || b.request_fingerprint.len() != 64
                || !b.whole_run_admission_covered
                || !view.runs.iter().any(|r| {
                    !r.active
                        && r.session_id == b.session_id
                        && r.workflow_id == i.workflow_id.as_str()
                        && r.workflow_run_id == i.workflow_run_id.as_str()
                        && r.request_fingerprint.as_deref() == Some(b.request_fingerprint.as_str())
                })
                || facts.bindings[..index].iter().any(|other| {
                    other.session_id == b.session_id
                        && other.intent.as_ref().workflow_run_id == i.workflow_run_id
                })
                || !facts
                    .opportunities
                    .iter()
                    .any(|o| o.intent == b.intent && o.generation == b.generation)
            {
                return Err(invalid("unqualified or mismatched queue binding"));
            }
        }
        let opportunities: Vec<_> = facts
            .opportunities
            .iter()
            .map(|o| SchedulerProtectedOpportunity {
                intent: &o.intent,
                generation: o.generation,
                continuously_eligible_since_us: o.since_us,
                eligibility: o.eligibility,
                feasibility: o.feasibility,
                episode_contract_identity: &o.episode_contract,
            })
            .collect();
        let blockers: Vec<_> = facts
            .blockers
            .iter()
            .map(|b| SchedulerProtectedBlockingContinuation {
                intent: &b.intent,
                generation: b.generation,
                envelope_identity: &b.envelope,
                full_growth_covered: b.full_growth_covered,
            })
            .collect();
        let snapshot = SchedulerProtectedSnapshot {
            owner_identity: &facts.owner,
            now_us: view.now_us,
            admitted_population_complete: facts.population_complete,
            admitted_apps: &facts.admitted_apps,
            waiting_population_complete: facts.population_complete,
            opportunities: &opportunities,
            blocking_continuations_complete: facts.blockers_complete,
            protected_target: facts.target.as_ref(),
            blocking_continuations: &blockers,
        };
        let (controller, decision) = p
            .controller
            .advance(&snapshot, event, p.budget)
            .map_err(|_| invalid("controller refused owner evidence/event"))?;
        let p = self.progress_commit.as_mut().expect("installed controller");
        p.evidence_refused = false;
        p.observed_at_us = view.now_us;
        p.valid_until_us = facts.valid_until_us;
        p.controller = controller;
        p.decision = Some(decision);
        Ok(facts)
    }
    pub(crate) fn queue_progress_choice(
        &mut self,
        session: &str,
        run: &str,
    ) -> Result<WorkflowQueueProgressChoice, WorkflowServiceError> {
        if self.progress_commit.is_none() {
            return Ok(WorkflowQueueProgressChoice::Ordinary);
        }
        let state = self.active.get(session).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session))
        })?;
        let active_match = state
            .active_run
            .as_ref()
            .is_some_and(|r| r.workflow_run_id == run);
        if !active_match && !state.queue.iter().any(|r| r.workflow_run_id == run) {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in session '{}'",
                run, session
            )));
        }
        let has_active = state.active_run.is_some();
        // Owner refusals hold the pending caller; no fallback or implicit retry
        // event may consume the turn. Actual explicit owner events still return
        // their refusal to their owner for reconciliation.
        let Ok(facts) = self.advance_queue_progress(SchedulerProtectedEvent::Observe) else {
            self.progress_commit
                .as_mut()
                .expect("installed")
                .evidence_refused = true;
            return Ok(WorkflowQueueProgressChoice::Hold);
        };
        if has_active {
            return Ok(WorkflowQueueProgressChoice::Hold);
        }
        let Some(binding) = facts
            .bindings
            .iter()
            .find(|b| b.session_id == session && b.intent.as_ref().workflow_run_id.as_str() == run)
        else {
            return Err(invalid("requested pending run missing"));
        };
        let opportunity = facts
            .opportunities
            .iter()
            .find(|o| o.intent == binding.intent && o.generation == binding.generation)
            .expect("verified binding");
        if opportunity.eligibility != SchedulerProtectedEligibility::Ready
            || opportunity.feasibility != SchedulerProtectedFeasibility::IndividuallyFeasible
            || binding.relation == SchedulerProtectedRelation::RequiredBlockingContinuation
        {
            return Ok(WorkflowQueueProgressChoice::Hold);
        }
        let d = self
            .progress_commit
            .as_ref()
            .and_then(|p| p.decision.as_ref())
            .expect("advanced");
        Ok(
            match d.permission(
                &facts.owner,
                d.revision(),
                &binding.intent,
                binding.generation,
                binding.relation,
            ) {
                SchedulerProtectedPermission::ConsiderProtectedTarget => {
                    WorkflowQueueProgressChoice::Protected
                }
                SchedulerProtectedPermission::ConsiderDiscretionary
                | SchedulerProtectedPermission::ConsiderNonconflicting => {
                    WorkflowQueueProgressChoice::Ordinary
                }
                _ => WorkflowQueueProgressChoice::Hold,
            },
        )
    }
    /// Only actual owner task lifecycle/physical cleanup observations belong
    /// here. Queue dequeue and workflow finish are not start/release facts.
    pub(crate) fn observe_queue_progress_event(
        &mut self,
        event: SchedulerProtectedEvent,
    ) -> Result<(), WorkflowServiceError> {
        #[cfg(feature = "native-task-release")]
        let finishing_native = self.check_native_finish(&event)?;
        self.advance_queue_progress(event)?;
        #[cfg(feature = "native-task-release")]
        if finishing_native {
            self.progress_commit
                .as_mut()
                .expect("installed")
                .native_release = None;
        }
        Ok(())
    }
    pub(crate) fn queue_progress_final_check(&self) -> Result<bool, WorkflowServiceError> {
        let Some(p) = &self.progress_commit else {
            return Ok(true);
        };
        let now = p.clock.now_us()?;
        let delta = now
            .checked_sub(p.observed_at_us)
            .ok_or_else(|| invalid("clock regression"))?;
        Ok(now <= p.valid_until_us
            && p.decision.as_ref().is_some_and(|d| {
                d.next_wake_after_us()
                    .is_none_or(|remaining| delta < remaining)
            }))
    }
    pub(crate) fn queue_progress_retry_after(&self) -> Duration {
        let poll = Duration::from_millis(WORKFLOW_SESSION_QUEUE_POLL_MS);
        let Some(p) = &self.progress_commit else {
            return poll;
        };
        if p.evidence_refused {
            return poll;
        }
        let Ok(now) = p.clock.now_us() else {
            return poll;
        };
        let Some(elapsed) = now.checked_sub(p.observed_at_us) else {
            return poll;
        };
        p.decision
            .as_ref()
            .and_then(|d| d.next_wake_after_us())
            .map_or(poll, |us| {
                poll.min(Duration::from_micros(us.saturating_sub(elapsed)))
            })
    }
}
#[cfg(test)]
#[path = "store_progress_commit_tests.rs"]
mod tests;

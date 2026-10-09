//! Opt-in protected opportunities before ordinary task/resource admission.
//! Owner declarations do not authenticate readiness, app identity or envelopes.
use crate::{
    SchedulerFairnessKey, SchedulerNodeId, SchedulerTaskId, SchedulerWorkflowId,
    SchedulerWorkflowRunId, ValidatedSchedulableTaskIntent,
};

pub const SCHEDULER_PROTECTION_MAX_APPS: usize = 4;
pub const SCHEDULER_PROTECTION_MAX_OPPORTUNITIES: usize = 16;
pub const SCHEDULER_PROTECTION_MAX_WORK: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerProtectedEligibility {
    Ready,
    RequiredContinuation,
    Ineligible,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerProtectedFeasibility {
    IndividuallyFeasible,
    Infeasible,
    Unknown,
}

/// Complete owner-qualified waiting population; not executable readiness.
pub struct SchedulerProtectedOpportunity<'a> {
    pub intent: &'a ValidatedSchedulableTaskIntent,
    pub generation: u64,
    pub continuously_eligible_since_us: u64,
    pub eligibility: SchedulerProtectedEligibility,
    /// Against the full setup/execution envelope, not current free capacity.
    pub feasibility: SchedulerProtectedFeasibility,
    pub episode_contract_identity: &'a str,
}
/// Exact pinned blocking continuation, verified by the physical/runtime owner.
/// The owner must cover full episode growth or its validated staged protocol.
pub struct SchedulerProtectedBlockingContinuation<'a> {
    pub intent: &'a ValidatedSchedulableTaskIntent,
    pub generation: u64,
    pub envelope_identity: &'a str,
    pub full_growth_covered: bool,
}
pub struct SchedulerProtectedSnapshot<'a> {
    pub owner_identity: &'a str,
    pub now_us: u64,
    pub admitted_population_complete: bool,
    pub admitted_apps: &'a [SchedulerFairnessKey],
    pub waiting_population_complete: bool,
    pub opportunities: &'a [SchedulerProtectedOpportunity<'a>],
    /// Owner-qualified against this decision's frozen target envelope.
    pub blocking_continuations_complete: bool,
    /// Exact frozen target whose full envelope the owner qualified. The first
    /// activation has no such frame: refresh it before considering new starts.
    pub protected_target: Option<&'a SchedulerProtectedTarget>,
    pub blocking_continuations: &'a [SchedulerProtectedBlockingContinuation<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerProtectedTarget {
    pub fairness_key: SchedulerFairnessKey,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub task_id: SchedulerTaskId,
    pub node_id: SchedulerNodeId,
    pub generation: u64,
    pub continuously_eligible_since_us: u64,
    pub episode_contract_identity: String,
}
impl SchedulerProtectedTarget {
    fn matches(&self, intent: &ValidatedSchedulableTaskIntent, generation: u64) -> bool {
        let i = intent.as_ref();
        Some(&self.fairness_key) == i.fairness_key.as_ref()
            && self.workflow_id == i.workflow_id
            && self.workflow_run_id == i.workflow_run_id
            && self.task_id == i.task_id
            && self.node_id == i.node_id
            && self.generation == generation
    }
    fn same_task(&self, other: &Self) -> bool {
        (&self.workflow_id, &self.workflow_run_id, &self.task_id)
            == (&other.workflow_id, &other.workflow_run_id, &other.task_id)
    }
    fn same_task_intent(&self, intent: &ValidatedSchedulableTaskIntent) -> bool {
        let i = intent.as_ref();
        self.workflow_id == i.workflow_id
            && self.workflow_run_id == i.workflow_run_id
            && self.task_id == i.task_id
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerProtectedPhase {
    Draining,
    Running,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerProtectedEpisode {
    pub turn_id: u64,
    pub attempt: u32,
    pub phase: SchedulerProtectedPhase,
    pub target: SchedulerProtectedTarget,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerProtectedOutcome {
    AcceptedOutput,
    Failed,
    Withdrawn,
}
/// Observed owner events, not commands to start, cancel or release resources.
pub enum SchedulerProtectedEvent {
    Observe,
    Started {
        turn_id: u64,
        attempt: u32,
    },
    Finished {
        turn_id: u64,
        attempt: u32,
        outcome: SchedulerProtectedOutcome,
        physical_release_acknowledged: bool,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerProtectionRefusal {
    WorkLimitExceeded,
    BudgetExhausted,
    InvalidContract,
    UnknownEvidence,
    ClockRegression,
    ContinuityChanged,
    StaleEvent,
    ReleaseUnacknowledged,
    ActiveAppRemoved,
    CounterOverflow,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct App {
    key: SchedulerFairnessKey,
    served_round: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Seen {
    target: SchedulerProtectedTarget,
    eligible: bool,
}
/// Caller persists each returned state across replans/boost changes. One
/// controller serves <=4 distinct apps and <=16 distinct task identities over
/// its finite admission cohort. Recreating it is not a replan operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerProgressProtection {
    owner_identity: String,
    gap_budget_us: u64,
    max_attempts: u32,
    now_us: u64,
    round: u64,
    gap_used_us: u64,
    revision: u64,
    next_turn: u64,
    apps: Vec<App>,
    pending: Vec<SchedulerFairnessKey>,
    seen: Vec<Seen>,
    active: Option<SchedulerProtectedEpisode>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Blocker {
    target: SchedulerProtectedTarget,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerProtectedDecision {
    owner_identity: String,
    revision: u64,
    round: u64,
    discretionary_used_us: u64,
    active: Option<SchedulerProtectedEpisode>,
    /// AcceptedOutput is reported only from the owner's matching final event.
    finished: Option<SchedulerProtectedOutcome>,
    work_units: usize,
    target_envelope_qualified: bool,
    next_wake_after_us: Option<u64>,
    blockers: Vec<Blocker>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerProtectedRelation {
    Nonconflicting,
    RequiredBlockingContinuation,
    Conflicting,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerProtectedPermission {
    ConsiderDiscretionary,
    ConsiderProtectedTarget,
    ConsiderNonconflicting,
    ConsiderRequiredBlockingContinuation,
    HoldNewAdmission,
}
impl SchedulerProtectedDecision {
    /// Owner must deliver this wake-up and advance before new commitment.
    /// The pure controller neither installs a timer nor hides callback lateness.
    pub fn next_wake_after_us(&self) -> Option<u64> {
        self.next_wake_after_us
    }
    pub fn owner_identity(&self) -> &str {
        &self.owner_identity
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn round(&self) -> u64 {
        self.round
    }
    pub fn discretionary_used_us(&self) -> u64 {
        self.discretionary_used_us
    }
    pub fn active(&self) -> Option<&SchedulerProtectedEpisode> {
        self.active.as_ref()
    }
    pub fn finished(&self) -> Option<SchedulerProtectedOutcome> {
        self.finished
    }
    pub fn work_units(&self) -> usize {
        self.work_units
    }
    pub fn target_envelope_qualified(&self) -> bool {
        self.target_envelope_qualified
    }

    /// Advisory gate shared by search and fallback. The physical commit must
    /// revalidate this revision, target/envelope, readiness and actual leases.
    pub fn permission(
        &self,
        owner_identity: &str,
        revision: u64,
        intent: &ValidatedSchedulableTaskIntent,
        generation: u64,
        relation: SchedulerProtectedRelation,
    ) -> SchedulerProtectedPermission {
        use SchedulerProtectedPermission::*;
        if owner_identity != self.owner_identity || revision != self.revision {
            return HoldNewAdmission;
        }
        let Some(active) = &self.active else {
            return ConsiderDiscretionary;
        };
        if !self.target_envelope_qualified {
            return HoldNewAdmission;
        }
        if active.target.matches(intent, generation) {
            if relation == SchedulerProtectedRelation::RequiredBlockingContinuation
                && self
                    .blockers
                    .iter()
                    .any(|b| b.target.matches(intent, generation))
            {
                return ConsiderRequiredBlockingContinuation;
            }
            return if active.phase == SchedulerProtectedPhase::Draining {
                ConsiderProtectedTarget
            } else {
                HoldNewAdmission
            };
        }
        if active.target.same_task_intent(intent) {
            return HoldNewAdmission;
        }
        match relation {
            SchedulerProtectedRelation::Nonconflicting => ConsiderNonconflicting,
            SchedulerProtectedRelation::RequiredBlockingContinuation
                if self
                    .blockers
                    .iter()
                    .any(|b| b.target.matches(intent, generation)) =>
            {
                ConsiderRequiredBlockingContinuation
            }
            _ => HoldNewAdmission,
        }
    }
}
struct Meter {
    used: usize,
    limit: usize,
}
impl Meter {
    fn charge(&mut self) -> Result<(), SchedulerProtectionRefusal> {
        if self.used == self.limit {
            return Err(SchedulerProtectionRefusal::BudgetExhausted);
        }
        self.used += 1;
        Ok(())
    }
}
fn label(s: &str) -> bool {
    s.len() <= 128 && !s.trim().is_empty() && !s.chars().any(char::is_control)
}
fn target(
    intent: &ValidatedSchedulableTaskIntent,
    generation: u64,
    since: u64,
    contract: &str,
) -> Result<SchedulerProtectedTarget, SchedulerProtectionRefusal> {
    let i = intent.as_ref();
    Ok(SchedulerProtectedTarget {
        fairness_key: i
            .fairness_key
            .clone()
            .ok_or(SchedulerProtectionRefusal::InvalidContract)?,
        workflow_id: i.workflow_id.clone(),
        workflow_run_id: i.workflow_run_id.clone(),
        task_id: i.task_id.clone(),
        node_id: i.node_id.clone(),
        generation,
        continuously_eligible_since_us: since,
        episode_contract_identity: contract.to_owned(),
    })
}
impl SchedulerProgressProtection {
    pub fn new(
        owner_identity: &str,
        start_us: u64,
        gap_budget_us: u64,
        max_attempts: u32,
    ) -> Result<Self, SchedulerProtectionRefusal> {
        if !label(owner_identity) || !(1..=8).contains(&max_attempts) {
            return Err(SchedulerProtectionRefusal::InvalidContract);
        }
        Ok(Self {
            owner_identity: owner_identity.to_owned(),
            gap_budget_us,
            max_attempts,
            now_us: start_us,
            round: 1,
            gap_used_us: 0,
            revision: 0,
            next_turn: 0,
            apps: vec![],
            pending: vec![],
            seen: vec![],
            active: None,
        })
    }
    /// Pure transactional transition. Any error leaves the borrowed state
    /// unchanged: hold new admission until fresh owner evidence is available;
    /// retain physically legal in-flight continuations under existing ownership.
    /// Work units bound operations, each on <=16 items/<=128-byte identifiers.
    pub fn advance(
        &self,
        snapshot: &SchedulerProtectedSnapshot<'_>,
        event: SchedulerProtectedEvent,
        max_work_units: usize,
    ) -> Result<(Self, SchedulerProtectedDecision), SchedulerProtectionRefusal> {
        use SchedulerProtectionRefusal::*;
        if snapshot.admitted_apps.len() > SCHEDULER_PROTECTION_MAX_APPS
            || snapshot.opportunities.len() > SCHEDULER_PROTECTION_MAX_OPPORTUNITIES
            || snapshot.blocking_continuations.len() > SCHEDULER_PROTECTION_MAX_OPPORTUNITIES
            || max_work_units > SCHEDULER_PROTECTION_MAX_WORK
        {
            return Err(WorkLimitExceeded);
        }
        let mut m = Meter {
            used: 0,
            limit: max_work_units,
        };
        m.charge()?;
        if snapshot.owner_identity != self.owner_identity
            || !snapshot.admitted_population_complete
            || !snapshot.waiting_population_complete
            || !snapshot.blocking_continuations_complete
        {
            return Err(InvalidContract);
        }
        let delta = snapshot
            .now_us
            .checked_sub(self.now_us)
            .ok_or(ClockRegression)?;
        for (index, app) in snapshot.admitted_apps.iter().enumerate() {
            for old in &snapshot.admitted_apps[..index] {
                m.charge()?;
                if app == old {
                    return Err(InvalidContract);
                }
            }
        }
        let mut waiting: Vec<SchedulerProtectedTarget> = vec![];
        let mut observed: Vec<(SchedulerProtectedTarget, bool)> = vec![];
        for o in snapshot.opportunities {
            m.charge()?;
            if o.generation == 0
                || o.continuously_eligible_since_us > snapshot.now_us
                || !label(o.episode_contract_identity)
            {
                return Err(InvalidContract);
            }
            let t = target(
                o.intent,
                o.generation,
                o.continuously_eligible_since_us,
                o.episode_contract_identity,
            )?;
            if !snapshot.admitted_apps.contains(&t.fairness_key) {
                return Err(InvalidContract);
            }
            for (other, _) in &observed {
                m.charge()?;
                if t.same_task(other) {
                    return Err(InvalidContract);
                }
            }
            let eligible = match (o.eligibility, o.feasibility) {
                (SchedulerProtectedEligibility::Unknown, _) => return Err(UnknownEvidence),
                (SchedulerProtectedEligibility::Ineligible, _) => false,
                (_, SchedulerProtectedFeasibility::Unknown) => return Err(UnknownEvidence),
                (_, SchedulerProtectedFeasibility::Infeasible) => false,
                _ => true,
            };
            if eligible {
                waiting.push(t.clone());
            }
            observed.push((t, eligible));
        }
        let mut blockers: Vec<Blocker> = vec![];
        if !snapshot.blocking_continuations.is_empty() && snapshot.protected_target.is_none() {
            return Err(InvalidContract);
        }
        for b in snapshot.blocking_continuations {
            m.charge()?;
            if b.generation == 0 || !label(b.envelope_identity) || !b.full_growth_covered {
                return Err(InvalidContract);
            }
            let t = target(b.intent, b.generation, 0, b.envelope_identity)?;
            for old in &blockers {
                m.charge()?;
                if t.same_task(&old.target) {
                    return Err(InvalidContract);
                }
            }
            blockers.push(Blocker { target: t });
        }
        // All cloning has fixed item/label bounds. Persisted membership prevents
        // same-app resubmission or omission from buying a second turn this round.
        let mut next = self.clone();
        next.now_us = snapshot.now_us;
        next.revision = next.revision.checked_add(1).ok_or(CounterOverflow)?;
        if self.active.is_none() {
            next.gap_used_us = next.gap_used_us.checked_add(delta).ok_or(CounterOverflow)?;
        }
        let mut finished = None;
        match event {
            SchedulerProtectedEvent::Observe => {}
            SchedulerProtectedEvent::Started { turn_id, attempt } => {
                let active = next.active.as_mut().ok_or(StaleEvent)?;
                if active.turn_id != turn_id
                    || active.attempt != attempt
                    || active.phase != SchedulerProtectedPhase::Draining
                {
                    return Err(StaleEvent);
                }
                active.phase = SchedulerProtectedPhase::Running;
            }
            SchedulerProtectedEvent::Finished {
                turn_id,
                attempt,
                outcome,
                physical_release_acknowledged,
            } => {
                let active = next.active.as_mut().ok_or(StaleEvent)?;
                if active.turn_id != turn_id || active.attempt != attempt {
                    return Err(StaleEvent);
                }
                if !physical_release_acknowledged {
                    return Err(ReleaseUnacknowledged);
                }
                if outcome == SchedulerProtectedOutcome::AcceptedOutput
                    && active.phase != SchedulerProtectedPhase::Running
                {
                    return Err(InvalidContract);
                }
                if outcome == SchedulerProtectedOutcome::AcceptedOutput
                    && waiting.iter().any(|t| t == &active.target)
                {
                    return Err(InvalidContract);
                }
                if outcome == SchedulerProtectedOutcome::Failed
                    && active.attempt < next.max_attempts
                {
                    active.attempt += 1;
                    active.phase = SchedulerProtectedPhase::Draining;
                } else {
                    let app = next
                        .apps
                        .iter_mut()
                        .find(|a| a.key == active.target.fairness_key)
                        .ok_or(InvalidContract)?;
                    app.served_round = next.round;
                    next.active = None;
                    finished = Some(outcome);
                }
            }
        }
        if next
            .active
            .as_ref()
            .is_some_and(|a| !snapshot.admitted_apps.contains(&a.target.fairness_key))
        {
            return Err(ActiveAppRemoved);
        }
        for seen in &mut next.seen {
            m.charge()?;
            seen.eligible = false;
        }
        for (t, eligible) in &observed {
            for old in &self.seen {
                m.charge()?;
                if !t.same_task(&old.target) {
                    continue;
                }
                if t.fairness_key != old.target.fairness_key || t.node_id != old.target.node_id {
                    return Err(ContinuityChanged);
                }
                if t.generation < old.target.generation
                    || (t.generation == old.target.generation && t != &old.target)
                    || (t.generation > old.target.generation && old.eligible)
                {
                    return Err(ContinuityChanged);
                }
            }
            if next
                .active
                .as_ref()
                .is_some_and(|a| t.same_task(&a.target) && t != &a.target)
            {
                return Err(ContinuityChanged);
            }
            if let Some(old) = next.seen.iter_mut().find(|old| t.same_task(&old.target)) {
                old.target = t.clone();
                old.eligible = *eligible;
            } else {
                if next.seen.len() == SCHEDULER_PROTECTION_MAX_OPPORTUNITIES {
                    return Err(WorkLimitExceeded);
                }
                next.seen.push(Seen {
                    target: t.clone(),
                    eligible: *eligible,
                });
            }
        }
        for _ in 0..next.seen.len() * next.seen.len() {
            m.charge()?;
        }
        next.seen.sort_by(|a, b| {
            (
                &a.target.fairness_key,
                &a.target.workflow_id,
                &a.target.workflow_run_id,
                &a.target.task_id,
            )
                .cmp(&(
                    &b.target.fairness_key,
                    &b.target.workflow_id,
                    &b.target.workflow_run_id,
                    &b.target.task_id,
                ))
        });
        for _ in 0..blockers.len() * blockers.len() {
            m.charge()?;
        }
        blockers.sort_by(|a, b| {
            (
                &a.target.fairness_key,
                &a.target.workflow_id,
                &a.target.workflow_run_id,
                &a.target.task_id,
            )
                .cmp(&(
                    &b.target.fairness_key,
                    &b.target.workflow_id,
                    &b.target.workflow_run_id,
                    &b.target.task_id,
                ))
        });
        let mut new_apps: Vec<_> = snapshot
            .admitted_apps
            .iter()
            .filter(|key| !next.apps.iter().any(|a| &a.key == *key))
            .cloned()
            .collect();
        // Bounded stable ordering; new apps never enter ahead of existing turns.
        for _ in 0..new_apps.len() * new_apps.len() {
            m.charge()?;
        }
        new_apps.sort();
        if next.apps.len() + new_apps.len() > SCHEDULER_PROTECTION_MAX_APPS {
            return Err(WorkLimitExceeded);
        }
        for key in new_apps {
            next.apps.push(App {
                key,
                served_round: 0,
            });
        }
        next.pending.retain(|key| {
            waiting.iter().any(|t| &t.fairness_key == key)
                && next
                    .apps
                    .iter()
                    .any(|a| &a.key == key && a.served_round != next.round)
        });
        for app in &next.apps {
            m.charge()?;
            if app.served_round != next.round
                && waiting.iter().any(|t| t.fairness_key == app.key)
                && !next.pending.contains(&app.key)
                && next
                    .active
                    .as_ref()
                    .is_none_or(|a| a.target.fairness_key != app.key)
            {
                next.pending.push(app.key.clone());
            }
        }
        if next.active.is_none() && next.pending.is_empty() && !waiting.is_empty() {
            next.round = next.round.checked_add(1).ok_or(CounterOverflow)?;
            next.gap_used_us = 0;
            for app in &next.apps {
                m.charge()?;
                if waiting.iter().any(|t| t.fairness_key == app.key) {
                    next.pending.push(app.key.clone());
                }
            }
        }
        if next.active.is_none()
            && next.gap_used_us >= next.gap_budget_us
            && !next.pending.is_empty()
        {
            let key = next.pending.remove(0);
            for _ in 0..waiting.len() * waiting.len() {
                m.charge()?;
            }
            let chosen = waiting
                .iter()
                .filter(|t| t.fairness_key == key)
                .min_by(|a, b| {
                    (
                        a.continuously_eligible_since_us,
                        &a.workflow_id,
                        &a.workflow_run_id,
                        &a.task_id,
                        a.generation,
                    )
                        .cmp(&(
                            b.continuously_eligible_since_us,
                            &b.workflow_id,
                            &b.workflow_run_id,
                            &b.task_id,
                            b.generation,
                        ))
                })
                .ok_or(InvalidContract)?
                .clone();
            next.next_turn = next.next_turn.checked_add(1).ok_or(CounterOverflow)?;
            next.active = Some(SchedulerProtectedEpisode {
                turn_id: next.next_turn,
                attempt: 1,
                phase: SchedulerProtectedPhase::Draining,
                target: chosen,
            });
        }
        let target_envelope_qualified = match (&next.active, snapshot.protected_target) {
            (Some(active), Some(t)) if &active.target == t => true,
            // An acknowledged finish can activate the next queued app in this
            // same transition. Its predecessor's owner frame is now obsolete.
            (Some(_), Some(t)) if self.active.as_ref().is_some_and(|old| &old.target == t) => false,
            (Some(_), Some(_)) => return Err(InvalidContract),
            _ => false,
        };
        let next_wake_after_us = if next.active.is_none() && !next.pending.is_empty() {
            Some(next.gap_budget_us.saturating_sub(next.gap_used_us))
        } else {
            None
        };
        let decision = SchedulerProtectedDecision {
            owner_identity: next.owner_identity.clone(),
            revision: next.revision,
            round: next.round,
            discretionary_used_us: next.gap_used_us,
            active: next.active.clone(),
            finished,
            work_units: m.used,
            target_envelope_qualified,
            next_wake_after_us,
            blockers,
        };
        Ok((next, decision))
    }
}

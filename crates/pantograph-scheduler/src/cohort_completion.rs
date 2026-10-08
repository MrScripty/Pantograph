//! Bounded, serialized, advisory cohort completion policy.
//!
//! Two completion events are optimized. Every remaining represented obligation
//! is then completed by the same earliest-predicted-completion continuation.
//! Owner-qualified projections are inputs, never executable readiness or leases.
use crate::dispatch_selection_policy::candidate_constraint_eligibility;
use crate::two_completion::bounded;
use crate::{
    SchedulableTaskIntent, SchedulerCompletionContext, SchedulerCompletionEvidenceSource,
    SchedulerCompletionRankingPolicy, SchedulerDispatchCandidate, SchedulerDispatchCandidateId,
    SchedulerResourceFitState, SchedulerTaskId, ValidatedSchedulerCompletionSuccessor,
    ValidatedSchedulerDispatchSelectionRequest,
};

mod workflow_objective;
pub use workflow_objective::{
    SchedulerCohortWorkflowCompletion, SchedulerCohortWorkflowIncomplete,
    SchedulerCohortWorkflowObjective, SchedulerCohortWorkflowObligation,
    SchedulerCohortWorkflowProfile, SchedulerCohortWorkflowRelease, SchedulerCohortWorkflowResult,
    SchedulerCohortWorkflowScore, SCHEDULER_COHORT_WORKFLOW_OUTPUT_CONVENTION,
};

pub const SCHEDULER_COHORT_MAX_TASKS: usize = 4;
pub const SCHEDULER_COHORT_MAX_PLACEMENTS: usize = 2;
// The complete fixed-first four-task tree has 2 + 12 + 48 + 96 edges.
pub const SCHEDULER_COHORT_MAX_EVIDENCE: usize = 158;
pub const SCHEDULER_COHORT_MAX_BRANCHES: usize = 64;
pub const SCHEDULER_COHORT_MAX_EVENTS: usize = 512;
pub const SCHEDULER_COHORT_MAX_WORK: usize = 131_072;

/// Exactly one task must borrow the already-admitted first request. Forecasts
/// have no Ready proof; their actual execution still needs ordinary admission.
#[derive(Clone, Copy)]
pub enum SchedulerCohortTaskRequest<'a> {
    Admitted(&'a ValidatedSchedulerDispatchSelectionRequest),
    Forecast(&'a ValidatedSchedulerCompletionSuccessor),
}
impl SchedulerCohortTaskRequest<'_> {
    fn intent(&self) -> &SchedulableTaskIntent {
        match self {
            Self::Admitted(r) => &r.as_ref().task_intent,
            Self::Forecast(r) => &r.as_ref().task_intent,
        }
    }
    fn candidates(&self) -> &[SchedulerDispatchCandidate] {
        match self {
            Self::Admitted(r) => &r.as_ref().candidates,
            Self::Forecast(r) => &r.as_ref().candidates,
        }
    }
}

/// An owner-certified capability-valid serialized placement. The kernel checks
/// dispatch constraints and exact association, not runtime capability discovery.
pub struct SchedulerCohortPlacement<'a> {
    pub candidate: &'a SchedulerDispatchCandidate,
    pub runtime_instance_id: &'a str,
    pub artifact_fingerprint: &'a str,
}
pub struct SchedulerCohortTask<'a> {
    pub request: SchedulerCohortTaskRequest<'a>,
    /// Complete placement universe, exactly matching the request's candidates.
    pub placements: &'a [SchedulerCohortPlacement<'a>],
    /// Complete internal prerequisite set. Unknown, duplicate and cyclic edges
    /// are rejected. External prerequisites must already be owner-satisfied.
    pub dependencies: &'a [SchedulerTaskId],
    pub workload_fingerprint: &'a str,
}

/// Owner-frozen represented population, objective, dependencies and observation.
/// This declares represented-task completion, not arbitrary workflow completion.
/// Identity labels bind this invocation; they do not authenticate calibration or
/// prove that the owner included every real queue obligation.
pub struct SchedulerFrozenCohort<'a> {
    pub identity: &'a str,
    pub objective_identity: &'a str,
    pub dependency_snapshot_identity: &'a str,
    pub first: &'a ValidatedSchedulerDispatchSelectionRequest,
    pub tasks: &'a [SchedulerCohortTask<'a>],
    pub host_id: &'a str,
    pub timing_convention: &'a str,
    pub initial_condition_fingerprint: &'a str,
    pub initial_residency_fingerprint: &'a str,
    pub source: SchedulerCompletionEvidenceSource,
}

#[derive(Clone, Copy)]
pub struct SchedulerCohortAction<'a> {
    pub task: &'a SchedulerCohortTask<'a>,
    pub placement: &'a SchedulerCohortPlacement<'a>,
}
/// Disjoint serialized stage durations, including explicit zeroes when absent.
/// Execution ends at output completion; cleanup/retention account for the
/// remaining owner work before the next task may begin. Reload is separate from
/// setup so that a warm label never silently removes its transition cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerCohortCosts {
    pub setup_us: Option<u64>,
    pub transfer_us: Option<u64>,
    pub execution_us: Option<u64>,
    pub cleanup_us: Option<u64>,
    pub retention_us: Option<u64>,
    pub reload_us: Option<u64>,
}
impl SchedulerCohortCosts {
    fn total(self) -> Result<u64, SchedulerCohortIncomplete> {
        let mut total = 0_u64;
        for stage in [
            self.setup_us,
            self.transfer_us,
            self.execution_us,
            self.cleanup_us,
            self.retention_us,
            self.reload_us,
        ] {
            total = total
                .checked_add(stage.ok_or(SchedulerCohortIncomplete::MissingStage)?)
                .ok_or(SchedulerCohortIncomplete::DurationOverflow)?;
        }
        Ok(total)
    }
}
#[derive(Clone, Copy)]
pub struct SchedulerCohortSample<'a> {
    pub context: SchedulerCompletionContext<'a>,
    pub source: SchedulerCompletionEvidenceSource,
    pub sample_count: u32,
    pub observed_at_ms: u64,
    pub costs: SchedulerCohortCosts,
}
#[derive(Clone, Copy)]
pub struct SchedulerCohortTransition<'a> {
    /// After completion, cleanup, acknowledged release/reconciliation, retention
    /// and any other required transition work. No capacity subtraction is used.
    pub condition_fingerprint: &'a str,
    pub residency_fingerprint: &'a str,
}
#[derive(Clone)]
pub struct SchedulerCohortEvidence<'a> {
    pub cohort: &'a SchedulerFrozenCohort<'a>,
    /// Exact ordered observation-conditioned path, including placements. Empty
    /// only before the admitted task. No hidden future outcome is an input.
    pub history: &'a [SchedulerCohortAction<'a>],
    pub action: SchedulerCohortAction<'a>,
    pub context: SchedulerCompletionContext<'a>,
    pub resource_fit: SchedulerResourceFitState,
    /// Fits requires both fields. Known blocked placements require neither.
    pub sample: Option<SchedulerCohortSample<'a>>,
    pub transition: Option<SchedulerCohortTransition<'a>>,
}

#[derive(Debug, Clone, Copy)]
pub struct SchedulerCohortBudget {
    pub max_branch_expansions: usize,
    pub max_completion_events: usize,
    /// Includes validation, membership/evidence comparisons, candidate scans,
    /// objective comparisons and continuation work, not just optimized branches.
    pub max_work_units: usize,
}
impl Default for SchedulerCohortBudget {
    fn default() -> Self {
        Self {
            max_branch_expansions: SCHEDULER_COHORT_MAX_BRANCHES,
            max_completion_events: SCHEDULER_COHORT_MAX_EVENTS,
            max_work_units: SCHEDULER_COHORT_MAX_WORK,
        }
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerCohortWork {
    pub work_units: usize,
    pub evidence_rows_validated: usize,
    pub candidate_evaluations: usize,
    pub branch_expansions: usize,
    pub completion_events: usize,
    pub continuation_events: usize,
    pub complete_plans: usize,
    pub dead_ends: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerCohortIncomplete {
    WorkLimitExceeded,
    InvalidPolicy,
    InvalidCohort,
    UnsupportedPlacement,
    InvalidDependencies,
    ForeignSnapshot,
    DuplicateEvidence,
    MissingEvidence,
    InvalidHistory,
    InvalidContext,
    StaleOrFutureSample,
    InsufficientSamples,
    IncomparableEvidence,
    UnknownCapacity,
    MissingStage,
    DurationOverflow,
    NoCompletePlan,
    BudgetExhausted,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerCohortCompletion {
    pub task_id: SchedulerTaskId,
    pub candidate_id: SchedulerDispatchCandidateId,
    pub costs: SchedulerCohortCosts,
    /// End of all six stages: safe serialized completion, including cleanup.
    pub completion_us: u64,
    pub optimized: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerCohortScore {
    pub cohort_identity: String,
    pub objective_identity: String,
    pub dependency_snapshot_identity: String,
    pub first_candidate_id: SchedulerDispatchCandidateId,
    pub terminal_completion_us: u64,
    pub completion_sum_us: u64,
    pub completions: Vec<SchedulerCohortCompletion>,
    pub source: SchedulerCompletionEvidenceSource,
    pub work: SchedulerCohortWork,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerCohortResult {
    Ranked(SchedulerCohortScore),
    /// No advisory action is returned from an incomplete comparison. Consumers
    /// retain their existing safe selector; this function never invokes it.
    Incomplete {
        reason: SchedulerCohortIncomplete,
        work: SchedulerCohortWork,
    },
}

struct Meter {
    budget: SchedulerCohortBudget,
    work: SchedulerCohortWork,
}
impl Meter {
    fn charge(&mut self) -> Result<(), SchedulerCohortIncomplete> {
        if self.work.work_units == self.budget.max_work_units {
            return Err(SchedulerCohortIncomplete::BudgetExhausted);
        }
        self.work.work_units += 1;
        Ok(())
    }
    fn event(&mut self, optimized: bool) -> Result<(), SchedulerCohortIncomplete> {
        self.charge()?;
        if self.work.completion_events == self.budget.max_completion_events
            || (optimized && self.work.branch_expansions == self.budget.max_branch_expansions)
        {
            return Err(SchedulerCohortIncomplete::BudgetExhausted);
        }
        self.work.completion_events += 1;
        if optimized {
            self.work.branch_expansions += 1;
        } else {
            self.work.continuation_events += 1;
        }
        Ok(())
    }
}

/// Exhaustively optimize the first two legal serialized completion events,
/// fixing the already-admitted first task. Then repeatedly choose the legal
/// (predicted completion, task ID, placement ID) minimum until ALL tasks finish.
/// Compare (terminal completion, completion sum, ordered stable action IDs).
/// A known dead end is rejected; missing/unknown/overflow/budget evidence anywhere
/// in the required comparison invalidates the whole result, never a partial win.
///
/// Length/field guards precede validation or search. They have fixed hard bounds;
/// work units count bounded operations (each handles <=4 actions or <=128-byte
/// labels), not instructions or a wall-time guarantee. No runtime/registry/store
/// calls, leases, executable readiness, clocks, or persistent mutation occur.
pub fn evaluate_scheduler_cohort_completion(
    cohort: &SchedulerFrozenCohort<'_>,
    evidence: &[SchedulerCohortEvidence<'_>],
    policy: SchedulerCompletionRankingPolicy,
    budget: SchedulerCohortBudget,
) -> SchedulerCohortResult {
    let (result, work) = evaluate(cohort, evidence, policy, budget, None);
    match result {
        Ok(best) => SchedulerCohortResult::Ranked(SchedulerCohortScore {
            cohort_identity: cohort.identity.to_owned(),
            objective_identity: cohort.objective_identity.to_owned(),
            dependency_snapshot_identity: cohort.dependency_snapshot_identity.to_owned(),
            first_candidate_id: best.state.completions[0].candidate_id.clone(),
            terminal_completion_us: best.state.elapsed,
            completion_sum_us: best.state.sum,
            completions: best.state.completions,
            source: cohort.source,
            work,
        }),
        Err(SchedulerCohortWorkflowIncomplete::Cohort(reason)) => {
            SchedulerCohortResult::Incomplete { reason, work }
        }
        // Objective-only failures cannot arise without an objective. Preserve
        // the legacy reason enum and fail closed if that invariant changes.
        Err(
            SchedulerCohortWorkflowIncomplete::InvalidObjective
            | SchedulerCohortWorkflowIncomplete::ObjectiveOverflow,
        ) => SchedulerCohortResult::Incomplete {
            reason: SchedulerCohortIncomplete::InvalidCohort,
            work,
        },
    }
}

/// Opt-in closed-workflow makespan or weighted flow-time loss on the SAME
/// bounded two-event search/common six-stage continuation. Owner declarations
/// must cover all required outputs/branches/retries and their acceptance boundary.
/// This authorizes no native execution, app entitlement or resource release.
/// Incomplete comparisons return no winner; retain the current safe selector.
pub fn evaluate_scheduler_cohort_workflow_objective(
    cohort: &SchedulerFrozenCohort<'_>,
    evidence: &[SchedulerCohortEvidence<'_>],
    objective: &SchedulerCohortWorkflowObjective<'_>,
    policy: SchedulerCompletionRankingPolicy,
    budget: SchedulerCohortBudget,
) -> SchedulerCohortWorkflowResult {
    let (result, work) = evaluate(cohort, evidence, policy, budget, Some(objective));
    match result {
        Ok(best) => {
            let values = best
                .objective
                .expect("explicit workflow objective scored every winner");
            SchedulerCohortWorkflowResult::Ranked(Box::new(SchedulerCohortWorkflowScore {
                cohort_identity: cohort.identity.to_owned(),
                objective_identity: cohort.objective_identity.to_owned(),
                dependency_snapshot_identity: cohort.dependency_snapshot_identity.to_owned(),
                first_candidate_id: best.state.completions[0].candidate_id.clone(),
                terminal_completion_us: best.state.elapsed,
                completions: best.state.completions,
                workflow_completions: values.completions,
                profile: objective.profile,
                objective_loss_us: values.loss,
                workflow_makespan_us: values.makespan,
                workflow_completion_sum_us: values.sum,
                source: cohort.source,
                work,
            }))
        }
        Err(reason) => SchedulerCohortWorkflowResult::Incomplete { reason, work },
    }
}

fn evaluate<'a>(
    cohort: &SchedulerFrozenCohort<'a>,
    evidence: &[SchedulerCohortEvidence<'a>],
    policy: SchedulerCompletionRankingPolicy,
    budget: SchedulerCohortBudget,
    objective: Option<&SchedulerCohortWorkflowObjective<'_>>,
) -> (
    Result<Winner<'a>, SchedulerCohortWorkflowIncomplete>,
    SchedulerCohortWork,
) {
    let mut meter = Meter {
        budget,
        work: SchedulerCohortWork::default(),
    };
    let result = (|| {
        if !within_bounds(cohort, evidence, budget)
            || objective.is_some_and(|o| !workflow_objective::bounded(o))
        {
            return Err(SchedulerCohortIncomplete::WorkLimitExceeded.into());
        }
        validate_cohort(cohort, policy, objective.is_some(), &mut meter)?;
        if let Some(objective) = objective {
            workflow_objective::validate(cohort, objective, &mut meter)?;
        }
        validate_evidence(cohort, evidence, policy, &mut meter)?;
        search(cohort, evidence, objective, &mut meter)
    })();
    (result, meter.work)
}
fn text(s: &str) -> bool {
    s.len() <= 128 && !s.trim().is_empty() && !s.chars().any(char::is_control)
}
fn within_bounds(
    c: &SchedulerFrozenCohort<'_>,
    rows: &[SchedulerCohortEvidence<'_>],
    b: SchedulerCohortBudget,
) -> bool {
    c.tasks.len() <= SCHEDULER_COHORT_MAX_TASKS
        && rows.len() <= SCHEDULER_COHORT_MAX_EVIDENCE
        && b.max_branch_expansions <= SCHEDULER_COHORT_MAX_BRANCHES
        && b.max_completion_events <= SCHEDULER_COHORT_MAX_EVENTS
        && b.max_work_units <= SCHEDULER_COHORT_MAX_WORK
        && bounded(c.first)
        && c.tasks.iter().all(|t| {
            t.placements.len() <= SCHEDULER_COHORT_MAX_PLACEMENTS
                && t.request.candidates().len() <= SCHEDULER_COHORT_MAX_PLACEMENTS
                && t.dependencies.len() < SCHEDULER_COHORT_MAX_TASKS
                && match t.request {
                    SchedulerCohortTaskRequest::Admitted(r) => bounded(r),
                    // The validated forecast already has fixed field bounds.
                    SchedulerCohortTaskRequest::Forecast(_) => true,
                }
        })
        && rows
            .iter()
            .all(|r| r.history.len() < SCHEDULER_COHORT_MAX_TASKS)
}
fn same_action(a: SchedulerCohortAction<'_>, b: SchedulerCohortAction<'_>) -> bool {
    std::ptr::eq(a.task, b.task) && std::ptr::eq(a.placement, b.placement)
}
fn same_history(a: &[SchedulerCohortAction<'_>], b: &[SchedulerCohortAction<'_>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_action(*a, *b))
}
fn completed(history: &[SchedulerCohortAction<'_>], task: &SchedulerCohortTask<'_>) -> bool {
    history.iter().any(|a| std::ptr::eq(a.task, task))
}
fn ready(history: &[SchedulerCohortAction<'_>], task: &SchedulerCohortTask<'_>) -> bool {
    task.dependencies.iter().all(|id| {
        history
            .iter()
            .any(|a| &a.task.request.intent().task_id == id)
    })
}
fn validate_cohort(
    c: &SchedulerFrozenCohort<'_>,
    policy: SchedulerCompletionRankingPolicy,
    workflow_objective: bool,
    m: &mut Meter,
) -> Result<(), SchedulerCohortIncomplete> {
    use SchedulerCohortIncomplete as R;
    m.charge()?;
    if policy.minimum_samples == 0 || policy.max_sample_age_ms == 0 {
        return Err(R::InvalidPolicy);
    }
    if c.source == SchedulerCompletionEvidenceSource::Synthetic && !policy.allow_synthetic {
        return Err(R::IncomparableEvidence);
    }
    if c.tasks.is_empty()
        || ![
            c.identity,
            c.objective_identity,
            c.dependency_snapshot_identity,
            c.host_id,
            c.timing_convention,
            c.initial_condition_fingerprint,
            c.initial_residency_fingerprint,
        ]
        .into_iter()
        .all(text)
    {
        return Err(R::InvalidCohort);
    }
    let mut admitted = 0;
    for (i, task) in c.tasks.iter().enumerate() {
        m.charge()?;
        let intent = task.request.intent();
        if !text(task.workload_fingerprint)
            || task.placements.is_empty()
            || task.placements.len() != task.request.candidates().len()
            || (!workflow_objective
                && (intent.workflow_id != c.first.as_ref().task_intent.workflow_id
                    || intent.workflow_run_id != c.first.as_ref().task_intent.workflow_run_id))
            || c.tasks[..i]
                .iter()
                .any(|old| old.request.intent().task_id == intent.task_id)
        {
            return Err(R::InvalidCohort);
        }
        if let SchedulerCohortTaskRequest::Forecast(r) = task.request {
            if task.workload_fingerprint != r.as_ref().snapshot_identity {
                return Err(R::InvalidCohort);
            }
        }
        if let SchedulerCohortTaskRequest::Admitted(r) = task.request {
            if !std::ptr::eq(r, c.first) || !task.dependencies.is_empty() {
                return Err(R::InvalidCohort);
            }
            admitted += 1;
        }
        for (j, p) in task.placements.iter().enumerate() {
            m.charge()?;
            if !task
                .request
                .candidates()
                .iter()
                .any(|x| std::ptr::eq(x, p.candidate))
                || task.placements[..j]
                    .iter()
                    .any(|old| old.candidate.candidate_id == p.candidate.candidate_id)
            {
                return Err(R::ForeignSnapshot);
            }
            if !text(p.runtime_instance_id)
                || !text(p.artifact_fingerprint)
                || p.candidate.selected_device_ids.len() != 1
                || p.candidate.batching_group_id.is_some()
                || !p.candidate.reservations.is_empty()
                || candidate_constraint_eligibility(intent, p.candidate).is_err()
            {
                return Err(R::UnsupportedPlacement);
            }
        }
        for (j, dep) in task.dependencies.iter().enumerate() {
            m.charge()?;
            if dep == &intent.task_id
                || task.dependencies[..j].contains(dep)
                || !c.tasks.iter().any(|t| &t.request.intent().task_id == dep)
            {
                return Err(R::InvalidDependencies);
            }
        }
    }
    if admitted != 1 {
        return Err(R::InvalidCohort);
    }
    // Bounded topological closure checks every represented obligation, even one
    // that resource evidence would otherwise make unreachable.
    let mut seen = Vec::new();
    for _ in 0..c.tasks.len() {
        for task in c.tasks {
            m.charge()?;
            let id = &task.request.intent().task_id;
            if !seen.contains(&id) && task.dependencies.iter().all(|d| seen.contains(&d)) {
                seen.push(id);
            }
        }
    }
    if seen.len() != c.tasks.len() {
        return Err(R::InvalidDependencies);
    }
    Ok(())
}
fn associated(c: &SchedulerFrozenCohort<'_>, a: SchedulerCohortAction<'_>) -> bool {
    c.tasks.iter().any(|t| std::ptr::eq(t, a.task))
        && a.task
            .placements
            .iter()
            .any(|p| std::ptr::eq(p, a.placement))
}
fn find_row<'r, 'a>(
    rows: &'r [SchedulerCohortEvidence<'a>],
    history: &[SchedulerCohortAction<'_>],
    action: SchedulerCohortAction<'_>,
    m: &mut Meter,
) -> Result<&'r SchedulerCohortEvidence<'a>, SchedulerCohortIncomplete> {
    let mut found = None;
    // Full scan keeps work counts independent of evidence ordering.
    for row in rows {
        m.charge()?;
        if same_action(row.action, action) && same_history(row.history, history) {
            if found.is_some() {
                return Err(SchedulerCohortIncomplete::DuplicateEvidence);
            }
            found = Some(row);
        }
    }
    found.ok_or(SchedulerCohortIncomplete::MissingEvidence)
}
fn validate_evidence(
    c: &SchedulerFrozenCohort<'_>,
    rows: &[SchedulerCohortEvidence<'_>],
    policy: SchedulerCompletionRankingPolicy,
    m: &mut Meter,
) -> Result<(), SchedulerCohortIncomplete> {
    use SchedulerCohortIncomplete as R;
    // Association and all field bounds precede dereferencing foreign histories
    // or comparing their context labels to other rows.
    for row in rows {
        m.charge()?;
        if !std::ptr::eq(row.cohort, c)
            || !associated(c, row.action)
            || row.history.iter().any(|a| !associated(c, *a))
        {
            return Err(R::ForeignSnapshot);
        }
        if !row.context.valid()
            || row.sample.is_some_and(|s| !s.context.valid())
            || row
                .transition
                .is_some_and(|t| !text(t.condition_fingerprint) || !text(t.residency_fingerprint))
        {
            return Err(R::InvalidContext);
        }
    }
    for row in rows {
        m.charge()?;
        let action = row.action;
        let p = action.placement;
        let context = row.context;
        for (i, a) in row
            .history
            .iter()
            .chain(std::iter::once(&action))
            .enumerate()
        {
            m.charge()?;
            if (i == 0) != matches!(a.task.request, SchedulerCohortTaskRequest::Admitted(_))
                || completed(&row.history[..i], a.task)
                || !ready(&row.history[..i], a.task)
            {
                return Err(R::InvalidHistory);
            }
        }
        find_row(rows, row.history, action, m)?;
        let condition = if let Some((last, prefix)) = row.history.split_last() {
            let previous = find_row(rows, prefix, *last, m)?;
            if previous.resource_fit != SchedulerResourceFitState::Fits {
                return Err(R::InvalidHistory);
            }
            previous.transition.ok_or(R::InvalidContext)?
        } else {
            let fit = p
                .candidate
                .resource_fit_assessment
                .as_ref()
                .ok_or(R::UnknownCapacity)?;
            if fit.state != row.resource_fit {
                return Err(R::IncomparableEvidence);
            }
            SchedulerCohortTransition {
                condition_fingerprint: c.initial_condition_fingerprint,
                residency_fingerprint: c.initial_residency_fingerprint,
            }
        };
        if context.host_id != c.host_id
            || context.timing_convention != c.timing_convention
            || context.workload_fingerprint != action.task.workload_fingerprint
            || context.runtime_instance_id != p.runtime_instance_id
            || context.artifact_fingerprint != p.artifact_fingerprint
            || context.resource_condition_fingerprint != condition.condition_fingerprint
            || context.residency_fingerprint != condition.residency_fingerprint
        {
            return Err(R::InvalidContext);
        }
        match row.resource_fit {
            SchedulerResourceFitState::Unknown => return Err(R::UnknownCapacity),
            SchedulerResourceFitState::WaitingForResources
            | SchedulerResourceFitState::ImpossibleFit => {
                if row.sample.is_some() || row.transition.is_some() {
                    return Err(R::IncomparableEvidence);
                }
            }
            SchedulerResourceFitState::Fits => {
                let sample = row.sample.ok_or(R::MissingEvidence)?;
                if row.transition.is_none()
                    || sample.context != context
                    || sample.source != c.source
                {
                    return Err(R::IncomparableEvidence);
                }
                if sample.sample_count < policy.minimum_samples {
                    return Err(R::InsufficientSamples);
                }
                if policy
                    .now_ms
                    .checked_sub(sample.observed_at_ms)
                    .is_none_or(|age| age > policy.max_sample_age_ms)
                {
                    return Err(R::StaleOrFutureSample);
                }
                sample.costs.total()?;
            }
        }
        m.work.evidence_rows_validated += 1;
    }
    Ok(())
}

#[derive(Clone, Default)]
struct State<'a> {
    history: Vec<SchedulerCohortAction<'a>>,
    completions: Vec<SchedulerCohortCompletion>,
    elapsed: u64,
    sum: u64,
}
fn choices<'r, 'a>(
    c: &SchedulerFrozenCohort<'a>,
    rows: &'r [SchedulerCohortEvidence<'a>],
    state: &State<'a>,
    m: &mut Meter,
) -> Result<Vec<(&'r SchedulerCohortEvidence<'a>, u64)>, SchedulerCohortIncomplete> {
    let mut choices = Vec::new();
    for task in c.tasks {
        m.charge()?;
        if completed(&state.history, task)
            || !ready(&state.history, task)
            || (state.history.is_empty()
                && !matches!(task.request, SchedulerCohortTaskRequest::Admitted(_)))
        {
            continue;
        }
        for placement in task.placements {
            m.charge()?;
            m.work.candidate_evaluations += 1;
            let row = find_row(
                rows,
                &state.history,
                SchedulerCohortAction { task, placement },
                m,
            )?;
            if row.resource_fit == SchedulerResourceFitState::Fits {
                let duration = row
                    .sample
                    .ok_or(SchedulerCohortIncomplete::MissingEvidence)?
                    .costs
                    .total()?;
                let end = state
                    .elapsed
                    .checked_add(duration)
                    .ok_or(SchedulerCohortIncomplete::DurationOverflow)?;
                choices.push((row, end));
            }
        }
    }
    // <=8 choices; charge a fixed quadratic bound before stable-ID sorting.
    for _ in 0..choices.len() * choices.len() {
        m.charge()?;
    }
    choices.sort_by(|(a, end_a), (b, end_b)| {
        (
            *end_a,
            &a.action.task.request.intent().task_id,
            &a.action.placement.candidate.candidate_id,
        )
            .cmp(&(
                *end_b,
                &b.action.task.request.intent().task_id,
                &b.action.placement.candidate.candidate_id,
            ))
    });
    Ok(choices)
}
fn advance<'a>(
    state: &State<'a>,
    row: &SchedulerCohortEvidence<'a>,
    end: u64,
    optimized: bool,
    legacy_objective: bool,
    m: &mut Meter,
) -> Result<State<'a>, SchedulerCohortIncomplete> {
    m.event(optimized)?;
    let mut next = state.clone();
    if legacy_objective {
        next.sum = next
            .sum
            .checked_add(end)
            .ok_or(SchedulerCohortIncomplete::DurationOverflow)?;
    }
    next.elapsed = end;
    next.history.push(row.action);
    next.completions.push(SchedulerCohortCompletion {
        task_id: row.action.task.request.intent().task_id.clone(),
        candidate_id: row.action.placement.candidate.candidate_id.clone(),
        costs: row
            .sample
            .ok_or(SchedulerCohortIncomplete::MissingEvidence)?
            .costs,
        completion_us: end,
        optimized,
    });
    Ok(next)
}
struct Winner<'a> {
    state: State<'a>,
    objective: Option<workflow_objective::Values>,
}
fn search<'a>(
    c: &SchedulerFrozenCohort<'a>,
    rows: &[SchedulerCohortEvidence<'a>],
    objective: Option<&SchedulerCohortWorkflowObjective<'_>>,
    m: &mut Meter,
) -> Result<Winner<'a>, SchedulerCohortWorkflowIncomplete> {
    let root = State::default();
    let mut winner = None;
    for (first, end) in choices(c, rows, &root, m)? {
        let initial = advance(&root, first, end, true, objective.is_none(), m)?;
        if c.tasks.len() == 1 {
            consider(initial, objective, &mut winner, m)?;
            continue;
        }
        let seconds = choices(c, rows, &initial, m)?;
        if seconds.is_empty() {
            m.work.dead_ends += 1;
        }
        for (second, end) in seconds {
            let mut state = advance(&initial, second, end, true, objective.is_none(), m)?;
            while state.history.len() < c.tasks.len() {
                let next = choices(c, rows, &state, m)?;
                let Some((row, end)) = next.first() else {
                    m.work.dead_ends += 1;
                    break;
                };
                state = advance(&state, row, *end, false, objective.is_none(), m)?;
            }
            if state.history.len() == c.tasks.len() {
                consider(state, objective, &mut winner, m)?;
            }
        }
    }
    winner.ok_or(SchedulerCohortIncomplete::NoCompletePlan.into())
}
fn consider<'a>(
    state: State<'a>,
    objective: Option<&SchedulerCohortWorkflowObjective<'_>>,
    winner: &mut Option<Winner<'a>>,
    m: &mut Meter,
) -> Result<(), SchedulerCohortWorkflowIncomplete> {
    m.charge()?;
    m.work.complete_plans += 1;
    let values = objective
        .map(|o| workflow_objective::score(o, &state, m))
        .transpose()?;
    let key = values
        .as_ref()
        .map_or((u128::from(state.elapsed), u128::from(state.sum), 0), |v| {
            v.key(objective.expect("scored explicit objective").profile)
        });
    if winner.as_ref().is_none_or(|old| {
        let old_key = old.objective.as_ref().map_or(
            (u128::from(old.state.elapsed), u128::from(old.state.sum), 0),
            |v| v.key(objective.expect("scored explicit objective").profile),
        );
        key.cmp(&old_key)
            .then_with(|| {
                state
                    .completions
                    .iter()
                    .map(|e| (&e.task_id, &e.candidate_id))
                    .cmp(
                        old.state
                            .completions
                            .iter()
                            .map(|e| (&e.task_id, &e.candidate_id)),
                    )
            })
            .is_lt()
    }) {
        *winner = Some(Winner {
            state,
            objective: values,
        });
    }
    Ok(())
}

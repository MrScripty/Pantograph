//! Frozen objective contracts and arithmetic; search remains in the parent.
use super::*;
use crate::{SchedulerWorkflowId, SchedulerWorkflowRunId};

/// Reload/setup/transfer/execution end at accepted output. Cleanup/retention
/// occur afterward and still block the next serialized start. Producers must
/// qualify this ordering; an opaque older timing label is insufficient.
pub const SCHEDULER_COHORT_WORKFLOW_OUTPUT_CONVENTION: &str =
    "serialized-six-stage-output-before-cleanup-us.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerCohortWorkflowProfile {
    Makespan,
    WeightedFlowTime,
}

/// Relative to the one frozen decision boundary, independently of sample age.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerCohortWorkflowRelease {
    Released { age_us: u64 },
    Unknown,
    Future,
}

/// A trusted owner's declaration of ALL required output, branch and accepted
/// retry obligations for one workflow run. A runtime-only capture is not such
/// a declaration. Labels/booleans do not authenticate this external authority.
pub struct SchedulerCohortWorkflowObligation<'a> {
    pub workflow_id: &'a SchedulerWorkflowId,
    pub workflow_run_id: &'a SchedulerWorkflowRunId,
    pub completion_contract_identity: &'a str,
    pub required_tasks: &'a [&'a SchedulerCohortTask<'a>],
    pub required_population_complete: bool,
    pub accepted_output_before_cleanup: bool,
    pub release: SchedulerCohortWorkflowRelease,
    /// Positive dimensionless integer weight, frozen for every compared path.
    /// Used only by WeightedFlowTime; it confers no progress entitlement.
    pub weight: u32,
}

/// Explicit opt-in mathematical profile, not an app priority/protection service.
/// The exact partition may include several workflow runs, with globally unique
/// task IDs and no cross-run prerequisite edges. All workflows must be released.
pub struct SchedulerCohortWorkflowObjective<'a> {
    pub cohort_identity: &'a str,
    pub objective_identity: &'a str,
    pub profile: SchedulerCohortWorkflowProfile,
    pub workflows: &'a [SchedulerCohortWorkflowObligation<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerCohortWorkflowCompletion {
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub completion_contract_identity: String,
    /// Latest required accepted output, relative to the frozen decision.
    pub output_completion_us: u64,
    pub release_age_us: u64,
    pub weight: u32,
    pub weighted_flow_time_us: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerCohortWorkflowScore {
    pub cohort_identity: String,
    pub objective_identity: String,
    pub dependency_snapshot_identity: String,
    pub first_candidate_id: SchedulerDispatchCandidateId,
    /// Full six-stage drain, including the last task's cleanup/retention.
    pub terminal_completion_us: u64,
    pub completions: Vec<SchedulerCohortCompletion>,
    pub workflow_completions: Vec<SchedulerCohortWorkflowCompletion>,
    pub profile: SchedulerCohortWorkflowProfile,
    /// Makespan: max workflow output end; speed: sum h_w(age_w + C_w).
    pub objective_loss_us: u128,
    pub workflow_makespan_us: u64,
    pub workflow_completion_sum_us: u128,
    pub source: SchedulerCompletionEvidenceSource,
    pub work: SchedulerCohortWork,
}

/// Objective-only refusals extend the new API without changing the legacy
/// public refusal enum or external exhaustive matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerCohortWorkflowIncomplete {
    Cohort(SchedulerCohortIncomplete),
    InvalidObjective,
    ObjectiveOverflow,
}
impl From<SchedulerCohortIncomplete> for SchedulerCohortWorkflowIncomplete {
    fn from(reason: SchedulerCohortIncomplete) -> Self {
        Self::Cohort(reason)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerCohortWorkflowResult {
    Ranked(Box<SchedulerCohortWorkflowScore>),
    Incomplete {
        reason: SchedulerCohortWorkflowIncomplete,
        work: SchedulerCohortWork,
    },
}

pub(super) fn bounded(objective: &SchedulerCohortWorkflowObjective<'_>) -> bool {
    objective.workflows.len() <= SCHEDULER_COHORT_MAX_TASKS
        && objective
            .workflows
            .iter()
            .all(|w| w.required_tasks.len() <= SCHEDULER_COHORT_MAX_TASKS)
        && objective
            .workflows
            .iter()
            .map(|w| w.required_tasks.len())
            .sum::<usize>()
            <= SCHEDULER_COHORT_MAX_TASKS
}

pub(super) fn validate(
    c: &SchedulerFrozenCohort<'_>,
    objective: &SchedulerCohortWorkflowObjective<'_>,
    m: &mut Meter,
) -> Result<(), SchedulerCohortWorkflowIncomplete> {
    use SchedulerCohortWorkflowIncomplete::InvalidObjective;
    m.charge()?;
    if objective.cohort_identity != c.identity
        || objective.objective_identity != c.objective_identity
        || c.timing_convention != SCHEDULER_COHORT_WORKFLOW_OUTPUT_CONVENTION
        || objective.workflows.is_empty()
    {
        return Err(InvalidObjective);
    }
    let mut seen = Vec::new();
    for (i, w) in objective.workflows.iter().enumerate() {
        m.charge()?;
        if !text(w.workflow_id.as_str())
            || !text(w.workflow_run_id.as_str())
            || !text(w.completion_contract_identity)
            || !w.required_population_complete
            || !w.accepted_output_before_cleanup
            || w.weight == 0
            || w.required_tasks.is_empty()
            || !matches!(w.release, SchedulerCohortWorkflowRelease::Released { .. })
            || objective.workflows[..i].iter().any(|old| {
                old.workflow_id == w.workflow_id && old.workflow_run_id == w.workflow_run_id
            })
        {
            return Err(InvalidObjective);
        }
        for task in w.required_tasks {
            m.charge()?;
            if !c.tasks.iter().any(|t| std::ptr::eq(t, *task))
                || seen.iter().any(|old| std::ptr::eq(*old, *task))
            {
                return Err(InvalidObjective);
            }
            let intent = task.request.intent();
            if &intent.workflow_id != w.workflow_id || &intent.workflow_run_id != w.workflow_run_id
            {
                return Err(InvalidObjective);
            }
            for dependency in task.dependencies {
                m.charge()?;
                let predecessor = c
                    .tasks
                    .iter()
                    .find(|t| &t.request.intent().task_id == dependency)
                    .ok_or(InvalidObjective)?
                    .request
                    .intent();
                if &predecessor.workflow_id != w.workflow_id
                    || &predecessor.workflow_run_id != w.workflow_run_id
                {
                    return Err(InvalidObjective);
                }
            }
            seen.push(*task);
        }
    }
    if seen.len() != c.tasks.len() {
        return Err(InvalidObjective);
    }
    Ok(())
}

pub(super) struct Values {
    pub completions: Vec<SchedulerCohortWorkflowCompletion>,
    pub loss: u128,
    pub makespan: u64,
    pub sum: u128,
}
impl Values {
    pub fn key(&self, profile: SchedulerCohortWorkflowProfile) -> (u128, u128, u128) {
        match profile {
            SchedulerCohortWorkflowProfile::Makespan => (self.loss, self.sum, 0),
            SchedulerCohortWorkflowProfile::WeightedFlowTime => {
                (self.loss, u128::from(self.makespan), self.sum)
            }
        }
    }
}

pub(super) fn score(
    objective: &SchedulerCohortWorkflowObjective<'_>,
    state: &State<'_>,
    m: &mut Meter,
) -> Result<Values, SchedulerCohortWorkflowIncomplete> {
    use SchedulerCohortWorkflowIncomplete::{InvalidObjective, ObjectiveOverflow};
    let mut result = Values {
        completions: Vec::new(),
        loss: 0,
        makespan: 0,
        sum: 0,
    };
    for w in objective.workflows {
        m.charge()?;
        let mut output_end = 0;
        for task in w.required_tasks {
            m.charge()?;
            let index = state
                .history
                .iter()
                .position(|a| std::ptr::eq(a.task, *task))
                .ok_or(InvalidObjective)?;
            let completion = &state.completions[index];
            let cleanup = completion.costs.cleanup_us.ok_or(InvalidObjective)?;
            let retention = completion.costs.retention_us.ok_or(InvalidObjective)?;
            let output = completion
                .completion_us
                .checked_sub(cleanup)
                .and_then(|t| t.checked_sub(retention))
                .ok_or(ObjectiveOverflow)?;
            output_end = output_end.max(output);
        }
        let SchedulerCohortWorkflowRelease::Released { age_us } = w.release else {
            return Err(InvalidObjective);
        };
        let flow = u128::from(age_us)
            .checked_add(u128::from(output_end))
            .and_then(|v| v.checked_mul(u128::from(w.weight)))
            .ok_or(ObjectiveOverflow)?;
        result.makespan = result.makespan.max(output_end);
        result.sum = result
            .sum
            .checked_add(u128::from(output_end))
            .ok_or(ObjectiveOverflow)?;
        result.loss = result.loss.checked_add(flow).ok_or(ObjectiveOverflow)?;
        result.completions.push(SchedulerCohortWorkflowCompletion {
            workflow_id: w.workflow_id.clone(),
            workflow_run_id: w.workflow_run_id.clone(),
            completion_contract_identity: w.completion_contract_identity.to_owned(),
            output_completion_us: output_end,
            release_age_us: age_us,
            weight: w.weight,
            weighted_flow_time_us: flow,
        });
    }
    if objective.profile == SchedulerCohortWorkflowProfile::Makespan {
        result.loss = u128::from(result.makespan);
    }
    // <=4 workflows; account for the full bounded stable-order comparison.
    for _ in 0..result.completions.len() * result.completions.len() {
        m.charge()?;
    }
    result.completions.sort_by(|a, b| {
        (&a.workflow_id, &a.workflow_run_id).cmp(&(&b.workflow_id, &b.workflow_run_id))
    });
    Ok(result)
}

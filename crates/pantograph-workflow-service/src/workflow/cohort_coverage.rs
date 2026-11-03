//! Read-only, constructor-opt-in cohort inspection. This is not a selector,
//! reservation API, Ready admission, or native evidence producer.
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use pantograph_dependency_planning::DependencyReadinessProofEnvelope;
use pantograph_runtime_host_contracts::RuntimeHostExecutionInput;
use pantograph_scheduler::{
    SchedulerTaskId, SchedulerTaskStateRecord, SCHEDULER_COHORT_MAX_PLACEMENTS,
};
use serde::Serialize;

use super::{
    WorkflowSchedulerTask, WorkflowSchedulerTaskInputBinding, WorkflowService, WorkflowServiceError,
};

/// Aggregate serialized capture budget, including graph/record fingerprinting,
/// proof, selected known inputs and the returned snapshot. Refuse, never truncate.
pub const WORKFLOW_COHORT_MAX_SNAPSHOT_BYTES: usize = 256 * 1024;
pub const WORKFLOW_COHORT_MAX_GRAPH_TASKS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkflowCohortKnownInputIdentity {
    pub binding: WorkflowSchedulerTaskInputBinding,
    pub source_state_version: u64,
    /// Exact selected persisted value, including model bindings omitted by the
    /// runtime mapper. This is identity, not a workload-shape qualification.
    pub value_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkflowCohortMember {
    pub task: WorkflowSchedulerTask,
    pub record: SchedulerTaskStateRecord,
    pub known_inputs: Vec<RuntimeHostExecutionInput>,
    pub known_input_identities: Vec<WorkflowCohortKnownInputIdentity>,
    /// Internal outputs that do not exist yet. No value/size/transfer estimate
    /// is inferred from these bindings.
    pub symbolic_inputs: Vec<WorkflowSchedulerTaskInputBinding>,
}

/// Created only by the execution store. No deserialization/client constructor.
/// Members are deterministic (task-id order); first_task_id keeps the actually
/// admitted first task rather than selecting a convenient member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkflowFrozenCohortSnapshot {
    pub(crate) session_id: String,
    pub(crate) workflow_run_id: String,
    pub(crate) first_task_id: SchedulerTaskId,
    pub(crate) first_readiness_proof: DependencyReadinessProofEnvelope,
    pub(crate) population_fingerprint: String,
    pub(crate) members: Vec<WorkflowCohortMember>,
    pub(crate) identity: String,
}
impl WorkflowFrozenCohortSnapshot {
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn first_task_id(&self) -> &SchedulerTaskId {
        &self.first_task_id
    }
    pub fn members(&self) -> &[WorkflowCohortMember] {
        &self.members
    }
    pub fn first_readiness_proof(&self) -> &DependencyReadinessProofEnvelope {
        &self.first_readiness_proof
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCohortRefusal {
    NoActiveRun,
    PopulationLimit,
    SnapshotByteLimit,
    UnknownRecord,
    InvalidIdentity,
    UnsupportedTask,
    UnsupportedState,
    FirstNotAdmitted,
    InvalidDependencies,
    ExternalUnfinishedPrerequisite,
    UnknownInput,
    CleanupPending,
    PlacementUnavailable,
    IncompletePlacements,
    PlacementLimit,
    UnsupportedPlacement,
    OwnerEpochChanged,
    SnapshotChanged,
}

/// A trusted owner's live monotonic generation. The owner must invalidate it
/// on every placement/capability/residency change and must never reuse a value.
/// Holding this guard does not reserve anything or freeze the external owner.
#[derive(Debug, Clone)]
pub struct WorkflowCohortOwnerEpoch {
    owner_id: String,
    observed: u64,
    current: Arc<AtomicU64>,
}
impl WorkflowCohortOwnerEpoch {
    pub fn capture(owner_id: String, current: Arc<AtomicU64>) -> Option<Self> {
        if !bounded_label(&owner_id) {
            return None;
        }
        let observed = current.load(Ordering::Acquire);
        Some(Self {
            owner_id,
            observed,
            current,
        })
    }
    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }
    pub fn generation(&self) -> u64 {
        self.observed
    }
    pub fn is_current(&self) -> bool {
        self.current.load(Ordering::Acquire) == self.observed
    }
}

/// Placement identity only. Deliberately contains no lease, resource-fit claim,
/// successor Ready proof, cleanup acknowledgement, or executable candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkflowCohortPlacementDescriptor {
    pub placement_id: String,
    pub runtime_id: String,
    pub runtime_variant_id: Option<String>,
    pub device_id: String,
    pub artifact_fingerprint: String,
    pub runtime_instance_id: Option<String>,
}
#[derive(Debug, Clone)]
pub struct WorkflowCohortPlacement {
    pub descriptor: WorkflowCohortPlacementDescriptor,
    pub owner_epoch: WorkflowCohortOwnerEpoch,
}
#[derive(Debug)]
pub struct WorkflowCohortTaskPlacements {
    pub task_id: SchedulerTaskId,
    /// An explicit owner assertion that no supported alternative was omitted.
    pub complete: bool,
    pub placements: Vec<WorkflowCohortPlacement>,
}

/// Dedicated trusted native read interface, separate from the dispatch provider
/// whose candidate collection may create provisional reservations. Callbacks
/// must be bounded/nonblocking/read-only; no I/O, refresh, load or reservation.
pub trait WorkflowCohortCoverageProvider: Send + Sync {
    fn supports_cohort_coverage(&self) -> bool {
        false
    }
    /// Return all task groups (at most four), each with one or two complete,
    /// capability-valid placements. Never truncate to the limit.
    fn cohort_placements(
        &self,
        _snapshot: &WorkflowFrozenCohortSnapshot,
    ) -> Result<Vec<WorkflowCohortTaskPlacements>, WorkflowCohortRefusal> {
        Err(WorkflowCohortRefusal::PlacementUnavailable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCohortMissingQualification {
    Workload,
    SymbolicOutputShape,
    SetupTiming,
    TransferTiming,
    ExecutionTiming,
    CleanupTiming,
    RetentionTiming,
    ReloadTiming,
    ReleaseReconciliation,
    ConditionalCapacity,
}
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowCohortPlacementCoverage {
    pub task_id: SchedulerTaskId,
    pub placement: WorkflowCohortPlacementDescriptor,
    pub owner_id: String,
    pub owner_epoch: u64,
    pub missing: Vec<WorkflowCohortMissingQualification>,
}
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowCohortEvidenceCoverage {
    pub snapshot: WorkflowFrozenCohortSnapshot,
    pub placements: Vec<WorkflowCohortPlacementCoverage>,
    /// Always false in this slice: there is no trusted six-stage, path-conditioned
    /// native evidence producer. Existing coarse/CPU calibration is not imported.
    pub native_evidence_complete: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", content = "details", rename_all = "snake_case")]
pub enum WorkflowCohortCoverageResult {
    Disabled,
    Refused(WorkflowCohortRefusal),
    Incomplete(Box<WorkflowCohortEvidenceCoverage>),
}

impl WorkflowService {
    #[cfg(test)]
    pub(crate) fn cohort_store_is_unlocked_for_test(&self) -> bool {
        self.session_store.try_lock().is_ok()
    }
    /// Explicit constructor opt-in; does not install a selector or alter dispatch.
    pub fn new_with_cohort_coverage(provider: Arc<dyn WorkflowCohortCoverageProvider>) -> Self {
        let mut service = Self::new();
        service.cohort_coverage_provider = Some(provider);
        service
    }
    pub fn cohort_coverage_enabled(&self) -> bool {
        self.cohort_coverage_provider
            .as_ref()
            .is_some_and(|p| p.supports_cohort_coverage())
    }
    /// Inspect one current frozen run without admission, access ticks, source
    /// refresh, reservation, execution or changes to ordinary fallback dispatch.
    pub fn workflow_cohort_evidence_coverage(
        &self,
        session: &str,
        run: &str,
        first_task: &str,
    ) -> Result<WorkflowCohortCoverageResult, WorkflowServiceError> {
        let Some(provider) = self.cohort_coverage_provider.as_ref() else {
            return Ok(WorkflowCohortCoverageResult::Disabled);
        };
        // Even capability callbacks run outside the store guard.
        if !provider.supports_cohort_coverage() {
            return Ok(WorkflowCohortCoverageResult::Disabled);
        }
        let snapshot = match self
            .session_store_guard()?
            .frozen_cohort_snapshot(session, run, first_task)
        {
            Ok(snapshot) => snapshot,
            Err(reason) => return Ok(WorkflowCohortCoverageResult::Refused(reason)),
        };
        let groups = match provider.cohort_placements(&snapshot) {
            Ok(groups) => groups,
            Err(reason) => return Ok(WorkflowCohortCoverageResult::Refused(reason)),
        };
        if let Err(reason) = validate_placements(&snapshot, &groups) {
            return Ok(WorkflowCohortCoverageResult::Refused(reason));
        }
        // Re-read ALL members, descriptors, dependencies, proofs and input values
        // under one lock, then read live atomics without invoking provider code.
        {
            let store = self.session_store_guard()?;
            if store.validate_frozen_cohort_snapshot(&snapshot).is_err() {
                return Ok(WorkflowCohortCoverageResult::Refused(
                    WorkflowCohortRefusal::SnapshotChanged,
                ));
            }
            if groups
                .iter()
                .flat_map(|g| &g.placements)
                .any(|p| !p.owner_epoch.is_current())
            {
                return Ok(WorkflowCohortCoverageResult::Refused(
                    WorkflowCohortRefusal::OwnerEpochChanged,
                ));
            }
        }
        let mut placements =
            Vec::with_capacity(snapshot.members.len() * SCHEDULER_COHORT_MAX_PLACEMENTS);
        for member in snapshot.members() {
            let group = groups
                .iter()
                .find(|g| g.task_id == member.task.task_id)
                .expect("validated membership");
            for placement in &group.placements {
                use WorkflowCohortMissingQualification::*;
                let mut missing = vec![
                    Workload,
                    SetupTiming,
                    TransferTiming,
                    ExecutionTiming,
                    CleanupTiming,
                    RetentionTiming,
                    ReloadTiming,
                    ReleaseReconciliation,
                    ConditionalCapacity,
                ];
                if !member.symbolic_inputs.is_empty() {
                    missing.insert(1, SymbolicOutputShape);
                }
                placements.push(WorkflowCohortPlacementCoverage {
                    task_id: group.task_id.clone(),
                    placement: placement.descriptor.clone(),
                    owner_id: placement.owner_epoch.owner_id.clone(),
                    owner_epoch: placement.owner_epoch.observed,
                    missing,
                });
            }
        }
        placements.sort_by(|a, b| {
            (&a.task_id, &a.placement.placement_id).cmp(&(&b.task_id, &b.placement.placement_id))
        });
        let coverage = WorkflowCohortEvidenceCoverage {
            snapshot,
            placements,
            native_evidence_complete: false,
        };
        let mut output_budget = WORKFLOW_COHORT_MAX_SNAPSHOT_BYTES;
        if cohort_fingerprint(&coverage, &mut output_budget).is_err() {
            return Ok(WorkflowCohortCoverageResult::Refused(
                WorkflowCohortRefusal::SnapshotByteLimit,
            ));
        }
        Ok(WorkflowCohortCoverageResult::Incomplete(Box::new(coverage)))
    }
}

fn validate_placements(
    snapshot: &WorkflowFrozenCohortSnapshot,
    groups: &[WorkflowCohortTaskPlacements],
) -> Result<(), WorkflowCohortRefusal> {
    use WorkflowCohortRefusal::*;
    if groups.len() != snapshot.members.len() {
        return Err(IncompletePlacements);
    }
    for (index, group) in groups.iter().enumerate() {
        if !group.complete {
            return Err(IncompletePlacements);
        }
        if group.placements.is_empty() || group.placements.len() > SCHEDULER_COHORT_MAX_PLACEMENTS {
            return Err(PlacementLimit);
        }
        let Some(member) = snapshot
            .members
            .iter()
            .find(|m| m.task.task_id == group.task_id)
        else {
            return Err(IncompletePlacements);
        };
        if groups[..index].iter().any(|g| g.task_id == group.task_id) {
            return Err(IncompletePlacements);
        }
        let constraints = &member
            .task
            .schedulable_intent
            .as_ref()
            .expect("bounded runtime task")
            .constraints;
        for (i, placement) in group.placements.iter().enumerate() {
            let p = &placement.descriptor;
            if ![
                &p.placement_id,
                &p.runtime_id,
                &p.device_id,
                &p.artifact_fingerprint,
            ]
            .into_iter()
            .all(|s| bounded_label(s))
                || p.runtime_variant_id
                    .as_ref()
                    .is_some_and(|s| !bounded_label(s))
                || p.runtime_instance_id
                    .as_ref()
                    .is_some_and(|s| !bounded_label(s))
                || constraints
                    .requested_runtime_id
                    .as_ref()
                    .is_some_and(|id| id.as_str() != p.runtime_id)
                || constraints
                    .requested_device_id
                    .as_ref()
                    .is_some_and(|id| id.as_str() != p.device_id)
                || group.placements[..i].iter().any(|other| {
                    let other = &other.descriptor;
                    other.placement_id == p.placement_id
                        || (other.runtime_id == p.runtime_id
                            && other.runtime_variant_id == p.runtime_variant_id
                            && other.device_id == p.device_id
                            && other.artifact_fingerprint == p.artifact_fingerprint
                            && other.runtime_instance_id == p.runtime_instance_id)
                })
            {
                return Err(UnsupportedPlacement);
            }
        }
    }
    Ok(())
}
fn bounded_label(s: &str) -> bool {
    s.len() <= 128 && !s.trim().is_empty()
}

/// Streaming bound: neither hashing nor validation allocates serialized payloads.
/// One caller-owned budget charges each serialized byte before any payload clone.
pub(crate) fn cohort_fingerprint<T: Serialize>(
    value: &T,
    remaining: &mut usize,
) -> Result<String, WorkflowCohortRefusal> {
    struct Sink<'a> {
        remaining: &'a mut usize,
        hash: blake3::Hasher,
    }
    impl std::io::Write for Sink<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            *self.remaining = self
                .remaining
                .checked_sub(bytes.len())
                .ok_or_else(|| std::io::Error::other("cohort byte budget"))?;
            self.hash.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut sink = Sink {
        remaining,
        hash: blake3::Hasher::new(),
    };
    serde_json::to_writer(&mut sink, value)
        .map_err(|_| WorkflowCohortRefusal::SnapshotByteLimit)?;
    Ok(sink.hash.finalize().to_hex().to_string())
}

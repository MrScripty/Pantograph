//! Constructor-owned, opt-in serial CPU service ordering. Observations cover
//! actual host execution/drain and matched release, not preparation/load plans.
use super::{WorkflowSchedulerTask, WorkflowService, WorkflowServiceError};
use async_trait::async_trait;
use pantograph_runtime_host_contracts::*;
use pantograph_scheduler::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod grouping;

const MAX_LOOKAHEAD_DIAGNOSTIC_BYTES: usize = 8192;

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

#[derive(PartialEq, Eq)]
pub(crate) struct WorkflowSerialServiceAdmission {
    pub(crate) identity: (
        crate::scheduler::service_opportunities::ServiceOpportunityKey,
        i32,
    ),
    pub(crate) member: WorkflowSerialReadyMember,
}

#[derive(Clone, Copy)]
pub struct WorkflowSerialReadyConfig {
    minimum_samples: usize,
    maximum_age: Duration,
    completion_lookahead: bool,
    fair_service_opportunities: bool,
    serial_residency: bool,
    bounded_grouping: bool,
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
            completion_lookahead: false,
            fair_service_opportunities: false,
            serial_residency: false,
            bounded_grouping: false,
        })
    }
    /// Enable bounded predecessor-conditioned completion advice. Actual dispatch
    /// still follows the existing immediate serial Ready service policy.
    pub fn with_completion_lookahead(mut self) -> Self {
        self.completion_lookahead = true;
        self
    }
    /// Protect authenticated client/bucket units after two other-unit grants.
    /// This opt-in admits at most 64 Ready waiters across 32 units. Missing
    /// attributed-session identity and overflow refuse before runtime claims.
    /// Priorities still order ordinary opportunities; protected units may override
    /// them. Existing session admission boosts remain at their original boundary.
    /// A turn lasts through native drain/cleanup; no wall-time bound is promised.
    pub fn with_fair_service_opportunities(mut self) -> Self {
        self.fair_service_opportunities = true;
        self
    }
    /// Prepare the actual CPU resident owner after the authenticated fair grant
    /// and before task reservation/Start. Requires a native port with explicit
    /// load-only preparation and owned residency cleanup; ordinary ports refuse.
    pub fn with_serial_residency(mut self) -> Result<Self, WorkflowServiceError> {
        if !self.fair_service_opportunities {
            return Err(WorkflowServiceError::InvalidRequest(
                "serial residency requires fair service opportunities".into(),
            ));
        }
        self.serial_residency = true;
        Ok(self)
    }
    /// Reuse pure workload-key planning for up to eight bounded Ready members.
    /// Every member still obtains its own fair turn, preparation, Start and
    /// cleanup. This does not enable native batching or supply memory/latency
    /// forecasts. Unknown or changed inputs retain ordinary key calculation.
    pub fn with_bounded_grouping(mut self) -> Result<Self, WorkflowServiceError> {
        if !self.fair_service_opportunities {
            return Err(WorkflowServiceError::InvalidRequest(
                "bounded grouping requires fair service opportunities".into(),
            ));
        }
        self.bounded_grouping = true;
        Ok(self)
    }
}
impl Default for WorkflowSerialReadyConfig {
    fn default() -> Self {
        Self {
            minimum_samples: 3,
            maximum_age: Duration::from_secs(60),
            completion_lookahead: false,
            fair_service_opportunities: false,
            serial_residency: false,
            bounded_grouping: false,
        }
    }
}

pub(crate) struct SerialReadyMode {
    pub(crate) port: Arc<dyn SerialRuntimeHostBatchExecutionPort>,
    admission: SchedulerSerialAdmission,
    semaphore: Arc<tokio::sync::Semaphore>,
    active: Mutex<Option<Arc<Mutex<SerialReadyContext>>>>,
    history: Mutex<SerialReadyHistory>,
    config: WorkflowSerialReadyConfig,
    service_opportunities: Arc<super::serial_service_opportunities::SerialServiceOpportunities>,
    grouping: Mutex<grouping::WorkloadKeyGroup>,
    #[cfg(any(test, feature = "test-support"))]
    key_encodings: std::sync::atomic::AtomicU64,
    #[cfg(any(test, feature = "test-support"))]
    key_cache_hits: std::sync::atomic::AtomicU64,
}
#[derive(Default)]
struct SerialReadyHistory {
    samples: VecDeque<Observation>,
    predecessor: Option<OwnerBoundWorkload>,
}
struct OwnerBoundWorkload {
    key: [u8; 32],
    owner: SerialRuntimeHostCpuOwnerEvidence,
}
struct Observation {
    key: [u8; 32],
    owner: SerialRuntimeHostCpuOwnerEvidence,
    at: Instant,
    cost_ns: u64,
    predecessor_key: Option<[u8; 32]>,
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
/// Diagnostic projection only; no dispatch, lease, or owner capability.
#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SerialLookaheadReason {
    DiagnosticTooLarge,
    RemainingObligations,
    OwnerUnavailable,
    IncomparableOwners,
    UnsupportedInputs,
    HistoryUnavailable,
    PredecessorUnavailable,
    PredecessorOwnerChanged,
    InvalidClock,
    InsufficientFreshSamples,
    OwnerChanged,
    MissingEvidence,
    BudgetExhausted,
    DurationOverflow,
    Ranked,
}
struct SerialLookaheadDecision {
    reason: SerialLookaheadReason,
    workload_keys: Option<[[u8; 32]; 2]>,
    predecessor_key: Option<[u8; 32]>,
    owner_generation: Option<u64>,
    // Row order is [A|P, B|P], [B|A, A|B].
    counts: [[usize; 2]; 2],
    maxima_ns: [[u64; 2]; 2],
    oldest_age_ns: [[u64; 2]; 2],
    selected_position: Option<usize>,
    completions_ns: Option<[[u64; 2]; 2]>,
    completion_sums_ns: Option<[u64; 2]>,
    plans_evaluated: usize,
    completion_events_evaluated: usize,
}
impl SerialLookaheadDecision {
    fn refused(reason: SerialLookaheadReason) -> Self {
        Self {
            reason,
            workload_keys: None,
            predecessor_key: None,
            owner_generation: None,
            counts: [[0; 2]; 2],
            maxima_ns: [[0; 2]; 2],
            oldest_age_ns: [[0; 2]; 2],
            selected_position: None,
            completions_ns: None,
            completion_sums_ns: None,
            plans_evaluated: 0,
            completion_events_evaluated: 0,
        }
    }
}
pub(super) fn same_owner(
    a: &SerialRuntimeHostCpuOwnerEvidence,
    b: &SerialRuntimeHostCpuOwnerEvidence,
) -> bool {
    a.snapshot == b.snapshot && Arc::ptr_eq(&a.generation, &b.generation)
}
pub(crate) struct SerialReadyContext {
    preparation: Option<SchedulerSerialPreparation>,
    dispatch: Option<SchedulerSerialDispatch>,
    drained: Option<SchedulerSerialDrainedDispatch>,
    cleanup: Option<Box<dyn SerialRuntimeHostCleanupReceipt>>,
    claimed: bool,
    cleanup_acknowledged: bool,
    native_preparation_pending: bool,
    successful: bool,
    cancellation: Option<RuntimeHostExecutionCancellationHandle>,
    pub(crate) task: Option<WorkflowSchedulerTask>,
    member: Option<WorkflowSerialReadyMember>,
    pub(crate) owner: Option<SerialRuntimeHostCpuOwnerEvidence>,
    key: Option<[u8; 32]>,
    predecessor: Option<OwnerBoundWorkload>,
    started: Option<Instant>,
    attempt_id: String,
    candidate_id: String,
    request_id: String,
}
pub(crate) struct SerialReadyScope {
    mode: Arc<SerialReadyMode>,
    context: Arc<Mutex<SerialReadyContext>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
    // Field drop order releases the serial permit before waking a new fair turn.
    service_turn: Option<super::serial_service_opportunities::SerialServiceTurn>,
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
            history: Mutex::new(SerialReadyHistory::default()),
            config,
            service_opportunities: Arc::default(),
            grouping: Mutex::default(),
            #[cfg(any(test, feature = "test-support"))]
            key_encodings: std::sync::atomic::AtomicU64::new(0),
            #[cfg(any(test, feature = "test-support"))]
            key_cache_hits: std::sync::atomic::AtomicU64::new(0),
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
            native_preparation_pending: false,
            successful: false,
            cancellation: None,
            task: None,
            member: None,
            owner: None,
            key: None,
            predecessor: None,
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
            service_turn: None,
        })
    }
    pub(crate) fn fair_service_enabled(&self) -> bool {
        self.config.fair_service_opportunities
    }
    pub(crate) fn residency_enabled(&self) -> bool {
        self.config.serial_residency
    }
    pub(crate) async fn enter_fair(
        self: &Arc<Self>,
        key: crate::scheduler::service_opportunities::ServiceOpportunityKey,
        priority: i32,
    ) -> Result<SerialReadyScope, WorkflowServiceError> {
        let turn = self.service_opportunities.enter(key, priority).await?;
        let mut scope = self.enter().await?;
        scope.service_turn = Some(turn);
        Ok(scope)
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
        let immediate = self.select_immediate(pair);
        if self.config.completion_lookahead {
            let advisory = self.completion_lookahead(pair);
            // Projection and logger callbacks run after the history guard drops.
            let diagnostic = serde_json::json!({
                "phase": "completion_advisory",
                "workflow_run_id": pair.members[0].task.workflow_run_id.as_str(),
                "task_ids": pair.members.each_ref().map(|m| m.task.task_id.as_str()),
                "immediate_position": immediate.position,
                "reason": advisory.reason,
                "workload_keys": advisory.workload_keys,
                "predecessor_key": advisory.predecessor_key,
                "owner_generation": advisory.owner_generation,
                "first_counts": advisory.counts[0],
                "successor_counts": advisory.counts[1],
                "first_ns": advisory.maxima_ns[0],
                "successor_ns": advisory.maxima_ns[1],
                "first_oldest_age_ns": advisory.oldest_age_ns[0],
                "successor_oldest_age_ns": advisory.oldest_age_ns[1],
                "selected_position": advisory.selected_position,
                "completions_ns": advisory.completions_ns,
                "completion_sums_ns": advisory.completion_sums_ns,
                "plans_evaluated": advisory.plans_evaluated,
                "completion_events_evaluated": advisory.completion_events_evaluated,
            })
            .to_string();
            if diagnostic.len() <= MAX_LOOKAHEAD_DIAGNOSTIC_BYTES {
                log::info!(target: "workflow::serial_lookahead", "{}", diagnostic);
            } else {
                log::info!(target: "workflow::serial_lookahead", "{}",
                    serde_json::json!({"reason": SerialLookaheadReason::DiagnosticTooLarge}));
            }
        }
        immediate
    }
    fn select_immediate(&self, pair: &super::WorkflowSerialReadyPair) -> SerialReadyDecision {
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
            self.key_for(&pair.members[0]),
            self.key_for(&pair.members[1]),
        ) else {
            decision.reason = SerialReadyReason::UnsupportedInputs;
            return decision;
        };
        let keys = [first_key, second_key];
        let now = Instant::now();
        let Ok(history) = self.history.lock() else {
            decision.reason = SerialReadyReason::HistoryUnavailable;
            return decision;
        };
        for sample in history.samples.iter() {
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
    fn completion_lookahead(&self, pair: &WorkflowSerialReadyPair) -> SerialLookaheadDecision {
        use SerialLookaheadReason as Reason;
        let mut advice = SerialLookaheadDecision::refused(Reason::RemainingObligations);
        let remaining = pair
            .population
            .iter()
            .filter(|(_, class, state, _)| {
                *class == super::WorkflowSchedulerTaskExecutionClass::RuntimeInference
                    && !matches!(
                        state,
                        SchedulerTaskStateKind::Completed | SchedulerTaskStateKind::TerminalFailed
                    )
            })
            .count();
        if remaining != 2 {
            return advice;
        }
        let (Some(first), Some(second)) = (
            self.owner_for(&pair.members[0].task),
            self.owner_for(&pair.members[1].task),
        ) else {
            advice.reason = Reason::OwnerUnavailable;
            return advice;
        };
        if !same_owner(&first, &second) {
            advice.reason = Reason::IncomparableOwners;
            return advice;
        }
        let (Some(a), Some(b)) = (
            self.key_for(&pair.members[0]),
            self.key_for(&pair.members[1]),
        ) else {
            advice.reason = Reason::UnsupportedInputs;
            return advice;
        };
        advice.workload_keys = Some([a, b]);
        advice.owner_generation = Some(first.snapshot.generation);
        let now = Instant::now();
        let Ok(history) = self.history.lock() else {
            advice.reason = Reason::HistoryUnavailable;
            return advice;
        };
        let Some(previous) = history.predecessor.as_ref() else {
            advice.reason = Reason::PredecessorUnavailable;
            return advice;
        };
        if !same_owner(&previous.owner, &first) {
            advice.reason = Reason::PredecessorOwnerChanged;
            return advice;
        }
        advice.predecessor_key = Some(previous.key);
        let rows = [[(a, previous.key), (b, previous.key)], [(b, a), (a, b)]];
        for sample in history.samples.iter() {
            let Some(age) = now.checked_duration_since(sample.at) else {
                return SerialLookaheadDecision::refused(Reason::InvalidClock);
            };
            if !same_owner(&sample.owner, &first) || age > self.config.maximum_age {
                continue;
            }
            for (row, cells) in rows.iter().enumerate() {
                for (column, (key, predecessor)) in cells.iter().enumerate() {
                    if sample.key == *key && sample.predecessor_key == Some(*predecessor) {
                        advice.counts[row][column] += 1;
                        advice.maxima_ns[row][column] =
                            advice.maxima_ns[row][column].max(sample.cost_ns);
                        advice.oldest_age_ns[row][column] = advice.oldest_age_ns[row][column]
                            .max(age.as_nanos().min(u64::MAX as u128) as u64);
                    }
                }
            }
        }
        // Release state before the numeric sibling kernel and any diagnostics.
        drop(history);
        if advice
            .counts
            .iter()
            .flatten()
            .any(|c| *c < self.config.minimum_samples)
        {
            advice.reason = Reason::InsufficientFreshSamples;
            return advice;
        }
        if !first.is_current() || !second.is_current() {
            advice.reason = Reason::OwnerChanged;
            return advice;
        }
        match evaluate_serial_service_completion(
            advice.maxima_ns[0].map(Some),
            advice.maxima_ns[1].map(Some),
            SchedulerSerialServiceBudget {
                max_plans: 2,
                max_completion_events: 4,
            },
        ) {
            Ok(completion) => {
                advice.reason = Reason::Ranked;
                advice.selected_position = Some(completion.selected_position);
                advice.completions_ns = Some(completion.completions_ns);
                advice.completion_sums_ns = Some(completion.completion_sums_ns);
                advice.plans_evaluated = completion.plans_evaluated;
                advice.completion_events_evaluated = completion.completion_events_evaluated;
            }
            Err(refusal) => {
                advice.reason = match refusal {
                    SchedulerSerialServiceRefusal::MissingEvidence => Reason::MissingEvidence,
                    SchedulerSerialServiceRefusal::BudgetExhausted => Reason::BudgetExhausted,
                    SchedulerSerialServiceRefusal::DurationOverflow => Reason::DurationOverflow,
                };
            }
        }
        advice
    }
    fn record(
        &self,
        key: [u8; 32],
        owner: SerialRuntimeHostCpuOwnerEvidence,
        started: Instant,
        _task: &WorkflowSchedulerTask,
        predecessor_key: Option<[u8; 32]>,
        reuse_previous: Option<&SerialRuntimeHostCpuOwnerEvidence>,
    ) {
        let now = Instant::now();
        let Some(elapsed) = now.checked_duration_since(started) else {
            return;
        };
        let Ok(cost_ns) = u64::try_from(elapsed.as_nanos()) else {
            return;
        };
        let Ok(mut history) = self.history.lock() else {
            return;
        };
        // This is reached only after consumed actual receipt cleanup and a
        // fresh private-owner query. Never migrate from a forecast or reload.
        if let Some(previous) = reuse_previous.filter(|_| owner.is_current()) {
            for sample in history.samples.iter_mut() {
                if same_owner(&sample.owner, previous) {
                    sample.owner = owner.clone();
                }
            }
        }
        if self.config.completion_lookahead && owner.is_current() {
            history.predecessor = Some(OwnerBoundWorkload {
                key,
                owner: owner.clone(),
            });
        }
        let samples = &mut history.samples;
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
            predecessor_key,
            #[cfg(any(test, feature = "test-support"))]
            task_id: _task.task_id.to_string(),
        });
    }
}
impl SerialReadyScope {
    /// The owned fair turn and serial permit outlive every preparation await.
    /// A dropped/failed native operation cannot make uncertain work reusable.
    pub(crate) async fn prepare_residency(
        &self,
        member: &WorkflowSerialReadyMember,
    ) -> Result<SerialRuntimeHostCpuOwnerEvidence, WorkflowServiceError> {
        let intent = member.task.schedulable_intent.as_ref().ok_or_else(|| {
            WorkflowServiceError::InvalidRequest("serial preparation requires a task intent".into())
        })?;
        let request = SerialRuntimeHostPreparationRequest::new(
            &format!("serial-preparation.{}", uuid::Uuid::new_v4()),
            intent,
            &member.inputs,
        )
        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        self.context
            .lock()
            .map_err(lock_error)?
            .native_preparation_pending = true;
        let owner = self
            .mode
            .port
            .prepare_serial_cpu(request)
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        if !self
            .mode
            .owner_for(&member.task)
            .is_some_and(|actual| same_owner(&actual, &owner))
        {
            return Err(WorkflowServiceError::InvalidRequest(
                "serial preparation did not establish the current CPU owner".into(),
            ));
        }
        {
            let mut history = self.mode.history.lock().map_err(lock_error)?;
            if history
                .samples
                .iter()
                .any(|sample| !same_owner(&sample.owner, &owner))
                || history
                    .predecessor
                    .as_ref()
                    .is_some_and(|previous| !same_owner(&previous.owner, &owner))
            {
                history.samples.clear();
                history.predecessor = None;
            }
        }
        self.context
            .lock()
            .map_err(lock_error)?
            .native_preparation_pending = false;
        Ok(owner)
    }

    pub(crate) fn service_grant(
        &self,
    ) -> Option<crate::scheduler::service_opportunities::ServiceOpportunityGrant> {
        self.service_turn.as_ref()?.grant()
    }
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
        context.key = self.mode.key_for(member);
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
        let failed = match self.context.lock() {
            Ok(c) => {
                let failed = c.native_preparation_pending || (c.claimed && !c.cleanup_acknowledged);
                self.mode
                    .finish_grouping_member(if c.successful && !failed {
                        c.member.as_ref()
                    } else {
                        None
                    });
                failed
            }
            Err(_) => {
                self.mode.finish_grouping_member(None);
                true
            }
        };
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
                if self.0.compute_key(snapshot, &member.materialized_inputs) != Some(key) {
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
        if self.0.config.completion_lookahead {
            let mut attempt = context
                .lock()
                .map_err(|_| execution_error("serial context lock poisoned"))?;
            let mut history = self
                .0
                .history
                .lock()
                .map_err(|_| execution_error("serial history lock poisoned"))?;
            attempt.predecessor = history.predecessor.take();
        }
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
        // The consumed transition belongs to the owner that actually executed,
        // including the receipt's real pre-reuse generation. Never relabel it.
        if self.0.config.completion_lookahead {
            context.predecessor = context.predecessor.take().filter(|previous| {
                predecessor
                    .as_ref()
                    .or(actual_owner.as_ref())
                    .is_some_and(|actual| same_owner(&previous.owner, actual))
            });
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
                    c.predecessor.as_ref().map(|p| p.key),
                )
            })
        };
        let Some((
            drained,
            cleanup,
            expected,
            attempt,
            request_id,
            key,
            started,
            task,
            successful,
            predecessor_key,
        )) = data
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
                self.0.record(
                    key,
                    expected,
                    started,
                    &task,
                    predecessor_key,
                    predecessor.as_ref(),
                );
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
#[cfg(test)]
fn observation_key(member: &super::WorkflowSerialReadyMember) -> Option<[u8; 32]> {
    observation_key_with_inputs(member, &member.inputs, || {})
}
fn observation_key_with_inputs(
    member: &super::WorkflowSerialReadyMember,
    inputs: &[RuntimeHostExecutionInput],
    on_encode: impl FnOnce(),
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
    on_encode();
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

#[cfg(any(test, feature = "test-support"))]
type SerialReadyConditionedSample = (String, [u8; 32], Option<[u8; 32]>, u64, u64, u64);

impl WorkflowService {
    /// Whether the explicit constructor-owned mode prepares CPU residency under
    /// an authenticated fair turn. Retained-only serial modes remain distinct.
    pub fn is_serial_cpu_residency(&self) -> bool {
        self.serial_ready_mode
            .as_ref()
            .is_some_and(|mode| mode.residency_enabled())
    }

    /// Number of registered Ready waiters, excluding the active service turn.
    /// Read-only qualification observation; it grants no execution capability.
    #[cfg(any(test, feature = "test-support"))]
    pub fn serial_service_pending_for_test(&self) -> Result<usize, WorkflowServiceError> {
        match &self.serial_ready_mode {
            Some(mode) => mode.service_opportunities.pending_len(),
            None => Ok(0),
        }
    }
    /// Test-only acceptance evidence from the actual cleanup-qualified ring.
    #[cfg(any(test, feature = "test-support"))]
    pub fn serial_ready_observations_for_test(&self) -> Vec<(u64, usize)> {
        let Some(mode) = &self.serial_ready_mode else {
            return Vec::new();
        };
        let samples = mode.history.lock().expect("serial history lock");
        let mut counts = std::collections::BTreeMap::new();
        for sample in samples.samples.iter() {
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
        mode.history
            .lock()
            .expect("serial history lock")
            .samples
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
    /// Read-only transition evidence: task, workload digest, predecessor digest,
    /// observed cost ns, age ns, and scalar owner generation. No owner capability.
    #[cfg(any(test, feature = "test-support"))]
    pub fn serial_ready_conditioned_samples_for_test(&self) -> Vec<SerialReadyConditionedSample> {
        let Some(mode) = &self.serial_ready_mode else {
            return Vec::new();
        };
        let now = Instant::now();
        mode.history
            .lock()
            .expect("serial history lock")
            .samples
            .iter()
            .map(|sample| {
                (
                    sample.task_id.clone(),
                    sample.key,
                    sample.predecessor_key,
                    sample.cost_ns,
                    now.saturating_duration_since(sample.at)
                        .as_nanos()
                        .min(u64::MAX as u128) as u64,
                    sample.owner.snapshot.generation,
                )
            })
            .collect()
    }
    /// Read-only current predecessor digest and scalar owner association.
    #[cfg(any(test, feature = "test-support"))]
    pub fn serial_ready_predecessor_for_test(&self) -> Option<([u8; 32], u64)> {
        let mode = self.serial_ready_mode.as_ref()?;
        let history = mode.history.lock().expect("serial history lock");
        let previous = history.predecessor.as_ref()?;
        Some((previous.key, previous.owner.snapshot.generation))
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

    #[test]
    fn completion_lookahead_requires_explicit_construction() {
        assert!(!WorkflowSerialReadyConfig::default().completion_lookahead);
        assert!(
            !WorkflowSerialReadyConfig::new(3, Duration::from_secs(60))
                .unwrap()
                .completion_lookahead
        );
        assert!(
            WorkflowSerialReadyConfig::default()
                .with_completion_lookahead()
                .completion_lookahead
        );
    }

    #[test]
    fn serial_residency_requires_explicit_fair_service() {
        assert!(!WorkflowSerialReadyConfig::default().serial_residency);
        assert!(matches!(
            WorkflowSerialReadyConfig::default().with_serial_residency(),
            Err(WorkflowServiceError::InvalidRequest(message))
                if message == "serial residency requires fair service opportunities"
        ));
        assert!(
            WorkflowSerialReadyConfig::default()
                .with_fair_service_opportunities()
                .with_serial_residency()
                .unwrap()
                .serial_residency
        );
    }

    pub(in crate::workflow) fn assert_conditioned_hand_oracle_and_refusals(
        mode: &SerialReadyMode,
        pair: &WorkflowSerialReadyPair,
    ) {
        let owner = mode.owner_for(&pair.members[0].task).unwrap();
        let a = observation_key(&pair.members[0]).unwrap();
        let b = observation_key(&pair.members[1]).unwrap();
        let at = Instant::now();
        {
            let mut history = mode.history.lock().unwrap();
            history.predecessor = Some(OwnerBoundWorkload {
                key: a,
                owner: owner.clone(),
            });
            for (key, predecessor_key, cost_ns) in [(a, a, 7), (b, a, 3), (a, b, 20)] {
                for _ in 0..3 {
                    history.samples.push_back(Observation {
                        key,
                        predecessor_key: Some(predecessor_key),
                        owner: owner.clone(),
                        at,
                        cost_ns,
                        task_id: "private hand oracle".into(),
                    });
                }
            }
        }
        let immediate = mode.select(pair);
        assert_eq!(immediate.reason, SerialReadyReason::Ranked);
        assert_eq!(immediate.position, 1);
        assert_eq!(immediate.counts, [6, 3]);
        assert_eq!(immediate.maxima_ns, [20, 3]);
        let advice = mode.completion_lookahead(pair);
        assert_eq!(advice.reason, SerialLookaheadReason::Ranked);
        assert_eq!(advice.selected_position, Some(0));
        assert_eq!(advice.counts, [[3, 3], [3, 3]]);
        assert_eq!(advice.maxima_ns, [[7, 3], [3, 20]]);
        assert_eq!(advice.completions_ns, Some([[7, 10], [3, 23]]));
        assert_eq!(advice.completion_sums_ns, Some([17, 26]));
        assert_eq!(
            (advice.plans_evaluated, advice.completion_events_evaluated),
            (2, 4)
        );

        let mut blocked = pair.clone();
        blocked.population.push((
            "extra.blocked.runtime".into(),
            super::super::WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            SchedulerTaskStateKind::AwaitingInputs,
            0,
        ));
        let refusal = mode.completion_lookahead(&blocked);
        assert_eq!(refusal.reason, SerialLookaheadReason::RemainingObligations);
        assert!(refusal.selected_position.is_none());
        assert_eq!(mode.select(&blocked).position, immediate.position);
        blocked.population.last_mut().unwrap().2 = SchedulerTaskStateKind::Completed;
        assert_eq!(
            mode.completion_lookahead(&blocked).reason,
            SerialLookaheadReason::Ranked
        );

        // A sufficient unconditioned M1 population cannot replace a missing row.
        mode.history.lock().unwrap().samples[6].predecessor_key = None;
        assert_eq!(
            mode.completion_lookahead(pair).reason,
            SerialLookaheadReason::InsufficientFreshSamples
        );
        assert_eq!(mode.select(pair).position, immediate.position);
        mode.history.lock().unwrap().samples[6].predecessor_key = Some(b);
        mode.history.lock().unwrap().samples[6].at =
            at.checked_sub(Duration::from_secs(120)).unwrap();
        assert_eq!(
            mode.completion_lookahead(pair).reason,
            SerialLookaheadReason::InsufficientFreshSamples
        );
        mode.history.lock().unwrap().samples[6].at = at;
        mode.history.lock().unwrap().samples[6]
            .owner
            .snapshot
            .loaded_instance = [9; 16];
        assert_eq!(
            mode.completion_lookahead(pair).reason,
            SerialLookaheadReason::InsufficientFreshSamples
        );
        mode.history.lock().unwrap().samples[6].owner = owner.clone();
        mode.history.lock().unwrap().samples[6].owner.generation =
            Arc::new(std::sync::atomic::AtomicU64::new(owner.snapshot.generation));
        assert_eq!(
            mode.completion_lookahead(pair).reason,
            SerialLookaheadReason::InsufficientFreshSamples
        );
        mode.history.lock().unwrap().samples[6].owner = owner.clone();

        mode.history
            .lock()
            .unwrap()
            .predecessor
            .as_mut()
            .unwrap()
            .owner
            .snapshot
            .generation += 2;
        assert_eq!(
            mode.completion_lookahead(pair).reason,
            SerialLookaheadReason::PredecessorOwnerChanged
        );
        mode.history.lock().unwrap().predecessor = None;
        assert_eq!(
            mode.completion_lookahead(pair).reason,
            SerialLookaheadReason::PredecessorUnavailable
        );
        mode.history.lock().unwrap().predecessor = Some(OwnerBoundWorkload { key: a, owner });
        mode.history.lock().unwrap().samples[0].cost_ns = u64::MAX;
        let refusal = mode.completion_lookahead(pair);
        assert_eq!(refusal.reason, SerialLookaheadReason::DurationOverflow);
        assert!(refusal.selected_position.is_none());
        assert!(refusal.completions_ns.is_none());
        assert_eq!(
            (refusal.plans_evaluated, refusal.completion_events_evaluated),
            (0, 0)
        );
        for sample in mode.history.lock().unwrap().samples.iter_mut() {
            sample.cost_ns = 7;
        }
        let tied = mode.completion_lookahead(pair);
        assert_eq!(tied.reason, SerialLookaheadReason::Ranked);
        assert_eq!(tied.selected_position, Some(0));
        assert_eq!(tied.completions_ns, Some([[7, 14], [7, 14]]));
        mode.history.lock().unwrap().samples[0].at =
            at.checked_add(Duration::from_secs(3600)).unwrap();
        let refusal = mode.completion_lookahead(pair);
        assert_eq!(refusal.reason, SerialLookaheadReason::InvalidClock);
        assert_eq!(refusal.counts, [[0; 2]; 2]);
        assert!(refusal.selected_position.is_none());
    }

    pub(in crate::workflow) fn assert_real_conditioned_advice(
        mode: &SerialReadyMode,
        pair: &WorkflowSerialReadyPair,
    ) {
        let advice = mode.completion_lookahead(pair);
        assert_eq!(advice.reason, SerialLookaheadReason::Ranked);
        assert_eq!(advice.counts, [[3, 3], [3, 3]]);
        let a = observation_key(&pair.members[0]).unwrap();
        assert_eq!(advice.predecessor_key, Some(a));
        assert_eq!(
            mode.select(pair).position,
            mode.select_immediate(pair).position
        );
    }

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
            let mut samples = mode.history.lock().unwrap();
            let previous_len = samples.samples.len();
            let first = samples.samples.front().unwrap();
            let future = Observation {
                key: first.key,
                owner: first.owner.clone(),
                at: Instant::now()
                    .checked_add(Duration::from_secs(3600))
                    .unwrap(),
                cost_ns: first.cost_ns,
                predecessor_key: first.predecessor_key,
                task_id: first.task_id.clone(),
            };
            samples.samples.push_back(future);
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
            let mut samples = mode.history.lock().unwrap();
            assert_eq!(samples.samples.len(), previous_len + 1);
            samples.samples.pop_back();
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

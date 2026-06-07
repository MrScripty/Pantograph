use std::collections::BTreeMap;

use uuid::Uuid;

use super::WorkflowOutputTarget;

pub(super) const WORKFLOW_RUNTIME_BRANCH_TASK_EVENT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventClaimOwnerId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventClaimLeaseId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventRequest {
    pub(super) event_id: WorkflowRuntimeBranchTaskEventId,
    pub(super) session_id: String,
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) scheduler_task_attempt_id: Option<String>,
    pub(super) attempt_generation: u64,
    pub(super) queued_input_keys: Vec<String>,
    pub(super) output_targets: Option<Vec<WorkflowOutputTarget>>,
    pub(super) timeout_ms: Option<u64>,
    pub(super) batching_key: Option<String>,
    pub(super) ready_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventRecord {
    pub(super) schema_version: u16,
    pub(super) event_id: WorkflowRuntimeBranchTaskEventId,
    pub(super) session_id: String,
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) scheduler_task_attempt_id: Option<String>,
    pub(super) attempt_generation: u64,
    pub(super) queued_input_keys: Vec<String>,
    pub(super) output_targets: Option<Vec<WorkflowOutputTarget>>,
    pub(super) timeout_ms: Option<u64>,
    pub(super) batching_key: Option<String>,
    pub(super) state: WorkflowRuntimeBranchTaskEventState,
    pub(super) claim: Option<WorkflowRuntimeBranchTaskEventClaim>,
    pub(super) ready_at_ms: u64,
    pub(super) completed_at_ms: Option<u64>,
    pub(super) deferred_at_ms: Option<u64>,
    pub(super) failed_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventClaim {
    pub(super) owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
    pub(super) lease_id: WorkflowRuntimeBranchTaskEventClaimLeaseId,
    pub(super) attempt_generation: u64,
    pub(super) claimed_at_ms: u64,
    pub(super) lease_expires_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeBranchTaskEventState {
    Ready,
    Claimed,
    Completed,
    Deferred,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventDiagnostic {
    pub(super) code: WorkflowRuntimeBranchTaskEventDiagnosticCode,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeBranchTaskEventDiagnosticCode {
    InvalidEvent,
    DuplicateEvent,
    EventNotFound,
    AlreadyClaimed,
    LeaseExpired,
    MissingClaim,
    StaleClaim,
    TerminalEvent,
    InvalidTransition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventClaimOutcome {
    pub(super) record: WorkflowRuntimeBranchTaskEventRecord,
    pub(super) claim: WorkflowRuntimeBranchTaskEventClaim,
}

pub(super) trait WorkflowRuntimeBranchTaskEventRepository {
    fn enqueue(
        &mut self,
        record: WorkflowRuntimeBranchTaskEventRecord,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn claim_event(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventClaimOutcome, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn claim_next_due(
        &mut self,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<
        Option<WorkflowRuntimeBranchTaskEventClaimOutcome>,
        WorkflowRuntimeBranchTaskEventDiagnostic,
    >;

    fn claim_next_due_for_workflow_run(
        &mut self,
        workflow_run_id: &str,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<
        Option<WorkflowRuntimeBranchTaskEventClaimOutcome>,
        WorkflowRuntimeBranchTaskEventDiagnostic,
    >;

    fn complete(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        completed_at_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn defer(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        deferred_at_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn release_claim(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        ready_at_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn fail(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        failed_at_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn get(
        &self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
    ) -> Option<WorkflowRuntimeBranchTaskEventRecord>;
}

#[derive(Debug, Default)]
#[must_use]
pub(super) struct InMemoryWorkflowRuntimeBranchTaskEventRepository {
    records: BTreeMap<String, WorkflowRuntimeBranchTaskEventRecord>,
}

impl InMemoryWorkflowRuntimeBranchTaskEventRepository {
    pub(super) fn new() -> Self {
        Self::default()
    }
}

impl WorkflowRuntimeBranchTaskEventId {
    pub(super) fn new() -> Self {
        Self(format!("runtime-branch-task-event.{}", Uuid::new_v4()))
    }

    pub(super) fn parse(
        value: impl Into<String>,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        parse_non_blank(value, "runtime branch task event id").map(Self)
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl WorkflowRuntimeBranchTaskEventClaimOwnerId {
    pub(super) fn parse(
        value: impl Into<String>,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        parse_non_blank(value, "runtime branch task event claim owner id").map(Self)
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl WorkflowRuntimeBranchTaskEventClaimLeaseId {
    fn new() -> Self {
        Self(format!(
            "runtime-branch-task-event-claim.{}",
            Uuid::new_v4()
        ))
    }

    pub(super) fn parse(
        value: impl Into<String>,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        parse_non_blank(value, "runtime branch task event claim lease id").map(Self)
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl WorkflowRuntimeBranchTaskEventRepository for InMemoryWorkflowRuntimeBranchTaskEventRepository {
    fn enqueue(
        &mut self,
        record: WorkflowRuntimeBranchTaskEventRecord,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
        let event_id = record.event_id.as_str().to_string();
        if self.records.contains_key(&event_id) {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::DuplicateEvent,
                "runtime branch task event is already enqueued",
            ));
        }
        self.records.insert(event_id, record);
        Ok(())
    }

    fn claim_event(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventClaimOutcome, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let outcome = record.claim(owner_id, now_ms, lease_duration_ms)?;
        self.records
            .insert(event_id.as_str().to_string(), outcome.record.clone());
        Ok(outcome)
    }

    fn claim_next_due(
        &mut self,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<
        Option<WorkflowRuntimeBranchTaskEventClaimOutcome>,
        WorkflowRuntimeBranchTaskEventDiagnostic,
    > {
        let Some(event_id) = self.next_due_event_id(now_ms) else {
            return Ok(None);
        };
        self.claim_event(&event_id, owner_id, now_ms, lease_duration_ms)
            .map(Some)
    }

    fn claim_next_due_for_workflow_run(
        &mut self,
        workflow_run_id: &str,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<
        Option<WorkflowRuntimeBranchTaskEventClaimOutcome>,
        WorkflowRuntimeBranchTaskEventDiagnostic,
    > {
        let Some(event_id) = self.next_due_event_id_for_workflow_run(workflow_run_id, now_ms)
        else {
            return Ok(None);
        };
        self.claim_event(&event_id, owner_id, now_ms, lease_duration_ms)
            .map(Some)
    }

    fn complete(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        completed_at_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let updated = record.complete(claim, completed_at_ms)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn defer(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        deferred_at_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let updated = record.defer(claim, deferred_at_ms)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn fail(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        failed_at_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let updated = record.fail(claim, failed_at_ms)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn release_claim(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        ready_at_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let updated = record.release_claim(claim, ready_at_ms)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn get(
        &self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
    ) -> Option<WorkflowRuntimeBranchTaskEventRecord> {
        self.records.get(event_id.as_str()).cloned()
    }
}

impl InMemoryWorkflowRuntimeBranchTaskEventRepository {
    fn record(
        &self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        self.records.get(event_id.as_str()).cloned().ok_or_else(|| {
            WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::EventNotFound,
                "runtime branch task event was not found",
            )
        })
    }

    fn next_due_event_id(&self, now_ms: u64) -> Option<WorkflowRuntimeBranchTaskEventId> {
        self.records
            .values()
            .filter(|record| record.is_due_for_claim(now_ms))
            .min_by(|left, right| {
                left.ready_at_ms
                    .cmp(&right.ready_at_ms)
                    .then_with(|| left.event_id.as_str().cmp(right.event_id.as_str()))
            })
            .map(|record| record.event_id.clone())
    }

    fn next_due_event_id_for_workflow_run(
        &self,
        workflow_run_id: &str,
        now_ms: u64,
    ) -> Option<WorkflowRuntimeBranchTaskEventId> {
        self.records
            .values()
            .filter(|record| record.workflow_run_id == workflow_run_id)
            .filter(|record| record.is_due_for_claim(now_ms))
            .min_by(|left, right| {
                left.ready_at_ms
                    .cmp(&right.ready_at_ms)
                    .then_with(|| left.event_id.as_str().cmp(right.event_id.as_str()))
            })
            .map(|record| record.event_id.clone())
    }
}

impl WorkflowRuntimeBranchTaskEventRecord {
    pub(super) fn ready(
        request: WorkflowRuntimeBranchTaskEventRequest,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        validate_request(&request)?;
        Ok(Self {
            schema_version: WORKFLOW_RUNTIME_BRANCH_TASK_EVENT_SCHEMA_VERSION,
            event_id: request.event_id,
            session_id: request.session_id,
            workflow_id: request.workflow_id,
            workflow_run_id: request.workflow_run_id,
            scheduler_task_id: request.scheduler_task_id,
            scheduler_task_attempt_id: request.scheduler_task_attempt_id,
            attempt_generation: request.attempt_generation,
            queued_input_keys: request.queued_input_keys,
            output_targets: request.output_targets,
            timeout_ms: request.timeout_ms,
            batching_key: request.batching_key,
            state: WorkflowRuntimeBranchTaskEventState::Ready,
            claim: None,
            ready_at_ms: request.ready_at_ms,
            completed_at_ms: None,
            deferred_at_ms: None,
            failed_at_ms: None,
        })
    }

    pub(super) fn claim(
        mut self,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventClaimOutcome, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        if lease_duration_ms == 0 {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
                "runtime branch task event claim lease duration must be greater than zero",
            ));
        }

        match self.state {
            WorkflowRuntimeBranchTaskEventState::Ready => {}
            WorkflowRuntimeBranchTaskEventState::Claimed => {
                let claim = self.claim.as_ref().ok_or_else(|| {
                    WorkflowRuntimeBranchTaskEventDiagnostic::new(
                        WorkflowRuntimeBranchTaskEventDiagnosticCode::MissingClaim,
                        "claimed runtime branch task event is missing claim details",
                    )
                })?;
                if now_ms < claim.lease_expires_at_ms {
                    return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                        WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed,
                        "runtime branch task event is already claimed by an active lease",
                    ));
                }
                self.attempt_generation = self.attempt_generation.saturating_add(1);
            }
            WorkflowRuntimeBranchTaskEventState::Deferred => {
                if self.ready_at_ms > now_ms {
                    return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                        WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed,
                        "deferred runtime branch task event is not ready for retry",
                    ));
                }
                self.attempt_generation = self.attempt_generation.saturating_add(1);
            }
            WorkflowRuntimeBranchTaskEventState::Completed
            | WorkflowRuntimeBranchTaskEventState::Failed => {
                return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                    WorkflowRuntimeBranchTaskEventDiagnosticCode::TerminalEvent,
                    "terminal runtime branch task event cannot be claimed",
                ));
            }
        }

        let claim = WorkflowRuntimeBranchTaskEventClaim {
            owner_id,
            lease_id: WorkflowRuntimeBranchTaskEventClaimLeaseId::new(),
            attempt_generation: self.attempt_generation,
            claimed_at_ms: now_ms,
            lease_expires_at_ms: now_ms.saturating_add(lease_duration_ms),
        };
        self.state = WorkflowRuntimeBranchTaskEventState::Claimed;
        self.claim = Some(claim.clone());
        Ok(WorkflowRuntimeBranchTaskEventClaimOutcome {
            record: self,
            claim,
        })
    }

    pub(super) fn complete(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        completed_at_ms: u64,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim(claim, completed_at_ms)?;
        self.state = WorkflowRuntimeBranchTaskEventState::Completed;
        self.completed_at_ms = Some(completed_at_ms);
        Ok(self)
    }

    pub(super) fn defer(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        deferred_at_ms: u64,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim(claim, deferred_at_ms)?;
        self.state = WorkflowRuntimeBranchTaskEventState::Deferred;
        self.claim = None;
        self.ready_at_ms = deferred_at_ms;
        self.deferred_at_ms = Some(deferred_at_ms);
        Ok(self)
    }

    pub(super) fn fail(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        failed_at_ms: u64,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim(claim, failed_at_ms)?;
        self.state = WorkflowRuntimeBranchTaskEventState::Failed;
        self.failed_at_ms = Some(failed_at_ms);
        Ok(self)
    }

    pub(super) fn release_claim(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        ready_at_ms: u64,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim(claim, ready_at_ms)?;
        self.state = WorkflowRuntimeBranchTaskEventState::Ready;
        self.claim = None;
        self.ready_at_ms = ready_at_ms;
        Ok(self)
    }

    fn validate_active_claim(
        &self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        now_ms: u64,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
        if self.state != WorkflowRuntimeBranchTaskEventState::Claimed {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition,
                "runtime branch task event must be claimed before terminal transition",
            ));
        }
        let current = self.claim.as_ref().ok_or_else(|| {
            WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::MissingClaim,
                "claimed runtime branch task event is missing claim details",
            )
        })?;
        if current != claim {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim,
                "runtime branch task event claim does not match the current lease",
            ));
        }
        if now_ms >= current.lease_expires_at_ms {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::LeaseExpired,
                "runtime branch task event claim lease expired before terminal transition",
            ));
        }
        Ok(())
    }

    fn is_due_for_claim(&self, now_ms: u64) -> bool {
        match self.state {
            WorkflowRuntimeBranchTaskEventState::Ready => self.ready_at_ms <= now_ms,
            WorkflowRuntimeBranchTaskEventState::Deferred => self.ready_at_ms <= now_ms,
            WorkflowRuntimeBranchTaskEventState::Claimed => self
                .claim
                .as_ref()
                .is_none_or(|claim| claim.lease_expires_at_ms <= now_ms),
            WorkflowRuntimeBranchTaskEventState::Completed
            | WorkflowRuntimeBranchTaskEventState::Failed => false,
        }
    }
}

impl WorkflowRuntimeBranchTaskEventDiagnostic {
    fn new(code: WorkflowRuntimeBranchTaskEventDiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

fn validate_request(
    request: &WorkflowRuntimeBranchTaskEventRequest,
) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
    validate_non_blank("session id", &request.session_id)?;
    validate_non_blank("workflow id", &request.workflow_id)?;
    validate_non_blank("workflow run id", &request.workflow_run_id)?;
    validate_non_blank("scheduler task id", &request.scheduler_task_id)?;
    if let Some(scheduler_task_attempt_id) = &request.scheduler_task_attempt_id {
        validate_non_blank("scheduler task attempt id", scheduler_task_attempt_id)?;
    }
    for queued_input_key in &request.queued_input_keys {
        validate_non_blank("queued input key", queued_input_key)?;
    }
    if let Some(batching_key) = &request.batching_key {
        validate_non_blank("runtime branch task event batching key", batching_key)?;
    }
    Ok(())
}

fn parse_non_blank(
    value: impl Into<String>,
    label: &str,
) -> Result<String, WorkflowRuntimeBranchTaskEventDiagnostic> {
    let value = value.into();
    validate_non_blank(label, &value)?;
    Ok(value)
}

fn validate_non_blank(
    label: &str,
    value: &str,
) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
    if value.trim().is_empty() {
        return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
            format!("{label} must not be blank"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_branch_task_event_starts_ready_with_stable_contract_fields() {
        let record = ready_record();

        assert_eq!(
            record.schema_version,
            WORKFLOW_RUNTIME_BRANCH_TASK_EVENT_SCHEMA_VERSION
        );
        assert_eq!(record.event_id.as_str(), "runtime-branch-task-event.test");
        assert_eq!(record.session_id, "session.test");
        assert_eq!(record.workflow_id, "workflow.image");
        assert_eq!(record.workflow_run_id, "run.test");
        assert_eq!(record.scheduler_task_id, "image-task");
        assert_eq!(
            record.scheduler_task_attempt_id.as_deref(),
            Some("attempt.1")
        );
        assert_eq!(record.attempt_generation, 1);
        assert_eq!(record.queued_input_keys, vec!["prompt".to_string()]);
        assert_eq!(record.timeout_ms, Some(30_000));
        assert_eq!(
            record.batching_key.as_deref(),
            Some("runtime.diffusers.cuda0")
        );
        assert_eq!(record.state, WorkflowRuntimeBranchTaskEventState::Ready);
        assert!(record.claim.is_none());
    }

    #[test]
    fn runtime_branch_task_event_rejects_blank_identity_fields() {
        let mut request = ready_request();
        request.workflow_run_id = " ".to_string();

        let error = WorkflowRuntimeBranchTaskEventRecord::ready(request)
            .expect_err("blank run id must fail");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent
        );
        assert!(error.message.contains("workflow run id"));
    }

    #[test]
    fn runtime_branch_task_event_claims_ready_event_with_lease() {
        let owner = owner_id("worker.alpha");
        let outcome = ready_record()
            .claim(owner.clone(), 100, 50)
            .expect("ready event claims");

        assert_eq!(
            outcome.record.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
        );
        assert_eq!(outcome.claim.owner_id, owner);
        assert_eq!(outcome.claim.attempt_generation, 1);
        assert_eq!(outcome.claim.claimed_at_ms, 100);
        assert_eq!(outcome.claim.lease_expires_at_ms, 150);
        assert_eq!(
            outcome.record.claim.as_ref().expect("stored claim"),
            &outcome.claim
        );
    }

    #[test]
    fn runtime_branch_task_event_rejects_duplicate_active_claim() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims")
            .record;

        let error = claimed
            .claim(owner_id("worker.beta"), 149, 50)
            .expect_err("active lease blocks duplicate claim");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed
        );
    }

    #[test]
    fn runtime_branch_task_event_reclaims_after_lease_expiry_with_new_generation() {
        let first = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");

        let second = first
            .record
            .claim(owner_id("worker.beta"), 150, 60)
            .expect("expired lease reclaims");

        assert_eq!(
            second.record.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
        );
        assert_eq!(second.record.attempt_generation, 2);
        assert_eq!(second.claim.attempt_generation, 2);
        assert_eq!(second.claim.owner_id.as_str(), "worker.beta");
        assert_ne!(second.claim.lease_id, first.claim.lease_id);
    }

    #[test]
    fn runtime_branch_task_event_completes_only_with_current_claim() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");

        let completed = claimed
            .record
            .complete(&claimed.claim, 120)
            .expect("current claim completes");

        assert_eq!(
            completed.state,
            WorkflowRuntimeBranchTaskEventState::Completed
        );
        assert_eq!(completed.completed_at_ms, Some(120));
    }

    #[test]
    fn runtime_branch_task_event_rejects_stale_claim_terminal_transition() {
        let first = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");
        let second = first
            .record
            .claim(owner_id("worker.beta"), 150, 60)
            .expect("expired lease reclaims");

        let error = second
            .record
            .complete(&first.claim, 160)
            .expect_err("stale claim cannot complete");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim
        );
    }

    #[test]
    fn runtime_branch_task_event_rejects_expired_claim_terminal_transition() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");

        let error = claimed
            .record
            .complete(&claimed.claim, 150)
            .expect_err("expired claim cannot complete");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::LeaseExpired
        );
    }

    #[test]
    fn runtime_branch_task_event_records_deferred_retry_and_failed_terminal_states() {
        let deferred_claim = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");
        let deferred = deferred_claim
            .record
            .defer(&deferred_claim.claim, 120)
            .expect("current claim defers");
        assert_eq!(
            deferred.state,
            WorkflowRuntimeBranchTaskEventState::Deferred
        );
        assert_eq!(deferred.deferred_at_ms, Some(120));
        assert_eq!(deferred.ready_at_ms, 120);
        assert!(deferred.claim.is_none());

        let not_due = deferred
            .clone()
            .claim(owner_id("worker.beta"), 119, 50)
            .expect_err("deferred event is not due before ready_at");
        assert_eq!(
            not_due.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed
        );
        let retry = deferred
            .claim(owner_id("worker.beta"), 120, 50)
            .expect("deferred event reclaims when due");
        assert_eq!(
            retry.record.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
        );
        assert_eq!(retry.claim.attempt_generation, 2);

        let failed_claim = ready_record()
            .claim(owner_id("worker.beta"), 200, 50)
            .expect("ready event claims");
        let failed = failed_claim
            .record
            .fail(&failed_claim.claim, 220)
            .expect("current claim fails");
        assert_eq!(failed.state, WorkflowRuntimeBranchTaskEventState::Failed);
        assert_eq!(failed.failed_at_ms, Some(220));
    }

    #[test]
    fn runtime_branch_task_event_releases_claim_back_to_ready() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");

        let released = claimed
            .record
            .release_claim(&claimed.claim, 120)
            .expect("current claim releases");

        assert_eq!(released.state, WorkflowRuntimeBranchTaskEventState::Ready);
        assert!(released.claim.is_none());
        assert_eq!(released.ready_at_ms, 120);
        assert_eq!(released.deferred_at_ms, None);
        assert_eq!(released.failed_at_ms, None);
        assert_eq!(released.completed_at_ms, None);
    }

    #[test]
    fn runtime_branch_task_event_rejects_claim_after_terminal_state() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");
        let completed = claimed
            .record
            .complete(&claimed.claim, 120)
            .expect("current claim completes");

        let error = completed
            .claim(owner_id("worker.beta"), 130, 50)
            .expect_err("terminal event cannot be claimed");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::TerminalEvent
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_enqueues_and_claims_due_event() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        repository.enqueue(ready_record()).expect("event enqueues");

        let outcome = repository
            .claim_next_due(owner_id("worker.alpha"), 42, 50)
            .expect("claim next succeeds")
            .expect("due event exists");

        assert_eq!(
            outcome.record.event_id.as_str(),
            "runtime-branch-task-event.test"
        );
        assert_eq!(
            outcome.record.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
        );
        let stored = repository
            .get(&outcome.record.event_id)
            .expect("claimed event is stored");
        assert_eq!(stored.claim.as_ref(), Some(&outcome.claim));
    }

    #[test]
    fn runtime_branch_task_event_repository_rejects_duplicate_enqueue() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        repository.enqueue(ready_record()).expect("event enqueues");

        let error = repository
            .enqueue(ready_record())
            .expect_err("duplicate event fails");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::DuplicateEvent
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_returns_none_without_due_event() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let mut request = ready_request();
        request.ready_at_ms = 200;
        repository
            .enqueue(
                WorkflowRuntimeBranchTaskEventRecord::ready(request).expect("future event ready"),
            )
            .expect("event enqueues");

        let claimed = repository
            .claim_next_due(owner_id("worker.alpha"), 199, 50)
            .expect("claim next succeeds");

        assert!(claimed.is_none());
    }

    #[test]
    fn runtime_branch_task_event_repository_claims_next_due_for_workflow_run_only() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        repository
            .enqueue(ready_record_with_id_and_run(
                "runtime-branch-task-event.other",
                "run.other",
                10,
            ))
            .expect("other event enqueues");
        repository
            .enqueue(ready_record_with_id_and_run(
                "runtime-branch-task-event.target",
                "run.target",
                20,
            ))
            .expect("target event enqueues");

        let claimed = repository
            .claim_next_due_for_workflow_run("run.target", owner_id("worker.alpha"), 25, 50)
            .expect("claim next succeeds")
            .expect("target event is due");

        assert_eq!(
            claimed.record.event_id.as_str(),
            "runtime-branch-task-event.target"
        );
        assert_eq!(
            repository
                .get(&event_id("runtime-branch-task-event.other"))
                .expect("other event")
                .state,
            WorkflowRuntimeBranchTaskEventState::Ready
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_reclaims_expired_claim() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let first = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 50)
            .expect("event claims");

        let second = repository
            .claim_next_due(owner_id("worker.beta"), 150, 60)
            .expect("claim next succeeds")
            .expect("expired event is due");

        assert_eq!(second.record.attempt_generation, 2);
        assert_eq!(second.claim.owner_id.as_str(), "worker.beta");
        assert_ne!(second.claim.lease_id, first.claim.lease_id);
    }

    #[test]
    fn runtime_branch_task_event_repository_persists_terminal_completion() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let claimed = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 50)
            .expect("event claims");

        let completed = repository
            .complete(&event_id, &claimed.claim, 120)
            .expect("event completes");

        assert_eq!(
            completed.state,
            WorkflowRuntimeBranchTaskEventState::Completed
        );
        assert_eq!(completed.completed_at_ms, Some(120));
        assert_eq!(
            repository.get(&event_id).expect("stored event").state,
            WorkflowRuntimeBranchTaskEventState::Completed
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_does_not_mutate_on_stale_terminal_claim() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let first = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 50)
            .expect("event claims");
        let second = repository
            .claim_event(&event_id, owner_id("worker.beta"), 150, 60)
            .expect("expired event reclaims");

        let error = repository
            .complete(&event_id, &first.claim, 160)
            .expect_err("stale claim cannot complete");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim
        );
        let stored = repository.get(&event_id).expect("stored event");
        assert_eq!(stored.claim.as_ref(), Some(&second.claim));
        assert_eq!(stored.state, WorkflowRuntimeBranchTaskEventState::Claimed);
    }

    #[test]
    fn runtime_branch_task_event_repository_persists_deferred_and_failed_events() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let deferred_id = event_id("runtime-branch-task-event.defer");
        repository
            .enqueue(ready_record_with_id("runtime-branch-task-event.defer"))
            .expect("deferred event enqueues");
        let deferred_claim = repository
            .claim_event(&deferred_id, owner_id("worker.alpha"), 100, 50)
            .expect("event claims");
        let deferred = repository
            .defer(&deferred_id, &deferred_claim.claim, 120)
            .expect("event defers");
        assert_eq!(
            deferred.state,
            WorkflowRuntimeBranchTaskEventState::Deferred
        );
        assert!(deferred.claim.is_none());
        assert_eq!(
            repository
                .claim_next_due_for_workflow_run("run.test", owner_id("worker.retry"), 119, 50)
                .expect("claim next succeeds"),
            None
        );
        let retry_claim = repository
            .claim_next_due_for_workflow_run("run.test", owner_id("worker.retry"), 120, 50)
            .expect("claim next succeeds")
            .expect("deferred event is due");
        assert_eq!(retry_claim.record.event_id, deferred_id);
        assert_eq!(retry_claim.claim.attempt_generation, 2);

        let failed_id = event_id("runtime-branch-task-event.fail");
        repository
            .enqueue(ready_record_with_id("runtime-branch-task-event.fail"))
            .expect("failed event enqueues");
        let failed_claim = repository
            .claim_event(&failed_id, owner_id("worker.beta"), 200, 50)
            .expect("event claims");
        let failed = repository
            .fail(&failed_id, &failed_claim.claim, 220)
            .expect("event fails");
        assert_eq!(failed.state, WorkflowRuntimeBranchTaskEventState::Failed);
    }

    #[test]
    fn runtime_branch_task_event_repository_releases_claimed_event_to_ready() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let claimed = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 50)
            .expect("event claims");

        let released = repository
            .release_claim(&event_id, &claimed.claim, 120)
            .expect("event claim releases");

        assert_eq!(released.state, WorkflowRuntimeBranchTaskEventState::Ready);
        assert!(released.claim.is_none());
        assert_eq!(
            repository.get(&event_id).expect("stored event").state,
            WorkflowRuntimeBranchTaskEventState::Ready
        );
        let reclaimed = repository
            .claim_next_due_for_workflow_run("run.test", owner_id("worker.beta"), 121, 50)
            .expect("claim next succeeds")
            .expect("released event is due");
        assert_eq!(reclaimed.record.event_id, event_id);
    }

    fn ready_record() -> WorkflowRuntimeBranchTaskEventRecord {
        WorkflowRuntimeBranchTaskEventRecord::ready(ready_request()).expect("ready record")
    }

    fn ready_record_with_id(event_id: &str) -> WorkflowRuntimeBranchTaskEventRecord {
        let mut request = ready_request();
        request.event_id = WorkflowRuntimeBranchTaskEventId::parse(event_id).expect("event id");
        WorkflowRuntimeBranchTaskEventRecord::ready(request).expect("ready record")
    }

    fn ready_record_with_id_and_run(
        event_id: &str,
        workflow_run_id: &str,
        ready_at_ms: u64,
    ) -> WorkflowRuntimeBranchTaskEventRecord {
        let mut request = ready_request();
        request.event_id = WorkflowRuntimeBranchTaskEventId::parse(event_id).expect("event id");
        request.workflow_run_id = workflow_run_id.to_string();
        request.ready_at_ms = ready_at_ms;
        WorkflowRuntimeBranchTaskEventRecord::ready(request).expect("ready record")
    }

    fn ready_request() -> WorkflowRuntimeBranchTaskEventRequest {
        WorkflowRuntimeBranchTaskEventRequest {
            event_id: WorkflowRuntimeBranchTaskEventId::parse("runtime-branch-task-event.test")
                .expect("event id"),
            session_id: "session.test".to_string(),
            workflow_id: "workflow.image".to_string(),
            workflow_run_id: "run.test".to_string(),
            scheduler_task_id: "image-task".to_string(),
            scheduler_task_attempt_id: Some("attempt.1".to_string()),
            attempt_generation: 1,
            queued_input_keys: vec!["prompt".to_string()],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "image-output".to_string(),
                port_id: "image".to_string(),
            }]),
            timeout_ms: Some(30_000),
            batching_key: Some("runtime.diffusers.cuda0".to_string()),
            ready_at_ms: 42,
        }
    }

    fn owner_id(value: &str) -> WorkflowRuntimeBranchTaskEventClaimOwnerId {
        WorkflowRuntimeBranchTaskEventClaimOwnerId::parse(value).expect("owner id")
    }

    fn event_id(value: &str) -> WorkflowRuntimeBranchTaskEventId {
        WorkflowRuntimeBranchTaskEventId::parse(value).expect("event id")
    }
}

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::graph::{WorkflowExecutionSessionKind, WorkflowGraphSessionStateView};

/// Stale workflow execution session cleanup request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionStaleCleanupRequest {
    pub idle_timeout_ms: u64,
}

/// Stale workflow execution session cleanup response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionStaleCleanupResponse {
    #[serde(default)]
    pub cleaned_session_ids: Vec<String>,
}

/// Background cleanup worker configuration for workflow execution sessions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowExecutionSessionStaleCleanupWorkerConfig {
    pub interval: Duration,
    pub idle_timeout: Duration,
}

impl Default for WorkflowExecutionSessionStaleCleanupWorkerConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(5 * 60),
        }
    }
}

/// Handle for a running stale workflow execution session cleanup worker.
pub struct WorkflowExecutionSessionStaleCleanupWorker {
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    join_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl WorkflowExecutionSessionStaleCleanupWorker {
    pub(crate) fn new(
        shutdown_tx: tokio::sync::watch::Sender<bool>,
        join_handle: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self {
            shutdown_tx,
            join_handle: tokio::sync::Mutex::new(Some(join_handle)),
        }
    }

    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        if let Some(join_handle) = self.join_handle.lock().await.take() {
            let _ = join_handle.await;
        }
    }
}

/// Session lifecycle state exposed to clients.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowExecutionSessionState {
    IdleLoaded,
    IdleUnloaded,
    Running,
}

/// Session summary payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionSummary {
    pub session_id: String,
    pub workflow_id: String,
    pub session_kind: WorkflowExecutionSessionKind,
    #[serde(default)]
    pub usage_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<WorkflowExecutionSessionAttributionContext>,
    pub keep_alive: bool,
    pub state: WorkflowExecutionSessionState,
    pub queued_runs: usize,
    pub run_count: u64,
}

/// Validated client/session/bucket context attached to a workflow execution session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionAttributionContext {
    pub client_id: String,
    pub client_session_id: String,
    pub bucket_id: String,
}

/// Session status request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionStatusRequest {
    pub session_id: String,
}

/// Session status response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionStatusResponse {
    pub session: WorkflowExecutionSessionSummary,
}

/// Session inspection request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionInspectionRequest {
    pub session_id: String,
}

/// Session inspection response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionInspectionResponse {
    pub session: WorkflowExecutionSessionSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_execution_session_state: Option<WorkflowGraphSessionStateView>,
}

/// Session queue item status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowExecutionSessionQueueItemStatus {
    Pending,
    Running,
}

/// Stable scheduler admission outcome vocabulary shared by queue, snapshot, and
/// trace projections.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSchedulerAdmissionOutcome {
    Queued,
    Admitted,
}

impl WorkflowSchedulerAdmissionOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkflowSchedulerAdmissionOutcome::Queued => "queued",
            WorkflowSchedulerAdmissionOutcome::Admitted => "admitted",
        }
    }
}

/// Stable scheduler decision reason vocabulary shared by queue, snapshot, and
/// trace projections.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSchedulerDecisionReason {
    SchedulerSnapshotFailed,
    MatchedPendingItem,
    MatchedRunningItem,
    SessionRunningWithBacklog,
    SessionRunning,
    SessionQueued,
    IdleLoaded,
    IdleUnloaded,
    HighestPriorityFirst,
    FifoPriorityTieBreak,
    WaitingForHigherPriority,
    WaitingForRuntimeCapacity,
    WaitingForRuntimeAdmission,
    StarvationProtection,
    WarmSessionReused,
    RuntimeReloadRequired,
    ColdStartRequired,
}

impl WorkflowSchedulerDecisionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkflowSchedulerDecisionReason::SchedulerSnapshotFailed => "scheduler_snapshot_failed",
            WorkflowSchedulerDecisionReason::MatchedPendingItem => "matched_pending_item",
            WorkflowSchedulerDecisionReason::MatchedRunningItem => "matched_running_item",
            WorkflowSchedulerDecisionReason::SessionRunningWithBacklog => {
                "session_running_with_backlog"
            }
            WorkflowSchedulerDecisionReason::SessionRunning => "session_running",
            WorkflowSchedulerDecisionReason::SessionQueued => "session_queued",
            WorkflowSchedulerDecisionReason::IdleLoaded => "idle_loaded",
            WorkflowSchedulerDecisionReason::IdleUnloaded => "idle_unloaded",
            WorkflowSchedulerDecisionReason::HighestPriorityFirst => "highest_priority_first",
            WorkflowSchedulerDecisionReason::FifoPriorityTieBreak => "fifo_priority_tie_break",
            WorkflowSchedulerDecisionReason::WaitingForHigherPriority => {
                "waiting_for_higher_priority"
            }
            WorkflowSchedulerDecisionReason::WaitingForRuntimeCapacity => {
                "waiting_for_runtime_capacity"
            }
            WorkflowSchedulerDecisionReason::WaitingForRuntimeAdmission => {
                "waiting_for_runtime_admission"
            }
            WorkflowSchedulerDecisionReason::StarvationProtection => "starvation_protection",
            WorkflowSchedulerDecisionReason::WarmSessionReused => "warm_session_reused",
            WorkflowSchedulerDecisionReason::RuntimeReloadRequired => "runtime_reload_required",
            WorkflowSchedulerDecisionReason::ColdStartRequired => "cold_start_required",
        }
    }
}

/// Session queue item metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionQueueItem {
    pub workflow_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enqueued_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dequeued_at_ms: Option<u64>,
    pub priority: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_position: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_admission_outcome: Option<WorkflowSchedulerAdmissionOutcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_decision_reason: Option<WorkflowSchedulerDecisionReason>,
    pub status: WorkflowExecutionSessionQueueItemStatus,
}

/// Session queue list request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionQueueListRequest {
    pub session_id: String,
}

/// Session queue list response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionQueueListResponse {
    pub session_id: String,
    pub items: Vec<WorkflowExecutionSessionQueueItem>,
}

/// Scheduler snapshot request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerSnapshotRequest {
    pub session_id: String,
}

/// Scheduler snapshot response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerSnapshotResponse {
    #[serde(default)]
    pub workflow_id: Option<String>,
    pub session_id: String,
    #[serde(default)]
    pub workflow_run_id: Option<String>,
    pub session: WorkflowExecutionSessionSummary,
    #[serde(default)]
    pub items: Vec<WorkflowExecutionSessionQueueItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<WorkflowSchedulerSnapshotDiagnostics>,
}

pub(crate) fn scheduler_snapshot_workflow_run_id(
    items: &[WorkflowExecutionSessionQueueItem],
) -> Option<String> {
    items
        .iter()
        .find(|item| item.status == WorkflowExecutionSessionQueueItemStatus::Running)
        .or_else(|| {
            let mut pending = items
                .iter()
                .filter(|item| item.status == WorkflowExecutionSessionQueueItemStatus::Pending);
            match (pending.next(), pending.next()) {
                (Some(item), None) => Some(item),
                _ => None,
            }
        })
        .map(|item| item.workflow_run_id.clone())
}

/// Session queue cancellation request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionQueueCancelRequest {
    pub session_id: String,
    pub workflow_run_id: String,
}

/// Session queue cancellation response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionQueueCancelResponse {
    pub ok: bool,
}

/// Privileged GUI-admin queue cancellation request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowAdminQueueCancelRequest {
    pub workflow_run_id: String,
}

/// Privileged GUI-admin queue cancellation response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowAdminQueueCancelResponse {
    pub ok: bool,
    pub session_id: String,
}

/// Privileged GUI-admin queue reprioritization request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowAdminQueueReprioritizeRequest {
    pub workflow_run_id: String,
    pub priority: i32,
}

/// Privileged GUI-admin queue reprioritization response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowAdminQueueReprioritizeResponse {
    pub ok: bool,
    pub session_id: String,
}

/// Privileged GUI-admin queue push-front request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowAdminQueuePushFrontRequest {
    pub workflow_run_id: String,
}

/// Privileged GUI-admin queue push-front response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowAdminQueuePushFrontResponse {
    pub ok: bool,
    pub session_id: String,
    pub priority: i32,
}

/// Session queue push-front request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionQueuePushFrontRequest {
    pub session_id: String,
    pub workflow_run_id: String,
}

/// Session queue push-front response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionQueuePushFrontResponse {
    pub ok: bool,
    pub priority: i32,
}

/// Session queue reprioritization request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionQueueReprioritizeRequest {
    pub session_id: String,
    pub workflow_run_id: String,
    pub priority: i32,
}

/// Session queue reprioritization response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionQueueReprioritizeResponse {
    pub ok: bool,
}

/// Session keep-alive update request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionKeepAliveRequest {
    pub session_id: String,
    pub keep_alive: bool,
}

/// Session keep-alive update response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionKeepAliveResponse {
    pub session_id: String,
    pub keep_alive: bool,
    pub state: WorkflowExecutionSessionState,
}

/// Host runtime retention hint derived from session behavior.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowExecutionSessionRetentionHint {
    Ephemeral,
    KeepAlive,
}

/// Host runtime unload reason.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowExecutionSessionUnloadReason {
    CapacityRebalance,
    KeepAliveDisabled,
    SessionClosed,
}

/// Scheduler-visible runtime capacity posture for the current session.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSchedulerRuntimeCapacityPressure {
    Available,
    RebalanceRequired,
    Saturated,
}

/// Backend-owned warmup/reuse posture for the runtime that would back the next
/// admission.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSchedulerRuntimeWarmupDecision {
    StartRuntime,
    ReuseLoadedRuntime,
    WaitForTransition,
}

/// Backend-owned explanation for the runtime warmup/reuse posture derived from
/// runtime-registry state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSchedulerRuntimeWarmupReason {
    NoLoadedInstance,
    RecoveryRequired,
    LoadedInstanceReady,
    LoadedInstanceBusy,
    WarmupInProgress,
    StopInProgress,
}

/// Optional runtime-registry facts attached to scheduler diagnostics. These
/// remain backend-owned and additive so transport layers only forward them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerRuntimeRegistryDiagnostics {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reclaim_candidate_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reclaim_candidate_runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_warmup_decision: Option<WorkflowSchedulerRuntimeWarmupDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_warmup_reason: Option<WorkflowSchedulerRuntimeWarmupReason>,
}

/// Additive backend-owned scheduler diagnostics derived from canonical queue
/// and loaded-session state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerSnapshotDiagnostics {
    pub loaded_session_count: usize,
    pub max_loaded_sessions: usize,
    pub reclaimable_loaded_session_count: usize,
    pub runtime_capacity_pressure: WorkflowSchedulerRuntimeCapacityPressure,
    pub active_run_blocks_admission: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_admission_workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_admission_bypassed_workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_admission_after_runs: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_admission_wait_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_admission_not_before_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_admission_reason: Option<WorkflowSchedulerDecisionReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_registry: Option<WorkflowSchedulerRuntimeRegistryDiagnostics>,
}

/// Idle session runtime candidate that may be unloaded under capacity pressure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowExecutionSessionRuntimeUnloadCandidate {
    pub session_id: String,
    pub workflow_id: String,
    pub usage_profile: Option<String>,
    pub required_backends: Vec<String>,
    pub required_models: Vec<String>,
    pub keep_alive: bool,
    pub access_tick: u64,
    pub run_count: u64,
}

/// Runtime-affinity target context used when selecting which idle session
/// runtime to unload under capacity pressure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowExecutionSessionRuntimeSelectionTarget {
    pub session_id: String,
    pub workflow_id: String,
    pub usage_profile: Option<String>,
    pub required_backends: Vec<String>,
    pub required_models: Vec<String>,
}

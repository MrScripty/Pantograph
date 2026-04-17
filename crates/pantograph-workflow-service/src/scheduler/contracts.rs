use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::graph::WorkflowSessionKind;

/// Stale workflow-session cleanup request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionStaleCleanupRequest {
    pub idle_timeout_ms: u64,
}

/// Stale workflow-session cleanup response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionStaleCleanupResponse {
    #[serde(default)]
    pub cleaned_session_ids: Vec<String>,
}

/// Background cleanup worker configuration for workflow sessions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSessionStaleCleanupWorkerConfig {
    pub interval: Duration,
    pub idle_timeout: Duration,
}

impl Default for WorkflowSessionStaleCleanupWorkerConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(5 * 60),
        }
    }
}

/// Handle for a running stale workflow-session cleanup worker.
pub struct WorkflowSessionStaleCleanupWorker {
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    join_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl WorkflowSessionStaleCleanupWorker {
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
pub enum WorkflowSessionState {
    IdleLoaded,
    IdleUnloaded,
    Running,
}

/// Session summary payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionSummary {
    pub session_id: String,
    pub workflow_id: String,
    pub session_kind: WorkflowSessionKind,
    #[serde(default)]
    pub usage_profile: Option<String>,
    pub keep_alive: bool,
    pub state: WorkflowSessionState,
    pub queued_runs: usize,
    pub run_count: u64,
}

/// Session status request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionStatusRequest {
    pub session_id: String,
}

/// Session status response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionStatusResponse {
    pub session: WorkflowSessionSummary,
}

/// Session queue item status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSessionQueueItemStatus {
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
pub struct WorkflowSessionQueueItem {
    pub queue_id: String,
    #[serde(default)]
    pub run_id: Option<String>,
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
    pub status: WorkflowSessionQueueItemStatus,
}

/// Session queue list request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionQueueListRequest {
    pub session_id: String,
}

/// Session queue list response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionQueueListResponse {
    pub session_id: String,
    pub items: Vec<WorkflowSessionQueueItem>,
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
    pub trace_execution_id: Option<String>,
    pub session: WorkflowSessionSummary,
    #[serde(default)]
    pub items: Vec<WorkflowSessionQueueItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<WorkflowSchedulerSnapshotDiagnostics>,
}

pub(crate) fn scheduler_snapshot_trace_execution_id(
    items: &[WorkflowSessionQueueItem],
) -> Option<String> {
    items
        .iter()
        .find(|item| item.status == WorkflowSessionQueueItemStatus::Running)
        .or_else(|| {
            let mut pending = items
                .iter()
                .filter(|item| item.status == WorkflowSessionQueueItemStatus::Pending);
            match (pending.next(), pending.next()) {
                (Some(item), None) => Some(item),
                _ => None,
            }
        })
        .map(|item| item.run_id.clone().unwrap_or_else(|| item.queue_id.clone()))
}

/// Session queue cancellation request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionQueueCancelRequest {
    pub session_id: String,
    pub queue_id: String,
}

/// Session queue cancellation response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionQueueCancelResponse {
    pub ok: bool,
}

/// Session queue reprioritization request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionQueueReprioritizeRequest {
    pub session_id: String,
    pub queue_id: String,
    pub priority: i32,
}

/// Session queue reprioritization response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionQueueReprioritizeResponse {
    pub ok: bool,
}

/// Session keep-alive update request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionKeepAliveRequest {
    pub session_id: String,
    pub keep_alive: bool,
}

/// Session keep-alive update response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionKeepAliveResponse {
    pub session_id: String,
    pub keep_alive: bool,
    pub state: WorkflowSessionState,
}

/// Host runtime retention hint derived from session behavior.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSessionRetentionHint {
    Ephemeral,
    KeepAlive,
}

/// Host runtime unload reason.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSessionUnloadReason {
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
    pub next_admission_queue_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_admission_after_runs: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_admission_reason: Option<WorkflowSchedulerDecisionReason>,
}

/// Idle session runtime candidate that may be unloaded under capacity pressure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSessionRuntimeUnloadCandidate {
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
pub struct WorkflowSessionRuntimeSelectionTarget {
    pub session_id: String,
    pub workflow_id: String,
    pub usage_profile: Option<String>,
    pub required_backends: Vec<String>,
    pub required_models: Vec<String>,
}

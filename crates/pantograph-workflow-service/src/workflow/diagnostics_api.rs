use std::collections::{BTreeMap, BTreeSet};

use pantograph_diagnostics_ledger::{
    ApplyArtifactRetentionPolicyCommand, ApplyArtifactRetentionPolicyResult,
    DiagnosticErrorSeverity, DiagnosticEventAppendRequest, DiagnosticEventPayload,
    DiagnosticEventPrivacyClass, DiagnosticEventRecord, DiagnosticEventRetentionClass,
    DiagnosticEventSourceComponent, DiagnosticsLedgerRepository, DiagnosticsQuery,
    DiagnosticsRetentionPolicy, ExecutionGuaranteeLevel, IoArtifactProjectionQuery,
    IoArtifactProjectionRecord, IoArtifactRetentionState, IoArtifactRetentionSummaryQuery,
    IoArtifactRetentionSummaryRecord, LibraryAssetAccessedPayload, LibraryAssetCacheStatus,
    LibraryAssetOperation, LibraryUsageProjectionQuery, LibraryUsageProjectionRecord,
    ModelLicenseUsageEvent, NodeExecutionProjectionStatus, NodeStatusProjectionQuery,
    NodeStatusProjectionRecord, ProjectionStateRecord, ProjectionStateUpdate, ProjectionStatus,
    RetentionClass, RetentionPolicyActorScope, RetentionPolicyChangedPayload,
    RunDetailProjectionQuery, RunDetailProjectionRecord, RunListFacetRecord,
    RunListProjectionQuery, RunListProjectionRecord, RunListProjectionStatus, RunTerminalPayload,
    RunTerminalStatus, RuntimeSelectionHistoryQuery, RuntimeSelectionHistorySummary,
    SchedulerModelCacheState, SchedulerTimelineProjectionQuery, SchedulerTimelineProjectionRecord,
    UpdateRetentionPolicyCommand, WorkflowExecutionSessionResumeState, IO_ARTIFACT_PROJECTION_NAME,
    IO_ARTIFACT_PROJECTION_VERSION, LIBRARY_USAGE_PROJECTION_NAME,
    LIBRARY_USAGE_PROJECTION_VERSION, NODE_STATUS_PROJECTION_NAME, NODE_STATUS_PROJECTION_VERSION,
    RUN_DETAIL_PROJECTION_NAME, RUN_DETAIL_PROJECTION_VERSION, RUN_LIST_PROJECTION_NAME,
    RUN_LIST_PROJECTION_VERSION, SCHEDULER_TIMELINE_PROJECTION_NAME,
    SCHEDULER_TIMELINE_PROJECTION_VERSION,
};
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use serde::{Deserialize, Serialize};

use crate::scheduler::unix_timestamp_ms;

use super::diagnostic_errors::{
    WorkflowDiagnosticErrorRecordRequest, WorkflowDiagnosticProjectionScope,
};
use super::{
    WorkflowErrorDiagnosticsLink, WorkflowRunGraphProjection, WorkflowRunGraphQueryRequest,
    WorkflowService, WorkflowServiceError,
};

const STARTUP_REPAIR_RUN_QUERY_LIMIT: u32 = 500;
const STARTUP_REPAIR_DRAIN_BATCH_SIZE: u32 = 500;
const STARTUP_REPAIR_MAX_DRAIN_PASSES: usize = 100;
const DEFAULT_PROJECTION_REFRESH_BATCH_SIZE: u32 = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsUsageQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_semantic_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_contract_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_contract_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guarantee_level: Option<ExecutionGuaranteeLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_before_ms: Option<i64>,
    #[serde(default)]
    pub page: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsUsageQueryResponse {
    pub events: Vec<ModelLicenseUsageEvent>,
    pub summaries: Vec<WorkflowDiagnosticsUsageSummary>,
    pub retention_policy: DiagnosticsRetentionPolicy,
    pub page: u32,
    pub page_size: u32,
    pub may_have_pruned_usage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsUsageSummary {
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_value: Option<String>,
    pub guarantee_level: ExecutionGuaranteeLevel,
    pub event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerTimelineQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_event_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerTimelineQueryResponse {
    pub events: Vec<SchedulerTimelineProjectionRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunListQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_semantic_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<RunListProjectionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_network_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at_from_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at_to_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_severity: Option<DiagnosticErrorSeverity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_event_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunListQueryResponse {
    pub runs: Vec<RunListProjectionRecord>,
    pub facets: Vec<RunListFacetRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunDetailQueryRequest {
    pub workflow_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunDetailQueryResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunDetailProjectionRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_statuses: Vec<NodeStatusProjectionRecord>,
    pub projection_state: ProjectionStateRecord,
    pub node_projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunInspectionQueryRequest {
    pub workflow_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunInspectionQueryResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_graph: Option<WorkflowRunGraphProjection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunDetailProjectionRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_statuses: Vec<NodeStatusProjectionRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub io_artifacts: Vec<IoArtifactProjectionRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_node_io: Vec<ResolvedNodeIoRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retention_summary: Vec<IoArtifactRetentionSummaryRecord>,
    pub run_projection_state: ProjectionStateRecord,
    pub node_projection_state: ProjectionStateRecord,
    pub io_projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedNodeIoDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedNodeIoResolution {
    ProducedOutput,
    DerivedFromEdge,
    ExplicitInput,
    WorkflowBoundary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedNodeIoProvenanceKind {
    ProducedOutput,
    GraphEdge,
    ExplicitInput,
    WorkflowInputBoundary,
    WorkflowOutputBoundary,
    CacheReplay,
    Coercion,
    Redaction,
    DynamicRoute,
    FanInAggregation,
    RuntimeInjected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedNodeIoRecord {
    pub node_id: String,
    pub port_id: String,
    pub direction: ResolvedNodeIoDirection,
    pub resolution: ResolvedNodeIoResolution,
    pub provenance_kind: ResolvedNodeIoProvenanceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_fact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_port_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_state: Option<IoArtifactRetentionState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerEstimateQueryRequest {
    pub workflow_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerEstimateQueryResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate: Option<WorkflowSchedulerEstimateRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerEstimateRecord {
    pub workflow_run_id: String,
    pub workflow_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_semantic_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_estimate_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate_confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_queue_wait_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_cache_state: Option<SchedulerModelCacheState>,
    pub last_event_seq: i64,
    pub last_updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowIoArtifactQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_state: Option<IoArtifactRetentionState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_event_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowIoArtifactQueryResponse {
    pub artifacts: Vec<IoArtifactProjectionRecord>,
    pub retention_summary: Vec<IoArtifactRetentionSummaryRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowNodeStatusQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<NodeExecutionProjectionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_event_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowNodeStatusQueryResponse {
    pub nodes: Vec<NodeStatusProjectionRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLibraryUsageQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_event_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLibraryUsageQueryResponse {
    pub assets: Vec<LibraryUsageProjectionRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLibraryAssetAccessRecordRequest {
    pub asset_id: String,
    pub operation: LibraryAssetOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_status: Option<LibraryAssetCacheStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_instance_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLibraryAssetAccessRecordResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_seq: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticEventRecordResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_seq: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowDiagnosticsProjectionKind {
    SchedulerTimeline,
    RunList,
    RunDetail,
    IoArtifact,
    NodeStatus,
    LibraryUsage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowDiagnosticsProjectionRefreshReason {
    DiagnosticEventAppended,
    ExplicitRefresh,
    ProjectionRebuild,
    StartupCatchUp,
    RetentionCleanup,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsProjectionRefreshRequest {
    pub projections: Vec<WorkflowDiagnosticsProjectionKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    pub reason: WorkflowDiagnosticsProjectionRefreshReason,
    pub batch_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsProjectionAdvance {
    pub projection_kind: WorkflowDiagnosticsProjectionKind,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsProjectionFailure {
    pub projection_kind: WorkflowDiagnosticsProjectionKind,
    pub projection_state: ProjectionStateRecord,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsProjectionInvalidation {
    pub projection_kind: WorkflowDiagnosticsProjectionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    pub last_event_seq: i64,
    pub reason: WorkflowDiagnosticsProjectionRefreshReason,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsProjectionRefreshResponse {
    pub advanced: Vec<WorkflowDiagnosticsProjectionAdvance>,
    pub failed: Vec<WorkflowDiagnosticsProjectionFailure>,
    pub invalidations: Vec<WorkflowDiagnosticsProjectionInvalidation>,
}

pub trait WorkflowDiagnosticsProjectionRefreshSink: Send + Sync {
    fn request_projection_refresh(&self, request: WorkflowDiagnosticsProjectionRefreshRequest);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowProjectionRebuildRequest {
    pub projection_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowProjectionRebuildResponse {
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionPolicyQueryRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionPolicyQueryResponse {
    pub retention_policy: DiagnosticsRetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionPolicyUpdateRequest {
    pub retention_days: u32,
    pub explanation: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionPolicyUpdateResponse {
    pub retention_policy: DiagnosticsRetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionCleanupRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionCleanupResponse {
    pub cleanup: ApplyArtifactRetentionPolicyResult,
}

impl WorkflowService {
    fn projection_error(
        &self,
        scope: WorkflowDiagnosticProjectionScope,
        error: WorkflowServiceError,
    ) -> WorkflowServiceError {
        let workflow_run_id = scope.workflow_run_id.clone();
        let link = self
            .record_workflow_diagnostic_error_if_configured(
                WorkflowDiagnosticErrorRecordRequest::projection_failed(scope, &error),
            )
            .map(|outcome| outcome.into_error_link(workflow_run_id.clone()))
            .unwrap_or_else(|record_error| WorkflowErrorDiagnosticsLink {
                workflow_run_id: workflow_run_id.map(|value| value.to_string()),
                diagnostic_event_id: None,
                diagnostics_unavailable: Some(record_error.message().to_string()),
            });
        error.with_diagnostics(link)
    }

    pub fn workflow_mark_abandoned_nonterminal_runs(
        &self,
        reason: &str,
    ) -> Result<usize, WorkflowServiceError> {
        let now_ms = unix_timestamp_ms() as i64;
        let mut ledger = self.diagnostics_ledger_guard()?;
        drain_run_list_projection_until_idle(&mut *ledger)?;

        let mut repaired = 0usize;
        for status in [
            RunListProjectionStatus::Accepted,
            RunListProjectionStatus::Future,
            RunListProjectionStatus::Scheduled,
            RunListProjectionStatus::Queued,
            RunListProjectionStatus::Delayed,
            RunListProjectionStatus::Running,
        ] {
            let runs = ledger
                .query_run_list_projection(RunListProjectionQuery {
                    status: Some(status),
                    limit: STARTUP_REPAIR_RUN_QUERY_LIMIT,
                    ..RunListProjectionQuery::default()
                })
                .map_err(WorkflowServiceError::from)?;

            for run in runs {
                let duration_ms =
                    startup_repair_duration_ms(now_ms, run.started_at_ms, &run.workflow_run_id)?;
                self.append_diagnostic_event_and_request_projection_refresh(
                    &mut *ledger,
                    DiagnosticEventAppendRequest {
                        source_component: DiagnosticEventSourceComponent::WorkflowService,
                        source_instance_id: Some("workflow-service-startup-repair".to_string()),
                        occurred_at_ms: now_ms,
                        workflow_run_id: Some(run.workflow_run_id),
                        workflow_id: Some(run.workflow_id),
                        workflow_version_id: run.workflow_version_id,
                        workflow_semantic_version: run.workflow_semantic_version,
                        node_id: None,
                        node_type: None,
                        node_version: None,
                        runtime_id: None,
                        runtime_version: None,
                        model_id: None,
                        model_version: None,
                        client_id: run.client_id,
                        client_session_id: run.client_session_id,
                        bucket_id: run.bucket_id,
                        scheduler_policy_id: run.scheduler_policy_id,
                        retention_policy_id: run.retention_policy_id,
                        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                        payload_ref: None,
                        payload: DiagnosticEventPayload::RunTerminal(RunTerminalPayload {
                            status: RunTerminalStatus::Failed,
                            duration_ms,
                            error: Some(reason.to_string()),
                            canonical_error_event_id: None,
                            resource_observation: None,
                        }),
                    },
                )
                .map_err(WorkflowServiceError::from)?;
                repaired = increment_startup_repair_count(repaired)?;
            }
        }

        if repaired > 0 {
            drain_run_list_projection_until_idle(&mut *ledger)?;
        }

        Ok(repaired)
    }

    pub fn workflow_diagnostics_usage_query(
        &self,
        request: WorkflowDiagnosticsUsageQueryRequest,
    ) -> Result<WorkflowDiagnosticsUsageQueryResponse, WorkflowServiceError> {
        let query = request.into_query()?;
        let ledger = self.diagnostics_ledger_guard()?;
        let projection = ledger
            .query_usage_events(query)
            .map_err(WorkflowServiceError::from)?;
        let retention_policy = ledger
            .retention_policy()
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowDiagnosticsUsageQueryResponse {
            summaries: summarize_usage(&projection.events),
            events: projection.events,
            retention_policy,
            page: projection.page,
            page_size: projection.page_size,
            may_have_pruned_usage: projection.may_have_pruned_usage,
        })
    }

    pub fn workflow_scheduler_timeline_query(
        &self,
        request: WorkflowSchedulerTimelineQueryRequest,
    ) -> Result<WorkflowSchedulerTimelineQueryResponse, WorkflowServiceError> {
        validate_optional_projection_batch_size(
            "projection_batch_size",
            request.projection_batch_size,
        )?;
        let query = request.into_scheduler_timeline_query()?;
        let ledger = self.diagnostics_ledger_guard()?;
        let projection_state = read_projection_state_or_empty(
            &*ledger,
            WorkflowDiagnosticsProjectionKind::SchedulerTimeline,
        )?;
        let events = match ledger.query_scheduler_timeline_projection(query.clone()) {
            Ok(events) => events,
            Err(error) => {
                drop(ledger);
                return Err(self.projection_error(
                    projection_error_scope(
                        "scheduler_timeline",
                        "query",
                        query.workflow_run_id,
                        query.workflow_id,
                    ),
                    WorkflowServiceError::from(error),
                ));
            }
        };

        Ok(WorkflowSchedulerTimelineQueryResponse {
            events,
            projection_state,
        })
    }

    pub fn workflow_run_list_query(
        &self,
        request: WorkflowRunListQueryRequest,
    ) -> Result<WorkflowRunListQueryResponse, WorkflowServiceError> {
        validate_optional_projection_batch_size(
            "projection_batch_size",
            request.projection_batch_size,
        )?;
        let query = request.into_run_list_query()?;
        let facet_query = query.clone();
        let ledger = self.diagnostics_ledger_guard()?;
        let projection_state =
            read_projection_state_or_empty(&*ledger, WorkflowDiagnosticsProjectionKind::RunList)?;
        let mut runs = match ledger.query_run_list_projection(query.clone()) {
            Ok(runs) => runs,
            Err(error) => {
                drop(ledger);
                return Err(self.projection_error(
                    projection_error_scope("run_list", "query", None, query.workflow_id),
                    WorkflowServiceError::from(error),
                ));
            }
        };
        let facets = match ledger.query_run_list_facets(facet_query.clone()) {
            Ok(facets) => facets,
            Err(error) => {
                drop(ledger);
                return Err(self.projection_error(
                    projection_error_scope("run_list", "facets", None, facet_query.workflow_id),
                    WorkflowServiceError::from(error),
                ));
            }
        };
        drop(ledger);
        self.annotate_run_list_resume_state(&mut runs)?;

        Ok(WorkflowRunListQueryResponse {
            runs,
            facets,
            projection_state,
        })
    }

    pub fn runtime_selection_history_summary(
        &self,
        query: RuntimeSelectionHistoryQuery,
    ) -> Result<Option<RuntimeSelectionHistorySummary>, WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(None);
        };
        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        drain_run_list_projection_until_idle(&mut *ledger)?;
        ledger
            .runtime_selection_history_summary(query)
            .map(Some)
            .map_err(WorkflowServiceError::from)
    }

    pub fn workflow_run_detail_query(
        &self,
        request: WorkflowRunDetailQueryRequest,
    ) -> Result<WorkflowRunDetailQueryResponse, WorkflowServiceError> {
        validate_optional_projection_batch_size(
            "projection_batch_size",
            request.projection_batch_size,
        )?;
        let query = request.into_run_detail_query()?;
        let ledger = self.diagnostics_ledger_guard()?;
        let projection_state =
            read_projection_state_or_empty(&*ledger, WorkflowDiagnosticsProjectionKind::RunDetail)?;
        let mut run = match ledger.query_run_detail_projection(query.clone()) {
            Ok(run) => run,
            Err(error) => {
                drop(ledger);
                return Err(self.projection_error(
                    projection_error_scope(
                        "run_detail",
                        "query",
                        Some(query.workflow_run_id),
                        None,
                    ),
                    WorkflowServiceError::from(error),
                ));
            }
        };
        let node_projection_state = read_projection_state_or_empty(
            &*ledger,
            WorkflowDiagnosticsProjectionKind::NodeStatus,
        )?;
        let node_query = NodeStatusProjectionQuery {
            workflow_run_id: Some(query.workflow_run_id.clone()),
            limit: 500,
            ..NodeStatusProjectionQuery::default()
        };
        let node_statuses = match ledger.query_node_status_projection(node_query.clone()) {
            Ok(nodes) => nodes,
            Err(error) => {
                drop(ledger);
                return Err(self.projection_error(
                    projection_error_scope(
                        "run_detail_node_status",
                        "query",
                        node_query.workflow_run_id,
                        None,
                    ),
                    WorkflowServiceError::from(error),
                ));
            }
        };
        drop(ledger);
        if let Some(run) = run.as_mut() {
            run.workflow_execution_session_resume_state = self
                .run_resume_state_for_projection_record(
                    run.workflow_execution_session_id.as_deref(),
                    run.workflow_run_id.as_str(),
                )?;
        }

        Ok(WorkflowRunDetailQueryResponse {
            run,
            node_statuses,
            projection_state,
            node_projection_state,
        })
    }

    fn annotate_run_list_resume_state(
        &self,
        runs: &mut [RunListProjectionRecord],
    ) -> Result<(), WorkflowServiceError> {
        for run in runs {
            run.workflow_execution_session_resume_state = self
                .run_resume_state_for_projection_record(
                    run.workflow_execution_session_id.as_deref(),
                    run.workflow_run_id.as_str(),
                )?;
        }
        Ok(())
    }

    fn run_resume_state_for_projection_record(
        &self,
        session_id: Option<&str>,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowExecutionSessionResumeState>, WorkflowServiceError> {
        let Some(session_id) = session_id else {
            return Ok(None);
        };
        let store = self.session_store_guard()?;
        Ok(store.active_run_dependency_readiness_resume_state(session_id, workflow_run_id))
    }

    pub fn workflow_run_inspection_query(
        &self,
        request: WorkflowRunInspectionQueryRequest,
    ) -> Result<WorkflowRunInspectionQueryResponse, WorkflowServiceError> {
        let run_graph = self
            .workflow_run_graph_query(WorkflowRunGraphQueryRequest {
                workflow_run_id: request.workflow_run_id.clone(),
            })?
            .run_graph;
        let run_detail = self.workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: request.workflow_run_id.clone(),
            projection_batch_size: request.projection_batch_size,
        })?;
        let io_artifacts = self.workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some(request.workflow_run_id),
            node_id: None,
            producer_node_id: None,
            consumer_node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            selected_backend_key: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(request.artifact_limit.unwrap_or(250)),
            projection_batch_size: request.projection_batch_size,
        })?;

        let resolved_node_io = resolve_workflow_run_node_io(&run_graph, &io_artifacts.artifacts);

        Ok(WorkflowRunInspectionQueryResponse {
            run_graph,
            run: run_detail.run,
            node_statuses: run_detail.node_statuses,
            io_artifacts: io_artifacts.artifacts,
            resolved_node_io,
            retention_summary: io_artifacts.retention_summary,
            run_projection_state: run_detail.projection_state,
            node_projection_state: run_detail.node_projection_state,
            io_projection_state: io_artifacts.projection_state,
        })
    }

    pub fn workflow_scheduler_estimate_query(
        &self,
        request: WorkflowSchedulerEstimateQueryRequest,
    ) -> Result<WorkflowSchedulerEstimateQueryResponse, WorkflowServiceError> {
        validate_optional_projection_batch_size(
            "projection_batch_size",
            request.projection_batch_size,
        )?;
        let query = request.into_run_detail_query()?;
        let ledger = self.diagnostics_ledger_guard()?;
        let projection_state =
            read_projection_state_or_empty(&*ledger, WorkflowDiagnosticsProjectionKind::RunDetail)?;
        let estimate = match ledger.query_run_detail_projection(query.clone()) {
            Ok(estimate) => estimate.map(WorkflowSchedulerEstimateRecord::from),
            Err(error) => {
                drop(ledger);
                return Err(self.projection_error(
                    projection_error_scope(
                        "scheduler_estimate",
                        "query",
                        Some(query.workflow_run_id),
                        None,
                    ),
                    WorkflowServiceError::from(error),
                ));
            }
        };

        Ok(WorkflowSchedulerEstimateQueryResponse {
            estimate,
            projection_state,
        })
    }

    pub fn workflow_io_artifact_query(
        &self,
        request: WorkflowIoArtifactQueryRequest,
    ) -> Result<WorkflowIoArtifactQueryResponse, WorkflowServiceError> {
        validate_optional_projection_batch_size(
            "projection_batch_size",
            request.projection_batch_size,
        )?;
        let query = request.into_io_artifact_query()?;
        let summary_query = io_artifact_retention_summary_query(&query);
        let ledger = self.diagnostics_ledger_guard()?;
        let projection_state = read_projection_state_or_empty(
            &*ledger,
            WorkflowDiagnosticsProjectionKind::IoArtifact,
        )?;
        let artifacts = match ledger.query_io_artifact_projection(query.clone()) {
            Ok(artifacts) => artifacts,
            Err(error) => {
                drop(ledger);
                return Err(self.projection_error(
                    projection_error_scope("io_artifact", "query", query.workflow_run_id, None),
                    WorkflowServiceError::from(error),
                ));
            }
        };
        let retention_summary = match ledger.query_io_artifact_retention_summary(summary_query) {
            Ok(retention_summary) => retention_summary,
            Err(error) => {
                drop(ledger);
                return Err(self.projection_error(
                    projection_error_scope("io_artifact", "retention_summary", None, None),
                    WorkflowServiceError::from(error),
                ));
            }
        };

        Ok(WorkflowIoArtifactQueryResponse {
            artifacts,
            retention_summary,
            projection_state,
        })
    }

    pub fn workflow_node_status_query(
        &self,
        request: WorkflowNodeStatusQueryRequest,
    ) -> Result<WorkflowNodeStatusQueryResponse, WorkflowServiceError> {
        validate_optional_projection_batch_size(
            "projection_batch_size",
            request.projection_batch_size,
        )?;
        let query = request.into_node_status_query()?;
        let ledger = self.diagnostics_ledger_guard()?;
        let projection_state = read_projection_state_or_empty(
            &*ledger,
            WorkflowDiagnosticsProjectionKind::NodeStatus,
        )?;
        let nodes = match ledger.query_node_status_projection(query.clone()) {
            Ok(nodes) => nodes,
            Err(error) => {
                drop(ledger);
                return Err(self.projection_error(
                    projection_error_scope("node_status", "query", query.workflow_run_id, None),
                    WorkflowServiceError::from(error),
                ));
            }
        };

        Ok(WorkflowNodeStatusQueryResponse {
            nodes,
            projection_state,
        })
    }

    pub fn workflow_library_usage_query(
        &self,
        request: WorkflowLibraryUsageQueryRequest,
    ) -> Result<WorkflowLibraryUsageQueryResponse, WorkflowServiceError> {
        validate_optional_projection_batch_size(
            "projection_batch_size",
            request.projection_batch_size,
        )?;
        let query = request.into_library_usage_query()?;
        let ledger = self.diagnostics_ledger_guard()?;
        let projection_state = read_projection_state_or_empty(
            &*ledger,
            WorkflowDiagnosticsProjectionKind::LibraryUsage,
        )?;
        let assets = match ledger.query_library_usage_projection(query.clone()) {
            Ok(assets) => assets,
            Err(error) => {
                drop(ledger);
                return Err(self.projection_error(
                    projection_error_scope(
                        "library_usage",
                        "query",
                        query.workflow_run_id,
                        query.workflow_id,
                    ),
                    WorkflowServiceError::from(error),
                ));
            }
        };

        Ok(WorkflowLibraryUsageQueryResponse {
            assets,
            projection_state,
        })
    }

    pub fn workflow_library_asset_access_record(
        &self,
        request: WorkflowLibraryAssetAccessRecordRequest,
    ) -> Result<WorkflowLibraryAssetAccessRecordResponse, WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(WorkflowLibraryAssetAccessRecordResponse { event_seq: None });
        };
        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        let event = ledger
            .append_diagnostic_event(DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Library,
                source_instance_id: request
                    .source_instance_id
                    .or_else(|| Some("workflow-library-audit".to_string())),
                occurred_at_ms: unix_timestamp_ms() as i64,
                workflow_run_id: None,
                workflow_id: None,
                workflow_version_id: None,
                workflow_semantic_version: None,
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: None,
                client_session_id: None,
                bucket_id: None,
                scheduler_policy_id: None,
                retention_policy_id: None,
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::LibraryAssetAccessed(
                    LibraryAssetAccessedPayload {
                        asset_id: request.asset_id,
                        operation: request.operation,
                        cache_status: request.cache_status,
                        network_bytes: request.network_bytes,
                    },
                ),
            })
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowLibraryAssetAccessRecordResponse {
            event_seq: Some(event.event_seq),
        })
    }

    pub fn workflow_diagnostic_event_record(
        &self,
        request: DiagnosticEventAppendRequest,
    ) -> Result<WorkflowDiagnosticEventRecordResponse, WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(WorkflowDiagnosticEventRecordResponse { event_seq: None });
        };
        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        let event =
            self.append_diagnostic_event_and_request_projection_refresh(&mut *ledger, request)?;

        Ok(WorkflowDiagnosticEventRecordResponse {
            event_seq: Some(event.event_seq),
        })
    }

    pub(crate) fn append_diagnostic_event_and_request_projection_refresh(
        &self,
        ledger: &mut impl DiagnosticsLedgerRepository,
        request: DiagnosticEventAppendRequest,
    ) -> Result<DiagnosticEventRecord, WorkflowServiceError> {
        let refresh_request = diagnostics_projection_refresh_request_for_event(&request);
        let event = DiagnosticsLedgerRepository::append_diagnostic_event(ledger, request)
            .map_err(WorkflowServiceError::from)?;
        if let Some(refresh_request) = refresh_request {
            self.request_diagnostics_projection_refresh(refresh_request);
        }
        Ok(event)
    }

    fn request_diagnostics_projection_refresh(
        &self,
        request: WorkflowDiagnosticsProjectionRefreshRequest,
    ) {
        let sink = self
            .diagnostics_projection_refresh_sink
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        if let Some(sink) = sink {
            sink.request_projection_refresh(request);
        }
    }

    pub fn workflow_diagnostics_projection_refresh(
        &self,
        request: WorkflowDiagnosticsProjectionRefreshRequest,
    ) -> Result<WorkflowDiagnosticsProjectionRefreshResponse, WorkflowServiceError> {
        if request.projections.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "projections must be non-empty".to_string(),
            ));
        }
        validate_projection_batch_size("batch_size", request.batch_size)?;
        let workflow_run_id =
            parse_optional_id::<WorkflowRunId>("workflow_run_id", request.workflow_run_id)?
                .map(|value| value.to_string());
        let workflow_id = parse_optional_id::<WorkflowId>("workflow_id", request.workflow_id)?
            .map(|value| value.to_string());

        let mut ledger = self.diagnostics_ledger_guard()?;
        let mut advanced = Vec::new();
        let mut failed = Vec::new();
        let mut invalidations = Vec::new();

        for projection_kind in request.projections {
            match drain_projection_kind(&mut *ledger, projection_kind, request.batch_size) {
                Ok(projection_state) => {
                    invalidations.push(WorkflowDiagnosticsProjectionInvalidation {
                        projection_kind,
                        workflow_run_id: workflow_run_id.clone(),
                        workflow_id: workflow_id.clone(),
                        last_event_seq: projection_state.last_applied_event_seq,
                        reason: request.reason,
                        updated_at_ms: projection_state.updated_at_ms,
                    });
                    advanced.push(WorkflowDiagnosticsProjectionAdvance {
                        projection_kind,
                        projection_state,
                    });
                }
                Err(error) => {
                    let error = WorkflowServiceError::from(error);
                    let error_message = error.message().to_string();
                    let projection_state = mark_projection_refresh_failed(
                        &mut *ledger,
                        projection_kind,
                        error_message.clone(),
                    )?;
                    failed.push(WorkflowDiagnosticsProjectionFailure {
                        projection_kind,
                        projection_state,
                        error: error_message,
                    });
                }
            }
        }

        Ok(WorkflowDiagnosticsProjectionRefreshResponse {
            advanced,
            failed,
            invalidations,
        })
    }

    pub fn workflow_projection_rebuild(
        &self,
        request: WorkflowProjectionRebuildRequest,
    ) -> Result<WorkflowProjectionRebuildResponse, WorkflowServiceError> {
        let batch_size = request.batch_size.unwrap_or(500);
        if batch_size == 0 {
            return Err(WorkflowServiceError::InvalidRequest(
                "batch_size must be greater than zero".to_string(),
            ));
        }
        if batch_size > 500 {
            return Err(WorkflowServiceError::InvalidRequest(
                "batch_size exceeds maximum 500".to_string(),
            ));
        }
        let projection_kind =
            WorkflowDiagnosticsProjectionKind::from_projection_name(&request.projection_name)?;
        let mut ledger = self.diagnostics_ledger_guard()?;
        let projection_state =
            match ledger.rebuild_projection(projection_kind.projection_name(), batch_size) {
                Ok(projection_state) => projection_state,
                Err(error) => {
                    let projection_name = projection_kind.projection_name().to_string();
                    drop(ledger);
                    return Err(self.projection_error(
                        projection_error_scope(projection_name, "rebuild", None, None),
                        WorkflowServiceError::from(error),
                    ));
                }
            };

        Ok(WorkflowProjectionRebuildResponse { projection_state })
    }

    pub fn workflow_retention_policy_query(
        &self,
        _request: WorkflowRetentionPolicyQueryRequest,
    ) -> Result<WorkflowRetentionPolicyQueryResponse, WorkflowServiceError> {
        let ledger = self.diagnostics_ledger_guard()?;
        let retention_policy = ledger
            .retention_policy()
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowRetentionPolicyQueryResponse { retention_policy })
    }

    pub fn workflow_retention_policy_update(
        &self,
        request: WorkflowRetentionPolicyUpdateRequest,
    ) -> Result<WorkflowRetentionPolicyUpdateResponse, WorkflowServiceError> {
        if request.reason.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "reason must be non-empty".to_string(),
            ));
        }

        let mut ledger = self.diagnostics_ledger_guard()?;
        let retention_policy = ledger
            .update_retention_policy(UpdateRetentionPolicyCommand {
                retention_class: RetentionClass::Standard,
                retention_days: request.retention_days,
                explanation: request.explanation,
            })
            .map_err(WorkflowServiceError::from)?;
        ledger
            .append_diagnostic_event(DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Retention,
                source_instance_id: Some("workflow-retention-policy".to_string()),
                occurred_at_ms: unix_timestamp_ms() as i64,
                workflow_run_id: None,
                workflow_id: None,
                workflow_version_id: None,
                workflow_semantic_version: None,
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: None,
                client_session_id: None,
                bucket_id: None,
                scheduler_policy_id: None,
                retention_policy_id: Some(retention_policy.policy_id.clone()),
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::RetentionPolicyChanged(
                    RetentionPolicyChangedPayload {
                        policy_id: retention_policy.policy_id.clone(),
                        policy_version: retention_policy.policy_version,
                        retention_days: retention_policy.retention_days,
                        actor_scope: RetentionPolicyActorScope::GuiAdmin,
                        reason: request.reason,
                    },
                ),
            })
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowRetentionPolicyUpdateResponse { retention_policy })
    }

    pub fn workflow_retention_cleanup_apply(
        &self,
        request: WorkflowRetentionCleanupRequest,
    ) -> Result<WorkflowRetentionCleanupResponse, WorkflowServiceError> {
        if request.reason.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "reason must be non-empty".to_string(),
            ));
        }
        let limit = request.limit.unwrap_or(500);
        if limit == 0 {
            return Err(WorkflowServiceError::InvalidRequest(
                "limit must be greater than zero".to_string(),
            ));
        }
        if limit > 500 {
            return Err(WorkflowServiceError::InvalidRequest(
                "limit exceeds maximum 500".to_string(),
            ));
        }

        let mut ledger = self.diagnostics_ledger_guard()?;
        let cleanup = ledger
            .apply_artifact_retention_policy(ApplyArtifactRetentionPolicyCommand {
                retention_class: RetentionClass::Standard,
                now_ms: unix_timestamp_ms() as i64,
                limit,
                actor_scope: RetentionPolicyActorScope::GuiAdmin,
                reason: request.reason,
            })
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowRetentionCleanupResponse { cleanup })
    }
}

impl WorkflowDiagnosticsProjectionKind {
    fn from_projection_name(projection_name: &str) -> Result<Self, WorkflowServiceError> {
        match projection_name {
            SCHEDULER_TIMELINE_PROJECTION_NAME => Ok(Self::SchedulerTimeline),
            RUN_LIST_PROJECTION_NAME => Ok(Self::RunList),
            RUN_DETAIL_PROJECTION_NAME => Ok(Self::RunDetail),
            IO_ARTIFACT_PROJECTION_NAME => Ok(Self::IoArtifact),
            NODE_STATUS_PROJECTION_NAME => Ok(Self::NodeStatus),
            LIBRARY_USAGE_PROJECTION_NAME => Ok(Self::LibraryUsage),
            _ => Err(WorkflowServiceError::InvalidRequest(format!(
                "unknown diagnostics projection '{}'",
                projection_name
            ))),
        }
    }

    fn projection_name(self) -> &'static str {
        match self {
            Self::SchedulerTimeline => SCHEDULER_TIMELINE_PROJECTION_NAME,
            Self::RunList => RUN_LIST_PROJECTION_NAME,
            Self::RunDetail => RUN_DETAIL_PROJECTION_NAME,
            Self::IoArtifact => IO_ARTIFACT_PROJECTION_NAME,
            Self::NodeStatus => NODE_STATUS_PROJECTION_NAME,
            Self::LibraryUsage => LIBRARY_USAGE_PROJECTION_NAME,
        }
    }

    fn projection_version(self) -> i64 {
        match self {
            Self::SchedulerTimeline => SCHEDULER_TIMELINE_PROJECTION_VERSION,
            Self::RunList => RUN_LIST_PROJECTION_VERSION,
            Self::RunDetail => RUN_DETAIL_PROJECTION_VERSION,
            Self::IoArtifact => IO_ARTIFACT_PROJECTION_VERSION,
            Self::NodeStatus => NODE_STATUS_PROJECTION_VERSION,
            Self::LibraryUsage => LIBRARY_USAGE_PROJECTION_VERSION,
        }
    }
}

fn validate_projection_batch_size(
    field: &'static str,
    batch_size: u32,
) -> Result<(), WorkflowServiceError> {
    if batch_size == 0 {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "{} must be at least 1",
            field
        )));
    }
    if batch_size > 500 {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "{} exceeds maximum 500",
            field
        )));
    }
    Ok(())
}

fn validate_optional_projection_batch_size(
    field: &'static str,
    batch_size: Option<u32>,
) -> Result<(), WorkflowServiceError> {
    if let Some(batch_size) = batch_size {
        validate_projection_batch_size(field, batch_size)?;
    }
    Ok(())
}

fn read_projection_state_or_empty(
    ledger: &impl DiagnosticsLedgerRepository,
    projection_kind: WorkflowDiagnosticsProjectionKind,
) -> Result<ProjectionStateRecord, WorkflowServiceError> {
    ledger
        .projection_state(projection_kind.projection_name())
        .map_err(WorkflowServiceError::from)
        .map(|projection_state| {
            projection_state.unwrap_or_else(|| empty_projection_state(projection_kind))
        })
}

fn empty_projection_state(
    projection_kind: WorkflowDiagnosticsProjectionKind,
) -> ProjectionStateRecord {
    ProjectionStateRecord {
        projection_name: projection_kind.projection_name().to_string(),
        projection_version: projection_kind.projection_version(),
        last_applied_event_seq: 0,
        status: ProjectionStatus::NeedsRebuild,
        rebuilt_at_ms: None,
        updated_at_ms: 0,
        last_error: None,
        last_error_at_ms: None,
        last_failed_event_seq: None,
    }
}

fn drain_projection_kind(
    ledger: &mut impl DiagnosticsLedgerRepository,
    projection_kind: WorkflowDiagnosticsProjectionKind,
    batch_size: u32,
) -> Result<ProjectionStateRecord, pantograph_diagnostics_ledger::DiagnosticsLedgerError> {
    match projection_kind {
        WorkflowDiagnosticsProjectionKind::SchedulerTimeline => {
            ledger.drain_scheduler_timeline_projection(batch_size)
        }
        WorkflowDiagnosticsProjectionKind::RunList => ledger.drain_run_list_projection(batch_size),
        WorkflowDiagnosticsProjectionKind::RunDetail => {
            ledger.drain_run_detail_projection(batch_size)
        }
        WorkflowDiagnosticsProjectionKind::IoArtifact => {
            ledger.drain_io_artifact_projection(batch_size)
        }
        WorkflowDiagnosticsProjectionKind::NodeStatus => {
            ledger.drain_node_status_projection(batch_size)
        }
        WorkflowDiagnosticsProjectionKind::LibraryUsage => {
            ledger.drain_library_usage_projection(batch_size)
        }
    }
}

fn mark_projection_refresh_failed(
    ledger: &mut impl DiagnosticsLedgerRepository,
    projection_kind: WorkflowDiagnosticsProjectionKind,
    error_message: String,
) -> Result<ProjectionStateRecord, WorkflowServiceError> {
    let current = ledger
        .projection_state(projection_kind.projection_name())
        .map_err(WorkflowServiceError::from)?;
    let last_applied_event_seq = current
        .as_ref()
        .map(|state| state.last_applied_event_seq)
        .unwrap_or(0);
    let rebuilt_at_ms = current.as_ref().and_then(|state| state.rebuilt_at_ms);

    ledger
        .upsert_projection_state(ProjectionStateUpdate {
            projection_name: projection_kind.projection_name().to_string(),
            projection_version: projection_kind.projection_version(),
            last_applied_event_seq,
            status: ProjectionStatus::Failed,
            rebuilt_at_ms,
            last_error: Some(error_message),
            last_error_at_ms: Some(unix_timestamp_ms() as i64),
            last_failed_event_seq: Some(last_applied_event_seq),
        })
        .map_err(WorkflowServiceError::from)
}

fn diagnostics_projection_refresh_request_for_event(
    request: &DiagnosticEventAppendRequest,
) -> Option<WorkflowDiagnosticsProjectionRefreshRequest> {
    let projections = diagnostics_projection_kinds_for_payload(&request.payload);
    if projections.is_empty() {
        return None;
    }

    Some(WorkflowDiagnosticsProjectionRefreshRequest {
        projections,
        workflow_run_id: request
            .workflow_run_id
            .as_ref()
            .map(|workflow_run_id| workflow_run_id.as_str().to_string()),
        workflow_id: request
            .workflow_id
            .as_ref()
            .map(|workflow_id| workflow_id.as_str().to_string()),
        reason: WorkflowDiagnosticsProjectionRefreshReason::DiagnosticEventAppended,
        batch_size: DEFAULT_PROJECTION_REFRESH_BATCH_SIZE,
    })
}

fn diagnostics_projection_kinds_for_payload(
    payload: &DiagnosticEventPayload,
) -> Vec<WorkflowDiagnosticsProjectionKind> {
    let mut projections = BTreeSet::new();
    match payload {
        DiagnosticEventPayload::SchedulerEstimateProduced(_)
        | DiagnosticEventPayload::SchedulerQueuePlacement(_)
        | DiagnosticEventPayload::SchedulerQueueControl(_)
        | DiagnosticEventPayload::SchedulerRunDelayed(_)
        | DiagnosticEventPayload::SchedulerModelLifecycleChanged(_)
        | DiagnosticEventPayload::SchedulerRunAdmitted(_)
        | DiagnosticEventPayload::SchedulerReservationChanged(_)
        | DiagnosticEventPayload::RunStarted(_)
        | DiagnosticEventPayload::RunTerminal(_)
        | DiagnosticEventPayload::RunSnapshotAccepted(_) => {
            projections.insert(WorkflowDiagnosticsProjectionKind::SchedulerTimeline);
            projections.insert(WorkflowDiagnosticsProjectionKind::RunList);
            projections.insert(WorkflowDiagnosticsProjectionKind::RunDetail);
        }
        DiagnosticEventPayload::IoArtifactObserved(_)
        | DiagnosticEventPayload::RetentionArtifactStateChanged(_) => {
            projections.insert(WorkflowDiagnosticsProjectionKind::IoArtifact);
            projections.insert(WorkflowDiagnosticsProjectionKind::RunDetail);
        }
        DiagnosticEventPayload::LibraryAssetAccessed(_) => {
            projections.insert(WorkflowDiagnosticsProjectionKind::LibraryUsage);
        }
        DiagnosticEventPayload::RetentionPolicyChanged(_) => {
            projections.insert(WorkflowDiagnosticsProjectionKind::IoArtifact);
            projections.insert(WorkflowDiagnosticsProjectionKind::RunList);
        }
        DiagnosticEventPayload::RuntimeCapabilityObserved(_)
        | DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(_)
        | DiagnosticEventPayload::DiagnosticErrorOccurred(_) => {
            projections.insert(WorkflowDiagnosticsProjectionKind::SchedulerTimeline);
            projections.insert(WorkflowDiagnosticsProjectionKind::RunList);
            projections.insert(WorkflowDiagnosticsProjectionKind::RunDetail);
        }
        DiagnosticEventPayload::NodeExecutionStatus(_) => {
            projections.insert(WorkflowDiagnosticsProjectionKind::NodeStatus);
            projections.insert(WorkflowDiagnosticsProjectionKind::RunDetail);
        }
    }
    projections.into_iter().collect()
}

impl WorkflowDiagnosticsUsageQueryRequest {
    fn into_query(self) -> Result<DiagnosticsQuery, WorkflowServiceError> {
        let query = DiagnosticsQuery {
            client_id: parse_optional_id("client_id", self.client_id)?,
            client_session_id: parse_optional_id("client_session_id", self.client_session_id)?,
            bucket_id: parse_optional_id("bucket_id", self.bucket_id)?,
            workflow_run_id: parse_optional_id("workflow_run_id", self.workflow_run_id)?,
            workflow_id: parse_optional_id("workflow_id", self.workflow_id)?,
            workflow_version_id: parse_optional_id(
                "workflow_version_id",
                self.workflow_version_id,
            )?,
            workflow_semantic_version: self.workflow_semantic_version,
            node_id: self.node_id,
            node_contract_version: self.node_contract_version,
            node_contract_digest: self.node_contract_digest,
            model_id: self.model_id,
            license_value: self.license_value,
            guarantee_level: self.guarantee_level,
            started_at_ms: self.started_at_ms,
            ended_before_ms: self.ended_before_ms,
            page: self.page,
            page_size: resolve_positive_optional_u32(
                "page_size",
                self.page_size,
                DiagnosticsQuery::default().page_size,
            )?,
        };
        query.validate().map_err(WorkflowServiceError::from)?;
        Ok(query)
    }
}

pub(super) fn startup_repair_duration_ms(
    now_ms: i64,
    started_at_ms: Option<i64>,
    workflow_run_id: &WorkflowRunId,
) -> Result<Option<u64>, WorkflowServiceError> {
    let Some(started_at_ms) = started_at_ms else {
        return Ok(None);
    };
    let duration_ms = now_ms.checked_sub(started_at_ms).ok_or_else(|| {
        WorkflowServiceError::Internal(format!(
            "startup repair duration overflow for workflow run {}",
            workflow_run_id
        ))
    })?;
    if duration_ms < 0 {
        return Err(WorkflowServiceError::Internal(format!(
            "startup repair duration underflow for workflow run {}: started_at_ms {} is after now_ms {}",
            workflow_run_id, started_at_ms, now_ms
        )));
    }

    u64::try_from(duration_ms).map(Some).map_err(|_| {
        WorkflowServiceError::Internal(format!(
            "startup repair duration overflow for workflow run {}",
            workflow_run_id
        ))
    })
}

pub(super) fn increment_startup_repair_count(
    repaired: usize,
) -> Result<usize, WorkflowServiceError> {
    repaired.checked_add(1).ok_or_else(|| {
        WorkflowServiceError::Internal(
            "startup repair count overflow while marking abandoned runs".to_string(),
        )
    })
}

fn drain_run_list_projection_until_idle(
    ledger: &mut impl DiagnosticsLedgerRepository,
) -> Result<(), WorkflowServiceError> {
    let mut previous_event_seq = -1;
    for _ in 0..STARTUP_REPAIR_MAX_DRAIN_PASSES {
        let state = ledger
            .drain_run_list_projection(STARTUP_REPAIR_DRAIN_BATCH_SIZE)
            .map_err(WorkflowServiceError::from)?;
        if state.last_applied_event_seq == previous_event_seq {
            return Ok(());
        }
        previous_event_seq = state.last_applied_event_seq;
    }
    Ok(())
}

impl WorkflowSchedulerTimelineQueryRequest {
    fn into_scheduler_timeline_query(
        self,
    ) -> Result<SchedulerTimelineProjectionQuery, WorkflowServiceError> {
        Ok(SchedulerTimelineProjectionQuery {
            workflow_run_id: parse_optional_id("workflow_run_id", self.workflow_run_id)?,
            workflow_id: parse_optional_id("workflow_id", self.workflow_id)?,
            scheduler_policy_id: self.scheduler_policy_id,
            after_event_seq: self.after_event_seq,
            limit: resolve_positive_optional_u32(
                "limit",
                self.limit,
                SchedulerTimelineProjectionQuery::default().limit,
            )?,
        })
    }
}

impl WorkflowRunListQueryRequest {
    fn into_run_list_query(self) -> Result<RunListProjectionQuery, WorkflowServiceError> {
        Ok(RunListProjectionQuery {
            workflow_id: parse_optional_id("workflow_id", self.workflow_id)?,
            workflow_version_id: parse_optional_id(
                "workflow_version_id",
                self.workflow_version_id,
            )?,
            workflow_semantic_version: self.workflow_semantic_version,
            status: self.status,
            scheduler_policy_id: self.scheduler_policy_id,
            retention_policy_id: self.retention_policy_id,
            selected_runtime_id: self.selected_runtime_id,
            selected_runtime_variant_id: self.selected_runtime_variant_id,
            selected_backend_key: self.selected_backend_key,
            selected_device_class: self.selected_device_class,
            selected_device_id: self.selected_device_id,
            selected_network_node_id: self.selected_network_node_id,
            client_id: parse_optional_id("client_id", self.client_id)?,
            client_session_id: parse_optional_id("client_session_id", self.client_session_id)?,
            bucket_id: parse_optional_id("bucket_id", self.bucket_id)?,
            accepted_at_from_ms: self.accepted_at_from_ms,
            accepted_at_to_ms: self.accepted_at_to_ms,
            error_severity: self.error_severity,
            error_phase: self.error_phase,
            after_event_seq: self.after_event_seq,
            limit: resolve_positive_optional_u32(
                "limit",
                self.limit,
                RunListProjectionQuery::default().limit,
            )?,
        })
    }
}

impl WorkflowRunDetailQueryRequest {
    fn into_run_detail_query(self) -> Result<RunDetailProjectionQuery, WorkflowServiceError> {
        Ok(RunDetailProjectionQuery {
            workflow_run_id: parse_id("workflow_run_id", self.workflow_run_id)?,
        })
    }
}

impl WorkflowSchedulerEstimateQueryRequest {
    fn into_run_detail_query(self) -> Result<RunDetailProjectionQuery, WorkflowServiceError> {
        Ok(RunDetailProjectionQuery {
            workflow_run_id: parse_id("workflow_run_id", self.workflow_run_id)?,
        })
    }
}

impl From<RunDetailProjectionRecord> for WorkflowSchedulerEstimateRecord {
    fn from(run: RunDetailProjectionRecord) -> Self {
        Self {
            workflow_run_id: run.workflow_run_id.to_string(),
            workflow_id: run.workflow_id.to_string(),
            workflow_version_id: run.workflow_version_id.map(|value| value.to_string()),
            workflow_semantic_version: run.workflow_semantic_version,
            scheduler_policy_id: run.scheduler_policy_id,
            latest_estimate_json: run.latest_estimate_json,
            estimate_confidence: run.estimate_confidence,
            estimated_queue_wait_ms: run.estimated_queue_wait_ms,
            estimated_duration_ms: run.estimated_duration_ms,
            model_cache_state: run.model_cache_state,
            last_event_seq: run.last_event_seq,
            last_updated_at_ms: run.last_updated_at_ms,
        }
    }
}

impl WorkflowIoArtifactQueryRequest {
    fn into_io_artifact_query(self) -> Result<IoArtifactProjectionQuery, WorkflowServiceError> {
        let query = IoArtifactProjectionQuery {
            workflow_run_id: parse_optional_id("workflow_run_id", self.workflow_run_id)?,
            node_id: self.node_id,
            producer_node_id: self.producer_node_id,
            consumer_node_id: self.consumer_node_id,
            artifact_role: self.artifact_role,
            media_type: self.media_type,
            retention_state: self.retention_state,
            retention_policy_id: self.retention_policy_id,
            runtime_id: self.runtime_id,
            selected_backend_key: self.selected_backend_key,
            model_id: self.model_id,
            after_event_seq: self.after_event_seq,
            limit: resolve_positive_optional_u32("limit", self.limit, 100)?,
        };
        query.validate(500).map_err(WorkflowServiceError::from)?;
        Ok(query)
    }
}

fn io_artifact_retention_summary_query(
    query: &IoArtifactProjectionQuery,
) -> IoArtifactRetentionSummaryQuery {
    IoArtifactRetentionSummaryQuery {
        workflow_run_id: query.workflow_run_id.clone(),
        node_id: query.node_id.clone(),
        producer_node_id: query.producer_node_id.clone(),
        consumer_node_id: query.consumer_node_id.clone(),
        artifact_role: query.artifact_role.clone(),
        media_type: query.media_type.clone(),
        retention_policy_id: query.retention_policy_id.clone(),
        runtime_id: query.runtime_id.clone(),
        selected_backend_key: query.selected_backend_key.clone(),
        model_id: query.model_id.clone(),
    }
}

impl WorkflowNodeStatusQueryRequest {
    fn into_node_status_query(self) -> Result<NodeStatusProjectionQuery, WorkflowServiceError> {
        let query = NodeStatusProjectionQuery {
            workflow_run_id: parse_optional_id("workflow_run_id", self.workflow_run_id)?,
            node_id: self.node_id,
            status: self.status,
            after_event_seq: self.after_event_seq,
            limit: resolve_positive_optional_u32("limit", self.limit, 250)?,
        };
        query.validate(500).map_err(WorkflowServiceError::from)?;
        Ok(query)
    }
}

impl WorkflowLibraryUsageQueryRequest {
    fn into_library_usage_query(self) -> Result<LibraryUsageProjectionQuery, WorkflowServiceError> {
        let query = LibraryUsageProjectionQuery {
            asset_id: self.asset_id,
            workflow_run_id: parse_optional_id("workflow_run_id", self.workflow_run_id)?,
            workflow_id: parse_optional_id("workflow_id", self.workflow_id)?,
            workflow_version_id: parse_optional_id(
                "workflow_version_id",
                self.workflow_version_id,
            )?,
            after_event_seq: self.after_event_seq,
            limit: resolve_positive_optional_u32("limit", self.limit, 100)?,
        };
        query.validate(500).map_err(WorkflowServiceError::from)?;
        Ok(query)
    }
}

fn resolve_positive_optional_u32(
    field: &'static str,
    value: Option<u32>,
    default_value: u32,
) -> Result<u32, WorkflowServiceError> {
    let resolved = value.unwrap_or(default_value);
    if resolved == 0 {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "{field} must be greater than zero"
        )));
    }
    Ok(resolved)
}

fn parse_id<T>(field: &'static str, value: String) -> Result<T, WorkflowServiceError>
where
    T: TryFrom<String>,
    T::Error: std::fmt::Display,
{
    T::try_from(value).map_err(|error| {
        WorkflowServiceError::InvalidRequest(format!("invalid {}: {}", field, error))
    })
}

fn parse_optional_id<T>(
    field: &'static str,
    value: Option<String>,
) -> Result<Option<T>, WorkflowServiceError>
where
    T: TryFrom<String>,
    T::Error: std::fmt::Display,
{
    value
        .map(|value| {
            T::try_from(value).map_err(|error| {
                WorkflowServiceError::InvalidRequest(format!("invalid {}: {}", field, error))
            })
        })
        .transpose()
}

fn resolve_workflow_run_node_io(
    run_graph: &Option<WorkflowRunGraphProjection>,
    artifacts: &[IoArtifactProjectionRecord],
) -> Vec<ResolvedNodeIoRecord> {
    let mut output_by_node_port: BTreeMap<(String, String), &IoArtifactProjectionRecord> =
        BTreeMap::new();
    for artifact in artifacts {
        if let (Some(node_id), Some(port_id)) = (
            artifact.producer_node_id.as_ref(),
            artifact.producer_port_id.as_ref(),
        ) {
            output_by_node_port
                .entry((node_id.clone(), port_id.clone()))
                .or_insert(artifact);
        }
    }

    let mut derived_input_ports = BTreeSet::new();
    let mut resolved = Vec::new();
    if let Some(run_graph) = run_graph {
        for edge in &run_graph.graph.edges {
            if let Some(upstream) =
                output_by_node_port.get(&(edge.source.clone(), edge.source_handle.clone()))
            {
                derived_input_ports.insert((edge.target.clone(), edge.target_handle.clone()));
                resolved.push(ResolvedNodeIoRecord {
                    node_id: edge.target.clone(),
                    port_id: edge.target_handle.clone(),
                    direction: ResolvedNodeIoDirection::Input,
                    resolution: ResolvedNodeIoResolution::DerivedFromEdge,
                    provenance_kind: ResolvedNodeIoProvenanceKind::GraphEdge,
                    artifact_fact_id: Some(upstream.artifact_fact_id.clone()),
                    payload_artifact_id: Some(upstream.payload_artifact_id.clone()),
                    artifact_id: Some(upstream.artifact_id.clone()),
                    artifact_role: Some(upstream.artifact_role.clone()),
                    upstream_node_id: Some(edge.source.clone()),
                    upstream_port_id: Some(edge.source_handle.clone()),
                    media_type: upstream.media_type.clone(),
                    retention_state: Some(upstream.retention_state),
                });
            }
        }
    }

    for artifact in artifacts {
        if let (Some(node_id), Some(port_id)) = (
            artifact.producer_node_id.as_ref(),
            artifact.producer_port_id.as_ref(),
        ) {
            let is_workflow_boundary = artifact.artifact_role == "workflow_output";
            resolved.push(ResolvedNodeIoRecord {
                node_id: node_id.clone(),
                port_id: port_id.clone(),
                direction: ResolvedNodeIoDirection::Output,
                resolution: if is_workflow_boundary {
                    ResolvedNodeIoResolution::WorkflowBoundary
                } else {
                    ResolvedNodeIoResolution::ProducedOutput
                },
                provenance_kind: if is_workflow_boundary {
                    ResolvedNodeIoProvenanceKind::WorkflowOutputBoundary
                } else {
                    ResolvedNodeIoProvenanceKind::ProducedOutput
                },
                artifact_fact_id: Some(artifact.artifact_fact_id.clone()),
                payload_artifact_id: Some(artifact.payload_artifact_id.clone()),
                artifact_id: Some(artifact.artifact_id.clone()),
                artifact_role: Some(artifact.artifact_role.clone()),
                upstream_node_id: None,
                upstream_port_id: None,
                media_type: artifact.media_type.clone(),
                retention_state: Some(artifact.retention_state),
            });
        }

        if let (Some(node_id), Some(port_id)) = (
            artifact.consumer_node_id.as_ref(),
            artifact.consumer_port_id.as_ref(),
        ) {
            if artifact.artifact_role == "node_input"
                && derived_input_ports.contains(&(node_id.clone(), port_id.clone()))
            {
                continue;
            }
            let is_workflow_boundary = artifact.artifact_role == "workflow_input";
            resolved.push(ResolvedNodeIoRecord {
                node_id: node_id.clone(),
                port_id: port_id.clone(),
                direction: ResolvedNodeIoDirection::Input,
                resolution: if is_workflow_boundary {
                    ResolvedNodeIoResolution::WorkflowBoundary
                } else {
                    ResolvedNodeIoResolution::ExplicitInput
                },
                provenance_kind: if is_workflow_boundary {
                    ResolvedNodeIoProvenanceKind::WorkflowInputBoundary
                } else {
                    ResolvedNodeIoProvenanceKind::ExplicitInput
                },
                artifact_fact_id: Some(artifact.artifact_fact_id.clone()),
                payload_artifact_id: Some(artifact.payload_artifact_id.clone()),
                artifact_id: Some(artifact.artifact_id.clone()),
                artifact_role: Some(artifact.artifact_role.clone()),
                upstream_node_id: None,
                upstream_port_id: None,
                media_type: artifact.media_type.clone(),
                retention_state: Some(artifact.retention_state),
            });
        }
    }

    resolved
}

fn projection_error_scope(
    projection_name: impl Into<String>,
    operation: impl Into<String>,
    workflow_run_id: Option<WorkflowRunId>,
    workflow_id: Option<WorkflowId>,
) -> WorkflowDiagnosticProjectionScope {
    WorkflowDiagnosticProjectionScope {
        workflow_run_id,
        workflow_id,
        projection_name: projection_name.into(),
        operation: operation.into(),
    }
}

fn summarize_usage(events: &[ModelLicenseUsageEvent]) -> Vec<WorkflowDiagnosticsUsageSummary> {
    let mut summaries = Vec::<WorkflowDiagnosticsUsageSummary>::new();
    for event in events {
        if let Some(summary) = summaries.iter_mut().find(|summary| {
            summary.model_id == event.model.model_id
                && summary.license_value == event.license_snapshot.license_value
                && summary.guarantee_level == event.guarantee_level
        }) {
            summary.event_count += 1;
        } else {
            summaries.push(WorkflowDiagnosticsUsageSummary {
                model_id: event.model.model_id.clone(),
                license_value: event.license_snapshot.license_value.clone(),
                guarantee_level: event.guarantee_level,
                event_count: 1,
            });
        }
    }

    summaries
}

import type {
  GraphMemoryImpactSummary,
  WorkflowGraphSessionStateView,
  WorkflowCapabilityModel,
  WorkflowRuntimeCapability,
  WorkflowRuntimeRequirements,
  WorkflowSessionQueueItem,
  WorkflowSessionSummary,
} from '../workflow/types';

export const DIAGNOSTICS_TABS = [
  'overview',
  'timeline',
  'events',
  'scheduler',
  'runtime',
  'graph',
] as const;

export type DiagnosticsTab = (typeof DIAGNOSTICS_TABS)[number];

export type DiagnosticsRunStatus =
  | 'running'
  | 'waiting'
  | 'completed'
  | 'cancelled'
  | 'failed';
export type DiagnosticsNodeStatus =
  | 'running'
  | 'waiting'
  | 'completed'
  | 'cancelled'
  | 'failed';

export type WorkflowTimingExpectationComparison =
  | 'insufficient_history'
  | 'no_current_duration'
  | 'faster_than_expected'
  | 'within_expected_range'
  | 'slower_than_expected';

export interface WorkflowTimingExpectation {
  comparison: WorkflowTimingExpectationComparison;
  sampleCount: number;
  currentDurationMs: number | null;
  medianDurationMs: number | null;
  typicalMinDurationMs: number | null;
  typicalMaxDurationMs: number | null;
}

export interface DiagnosticsWorkflowNodeTimingHistory {
  nodeId: string;
  nodeType: string | null;
  timingExpectation?: WorkflowTimingExpectation | null;
}

export interface DiagnosticsWorkflowTimingHistory {
  workflowId: string;
  graphFingerprint: string | null;
  timingExpectation?: WorkflowTimingExpectation | null;
  nodes: Record<string, DiagnosticsWorkflowNodeTimingHistory>;
}

export interface DiagnosticsEventRecord {
  id: string;
  sequence: number;
  timestampMs: number;
  type: string;
  workflowRunId: string;
  workflowId: string | null;
  nodeId: string | null;
  summary: string;
  payload: unknown;
}

export interface DiagnosticsNodeTrace {
  nodeId: string;
  nodeType: string | null;
  status: DiagnosticsNodeStatus;
  startedAtMs: number | null;
  endedAtMs: number | null;
  durationMs: number | null;
  lastProgress: number | null;
  lastMessage: string | null;
  streamEventCount: number;
  eventCount: number;
  error: string | null;
  timingExpectation?: WorkflowTimingExpectation | null;
}

export interface DiagnosticsRunTrace {
  workflowRunId: string;
  sessionId: string | null;
  workflowId: string | null;
  graphFingerprintAtStart: string | null;
  nodeCountAtStart: number;
  status: DiagnosticsRunStatus;
  startedAtMs: number;
  endedAtMs: number | null;
  durationMs: number | null;
  lastUpdatedAtMs: number;
  error: string | null;
  waitingForInput: boolean;
  runtime: DiagnosticsTraceRuntimeMetrics;
  eventCount: number;
  streamEventCount: number;
  lastDirtyTasks: string[];
  lastIncrementalTaskIds: string[];
  lastGraphMemoryImpact: GraphMemoryImpactSummary | null;
  timingExpectation?: WorkflowTimingExpectation | null;
  nodes: Record<string, DiagnosticsNodeTrace>;
  events: DiagnosticsEventRecord[];
}

export interface DiagnosticsTraceRuntimeMetrics {
  runtimeId: string | null;
  runtimeInstanceId: string | null;
  modelTarget: string | null;
  warmupStartedAtMs: number | null;
  warmupCompletedAtMs: number | null;
  warmupDurationMs: number | null;
  runtimeReused: boolean | null;
  lifecycleDecisionReason: string | null;
}

export interface DiagnosticsRuntimeSnapshot {
  workflowId: string | null;
  capturedAtMs: number | null;
  maxInputBindings: number | null;
  maxOutputTargets: number | null;
  maxValueBytes: number | null;
  runtimeRequirements: WorkflowRuntimeRequirements | null;
  runtimeCapabilities: WorkflowRuntimeCapability[];
  models: WorkflowCapabilityModel[];
  lastError: string | null;
  activeModelTarget: string | null;
  embeddingModelTarget: string | null;
  activeRuntime: DiagnosticsRuntimeLifecycleSnapshot | null;
  embeddingRuntime: DiagnosticsRuntimeLifecycleSnapshot | null;
}

export interface DiagnosticsRuntimeLifecycleSnapshot {
  runtimeId: string | null;
  runtimeInstanceId: string | null;
  warmupStartedAtMs: number | null;
  warmupCompletedAtMs: number | null;
  warmupDurationMs: number | null;
  runtimeReused: boolean | null;
  lifecycleDecisionReason: string | null;
  active: boolean;
  lastError: string | null;
}

export interface DiagnosticsSchedulerSnapshot {
  workflowId: string | null;
  sessionId: string | null;
  workflowRunId: string | null;
  capturedAtMs: number | null;
  session: WorkflowSessionSummary | null;
  items: WorkflowSessionQueueItem[];
  lastError: string | null;
}

export interface WorkflowDiagnosticsProjectionContext {
  requestedWorkflowRunId: string | null;
  requestedSessionId: string | null;
  requestedWorkflowId: string | null;
  sourceWorkflowRunId: string | null;
  relevantWorkflowRunId: string | null;
  relevant: boolean;
}

export interface WorkflowDiagnosticsProjection {
  context: WorkflowDiagnosticsProjectionContext;
  runsById: Record<string, DiagnosticsRunTrace>;
  runOrder: string[];
  runtime: DiagnosticsRuntimeSnapshot;
  scheduler: DiagnosticsSchedulerSnapshot;
  currentSessionState: WorkflowGraphSessionStateView | null;
  workflowTimingHistory: DiagnosticsWorkflowTimingHistory | null;
  retainedEventLimit: number;
}

export type WorkflowTraceStatus =
  | 'queued'
  | 'running'
  | 'waiting'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type WorkflowTraceNodeStatus =
  | 'pending'
  | 'running'
  | 'waiting'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface WorkflowTraceQueueMetrics {
  enqueued_at_ms?: number | null;
  dequeued_at_ms?: number | null;
  queue_wait_ms?: number | null;
  scheduler_decision_reason?: string | null;
}

export interface WorkflowTraceRuntimeMetrics {
  runtime_id?: string | null;
  observed_runtime_ids?: string[];
  runtime_instance_id?: string | null;
  model_target?: string | null;
  warmup_started_at_ms?: number | null;
  warmup_completed_at_ms?: number | null;
  warmup_duration_ms?: number | null;
  runtime_reused?: boolean | null;
  lifecycle_decision_reason?: string | null;
}

export interface WorkflowTraceNodeRecord {
  node_id: string;
  node_type?: string | null;
  status: WorkflowTraceNodeStatus;
  started_at_ms?: number | null;
  ended_at_ms?: number | null;
  duration_ms?: number | null;
  event_count: number;
  stream_event_count: number;
  last_error?: string | null;
}

export interface WorkflowTraceSummary {
  workflow_run_id: string;
  session_id?: string | null;
  workflow_id?: string | null;
  graph_fingerprint?: string | null;
  status: WorkflowTraceStatus;
  started_at_ms: number;
  ended_at_ms?: number | null;
  duration_ms?: number | null;
  queue: WorkflowTraceQueueMetrics;
  runtime: WorkflowTraceRuntimeMetrics;
  node_count_at_start: number;
  event_count: number;
  stream_event_count: number;
  last_dirty_tasks?: string[];
  last_incremental_task_ids?: string[];
  last_graph_memory_impact?: GraphMemoryImpactSummary | null;
  waiting_for_input: boolean;
  last_error?: string | null;
  nodes: WorkflowTraceNodeRecord[];
}

export interface WorkflowTraceSnapshotRequest {
  workflow_run_id?: string | null;
  session_id?: string | null;
  workflow_id?: string | null;
  include_completed?: boolean | null;
}

export interface WorkflowTraceSnapshotResponse {
  traces: WorkflowTraceSummary[];
  retained_trace_limit: number;
}

export type ProjectionStatus = 'current' | 'rebuilding' | 'needs_rebuild' | 'failed';

export interface ProjectionStateRecord {
  projection_name: string;
  projection_version: number;
  last_applied_event_seq: number;
  status: ProjectionStatus;
  rebuilt_at_ms?: number | null;
  updated_at_ms: number;
}

export type DiagnosticEventKind =
  | 'scheduler_estimate_produced'
  | 'scheduler_queue_placement'
  | 'run_started'
  | 'run_terminal'
  | 'run_snapshot_accepted'
  | 'io_artifact_observed'
  | 'library_asset_accessed'
  | 'retention_policy_changed'
  | 'runtime_capability_observed';

export type DiagnosticEventSourceComponent =
  | 'scheduler'
  | 'workflow_service'
  | 'runtime'
  | 'node_execution'
  | 'retention'
  | 'library'
  | 'local_observer';

export interface SchedulerTimelineProjectionRecord {
  event_seq: number;
  event_id: string;
  event_kind: DiagnosticEventKind;
  source_component: DiagnosticEventSourceComponent;
  occurred_at_ms: number;
  recorded_at_ms: number;
  workflow_run_id: string;
  workflow_id: string;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  scheduler_policy_id?: string | null;
  retention_policy_id?: string | null;
  summary: string;
  detail?: string | null;
  payload_json: string;
}

export type RunListProjectionStatus =
  | 'accepted'
  | 'queued'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface RunListProjectionRecord {
  workflow_run_id: string;
  workflow_id: string;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  status: RunListProjectionStatus;
  accepted_at_ms?: number | null;
  enqueued_at_ms?: number | null;
  started_at_ms?: number | null;
  completed_at_ms?: number | null;
  duration_ms?: number | null;
  scheduler_policy_id?: string | null;
  retention_policy_id?: string | null;
  last_event_seq: number;
  last_updated_at_ms: number;
}

export interface RunDetailProjectionRecord extends RunListProjectionRecord {
  client_id?: string | null;
  client_session_id?: string | null;
  bucket_id?: string | null;
  workflow_run_snapshot_id?: string | null;
  workflow_presentation_revision_id?: string | null;
  latest_estimate_json?: string | null;
  latest_queue_placement_json?: string | null;
  started_payload_json?: string | null;
  terminal_payload_json?: string | null;
  terminal_error?: string | null;
  timeline_event_count: number;
}

export interface IoArtifactProjectionRecord {
  event_seq: number;
  event_id: string;
  occurred_at_ms: number;
  recorded_at_ms: number;
  workflow_run_id: string;
  workflow_id: string;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  node_id?: string | null;
  node_type?: string | null;
  node_version?: string | null;
  runtime_id?: string | null;
  runtime_version?: string | null;
  model_id?: string | null;
  model_version?: string | null;
  artifact_id: string;
  artifact_role: string;
  media_type?: string | null;
  size_bytes?: number | null;
  content_hash?: string | null;
  payload_ref?: string | null;
  retention_policy_id?: string | null;
}

export interface WorkflowRunListQueryRequest {
  workflow_id?: string | null;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  status?: RunListProjectionStatus | null;
  scheduler_policy_id?: string | null;
  after_event_seq?: number | null;
  limit?: number | null;
  projection_batch_size?: number | null;
}

export interface WorkflowRunListQueryResponse {
  runs: RunListProjectionRecord[];
  projection_state: ProjectionStateRecord;
}

export interface WorkflowRunDetailQueryRequest {
  workflow_run_id: string;
  projection_batch_size?: number | null;
}

export interface WorkflowRunDetailQueryResponse {
  run?: RunDetailProjectionRecord | null;
  projection_state: ProjectionStateRecord;
}

export interface WorkflowIoArtifactQueryRequest {
  workflow_run_id?: string | null;
  node_id?: string | null;
  artifact_role?: string | null;
  media_type?: string | null;
  retention_policy_id?: string | null;
  runtime_id?: string | null;
  model_id?: string | null;
  after_event_seq?: number | null;
  limit?: number | null;
  projection_batch_size?: number | null;
}

export interface WorkflowIoArtifactQueryResponse {
  artifacts: IoArtifactProjectionRecord[];
  projection_state: ProjectionStateRecord;
}

export interface WorkflowProjectionRebuildRequest {
  projection_name: string;
  batch_size?: number | null;
}

export interface WorkflowProjectionRebuildResponse {
  projection_state: ProjectionStateRecord;
}

export interface LibraryUsageProjectionRecord {
  asset_id: string;
  total_access_count: number;
  run_access_count: number;
  total_network_bytes: number;
  last_accessed_at_ms: number;
  last_operation: string;
  last_cache_status?: string | null;
  last_workflow_run_id?: string | null;
  last_workflow_id?: string | null;
  last_workflow_version_id?: string | null;
  last_workflow_semantic_version?: string | null;
  last_client_id?: string | null;
  last_client_session_id?: string | null;
  last_bucket_id?: string | null;
  last_event_seq: number;
  last_updated_at_ms: number;
}

export interface WorkflowLibraryUsageQueryRequest {
  asset_id?: string | null;
  workflow_id?: string | null;
  workflow_version_id?: string | null;
  after_event_seq?: number | null;
  limit?: number | null;
  projection_batch_size?: number | null;
}

export interface WorkflowLibraryUsageQueryResponse {
  assets: LibraryUsageProjectionRecord[];
  projection_state: ProjectionStateRecord;
}

export type DiagnosticsRetentionClass = 'standard';

export interface DiagnosticsRetentionPolicy {
  policy_id: string;
  retention_class: DiagnosticsRetentionClass;
  retention_days: number;
  applied_at_ms: number;
  explanation: string;
}

export type WorkflowRetentionPolicyQueryRequest = Record<string, never>;

export interface WorkflowRetentionPolicyQueryResponse {
  retention_policy: DiagnosticsRetentionPolicy;
}

export interface WorkflowRetentionPolicyUpdateRequest {
  retention_days: number;
  explanation: string;
  reason: string;
}

export interface WorkflowRetentionPolicyUpdateResponse {
  retention_policy: DiagnosticsRetentionPolicy;
}

export interface WorkflowSchedulerTimelineQueryRequest {
  workflow_run_id?: string | null;
  workflow_id?: string | null;
  scheduler_policy_id?: string | null;
  after_event_seq?: number | null;
  limit?: number | null;
  projection_batch_size?: number | null;
}

export interface WorkflowSchedulerTimelineQueryResponse {
  events: SchedulerTimelineProjectionRecord[];
  projection_state: ProjectionStateRecord;
}

export interface WorkflowDiagnosticsState extends WorkflowDiagnosticsProjection {
  panelOpen: boolean;
  activeTab: DiagnosticsTab;
  selectedRunId: string | null;
  selectedNodeId: string | null;
  currentSessionId: string | null;
  currentWorkflowId: string | null;
  currentGraphFingerprint: string | null;
  currentGraphNodeCount: number;
  currentGraphEdgeCount: number;
}

export interface DiagnosticsSnapshot {
  state: WorkflowDiagnosticsState;
  selectedRun: DiagnosticsRunTrace | null;
  selectedNode: DiagnosticsNodeTrace | null;
}

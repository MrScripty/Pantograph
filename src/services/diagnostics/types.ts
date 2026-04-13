import type {
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

export interface DiagnosticsEventRecord {
  id: string;
  sequence: number;
  timestampMs: number;
  type: string;
  executionId: string;
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
}

export interface DiagnosticsRunTrace {
  executionId: string;
  sessionId: string | null;
  workflowId: string | null;
  workflowName: string | null;
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
  traceExecutionId: string | null;
  capturedAtMs: number | null;
  session: WorkflowSessionSummary | null;
  items: WorkflowSessionQueueItem[];
  lastError: string | null;
}

export interface WorkflowDiagnosticsProjection {
  runsById: Record<string, DiagnosticsRunTrace>;
  runOrder: string[];
  runtime: DiagnosticsRuntimeSnapshot;
  scheduler: DiagnosticsSchedulerSnapshot;
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
  execution_id: string;
  session_id?: string | null;
  workflow_id?: string | null;
  workflow_name?: string | null;
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
  waiting_for_input: boolean;
  last_error?: string | null;
  nodes: WorkflowTraceNodeRecord[];
}

export interface WorkflowTraceSnapshotRequest {
  execution_id?: string | null;
  session_id?: string | null;
  workflow_id?: string | null;
  include_completed?: boolean | null;
}

export interface WorkflowTraceSnapshotResponse {
  traces: WorkflowTraceSummary[];
  retained_trace_limit: number;
}

export interface WorkflowDiagnosticsState extends WorkflowDiagnosticsProjection {
  panelOpen: boolean;
  activeTab: DiagnosticsTab;
  selectedRunId: string | null;
  selectedNodeId: string | null;
  currentSessionId: string | null;
  currentWorkflowId: string | null;
  currentWorkflowName: string | null;
  currentGraphFingerprint: string | null;
  currentGraphNodeCount: number;
  currentGraphEdgeCount: number;
}

export interface DiagnosticsSnapshot {
  state: WorkflowDiagnosticsState;
  selectedRun: DiagnosticsRunTrace | null;
  selectedNode: DiagnosticsNodeTrace | null;
}

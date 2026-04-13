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

export type DiagnosticsRunStatus = 'running' | 'waiting' | 'completed' | 'failed';
export type DiagnosticsNodeStatus = 'running' | 'waiting' | 'completed' | 'failed';

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
  eventCount: number;
  streamEventCount: number;
  lastDirtyTasks: string[];
  lastIncrementalTaskIds: string[];
  nodes: Record<string, DiagnosticsNodeTrace>;
  events: DiagnosticsEventRecord[];
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
}

export interface DiagnosticsSchedulerSnapshot {
  workflowId: string | null;
  sessionId: string | null;
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

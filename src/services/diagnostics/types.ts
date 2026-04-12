import type { WorkflowEvent, WorkflowEventData, WorkflowEventType, WorkflowGraph } from '../workflow/types';

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

export interface DiagnosticsWorkflowContext {
  workflowId: string | null;
  workflowName: string | null;
  graphFingerprint: string | null;
  graph: WorkflowGraph | null;
  nodeTypesById: Record<string, string>;
}

export interface DiagnosticsEventRecord<T extends WorkflowEventType = WorkflowEventType> {
  id: string;
  sequence: number;
  timestampMs: number;
  type: T;
  executionId: string;
  workflowId: string | null;
  nodeId: string | null;
  summary: string;
  payload: WorkflowEventData[T];
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

export interface WorkflowDiagnosticsState {
  panelOpen: boolean;
  activeTab: DiagnosticsTab;
  selectedRunId: string | null;
  selectedNodeId: string | null;
  currentWorkflowId: string | null;
  currentWorkflowName: string | null;
  currentGraphFingerprint: string | null;
  runsById: Record<string, DiagnosticsRunTrace>;
  runOrder: string[];
  retainedEventLimit: number;
}

export interface DiagnosticsSnapshot {
  state: WorkflowDiagnosticsState;
  selectedRun: DiagnosticsRunTrace | null;
  selectedNode: DiagnosticsNodeTrace | null;
}

export type DiagnosticsEventPayload = WorkflowEvent;

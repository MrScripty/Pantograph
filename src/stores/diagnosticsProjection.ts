import type {
  DiagnosticsNodeTrace,
  DiagnosticsRunTrace,
  DiagnosticsSnapshot,
  DiagnosticsTab,
  WorkflowDiagnosticsProjectionContext,
  WorkflowDiagnosticsProjection,
} from '../services/diagnostics/types';
import type { WorkflowGraph } from '../services/workflow/types';

export type DiagnosticsUiState = {
  panelOpen: boolean;
  activeTab: DiagnosticsTab;
  selectedRunId: string | null;
  selectedNodeId: string | null;
};

type NormalizeDiagnosticsUiStateParams = {
  projection: WorkflowDiagnosticsProjection;
  uiState: DiagnosticsUiState;
  workflowChanged?: boolean;
};

function isSelectableDiagnosticsRun(
  projection: WorkflowDiagnosticsProjection,
  runId: string | null,
): runId is string {
  if (!runId) {
    return false;
  }

  const run = projection.runsById[runId] ?? null;
  return run !== null && run.workflowRunId === runId;
}

export function normalizeDiagnosticsUiState({
  projection,
  uiState,
  workflowChanged = false,
}: NormalizeDiagnosticsUiStateParams): DiagnosticsUiState {
  let selectedRunId = workflowChanged ? null : uiState.selectedRunId;
  let selectedNodeId = workflowChanged ? null : uiState.selectedNodeId;

  if (!isSelectableDiagnosticsRun(projection, selectedRunId)) {
    selectedRunId = null;
    selectedNodeId = null;
  }

  if (
    !workflowChanged &&
    selectedRunId === null &&
    isSelectableDiagnosticsRun(projection, projection.context.relevantWorkflowRunId)
  ) {
    selectedRunId = projection.context.relevantWorkflowRunId;
  }

  const selectedRun = selectedRunId ? projection.runsById[selectedRunId] ?? null : null;
  if (!selectedRun) {
    selectedNodeId = null;
  } else if (selectedNodeId !== null && !(selectedNodeId in selectedRun.nodes)) {
    selectedNodeId = null;
  }

  return {
    ...uiState,
    selectedRunId,
    selectedNodeId,
  };
}

export function createEmptyDiagnosticsProjection(): WorkflowDiagnosticsProjection {
  return {
    context: createDefaultDiagnosticsProjectionContext(),
    runsById: {},
    runOrder: [],
    runtime: {
      workflowId: null,
      capturedAtMs: null,
      maxInputBindings: null,
      maxOutputTargets: null,
      maxValueBytes: null,
      runtimeRequirements: null,
      runtimeCapabilities: [],
      models: [],
      lastError: null,
      activeModelTarget: null,
      embeddingModelTarget: null,
      activeRuntime: null,
      embeddingRuntime: null,
    },
    scheduler: {
      workflowId: null,
      sessionId: null,
      workflowRunId: null,
      capturedAtMs: null,
      session: null,
      items: [],
      lastError: null,
    },
    currentSessionState: null,
    workflowTimingHistory: null,
    retainedEventLimit: 200,
  };
}

function createDefaultDiagnosticsProjectionContext(
  previous?: WorkflowDiagnosticsProjection,
): WorkflowDiagnosticsProjectionContext {
  return {
    requestedWorkflowRunId: previous?.context?.requestedWorkflowRunId ?? null,
    requestedSessionId: previous?.context?.requestedSessionId ?? null,
    requestedWorkflowId: previous?.context?.requestedWorkflowId ?? null,
    sourceWorkflowRunId: null,
    relevantWorkflowRunId: previous?.context?.relevantWorkflowRunId ?? null,
    relevant: true,
  };
}

export function normalizeDiagnosticsProjection(
  incoming: WorkflowDiagnosticsProjection,
  previous: WorkflowDiagnosticsProjection,
): WorkflowDiagnosticsProjection {
  const projection = {
    ...incoming,
    context: incoming.context ?? createDefaultDiagnosticsProjectionContext(previous),
  };

  return {
    ...projection,
    currentSessionState: Object.prototype.hasOwnProperty.call(
      incoming,
      'currentSessionState',
    )
      ? incoming.currentSessionState
      : previous.currentSessionState,
    workflowTimingHistory: Object.prototype.hasOwnProperty.call(
      incoming,
      'workflowTimingHistory',
    )
      ? incoming.workflowTimingHistory
      : previous.workflowTimingHistory,
  };
}

type SnapshotParams = {
  projection: WorkflowDiagnosticsProjection;
  uiState: DiagnosticsUiState;
  workflowId: string | null;
  workflowGraph: WorkflowGraph | null;
  sessionId: string | null;
};

export function createDiagnosticsSnapshot({
  projection,
  uiState,
  workflowId,
  workflowGraph,
  sessionId,
}: SnapshotParams): DiagnosticsSnapshot {
  const selectedRunId = uiState.selectedRunId;
  const selectedRun: DiagnosticsRunTrace | null = selectedRunId
    ? projection.runsById[selectedRunId] ?? null
    : null;
  const selectedNode: DiagnosticsNodeTrace | null =
    selectedRun && uiState.selectedNodeId
      ? selectedRun.nodes[uiState.selectedNodeId] ?? null
      : null;

  return {
    state: {
      ...projection,
      ...uiState,
      currentSessionId: sessionId,
      currentWorkflowId: workflowId,
      currentGraphFingerprint: workflowGraph?.derived_graph?.graph_fingerprint ?? null,
      currentGraphNodeCount: workflowGraph?.nodes.length ?? 0,
      currentGraphEdgeCount: workflowGraph?.edges.length ?? 0,
    },
    selectedRun,
    selectedNode,
  };
}

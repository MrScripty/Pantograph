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
      traceExecutionId: null,
      capturedAtMs: null,
      session: null,
      items: [],
      lastError: null,
    },
    currentSessionState: null,
    retainedEventLimit: 200,
  };
}

function createDefaultDiagnosticsProjectionContext(
  previous?: WorkflowDiagnosticsProjection,
): WorkflowDiagnosticsProjectionContext {
  return {
    requestedSessionId: previous?.context?.requestedSessionId ?? null,
    requestedWorkflowId: previous?.context?.requestedWorkflowId ?? null,
    requestedWorkflowName: previous?.context?.requestedWorkflowName ?? null,
    sourceExecutionId: null,
    relevantExecutionId: previous?.context?.relevantExecutionId ?? null,
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
  };
}

type SnapshotParams = {
  projection: WorkflowDiagnosticsProjection;
  uiState: DiagnosticsUiState;
  workflowId: string | null;
  workflowName: string | null;
  workflowGraph: WorkflowGraph | null;
  sessionId: string | null;
};

export function createDiagnosticsSnapshot({
  projection,
  uiState,
  workflowId,
  workflowName,
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
      currentWorkflowName: workflowName,
      currentGraphFingerprint: workflowGraph?.derived_graph?.graph_fingerprint ?? null,
      currentGraphNodeCount: workflowGraph?.nodes.length ?? 0,
      currentGraphEdgeCount: workflowGraph?.edges.length ?? 0,
    },
    selectedRun,
    selectedNode,
  };
}

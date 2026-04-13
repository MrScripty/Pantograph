import { writable, type Readable } from 'svelte/store';
import {
  claimWorkflowExecutionIdFromEvent,
  isWorkflowEventRelevantToExecution,
} from '@pantograph/svelte-graph';

import type {
  DiagnosticsSnapshot,
  DiagnosticsTab,
  WorkflowDiagnosticsProjection,
  WorkflowDiagnosticsState,
} from '../services/diagnostics/types';
import type { WorkflowGraph, WorkflowEvent } from '../services/workflow/types';
import { workflowGraph } from './workflowStore';
import { currentGraphId, currentGraphName } from './graphSessionStore';
import { workflowService } from '../services/workflow/WorkflowService';
import { sessionStores } from './storeInstances';

type DiagnosticsUiState = Pick<
  WorkflowDiagnosticsState,
  'panelOpen' | 'activeTab' | 'selectedRunId' | 'selectedNodeId'
>;

const DEFAULT_UI_STATE: DiagnosticsUiState = {
  panelOpen: false,
  activeTab: 'overview',
  selectedRunId: null,
  selectedNodeId: null,
};

function createEmptyProjection(): WorkflowDiagnosticsProjection {
  return {
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
    retainedEventLimit: 200,
  };
}

let latestProjection: WorkflowDiagnosticsProjection = createEmptyProjection();
let latestWorkflowId: string | null = null;
let latestWorkflowName: string | null = null;
let latestWorkflowGraph: WorkflowGraph | null = null;
let latestSessionId: string | null = null;
let uiState: DiagnosticsUiState = { ...DEFAULT_UI_STATE };

function createSnapshot(): DiagnosticsSnapshot {
  const selectedRunId = uiState.selectedRunId;
  const selectedRun = selectedRunId
    ? latestProjection.runsById[selectedRunId] ?? null
    : null;
  const selectedNode = selectedRun && uiState.selectedNodeId
    ? selectedRun.nodes[uiState.selectedNodeId] ?? null
    : null;

  return {
    state: {
      ...latestProjection,
      ...uiState,
      currentSessionId: latestSessionId,
      currentWorkflowId: latestWorkflowId,
      currentWorkflowName: latestWorkflowName,
      currentGraphFingerprint: latestWorkflowGraph?.derived_graph?.graph_fingerprint ?? null,
      currentGraphNodeCount: latestWorkflowGraph?.nodes.length ?? 0,
      currentGraphEdgeCount: latestWorkflowGraph?.edges.length ?? 0,
    },
    selectedRun,
    selectedNode,
  };
}

function normalizeUiSelections(): void {
  if (
    uiState.selectedRunId !== null &&
    !(uiState.selectedRunId in latestProjection.runsById)
  ) {
    uiState.selectedRunId = null;
    uiState.selectedNodeId = null;
  }

  if (uiState.selectedRunId === null && latestProjection.runOrder.length > 0) {
    uiState.selectedRunId = latestProjection.runOrder[0] ?? null;
  }

  const selectedRun = uiState.selectedRunId
    ? latestProjection.runsById[uiState.selectedRunId] ?? null
    : null;
  if (!selectedRun) {
    uiState.selectedNodeId = null;
    return;
  }

  if (
    uiState.selectedNodeId !== null &&
    !(uiState.selectedNodeId in selectedRun.nodes)
  ) {
    uiState.selectedNodeId = null;
  }
}

function applyProjection(projection: WorkflowDiagnosticsProjection): void {
  latestProjection = projection;
  normalizeUiSelections();
  diagnosticsSnapshotStore.set(createSnapshot());
}

function emitSnapshot(): void {
  normalizeUiSelections();
  diagnosticsSnapshotStore.set(createSnapshot());
}

const diagnosticsSnapshotStore = writable<DiagnosticsSnapshot>(createSnapshot());

let workflowEventUnsubscribe: (() => void) | null = null;
let workflowGraphUnsubscribe: (() => void) | null = null;
let workflowIdUnsubscribe: (() => void) | null = null;
let workflowNameUnsubscribe: (() => void) | null = null;
let sessionIdUnsubscribe: (() => void) | null = null;
let diagnosticsStarted = false;
let refreshToken = 0;

function normalizeError(error: unknown): string {
  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message;
  }
  if (typeof error === 'string' && error.trim().length > 0) {
    return error;
  }
  return String(error);
}

async function refreshDiagnosticsProjection(): Promise<void> {
  const token = ++refreshToken;

  try {
    const projection = await workflowService.getDiagnosticsSnapshot(
      latestWorkflowId,
      latestWorkflowName,
      latestSessionId,
    );
    if (token !== refreshToken) {
      return;
    }
    applyProjection(projection);
  } catch (error) {
    if (token !== refreshToken) {
      return;
    }
    applyProjection({
      ...latestProjection,
      runtime: {
        ...latestProjection.runtime,
        workflowId: latestWorkflowId,
        lastError: normalizeError(error),
      },
      scheduler: {
        ...latestProjection.scheduler,
        workflowId: latestWorkflowId,
        sessionId: latestSessionId,
        lastError: normalizeError(error),
      },
    });
  }
}

function isDiagnosticsSnapshotEvent(
  event: WorkflowEvent,
): event is WorkflowEvent<'DiagnosticsSnapshot'> {
  return event.type === 'DiagnosticsSnapshot';
}

function bindDiagnosticsStore(): void {
  workflowEventUnsubscribe = workflowService.subscribeEvents((event) => {
    const currentExecutionId = workflowService.getCurrentExecutionId() ?? latestSessionId;
    const expectedExecutionId = claimWorkflowExecutionIdFromEvent(event, currentExecutionId);
    if (!isWorkflowEventRelevantToExecution(event, expectedExecutionId)) {
      return;
    }

    if (isDiagnosticsSnapshotEvent(event)) {
      applyProjection(event.data.snapshot as WorkflowDiagnosticsProjection);
    }
  });

  workflowGraphUnsubscribe = workflowGraph.subscribe((graph) => {
    latestWorkflowGraph = graph as WorkflowGraph | null;
    emitSnapshot();
  });

  workflowIdUnsubscribe = currentGraphId.subscribe((workflowId) => {
    latestWorkflowId = workflowId;
    emitSnapshot();
    void refreshDiagnosticsProjection();
  });

  workflowNameUnsubscribe = currentGraphName.subscribe((workflowName) => {
    latestWorkflowName = workflowName;
    emitSnapshot();
    void refreshDiagnosticsProjection();
  });

  sessionIdUnsubscribe = sessionStores.currentSessionId.subscribe((sessionId) => {
    latestSessionId = sessionId;
    emitSnapshot();
    void refreshDiagnosticsProjection();
  });
}

function unbindDiagnosticsStore(): void {
  workflowEventUnsubscribe?.();
  workflowGraphUnsubscribe?.();
  workflowIdUnsubscribe?.();
  workflowNameUnsubscribe?.();
  sessionIdUnsubscribe?.();

  workflowEventUnsubscribe = null;
  workflowGraphUnsubscribe = null;
  workflowIdUnsubscribe = null;
  workflowNameUnsubscribe = null;
  sessionIdUnsubscribe = null;
}

export function startDiagnosticsStore(): void {
  if (diagnosticsStarted) {
    return;
  }
  diagnosticsStarted = true;
  bindDiagnosticsStore();
  void refreshDiagnosticsProjection();
}

export function stopDiagnosticsStore(): void {
  if (!diagnosticsStarted) {
    return;
  }
  diagnosticsStarted = false;
  unbindDiagnosticsStore();
}

export const diagnosticsSnapshot: Readable<DiagnosticsSnapshot> = {
  subscribe: diagnosticsSnapshotStore.subscribe,
};

export function setDiagnosticsPanelOpen(panelOpen: boolean): void {
  uiState = { ...uiState, panelOpen };
  emitSnapshot();
}

export function toggleDiagnosticsPanel(): void {
  uiState = { ...uiState, panelOpen: !uiState.panelOpen };
  emitSnapshot();
}

export function setDiagnosticsTab(tab: DiagnosticsTab): void {
  uiState = { ...uiState, activeTab: tab };
  emitSnapshot();
}

export function selectDiagnosticsRun(runId: string | null): void {
  uiState = {
    ...uiState,
    selectedRunId: runId,
    selectedNodeId: null,
  };
  emitSnapshot();
}

export function selectDiagnosticsNode(nodeId: string | null): void {
  uiState = { ...uiState, selectedNodeId: nodeId };
  emitSnapshot();
}

export async function clearDiagnosticsHistory(): Promise<void> {
  const projection = await workflowService.clearDiagnosticsHistory();
  uiState = {
    ...uiState,
    selectedRunId: null,
    selectedNodeId: null,
  };
  applyProjection(projection);
}

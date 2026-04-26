import { writable, type Readable } from 'svelte/store';

import type {
  DiagnosticsSnapshot,
  DiagnosticsTab,
  WorkflowDiagnosticsProjection,
} from '../services/diagnostics/types';
import type { WorkflowGraph, WorkflowEvent } from '../services/workflow/types';
import { workflowGraph } from './workflowStore';
import { currentGraphId } from './graphSessionStore';
import { workflowService } from '../services/workflow/WorkflowService';
import { sessionStores } from './storeInstances';
import {
  type DiagnosticsUiState,
  createDiagnosticsSnapshot,
  createEmptyDiagnosticsProjection,
  normalizeDiagnosticsProjection,
} from './diagnosticsProjection';

const DEFAULT_UI_STATE: DiagnosticsUiState = {
  panelOpen: false,
  activeTab: 'overview',
  selectedRunId: null,
  selectedNodeId: null,
};

let latestProjection: WorkflowDiagnosticsProjection = createEmptyDiagnosticsProjection();
let latestWorkflowId: string | null = null;
let latestWorkflowGraph: WorkflowGraph | null = null;
let latestSessionId: string | null = null;
let uiState: DiagnosticsUiState = { ...DEFAULT_UI_STATE };

function createSnapshot(): DiagnosticsSnapshot {
  return createDiagnosticsSnapshot({
    projection: latestProjection,
    uiState,
    workflowId: latestWorkflowId,
    workflowGraph: latestWorkflowGraph,
    sessionId: latestSessionId,
  });
}

function clearMismatchedWorkflowTimingHistory(): void {
  const history = latestProjection.workflowTimingHistory;
  if (!history) {
    return;
  }

  const graphFingerprint = latestWorkflowGraph?.derived_graph?.graph_fingerprint ?? null;
  if (
    history.workflowId !== latestWorkflowId ||
    history.graphFingerprint !== graphFingerprint
  ) {
    latestProjection = { ...latestProjection, workflowTimingHistory: null };
  }
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
  latestProjection = normalizeDiagnosticsProjection(projection, latestProjection);
  clearMismatchedWorkflowTimingHistory();
  normalizeUiSelections();
  diagnosticsSnapshotStore.set(createSnapshot());
}

function emitSnapshot(): void {
  clearMismatchedWorkflowTimingHistory();
  normalizeUiSelections();
  diagnosticsSnapshotStore.set(createSnapshot());
}

const diagnosticsSnapshotStore = writable<DiagnosticsSnapshot>(createSnapshot());

let workflowEventUnsubscribe: (() => void) | null = null;
let workflowGraphUnsubscribe: (() => void) | null = null;
let workflowIdUnsubscribe: (() => void) | null = null;
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
      latestSessionId,
      latestWorkflowGraph,
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
    if (isDiagnosticsSnapshotEvent(event)) {
      const projection = normalizeDiagnosticsProjection(
        event.data.snapshot as WorkflowDiagnosticsProjection,
        latestProjection,
      );
      if (!projection.context.relevant) {
        return;
      }
      applyProjection(projection);
    }
  });

  workflowGraphUnsubscribe = workflowGraph.subscribe((graph) => {
    const previousFingerprint =
      latestWorkflowGraph?.derived_graph?.graph_fingerprint ?? null;
    latestWorkflowGraph = graph as WorkflowGraph | null;
    const nextFingerprint = latestWorkflowGraph?.derived_graph?.graph_fingerprint ?? null;
    emitSnapshot();
    if (nextFingerprint !== previousFingerprint) {
      void refreshDiagnosticsProjection();
    }
  });

  workflowIdUnsubscribe = currentGraphId.subscribe((workflowId) => {
    latestWorkflowId = workflowId;
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
  sessionIdUnsubscribe?.();

  workflowEventUnsubscribe = null;
  workflowGraphUnsubscribe = null;
  workflowIdUnsubscribe = null;
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

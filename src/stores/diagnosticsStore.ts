import { writable, type Readable } from 'svelte/store';

import { DiagnosticsService } from '../services/diagnostics/DiagnosticsService';
import type {
  DiagnosticsSnapshot,
  DiagnosticsTab,
} from '../services/diagnostics/types';
import type { WorkflowGraph } from '../services/workflow/types';
import { workflowGraph } from './workflowStore';
import { currentGraphId, currentGraphName } from './graphSessionStore';
import { workflowService } from '../services/workflow/WorkflowService';
import { sessionStores } from './storeInstances';

const diagnosticsService = new DiagnosticsService();
const diagnosticsSnapshotStore = writable<DiagnosticsSnapshot>(diagnosticsService.getSnapshot());

let diagnosticsUnsubscribe: (() => void) | null = null;
let workflowEventUnsubscribe: (() => void) | null = null;
let workflowGraphUnsubscribe: (() => void) | null = null;
let workflowIdUnsubscribe: (() => void) | null = null;
let workflowNameUnsubscribe: (() => void) | null = null;
let sessionIdUnsubscribe: (() => void) | null = null;
let diagnosticsStarted = false;
let latestWorkflowId: string | null = null;
let latestSessionId: string | null = null;
let runtimeRefreshToken = 0;
let schedulerRefreshToken = 0;

interface WorkflowErrorEnvelope {
  code?: string;
  message?: string;
}

function normalizeError(error: unknown): string {
  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message;
  }
  if (typeof error === 'string' && error.trim().length > 0) {
    return error;
  }
  return String(error);
}

function parseWorkflowErrorEnvelope(error: unknown): WorkflowErrorEnvelope | null {
  const message = normalizeError(error);
  try {
    const parsed = JSON.parse(message) as WorkflowErrorEnvelope;
    if (parsed && typeof parsed === 'object') {
      return parsed;
    }
  } catch {
    return null;
  }
  return null;
}

function isSessionLookupMiss(error: unknown): boolean {
  return parseWorkflowErrorEnvelope(error)?.code === 'session_not_found';
}

async function refreshRuntimeSnapshot(): Promise<void> {
  const workflowId = latestWorkflowId;
  const refreshToken = ++runtimeRefreshToken;
  const capturedAtMs = Date.now();

  if (!workflowId) {
    diagnosticsService.updateRuntimeSnapshot(null, null, null, capturedAtMs);
    return;
  }

  try {
    const capabilities = await workflowService.getWorkflowCapabilities(workflowId);
    if (refreshToken !== runtimeRefreshToken) {
      return;
    }
    diagnosticsService.updateRuntimeSnapshot(workflowId, capabilities, null, capturedAtMs);
  } catch (error) {
    if (refreshToken !== runtimeRefreshToken) {
      return;
    }
    diagnosticsService.updateRuntimeSnapshot(
      workflowId,
      null,
      normalizeError(error),
      capturedAtMs,
    );
  }
}

async function refreshSchedulerSnapshot(): Promise<void> {
  const sessionId = latestSessionId;
  const workflowId = latestWorkflowId;
  const refreshToken = ++schedulerRefreshToken;
  const capturedAtMs = Date.now();

  if (!sessionId) {
    diagnosticsService.updateSchedulerSnapshot(workflowId, null, null, null, null, capturedAtMs);
    return;
  }

  try {
    const [sessionStatus, sessionQueue] = await Promise.all([
      workflowService.getSessionStatus(sessionId),
      workflowService.listSessionQueue(sessionId),
    ]);
    if (refreshToken !== schedulerRefreshToken) {
      return;
    }
    diagnosticsService.updateSchedulerSnapshot(
      workflowId,
      sessionId,
      sessionStatus,
      sessionQueue,
      null,
      capturedAtMs,
    );
  } catch (error) {
    if (refreshToken !== schedulerRefreshToken) {
      return;
    }
    if (isSessionLookupMiss(error)) {
      diagnosticsService.ensureSchedulerSession(workflowId, sessionId, capturedAtMs);
      return;
    }
    diagnosticsService.updateSchedulerSnapshot(
      workflowId,
      sessionId,
      null,
      null,
      normalizeError(error),
      capturedAtMs,
    );
  }
}

function bindDiagnosticsStore(): void {
  diagnosticsUnsubscribe = diagnosticsService.subscribe((snapshot) => {
    diagnosticsSnapshotStore.set(snapshot);
  });

  workflowEventUnsubscribe = workflowService.subscribeEvents((event) => {
    diagnosticsService.recordWorkflowEvent(event);
    switch (event.type) {
      case 'RuntimeSnapshot':
        diagnosticsService.updateRuntimeSnapshot(
          event.data.workflow_id ?? latestWorkflowId,
          event.data.capabilities ?? null,
          event.data.error ?? null,
          event.data.captured_at_ms,
        );
        break;
      case 'SchedulerSnapshot':
        diagnosticsService.updateSchedulerSnapshot(
          event.data.workflow_id ?? latestWorkflowId,
          event.data.session_id,
          event.data.session ? { session: event.data.session } : null,
          {
            session_id: event.data.session_id,
            items: event.data.items,
          },
          event.data.error ?? null,
          event.data.captured_at_ms,
        );
        break;
      case 'Started':
      case 'Completed':
      case 'Failed':
      case 'WaitingForInput':
      case 'IncrementalExecutionStarted':
        diagnosticsService.applySchedulerEvent(
          latestWorkflowId,
          latestSessionId ?? event.data.execution_id ?? null,
          event,
        );
        void refreshSchedulerSnapshot();
        break;
      default:
        break;
    }
  });

  workflowGraphUnsubscribe = workflowGraph.subscribe((graph) => {
    diagnosticsService.updateWorkflowGraph(graph as WorkflowGraph | null);
  });

  workflowIdUnsubscribe = currentGraphId.subscribe((workflowId) => {
    latestWorkflowId = workflowId;
    diagnosticsService.updateWorkflowMetadata({ workflowId });
    void refreshRuntimeSnapshot();
    void refreshSchedulerSnapshot();
  });

  workflowNameUnsubscribe = currentGraphName.subscribe((workflowName) => {
    diagnosticsService.updateWorkflowMetadata({ workflowName });
  });

  sessionIdUnsubscribe = sessionStores.currentSessionId.subscribe((sessionId) => {
    latestSessionId = sessionId;
    diagnosticsService.setCurrentSessionId(sessionId);
    diagnosticsService.ensureSchedulerSession(latestWorkflowId, sessionId);
    void refreshSchedulerSnapshot();
  });
}

function unbindDiagnosticsStore(): void {
  diagnosticsUnsubscribe?.();
  workflowEventUnsubscribe?.();
  workflowGraphUnsubscribe?.();
  workflowIdUnsubscribe?.();
  workflowNameUnsubscribe?.();
  sessionIdUnsubscribe?.();

  diagnosticsUnsubscribe = null;
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
  diagnosticsService.setPanelOpen(panelOpen);
}

export function toggleDiagnosticsPanel(): void {
  const current = diagnosticsService.getSnapshot();
  diagnosticsService.setPanelOpen(!current.state.panelOpen);
}

export function setDiagnosticsTab(tab: DiagnosticsTab): void {
  diagnosticsService.setActiveTab(tab);
}

export function selectDiagnosticsRun(runId: string | null): void {
  diagnosticsService.selectRun(runId);
}

export function selectDiagnosticsNode(nodeId: string | null): void {
  diagnosticsService.selectNode(nodeId);
}

export function clearDiagnosticsHistory(): void {
  diagnosticsService.clearHistory();
}

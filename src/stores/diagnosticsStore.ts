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

const diagnosticsService = new DiagnosticsService();
const diagnosticsSnapshotStore = writable<DiagnosticsSnapshot>(diagnosticsService.getSnapshot());

let diagnosticsUnsubscribe: (() => void) | null = null;
let workflowEventUnsubscribe: (() => void) | null = null;
let workflowGraphUnsubscribe: (() => void) | null = null;
let workflowIdUnsubscribe: (() => void) | null = null;
let workflowNameUnsubscribe: (() => void) | null = null;
let diagnosticsStarted = false;

function bindDiagnosticsStore(): void {
  diagnosticsUnsubscribe = diagnosticsService.subscribe((snapshot) => {
    diagnosticsSnapshotStore.set(snapshot);
  });

  workflowEventUnsubscribe = workflowService.subscribeEvents((event) => {
    diagnosticsService.recordWorkflowEvent(event);
  });

  workflowGraphUnsubscribe = workflowGraph.subscribe((graph) => {
    diagnosticsService.updateWorkflowGraph(graph as WorkflowGraph | null);
  });

  workflowIdUnsubscribe = currentGraphId.subscribe((workflowId) => {
    diagnosticsService.updateWorkflowMetadata({ workflowId });
  });

  workflowNameUnsubscribe = currentGraphName.subscribe((workflowName) => {
    diagnosticsService.updateWorkflowMetadata({ workflowName });
  });
}

function unbindDiagnosticsStore(): void {
  diagnosticsUnsubscribe?.();
  workflowEventUnsubscribe?.();
  workflowGraphUnsubscribe?.();
  workflowIdUnsubscribe?.();
  workflowNameUnsubscribe?.();

  diagnosticsUnsubscribe = null;
  workflowEventUnsubscribe = null;
  workflowGraphUnsubscribe = null;
  workflowIdUnsubscribe = null;
  workflowNameUnsubscribe = null;
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

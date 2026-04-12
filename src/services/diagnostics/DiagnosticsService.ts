import type {
  WorkflowCapabilitiesResponse,
  WorkflowEvent,
  WorkflowGraph,
  WorkflowSessionQueueListResponse,
  WorkflowSessionStatusResponse,
} from '../workflow/types';
import {
  clearDiagnosticsHistory,
  createDiagnosticsSnapshot,
  createWorkflowDiagnosticsContext,
  createWorkflowDiagnosticsState,
  deriveNodeTypesById,
  recordWorkflowEvent,
  selectDiagnosticsNode,
  selectDiagnosticsRun,
  setDiagnosticsCurrentSessionId,
  setDiagnosticsPanelOpen,
  setDiagnosticsTab,
  updateDiagnosticsStateRuntimeSnapshot,
  updateDiagnosticsStateSchedulerSnapshot,
  updateDiagnosticsStateWorkflowContext,
  updateWorkflowContext,
} from './traceAccumulator.ts';
import type {
  DiagnosticsSnapshot,
  DiagnosticsTab,
  DiagnosticsWorkflowContext,
  WorkflowDiagnosticsState,
} from './types';

type DiagnosticsListener = (snapshot: DiagnosticsSnapshot) => void;
type WorkflowMetadataUpdate = Partial<Pick<DiagnosticsWorkflowContext, 'workflowId' | 'workflowName'>>;

export class DiagnosticsService {
  private state: WorkflowDiagnosticsState;
  private context = createWorkflowDiagnosticsContext();
  private listeners = new Set<DiagnosticsListener>();

  constructor(retainedEventLimit?: number) {
    this.state = createWorkflowDiagnosticsState(retainedEventLimit);
  }

  subscribe(listener: DiagnosticsListener): () => void {
    this.listeners.add(listener);
    listener(this.getSnapshot());
    return () => {
      this.listeners.delete(listener);
    };
  }

  getSnapshot(): DiagnosticsSnapshot {
    return createDiagnosticsSnapshot(structuredClone(this.state));
  }

  setPanelOpen(panelOpen: boolean): void {
    this.state = setDiagnosticsPanelOpen(this.state, panelOpen);
    this.emit();
  }

  setActiveTab(activeTab: DiagnosticsTab): void {
    this.state = setDiagnosticsTab(this.state, activeTab);
    this.emit();
  }

  selectRun(runId: string | null): void {
    this.state = selectDiagnosticsRun(this.state, runId);
    this.emit();
  }

  selectNode(nodeId: string | null): void {
    this.state = selectDiagnosticsNode(this.state, nodeId);
    this.emit();
  }

  clearHistory(): void {
    this.state = clearDiagnosticsHistory(this.state);
    this.emit();
  }

  setCurrentSessionId(sessionId: string | null): void {
    this.state = setDiagnosticsCurrentSessionId(this.state, sessionId);
    this.emit();
  }

  updateWorkflowMetadata(update: WorkflowMetadataUpdate): void {
    this.context = updateWorkflowContext(this.context, update);
    this.state = updateDiagnosticsStateWorkflowContext(this.state, this.context);
    this.emit();
  }

  updateWorkflowGraph(graph: WorkflowGraph | null): void {
    this.context = updateWorkflowContext(this.context, {
      graph,
      graphFingerprint: graph?.derived_graph?.graph_fingerprint ?? null,
      nodeTypesById: deriveNodeTypesById(graph),
    });
    this.state = updateDiagnosticsStateWorkflowContext(this.state, this.context);
    this.emit();
  }

  recordWorkflowEvent(event: WorkflowEvent, timestampMs = Date.now()): void {
    recordWorkflowEvent(this.state, this.context, event, timestampMs);
    this.emit();
  }

  updateRuntimeSnapshot(
    workflowId: string | null,
    capabilities: WorkflowCapabilitiesResponse | null,
    lastError: string | null,
    capturedAtMs = Date.now(),
  ): void {
    this.state = updateDiagnosticsStateRuntimeSnapshot(
      this.state,
      workflowId,
      capabilities,
      capturedAtMs,
      lastError,
    );
    this.emit();
  }

  updateSchedulerSnapshot(
    workflowId: string | null,
    sessionId: string | null,
    sessionStatus: WorkflowSessionStatusResponse | null,
    sessionQueue: WorkflowSessionQueueListResponse | null,
    lastError: string | null,
    capturedAtMs = Date.now(),
  ): void {
    this.state = updateDiagnosticsStateSchedulerSnapshot(
      this.state,
      workflowId,
      sessionId,
      sessionStatus,
      sessionQueue,
      capturedAtMs,
      lastError,
    );
    this.emit();
  }

  private emit(): void {
    const snapshot = this.getSnapshot();
    this.listeners.forEach(listener => listener(snapshot));
  }
}

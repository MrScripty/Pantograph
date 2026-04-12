import type { WorkflowEvent, WorkflowGraph } from '../workflow/types';
import {
  clearDiagnosticsHistory,
  createDiagnosticsSnapshot,
  createWorkflowDiagnosticsContext,
  createWorkflowDiagnosticsState,
  deriveNodeTypesById,
  recordWorkflowEvent,
  selectDiagnosticsNode,
  selectDiagnosticsRun,
  setDiagnosticsPanelOpen,
  setDiagnosticsTab,
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

  private emit(): void {
    const snapshot = this.getSnapshot();
    this.listeners.forEach(listener => listener(snapshot));
  }
}

import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  WorkflowDiagnosticsProjection,
  WorkflowTraceSnapshotRequest,
  WorkflowTraceSnapshotResponse,
} from '../diagnostics/types.ts';
import type {
  NodeDefinition,
  WorkflowCapabilitiesResponse,
  WorkflowEvent,
  WorkflowGraph,
  WorkflowFile,
  WorkflowSessionHandle,
  WorkflowSchedulerSnapshotResponse,
  WorkflowMetadata,
  WorkflowSessionQueueListResponse,
  WorkflowSessionStatusResponse,
} from './types.ts';
import {
  MOCK_NODE_DEFINITIONS,
  mockValidateConnection,
} from './mocks.ts';
import {
  getWorkflowEventExecutionId,
  projectWorkflowEventOwnership,
} from '@pantograph/svelte-graph';
import { parseWorkflowGraphMutationResponse } from '../../lib/workflowGraphMutationResponse.ts';
import { WorkflowGraphMutationService } from './WorkflowGraphMutationService.ts';
import { USE_WORKFLOW_MOCKS } from './workflowServiceConfig.ts';

/** Undo/redo state from the backend */
export interface UndoRedoState {
  canUndo: boolean;
  canRedo: boolean;
  undoCount: number;
}

interface WorkflowCapabilitiesRequest {
  workflow_id: string;
}

interface WorkflowSessionStatusRequest {
  session_id: string;
}

interface WorkflowSessionQueueListRequest {
  session_id: string;
}

export class WorkflowService extends WorkflowGraphMutationService {
  private channel: Channel<WorkflowEvent> | null = null;
  private eventListeners: Set<(event: WorkflowEvent) => void> = new Set();

  private publishEvent(event: WorkflowEvent): void {
    this.currentRunExecutionId = projectWorkflowEventOwnership(
      event,
      this.currentRunExecutionId,
    ).activeExecutionId;
    if (event.type === 'Started' && this.currentExecutionId === null) {
      this.currentExecutionId = getWorkflowEventExecutionId(event);
    }
    this.eventListeners.forEach((listener) => listener(event));
  }

  // --- Node Definitions ---

  async getNodeDefinitions(): Promise<NodeDefinition[]> {
    if (USE_WORKFLOW_MOCKS) {
      return MOCK_NODE_DEFINITIONS;
    }
    return invoke<NodeDefinition[]>('get_node_definitions');
  }

  getNodeDefinition(nodeType: string): NodeDefinition | undefined {
    if (USE_WORKFLOW_MOCKS) {
      return MOCK_NODE_DEFINITIONS.find((d) => d.node_type === nodeType);
    }
    // When using real backend, definitions should be cached
    return undefined;
  }

  // --- Connection Validation ---

  async validateConnection(sourceType: string, targetType: string): Promise<boolean> {
    if (USE_WORKFLOW_MOCKS) {
      return mockValidateConnection(sourceType, targetType);
    }
    return invoke<boolean>('validate_workflow_connection', {
      sourceType,
      targetType,
    });
  }

  /**
   * Get the current execution ID, if any.
   */
  getCurrentExecutionId(): string | null {
    return this.currentExecutionId;
  }

  /**
   * Get the current run execution ID, if any.
   */
  getCurrentRunExecutionId(): string | null {
    return this.currentRunExecutionId;
  }

  /**
   * Set the current execution ID externally.
   * Used by storeInstances to sync session IDs created via WorkflowBackend.
   */
  setCurrentExecutionId(id: string | null): void {
    this.currentExecutionId = id;
    this.currentRunExecutionId = null;
  }

  // --- Session Management ---

  /**
   * Create a workflow editing session without executing.
   * This enables editing the graph with undo/redo support before running.
   * Returns the session ID which can be used for graph modifications.
   */
  async createSession(
    graph: WorkflowGraph,
    workflowId?: string | null,
  ): Promise<WorkflowSessionHandle> {
    if (USE_WORKFLOW_MOCKS) {
      this.currentExecutionId = 'mock-session-id';
      this.currentRunExecutionId = null;
      return {
        session_id: this.currentExecutionId,
        session_kind: 'edit',
      };
    }

    const session = await invoke<WorkflowSessionHandle>('create_workflow_execution_session', {
      graph,
      workflowId: workflowId ?? null,
    });
    this.currentExecutionId = session.session_id;
    this.currentRunExecutionId = null;
    return session;
  }

  /**
   * Run an existing workflow session by demanding outputs from terminal nodes.
   * Uses the current session if no sessionId is provided.
   */
  async runSession(sessionId?: string, workflowName?: string | null): Promise<void> {
    const id = sessionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_WORKFLOW_MOCKS) {
      console.log('[WorkflowService] Mock: Run session', id);
      return;
    }

    this.currentRunExecutionId = null;
    this.channel = new Channel<WorkflowEvent>();

    this.channel.onmessage = (event) => {
      this.publishEvent(event);
    };

    await invoke('run_workflow_execution_session', {
      sessionId: id,
      workflowName: workflowName ?? null,
      channel: this.channel,
    });
  }

  async getWorkflowCapabilities(workflowId: string): Promise<WorkflowCapabilitiesResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        max_input_bindings: 8,
        max_output_targets: 8,
        max_value_bytes: 10_000_000,
        runtime_requirements: {
          estimation_confidence: 'unknown',
          required_models: [],
          required_backends: [],
          required_extensions: [],
        },
        models: [],
        runtime_capabilities: [],
      };
    }

    return invoke<WorkflowCapabilitiesResponse>('workflow_get_capabilities', {
      request: {
        workflow_id: workflowId,
      } satisfies WorkflowCapabilitiesRequest,
    });
  }

  async getSessionStatus(sessionId?: string): Promise<WorkflowSessionStatusResponse | null> {
    const id = sessionId ?? this.currentExecutionId;
    if (!id) {
      return null;
    }

    if (USE_WORKFLOW_MOCKS) {
      return {
        session: {
          session_id: id,
          workflow_id: 'mock-workflow',
          session_kind: 'workflow',
          keep_alive: true,
          state: 'idle_loaded',
          queued_runs: 0,
          run_count: 0,
        },
      };
    }

    return invoke<WorkflowSessionStatusResponse>('workflow_get_session_status', {
      request: {
        session_id: id,
      } satisfies WorkflowSessionStatusRequest,
    });
  }

  async listSessionQueue(sessionId?: string): Promise<WorkflowSessionQueueListResponse | null> {
    const id = sessionId ?? this.currentExecutionId;
    if (!id) {
      return null;
    }

    if (USE_WORKFLOW_MOCKS) {
      return {
        session_id: id,
        items: [],
      };
    }

    return invoke<WorkflowSessionQueueListResponse>('workflow_list_session_queue', {
      request: {
        session_id: id,
      } satisfies WorkflowSessionQueueListRequest,
    });
  }

  async getSchedulerSnapshot(sessionId?: string): Promise<WorkflowSchedulerSnapshotResponse | null> {
    const id = sessionId ?? this.currentExecutionId;
    if (!id) {
      return null;
    }

    if (USE_WORKFLOW_MOCKS) {
      return {
        workflow_id: 'mock-workflow',
        session_id: id,
        trace_execution_id: id,
        session: {
          session_id: id,
          workflow_id: 'mock-workflow',
          session_kind: 'edit',
          keep_alive: false,
          state: 'idle_loaded',
          queued_runs: 0,
          run_count: 0,
        },
        items: [],
      };
    }

    return invoke<WorkflowSchedulerSnapshotResponse>('workflow_get_scheduler_snapshot', {
      request: {
        session_id: id,
      },
    });
  }

  async getDiagnosticsSnapshot(
    workflowId?: string | null,
    workflowName?: string | null,
    sessionId?: string | null,
    workflowGraph?: WorkflowGraph | null,
  ): Promise<WorkflowDiagnosticsProjection> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        context: {
          requestedSessionId: sessionId ?? null,
          requestedWorkflowId: workflowId ?? null,
          requestedWorkflowName: workflowName ?? null,
          sourceExecutionId: null,
          relevantExecutionId: null,
          relevant: true,
        },
        runsById: {},
        runOrder: [],
        runtime: {
          workflowId: workflowId ?? null,
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
          workflowId: workflowId ?? null,
          sessionId: sessionId ?? null,
          traceExecutionId: null,
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

    return invoke<WorkflowDiagnosticsProjection>('workflow_get_diagnostics_snapshot', {
      request: {
        workflow_id: workflowId ?? null,
        workflow_name: workflowName ?? null,
        session_id: sessionId ?? null,
        workflow_graph: workflowGraph ?? null,
      },
    });
  }

  async getTraceSnapshot(
    request: WorkflowTraceSnapshotRequest = {},
  ): Promise<WorkflowTraceSnapshotResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        traces: [],
        retained_trace_limit: 200,
      };
    }

    return invoke<WorkflowTraceSnapshotResponse>('workflow_get_trace_snapshot', {
      request,
    });
  }

  async clearDiagnosticsHistory(): Promise<WorkflowDiagnosticsProjection> {
    if (USE_WORKFLOW_MOCKS) {
      return this.getDiagnosticsSnapshot(null, null, null);
    }

    return invoke<WorkflowDiagnosticsProjection>('workflow_clear_diagnostics_history');
  }

  // --- Undo/Redo ---

  /**
   * Get the current undo/redo state for an execution.
   */
  async getUndoRedoState(executionId?: string): Promise<UndoRedoState> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      return { canUndo: false, canRedo: false, undoCount: 0 };
    }

    if (USE_WORKFLOW_MOCKS) {
      return { canUndo: false, canRedo: false, undoCount: 0 };
    }

    return invoke<UndoRedoState>('get_undo_redo_state', { executionId: id });
  }

  /**
   * Undo the last graph modification.
   * Returns the restored graph state.
   */
  async undo(executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_WORKFLOW_MOCKS) {
      throw new Error('Undo not supported in mock mode');
    }

    return invoke<unknown>('undo_workflow', {
      executionId: id,
    }).then((response) => parseWorkflowGraphMutationResponse(response).graph);
  }

  /**
   * Redo the last undone graph modification.
   * Returns the restored graph state.
   */
  async redo(executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_WORKFLOW_MOCKS) {
      throw new Error('Redo not supported in mock mode');
    }

    return invoke<unknown>('redo_workflow', {
      executionId: id,
    }).then((response) => parseWorkflowGraphMutationResponse(response).graph);
  }

  // --- Event Subscription ---

  subscribeEvents(listener: (event: WorkflowEvent) => void): () => void {
    this.eventListeners.add(listener);
    return () => this.eventListeners.delete(listener);
  }

  // --- Workflow Persistence ---

  async saveWorkflow(name: string, graph: WorkflowGraph): Promise<string> {
    if (USE_WORKFLOW_MOCKS) {
      console.log('[WorkflowService] Mock: Saving workflow', name, graph);
      return `/mock/workflows/${name}.json`;
    }
    return invoke<string>('save_workflow', { name, graph });
  }

  async loadWorkflow(path: string): Promise<WorkflowFile> {
    if (USE_WORKFLOW_MOCKS) {
      console.log('[WorkflowService] Mock: Loading workflow', path);
      return {
        version: '1.0',
        metadata: {
          name: 'Mock Workflow',
          created: new Date().toISOString(),
          modified: new Date().toISOString(),
        },
        graph: { nodes: [], edges: [] },
      };
    }
    return invoke<WorkflowFile>('load_workflow', { path });
  }

  async listWorkflows(): Promise<WorkflowMetadata[]> {
    if (USE_WORKFLOW_MOCKS) {
      return [
        {
          name: 'coding-agent',
          description: 'Agent for generating Svelte GUI elements',
          created: new Date().toISOString(),
          modified: new Date().toISOString(),
        },
      ];
    }
    return invoke<WorkflowMetadata[]>('list_workflows');
  }

  async deleteWorkflow(name: string): Promise<void> {
    if (USE_WORKFLOW_MOCKS) {
      console.log('[WorkflowService] Mock: Deleting workflow', name);
      return;
    }
    return invoke('delete_workflow', { name });
  }

  /**
   * Get a list of built-in workflow templates
   */
  getBuiltInWorkflows(): WorkflowMetadata[] {
    return [
      {
        name: 'coding-agent',
        description: 'Agent for generating Svelte GUI elements with tool-loop',
        created: '2026-01-20T00:00:00Z',
        modified: '2026-01-20T00:00:00Z',
      },
    ];
  }
}

export const workflowService = new WorkflowService();

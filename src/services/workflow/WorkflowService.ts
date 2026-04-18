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
  WorkflowGraphMutationResponse,
  WorkflowFile,
  WorkflowSessionHandle,
  WorkflowSchedulerSnapshotResponse,
  WorkflowMetadata,
  WorkflowSessionQueueListResponse,
  WorkflowSessionStatusResponse,
  GraphNode,
  GraphEdge,
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  EdgeInsertionPreviewResponse,
  InsertNodePositionHint,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
} from './types.ts';
import {
  MOCK_NODE_DEFINITIONS,
  mockExecuteWorkflow,
  mockValidateConnection,
} from './mocks.ts';
import {
  claimWorkflowExecutionIdFromEvent,
  getWorkflowEventExecutionId,
} from '@pantograph/svelte-graph';
import {
  normalizeConnectionCandidatesResponse,
  normalizeConnectionCommitResponse,
  normalizeEdgeInsertionPreviewResponse,
  normalizeInsertNodeConnectionResponse,
  normalizeInsertNodeOnEdgeResponse,
  serializeConnectionAnchor,
} from '../../lib/tauriConnectionIntentWire.ts';

// Set to false to use real Rust backend, true to use frontend mocks
const USE_MOCKS = false;

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

export class WorkflowService {
  private channel: Channel<WorkflowEvent> | null = null;
  private eventListeners: Set<(event: WorkflowEvent) => void> = new Set();
  private currentExecutionId: string | null = null;
  private currentRunExecutionId: string | null = null;

  private publishEvent(event: WorkflowEvent): void {
    this.currentRunExecutionId = claimWorkflowExecutionIdFromEvent(
      event,
      this.currentRunExecutionId,
    );
    if (event.type === 'Started' && this.currentExecutionId === null) {
      this.currentExecutionId = getWorkflowEventExecutionId(event);
    }
    this.eventListeners.forEach((listener) => listener(event));
  }

  // --- Node Definitions ---

  async getNodeDefinitions(): Promise<NodeDefinition[]> {
    if (USE_MOCKS) {
      return MOCK_NODE_DEFINITIONS;
    }
    return invoke<NodeDefinition[]>('get_node_definitions');
  }

  getNodeDefinition(nodeType: string): NodeDefinition | undefined {
    if (USE_MOCKS) {
      return MOCK_NODE_DEFINITIONS.find((d) => d.node_type === nodeType);
    }
    // When using real backend, definitions should be cached
    return undefined;
  }

  // --- Connection Validation ---

  async validateConnection(sourceType: string, targetType: string): Promise<boolean> {
    if (USE_MOCKS) {
      return mockValidateConnection(sourceType, targetType);
    }
    return invoke<boolean>('validate_workflow_connection', {
      sourceType,
      targetType,
    });
  }

  // --- Workflow Execution ---

  /**
   * Execute a workflow using the node-engine.
   * This is a convenience wrapper around executeWorkflowV2.
   */
  async executeWorkflow(graph: WorkflowGraph): Promise<void> {
    if (USE_MOCKS) {
      this.currentExecutionId = null;
      this.currentRunExecutionId = null;
      return mockExecuteWorkflow(graph, (event) => {
        this.publishEvent(event);
      });
    }

    this.currentExecutionId = null;
    this.currentRunExecutionId = null;
    this.channel = new Channel<WorkflowEvent>();

    this.channel.onmessage = (event) => {
      this.publishEvent(event);
    };

    // Use execute_workflow_v2 (the node-engine based command)
    await invoke('execute_workflow_v2', {
      graph,
      channel: this.channel,
    });
  }

  // --- Workflow Execution V2 (Node-Engine Based) ---

  /**
   * Execute a workflow using the node-engine with demand-driven evaluation.
   * Returns the execution ID which can be used for undo/redo and graph modifications.
   */
  async executeWorkflowV2(graph: WorkflowGraph): Promise<string> {
    if (USE_MOCKS) {
      // For mocks, fall back to legacy execution and return a fake ID
      this.currentRunExecutionId = null;
      await mockExecuteWorkflow(graph, (event) => {
        this.publishEvent(event);
      });
      this.currentExecutionId ??= 'mock-execution-id';
      this.currentRunExecutionId ??= this.currentExecutionId;
      return this.currentExecutionId;
    }

    this.currentRunExecutionId = null;
    this.channel = new Channel<WorkflowEvent>();

    this.channel.onmessage = (event) => {
      this.publishEvent(event);
    };

    const executionId = await invoke<string>('execute_workflow_v2', {
      graph,
      channel: this.channel,
    });

    this.currentExecutionId = executionId;
    this.currentRunExecutionId = executionId;
    return executionId;
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
  async createSession(graph: WorkflowGraph): Promise<WorkflowSessionHandle> {
    if (USE_MOCKS) {
      this.currentExecutionId = 'mock-session-id';
      this.currentRunExecutionId = null;
      return {
        session_id: this.currentExecutionId,
        session_kind: 'edit',
      };
    }

    const session = await invoke<WorkflowSessionHandle>('create_workflow_session', { graph });
    this.currentExecutionId = session.session_id;
    this.currentRunExecutionId = null;
    return session;
  }

  /**
   * Run an existing workflow session by demanding outputs from terminal nodes.
   * Uses the current session if no sessionId is provided.
   */
  async runSession(sessionId?: string): Promise<void> {
    const id = sessionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Run session', id);
      return;
    }

    this.currentRunExecutionId = null;
    this.channel = new Channel<WorkflowEvent>();

    this.channel.onmessage = (event) => {
      this.publishEvent(event);
    };

    await invoke('run_workflow_session', {
      sessionId: id,
      channel: this.channel,
    });
  }

  async getWorkflowCapabilities(workflowId: string): Promise<WorkflowCapabilitiesResponse> {
    if (USE_MOCKS) {
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

    if (USE_MOCKS) {
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

    if (USE_MOCKS) {
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

    if (USE_MOCKS) {
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
  ): Promise<WorkflowDiagnosticsProjection> {
    if (USE_MOCKS) {
      return {
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
        retainedEventLimit: 200,
      };
    }

    return invoke<WorkflowDiagnosticsProjection>('workflow_get_diagnostics_snapshot', {
      request: {
        workflow_id: workflowId ?? null,
        workflow_name: workflowName ?? null,
        session_id: sessionId ?? null,
      },
    });
  }

  async getTraceSnapshot(
    request: WorkflowTraceSnapshotRequest = {},
  ): Promise<WorkflowTraceSnapshotResponse> {
    if (USE_MOCKS) {
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
    if (USE_MOCKS) {
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

    if (USE_MOCKS) {
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

    if (USE_MOCKS) {
      throw new Error('Undo not supported in mock mode');
    }

    return invoke<WorkflowGraphMutationResponse>('undo_workflow', {
      executionId: id,
    }).then((response) => response.graph);
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

    if (USE_MOCKS) {
      throw new Error('Redo not supported in mock mode');
    }

    return invoke<WorkflowGraphMutationResponse>('redo_workflow', {
      executionId: id,
    }).then((response) => response.graph);
  }

  // --- Graph Modification During Execution ---

  /**
   * Update a node's data during execution.
   * This marks the node as modified and will trigger re-execution of downstream nodes.
   */
  async updateNodeData(
    nodeId: string,
    data: Record<string, unknown>,
    executionId?: string
  ): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Update node data', nodeId, data);
      return { nodes: [], edges: [] };
    }

    return invoke<WorkflowGraphMutationResponse>('update_node_data', {
      executionId: id,
      nodeId,
      data,
    }).then((response) => response.graph);
  }

  async updateNodePosition(
    nodeId: string,
    position: { x: number; y: number },
    executionId?: string
  ): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Update node position', nodeId, position);
      return { nodes: [], edges: [] };
    }

    return invoke<WorkflowGraphMutationResponse>('update_node_position_in_execution', {
      executionId: id,
      nodeId,
      position,
    }).then((response) => response.graph);
  }

  /**
   * Add a node to the graph during execution.
   */
  async addNode(node: GraphNode, executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Add node', node);
      return { nodes: [], edges: [] };
    }

    return invoke<WorkflowGraphMutationResponse>('add_node_to_execution', {
      executionId: id,
      node,
    }).then((response) => response.graph);
  }

  async removeNode(nodeId: string, executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Remove node', nodeId);
      return { nodes: [], edges: [] };
    }

    return invoke<WorkflowGraphMutationResponse>('remove_node_from_execution', {
      executionId: id,
      nodeId,
    }).then((response) => response.graph);
  }

  /**
   * Add an edge to the graph during execution.
   * Returns the updated graph for syncing frontend state.
   */
  async addEdge(edge: GraphEdge, executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Add edge', edge);
      return { nodes: [], edges: [] };
    }

    return invoke<WorkflowGraphMutationResponse>('add_edge_to_execution', {
      executionId: id,
      edge,
    }).then((response) => response.graph);
  }

  async getConnectionCandidates(
    sourceAnchor: ConnectionAnchor,
    executionId?: string,
    graphRevision?: string
  ): Promise<ConnectionCandidatesResponse> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_MOCKS) {
      return {
        graph_revision: '',
        revision_matches: true,
        source_anchor: sourceAnchor,
        compatible_nodes: [],
        insertable_node_types: [],
      };
    }

    const response = await invoke<Parameters<typeof normalizeConnectionCandidatesResponse>[0]>(
      'get_connection_candidates',
      {
      executionId: id,
      sourceAnchor: serializeConnectionAnchor(sourceAnchor),
      graphRevision,
      }
    );
    return normalizeConnectionCandidatesResponse(response);
  }

  async connectAnchors(
    sourceAnchor: ConnectionAnchor,
    targetAnchor: ConnectionAnchor,
    graphRevision: string,
    executionId?: string
  ): Promise<ConnectionCommitResponse> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_MOCKS) {
      return {
        accepted: false,
        graph_revision: graphRevision,
        rejection: {
          reason: 'unknown_target_anchor',
          message: 'Mock mode does not implement connection commits',
        },
      };
    }

    const response = await invoke<Parameters<typeof normalizeConnectionCommitResponse>[0]>(
      'connect_anchors_in_execution',
      {
      executionId: id,
      sourceAnchor: serializeConnectionAnchor(sourceAnchor),
      targetAnchor: serializeConnectionAnchor(targetAnchor),
      graphRevision,
      }
    );
    return normalizeConnectionCommitResponse(response);
  }

  async insertNodeAndConnect(
    sourceAnchor: ConnectionAnchor,
    nodeType: string,
    graphRevision: string,
    positionHint: InsertNodePositionHint,
    preferredInputPortId?: string,
    executionId?: string
  ): Promise<InsertNodeConnectionResponse> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_MOCKS) {
      return {
        accepted: false,
        graph_revision: graphRevision,
        rejection: {
          reason: 'unknown_insert_node_type',
          message: 'Mock mode does not implement insert-and-connect',
        },
      };
    }

    const response = await invoke<Parameters<typeof normalizeInsertNodeConnectionResponse>[0]>(
      'insert_node_and_connect_in_execution',
      {
      executionId: id,
      sourceAnchor: serializeConnectionAnchor(sourceAnchor),
      nodeType,
      graphRevision,
      positionHint,
      preferredInputPortId,
      }
    );
    return normalizeInsertNodeConnectionResponse(response);
  }

  async previewNodeInsertOnEdge(
    edgeId: string,
    nodeType: string,
    graphRevision: string,
    executionId?: string
  ): Promise<EdgeInsertionPreviewResponse> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_MOCKS) {
      return {
        accepted: false,
        graph_revision: graphRevision,
        rejection: {
          reason: 'unknown_edge',
          message: 'Mock mode does not implement edge insertion preview',
        },
      };
    }

    const response = await invoke<Parameters<typeof normalizeEdgeInsertionPreviewResponse>[0]>(
      'preview_node_insert_on_edge_in_execution',
      {
        executionId: id,
        edgeId,
        nodeType,
        graphRevision,
      }
    );
    return normalizeEdgeInsertionPreviewResponse(response);
  }

  async insertNodeOnEdge(
    edgeId: string,
    nodeType: string,
    graphRevision: string,
    positionHint: InsertNodePositionHint,
    executionId?: string
  ): Promise<InsertNodeOnEdgeResponse> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_MOCKS) {
      return {
        accepted: false,
        graph_revision: graphRevision,
        rejection: {
          reason: 'unknown_edge',
          message: 'Mock mode does not implement edge insertion',
        },
      };
    }

    const response = await invoke<Parameters<typeof normalizeInsertNodeOnEdgeResponse>[0]>(
      'insert_node_on_edge_in_execution',
      {
        executionId: id,
        edgeId,
        nodeType,
        graphRevision,
        positionHint,
      }
    );
    return normalizeInsertNodeOnEdgeResponse(response);
  }

  /**
   * Remove an edge from the graph during execution.
   * Returns the updated graph for syncing frontend state.
   */
  async removeEdge(edgeId: string, executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Remove edge', edgeId);
      return { nodes: [], edges: [] };
    }

    return invoke<WorkflowGraphMutationResponse>('remove_edge_from_execution', {
      executionId: id,
      edgeId,
    }).then((response) => response.graph);
  }

  /**
   * Get the current graph state from an execution.
   */
  async getExecutionGraph(executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_MOCKS) {
      return { nodes: [], edges: [] };
    }

    return invoke<WorkflowGraph>('get_execution_graph', { executionId: id });
  }

  /**
   * Clean up an execution when done.
   */
  async removeExecution(executionId?: string): Promise<void> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      return;
    }

    if (USE_MOCKS) {
      this.currentExecutionId = null;
      this.currentRunExecutionId = null;
      return;
    }

    await invoke('remove_execution', { executionId: id });

    if (id === this.currentExecutionId) {
      this.currentExecutionId = null;
    }
    if (id === this.currentRunExecutionId) {
      this.currentRunExecutionId = null;
    }
  }

  // --- Event Subscription ---

  subscribeEvents(listener: (event: WorkflowEvent) => void): () => void {
    this.eventListeners.add(listener);
    return () => this.eventListeners.delete(listener);
  }

  // --- Workflow Persistence ---

  async saveWorkflow(name: string, graph: WorkflowGraph): Promise<string> {
    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Saving workflow', name, graph);
      return `/mock/workflows/${name}.json`;
    }
    return invoke<string>('save_workflow', { name, graph });
  }

  async loadWorkflow(path: string): Promise<WorkflowFile> {
    if (USE_MOCKS) {
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
    if (USE_MOCKS) {
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
    if (USE_MOCKS) {
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

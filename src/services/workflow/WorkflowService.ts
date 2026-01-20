import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  NodeDefinition,
  WorkflowEvent,
  WorkflowGraph,
  WorkflowFile,
  WorkflowMetadata,
  GraphNode,
  GraphEdge,
} from './types';
import {
  MOCK_NODE_DEFINITIONS,
  mockExecuteWorkflow,
  mockValidateConnection,
} from './mocks';

// Set to false to use real Rust backend, true to use frontend mocks
const USE_MOCKS = false;

/** Undo/redo state from the backend */
export interface UndoRedoState {
  canUndo: boolean;
  canRedo: boolean;
  undoCount: number;
}

export class WorkflowService {
  private channel: Channel<WorkflowEvent> | null = null;
  private eventListeners: Set<(event: WorkflowEvent) => void> = new Set();
  private currentExecutionId: string | null = null;

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

  // --- Workflow Execution (Legacy) ---

  async executeWorkflow(graph: WorkflowGraph): Promise<void> {
    if (USE_MOCKS) {
      return mockExecuteWorkflow(graph, (event) => {
        this.eventListeners.forEach((listener) => listener(event));
      });
    }

    this.channel = new Channel<WorkflowEvent>();

    this.channel.onmessage = (event) => {
      this.eventListeners.forEach((listener) => listener(event));
    };

    await invoke('execute_workflow', {
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
      await mockExecuteWorkflow(graph, (event) => {
        this.eventListeners.forEach((listener) => listener(event));
      });
      this.currentExecutionId = 'mock-execution-id';
      return this.currentExecutionId;
    }

    this.channel = new Channel<WorkflowEvent>();

    this.channel.onmessage = (event) => {
      this.eventListeners.forEach((listener) => listener(event));
    };

    const executionId = await invoke<string>('execute_workflow_v2', {
      graph,
      channel: this.channel,
    });

    this.currentExecutionId = executionId;
    return executionId;
  }

  /**
   * Get the current execution ID, if any.
   */
  getCurrentExecutionId(): string | null {
    return this.currentExecutionId;
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

    return invoke<WorkflowGraph>('undo_workflow', { executionId: id });
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

    return invoke<WorkflowGraph>('redo_workflow', { executionId: id });
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
  ): Promise<void> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Update node data', nodeId, data);
      return;
    }

    return invoke('update_node_data', { executionId: id, nodeId, data });
  }

  /**
   * Add a node to the graph during execution.
   */
  async addNode(node: GraphNode, executionId?: string): Promise<void> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Add node', node);
      return;
    }

    return invoke('add_node_to_execution', { executionId: id, node });
  }

  /**
   * Add an edge to the graph during execution.
   */
  async addEdge(edge: GraphEdge, executionId?: string): Promise<void> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Add edge', edge);
      return;
    }

    return invoke('add_edge_to_execution', { executionId: id, edge });
  }

  /**
   * Remove an edge from the graph during execution.
   */
  async removeEdge(edgeId: string, executionId?: string): Promise<void> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Remove edge', edgeId);
      return;
    }

    return invoke('remove_edge_from_execution', { executionId: id, edgeId });
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
      return;
    }

    await invoke('remove_execution', { executionId: id });

    if (id === this.currentExecutionId) {
      this.currentExecutionId = null;
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

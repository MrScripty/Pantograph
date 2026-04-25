/**
 * TauriWorkflowBackend — implements WorkflowBackend using Tauri invoke calls.
 *
 * This is the Pantograph-specific backend implementation. Each method maps
 * directly to a Tauri command defined in src-tauri/src/commands/.
 */
import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  WorkflowBackend,
  UndoRedoState,
  NodeDefinition,
  WorkflowGraph,
  WorkflowGraphMutationResponse,
  WorkflowFile,
  WorkflowMetadata,
  WorkflowEvent,
  WorkflowSessionHandle,
  GraphNode,
  GraphEdge,
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  EdgeInsertionPreviewResponse,
  InsertNodePositionHint,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
  PortMapping,
} from '@pantograph/svelte-graph';
import {
  normalizeConnectionCandidatesResponse,
  normalizeConnectionCommitResponse,
  normalizeEdgeInsertionPreviewResponse,
  normalizeInsertNodeConnectionResponse,
  normalizeInsertNodeOnEdgeResponse,
  serializeConnectionAnchor,
} from '../lib/tauriConnectionIntentWire';
import { parseWorkflowGraphMutationResponse } from '../lib/workflowGraphMutationResponse';

export class TauriWorkflowBackend implements WorkflowBackend {
  private channel: Channel<WorkflowEvent> | null = null;
  private eventListeners: Set<(event: WorkflowEvent) => void> = new Set();

  // --- Node Definitions ---

  async getNodeDefinitions(): Promise<NodeDefinition[]> {
    return invoke<NodeDefinition[]>('get_node_definitions');
  }

  async validateConnection(sourceType: string, targetType: string): Promise<boolean> {
    return invoke<boolean>('validate_workflow_connection', { sourceType, targetType });
  }

  // --- Session Management ---

  async createSession(
    graph: WorkflowGraph,
    workflowId?: string | null,
  ): Promise<WorkflowSessionHandle> {
    return invoke<WorkflowSessionHandle>('create_workflow_execution_session', {
      graph,
      workflowId: workflowId ?? null,
    });
  }

  async runSession(sessionId: string): Promise<void> {
    this.channel = new Channel<WorkflowEvent>();
    this.channel.onmessage = (event) => {
      this.eventListeners.forEach((listener) => listener(event));
    };
    await invoke('run_workflow_execution_session', { sessionId, channel: this.channel });
  }

  async removeSession(sessionId: string): Promise<void> {
    await invoke('remove_execution', { executionId: sessionId });
  }

  // --- Graph Mutation ---

  async addNode(node: GraphNode, sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('add_node_to_execution', {
      executionId: sessionId,
      node,
    });
    return parseWorkflowGraphMutationResponse(response);
  }

  async removeNode(nodeId: string, sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('remove_node_from_execution', {
      executionId: sessionId,
      nodeId,
    });
    return parseWorkflowGraphMutationResponse(response);
  }

  async addEdge(edge: GraphEdge, sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('add_edge_to_execution', {
      executionId: sessionId,
      edge,
    });
    return parseWorkflowGraphMutationResponse(response);
  }

  async getConnectionCandidates(
    sourceAnchor: ConnectionAnchor,
    sessionId: string,
    graphRevision?: string,
  ): Promise<ConnectionCandidatesResponse> {
    const response = await invoke<Parameters<typeof normalizeConnectionCandidatesResponse>[0]>(
      'get_connection_candidates',
      {
      executionId: sessionId,
      sourceAnchor: serializeConnectionAnchor(sourceAnchor),
      graphRevision,
      }
    );
    return normalizeConnectionCandidatesResponse(response) as ConnectionCandidatesResponse;
  }

  async connectAnchors(
    sourceAnchor: ConnectionAnchor,
    targetAnchor: ConnectionAnchor,
    sessionId: string,
    graphRevision: string,
  ): Promise<ConnectionCommitResponse> {
    const response = await invoke<Parameters<typeof normalizeConnectionCommitResponse>[0]>(
      'connect_anchors_in_execution',
      {
      executionId: sessionId,
      sourceAnchor: serializeConnectionAnchor(sourceAnchor),
      targetAnchor: serializeConnectionAnchor(targetAnchor),
      graphRevision,
      }
    );
    return normalizeConnectionCommitResponse(response) as ConnectionCommitResponse;
  }

  async insertNodeAndConnect(
    sourceAnchor: ConnectionAnchor,
    nodeType: string,
    sessionId: string,
    graphRevision: string,
    positionHint: InsertNodePositionHint,
    preferredInputPortId?: string,
  ): Promise<InsertNodeConnectionResponse> {
    const response = await invoke<Parameters<typeof normalizeInsertNodeConnectionResponse>[0]>(
      'insert_node_and_connect_in_execution',
      {
      executionId: sessionId,
      sourceAnchor: serializeConnectionAnchor(sourceAnchor),
      nodeType,
      graphRevision,
      positionHint,
      preferredInputPortId,
      }
    );
    return normalizeInsertNodeConnectionResponse(response) as InsertNodeConnectionResponse;
  }

  async previewNodeInsertOnEdge(
    edgeId: string,
    nodeType: string,
    sessionId: string,
    graphRevision: string,
  ): Promise<EdgeInsertionPreviewResponse> {
    const response = await invoke<Parameters<typeof normalizeEdgeInsertionPreviewResponse>[0]>(
      'preview_node_insert_on_edge_in_execution',
      {
        executionId: sessionId,
        edgeId,
        nodeType,
        graphRevision,
      },
    );
    return normalizeEdgeInsertionPreviewResponse(response) as EdgeInsertionPreviewResponse;
  }

  async insertNodeOnEdge(
    edgeId: string,
    nodeType: string,
    sessionId: string,
    graphRevision: string,
    positionHint: InsertNodePositionHint,
  ): Promise<InsertNodeOnEdgeResponse> {
    const response = await invoke<Parameters<typeof normalizeInsertNodeOnEdgeResponse>[0]>(
      'insert_node_on_edge_in_execution',
      {
        executionId: sessionId,
        edgeId,
        nodeType,
        graphRevision,
        positionHint,
      },
    );
    return normalizeInsertNodeOnEdgeResponse(response) as InsertNodeOnEdgeResponse;
  }

  async removeEdge(edgeId: string, sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('remove_edge_from_execution', {
      executionId: sessionId,
      edgeId,
    });
    return parseWorkflowGraphMutationResponse(response);
  }

  async updateNodeData(
    nodeId: string,
    data: Record<string, unknown>,
    sessionId: string,
  ): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('update_node_data', {
      executionId: sessionId,
      nodeId,
      data,
    });
    return parseWorkflowGraphMutationResponse(response);
  }

  async updateNodePosition(
    nodeId: string,
    position: { x: number; y: number },
    sessionId: string,
  ): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('update_node_position_in_execution', {
      executionId: sessionId,
      nodeId,
      position,
    });
    return parseWorkflowGraphMutationResponse(response);
  }

  async getExecutionGraph(sessionId: string): Promise<WorkflowGraph> {
    return invoke<WorkflowGraph>('get_execution_graph', { executionId: sessionId });
  }

  // --- Undo/Redo ---

  async getUndoRedoState(sessionId: string): Promise<UndoRedoState> {
    return invoke<UndoRedoState>('get_undo_redo_state', { executionId: sessionId });
  }

  async undo(sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('undo_workflow', { executionId: sessionId });
    return parseWorkflowGraphMutationResponse(response);
  }

  async redo(sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('redo_workflow', { executionId: sessionId });
    return parseWorkflowGraphMutationResponse(response);
  }

  // --- Persistence ---

  async saveWorkflow(name: string, graph: WorkflowGraph): Promise<string> {
    return invoke<string>('save_workflow', { name, graph });
  }

  async loadWorkflow(path: string): Promise<WorkflowFile> {
    return invoke<WorkflowFile>('load_workflow', { path });
  }

  async listWorkflows(): Promise<WorkflowMetadata[]> {
    return invoke<WorkflowMetadata[]>('list_workflows');
  }

  async deleteWorkflow(name: string): Promise<void> {
    return invoke('delete_workflow', { name });
  }

  // --- Node Groups ---

  async createGroup(
    name: string,
    selectedNodeIds: string[],
    sessionId: string,
  ): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('create_group_in_execution', {
      executionId: sessionId,
      name,
      selectedNodeIds,
    });
    return parseWorkflowGraphMutationResponse(response);
  }

  async updateGroupPorts(
    groupId: string,
    exposedInputs: PortMapping[],
    exposedOutputs: PortMapping[],
    sessionId: string,
  ): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('update_group_ports_in_execution', {
      executionId: sessionId,
      groupId,
      exposedInputs,
      exposedOutputs,
    });
    return parseWorkflowGraphMutationResponse(response);
  }

  async ungroup(groupId: string, sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const response = await invoke<unknown>('ungroup_in_execution', {
      executionId: sessionId,
      groupId,
    });
    return parseWorkflowGraphMutationResponse(response);
  }

  // --- Events ---

  subscribeEvents(listener: (event: WorkflowEvent) => void): () => void {
    this.eventListeners.add(listener);
    return () => this.eventListeners.delete(listener);
  }
}

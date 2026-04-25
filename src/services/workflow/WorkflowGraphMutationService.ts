import { invoke } from '@tauri-apps/api/core';
import type {
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  EdgeInsertionPreviewResponse,
  GraphEdge,
  GraphNode,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
  InsertNodePositionHint,
  WorkflowGraph,
} from './types.ts';
import {
  connectWorkflowAnchors,
  getWorkflowConnectionCandidates,
  insertWorkflowNodeAndConnect,
  insertWorkflowNodeOnEdge,
  previewWorkflowNodeInsertOnEdge,
} from './workflowConnectionActions.ts';
import { parseWorkflowGraphMutationResponse } from '../../lib/workflowGraphMutationResponse.ts';
import { USE_WORKFLOW_MOCKS } from './workflowServiceConfig.ts';

export abstract class WorkflowGraphMutationService {
  protected currentExecutionId: string | null = null;
  protected currentRunExecutionId: string | null = null;

  async updateNodeData(
    nodeId: string,
    data: Record<string, unknown>,
    executionId?: string
  ): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_WORKFLOW_MOCKS) {
      console.log('[WorkflowService] Mock: Update node data', nodeId, data);
      return { nodes: [], edges: [] };
    }

    return invoke<unknown>('update_node_data', {
      executionId: id,
      nodeId,
      data,
    }).then((response) => parseWorkflowGraphMutationResponse(response).graph);
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

    if (USE_WORKFLOW_MOCKS) {
      console.log('[WorkflowService] Mock: Update node position', nodeId, position);
      return { nodes: [], edges: [] };
    }

    return invoke<unknown>('update_node_position_in_execution', {
      executionId: id,
      nodeId,
      position,
    }).then((response) => parseWorkflowGraphMutationResponse(response).graph);
  }

  async addNode(node: GraphNode, executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_WORKFLOW_MOCKS) {
      console.log('[WorkflowService] Mock: Add node', node);
      return { nodes: [], edges: [] };
    }

    return invoke<unknown>('add_node_to_execution', {
      executionId: id,
      node,
    }).then((response) => parseWorkflowGraphMutationResponse(response).graph);
  }

  async removeNode(nodeId: string, executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_WORKFLOW_MOCKS) {
      console.log('[WorkflowService] Mock: Remove node', nodeId);
      return { nodes: [], edges: [] };
    }

    return invoke<unknown>('remove_node_from_execution', {
      executionId: id,
      nodeId,
    }).then((response) => parseWorkflowGraphMutationResponse(response).graph);
  }

  async addEdge(edge: GraphEdge, executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_WORKFLOW_MOCKS) {
      console.log('[WorkflowService] Mock: Add edge', edge);
      return { nodes: [], edges: [] };
    }

    return invoke<unknown>('add_edge_to_execution', {
      executionId: id,
      edge,
    }).then((response) => parseWorkflowGraphMutationResponse(response).graph);
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

    if (USE_WORKFLOW_MOCKS) {
      return {
        graph_revision: '',
        revision_matches: true,
        source_anchor: sourceAnchor,
        compatible_nodes: [],
        insertable_node_types: [],
      };
    }

    return getWorkflowConnectionCandidates(id, sourceAnchor, graphRevision);
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

    if (USE_WORKFLOW_MOCKS) {
      return {
        accepted: false,
        graph_revision: graphRevision,
        rejection: {
          reason: 'unknown_target_anchor',
          message: 'Mock mode does not implement connection commits',
        },
      };
    }

    return connectWorkflowAnchors(id, sourceAnchor, targetAnchor, graphRevision);
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

    if (USE_WORKFLOW_MOCKS) {
      return {
        accepted: false,
        graph_revision: graphRevision,
        rejection: {
          reason: 'unknown_insert_node_type',
          message: 'Mock mode does not implement insert-and-connect',
        },
      };
    }

    return insertWorkflowNodeAndConnect(
      id,
      sourceAnchor,
      nodeType,
      graphRevision,
      positionHint,
      preferredInputPortId,
    );
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

    if (USE_WORKFLOW_MOCKS) {
      return {
        accepted: false,
        graph_revision: graphRevision,
        rejection: {
          reason: 'unknown_edge',
          message: 'Mock mode does not implement edge insertion preview',
        },
      };
    }

    return previewWorkflowNodeInsertOnEdge(id, edgeId, nodeType, graphRevision);
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

    if (USE_WORKFLOW_MOCKS) {
      return {
        accepted: false,
        graph_revision: graphRevision,
        rejection: {
          reason: 'unknown_edge',
          message: 'Mock mode does not implement edge insertion',
        },
      };
    }

    return insertWorkflowNodeOnEdge(id, edgeId, nodeType, graphRevision, positionHint);
  }

  async removeEdge(edgeId: string, executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active session');
    }

    if (USE_WORKFLOW_MOCKS) {
      console.log('[WorkflowService] Mock: Remove edge', edgeId);
      return { nodes: [], edges: [] };
    }

    return invoke<unknown>('remove_edge_from_execution', {
      executionId: id,
      edgeId,
    }).then((response) => parseWorkflowGraphMutationResponse(response).graph);
  }

  async getExecutionGraph(executionId?: string): Promise<WorkflowGraph> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      throw new Error('No active execution');
    }

    if (USE_WORKFLOW_MOCKS) {
      return { nodes: [], edges: [] };
    }

    return invoke<WorkflowGraph>('get_execution_graph', { executionId: id });
  }

  async removeExecution(executionId?: string): Promise<void> {
    const id = executionId ?? this.currentExecutionId;
    if (!id) {
      return;
    }

    if (USE_WORKFLOW_MOCKS) {
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
}

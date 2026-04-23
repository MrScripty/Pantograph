/**
 * MockWorkflowBackend — in-memory mock for testing and prototyping.
 *
 * Provides realistic-enough behavior for frontend development without
 * a real backend. Node definitions, validation, and execution are all mocked.
 */
import type { WorkflowBackend } from '../types/backend.js';
import type { UndoRedoState } from '../types/backend.js';
import type {
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  EdgeInsertionPreviewResponse,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
  InsertNodePositionHint,
  NodeDefinition,
  PortDataType,
  WorkflowGraph,
  WorkflowGraphMutationResponse,
  WorkflowFile,
  WorkflowMetadata,
  WorkflowEvent,
  WorkflowSessionHandle,
  GraphNode,
  GraphEdge,
} from '../types/workflow.js';
import type { NodeGroup, PortMapping } from '../types/groups.js';
import { isPortTypeCompatible } from '../portTypeCompatibility.js';
import { buildDerivedGraph } from '../graphRevision.js';
import {
  mockConnectAnchors,
  mockGetConnectionCandidates,
  mockInsertNodeAndConnect,
  mockInsertNodeOnEdge,
  mockPreviewNodeInsertOnEdge,
} from './mockConnectionIntent.js';

/** Default mock node definitions */
export const MOCK_NODE_DEFINITIONS: NodeDefinition[] = [
  {
    node_type: 'text-input',
    category: 'input',
    label: 'Text Input',
    description: 'User text input field',
    io_binding_origin: 'client_session',
    inputs: [{ id: 'text', label: 'Text', data_type: 'string', required: false, multiple: false }],
    outputs: [{ id: 'text', label: 'Text', data_type: 'string', required: true, multiple: false }],
    execution_mode: 'reactive',
  },
  {
    node_type: 'number-input',
    category: 'input',
    label: 'Number Input',
    description: 'User numeric input field',
    io_binding_origin: 'client_session',
    inputs: [{ id: 'value', label: 'Value', data_type: 'number', required: false, multiple: false }],
    outputs: [{ id: 'value', label: 'Value', data_type: 'number', required: true, multiple: false }],
    execution_mode: 'reactive',
  },
  {
    node_type: 'boolean-input',
    category: 'input',
    label: 'Boolean Input',
    description: 'User true/false input field',
    io_binding_origin: 'client_session',
    inputs: [{ id: 'value', label: 'Value', data_type: 'boolean', required: false, multiple: false }],
    outputs: [{ id: 'value', label: 'Value', data_type: 'boolean', required: true, multiple: false }],
    execution_mode: 'reactive',
  },
  {
    node_type: 'llm-inference',
    category: 'processing',
    label: 'LLM Inference',
    description: 'Text completion via LLM',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: true, multiple: false },
      { id: 'system_prompt', label: 'System Prompt', data_type: 'string', required: false, multiple: false },
    ],
    outputs: [
      { id: 'response', label: 'Response', data_type: 'string', required: true, multiple: false },
      { id: 'stream', label: 'Stream', data_type: 'stream', required: true, multiple: false },
    ],
    execution_mode: 'stream',
  },
  {
    node_type: 'text-output',
    category: 'output',
    label: 'Text Output',
    description: 'Display text result',
    io_binding_origin: 'client_session',
    inputs: [{ id: 'text', label: 'Text', data_type: 'string', required: true, multiple: false }],
    outputs: [],
    execution_mode: 'reactive',
  },
];

function mockValidateConnection(sourceType: string, targetType: string): boolean {
  return isPortTypeCompatible(
    sourceType as Parameters<typeof isPortTypeCompatible>[0],
    targetType as Parameters<typeof isPortTypeCompatible>[1]
  );
}

export class MockWorkflowBackend implements WorkflowBackend {
  private eventListeners: Set<(event: WorkflowEvent) => void> = new Set();
  private savedWorkflows: Map<string, { graph: WorkflowGraph; metadata: WorkflowMetadata }> = new Map();
  private sessions: Map<string, WorkflowGraph> = new Map();
  private sessionCounter = 0;

  /** Optionally override mock node definitions */
  constructor(private nodeDefinitions: NodeDefinition[] = MOCK_NODE_DEFINITIONS) {}

  private graphMutationResponse(
    graph: WorkflowGraph,
    sessionId: string,
    dirtyTasks: string[],
  ): WorkflowGraphMutationResponse {
    return {
      graph: structuredClone(graph),
      workflow_event: {
        type: 'GraphModified',
        data: {
          workflow_id: sessionId,
          execution_id: sessionId,
          dirty_tasks: dirtyTasks,
        },
      },
    };
  }

  async getNodeDefinitions(): Promise<NodeDefinition[]> {
    return this.nodeDefinitions;
  }

  async validateConnection(sourceType: string, targetType: string): Promise<boolean> {
    return mockValidateConnection(sourceType, targetType);
  }

  async createSession(graph: WorkflowGraph): Promise<WorkflowSessionHandle> {
    const sessionId = `mock-session-${++this.sessionCounter}`;
    this.sessions.set(sessionId, { ...structuredClone(graph), derived_graph: buildDerivedGraph(graph) });
    return {
      session_id: sessionId,
      session_kind: 'edit',
    };
  }

  async runSession(sessionId: string): Promise<void> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);

    await this.simulateExecution(graph);
  }

  async removeSession(sessionId: string): Promise<void> {
    this.sessions.delete(sessionId);
  }

  async executeWorkflow(graph: WorkflowGraph): Promise<void> {
    await this.simulateExecution(graph);
  }

  async addNode(node: GraphNode, sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    graph.nodes.push(node);
    graph.derived_graph = buildDerivedGraph(graph);
    return this.graphMutationResponse(graph, sessionId, [node.id]);
  }

  async removeNode(nodeId: string, sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    graph.nodes = graph.nodes.filter((node) => node.id !== nodeId);
    graph.edges = graph.edges.filter((edge) => edge.source !== nodeId && edge.target !== nodeId);
    graph.derived_graph = buildDerivedGraph(graph);
    return this.graphMutationResponse(graph, sessionId, [nodeId]);
  }

  async addEdge(edge: GraphEdge, sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    graph.edges.push(edge);
    graph.derived_graph = buildDerivedGraph(graph);
    return this.graphMutationResponse(graph, sessionId, [edge.target]);
  }

  async getConnectionCandidates(
    sourceAnchor: ConnectionAnchor,
    sessionId: string,
    graphRevision?: string,
  ): Promise<ConnectionCandidatesResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    return mockGetConnectionCandidates(this.nodeDefinitions, graph, sourceAnchor, graphRevision);
  }

  async connectAnchors(
    sourceAnchor: ConnectionAnchor,
    targetAnchor: ConnectionAnchor,
    sessionId: string,
    graphRevision: string,
  ): Promise<ConnectionCommitResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    return mockConnectAnchors(
      this.nodeDefinitions,
      graph,
      sourceAnchor,
      targetAnchor,
      graphRevision,
    );
  }

  async insertNodeAndConnect(
    sourceAnchor: ConnectionAnchor,
    nodeType: string,
    sessionId: string,
    graphRevision: string,
    positionHint: InsertNodePositionHint,
    preferredInputPortId?: string,
  ): Promise<InsertNodeConnectionResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    return mockInsertNodeAndConnect(
      this.nodeDefinitions,
      graph,
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
    sessionId: string,
    graphRevision: string,
  ): Promise<EdgeInsertionPreviewResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    return mockPreviewNodeInsertOnEdge(
      this.nodeDefinitions,
      graph,
      edgeId,
      nodeType,
      graphRevision,
    );
  }

  async insertNodeOnEdge(
    edgeId: string,
    nodeType: string,
    sessionId: string,
    graphRevision: string,
    positionHint: InsertNodePositionHint,
  ): Promise<InsertNodeOnEdgeResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    return mockInsertNodeOnEdge(
      this.nodeDefinitions,
      graph,
      edgeId,
      nodeType,
      graphRevision,
      positionHint,
    );
  }

  async removeEdge(edgeId: string, sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    const targetNodeId = graph.edges.find((edge) => edge.id === edgeId)?.target ?? null;
    graph.edges = graph.edges.filter((e) => e.id !== edgeId);
    graph.derived_graph = buildDerivedGraph(graph);
    return this.graphMutationResponse(graph, sessionId, targetNodeId ? [targetNodeId] : []);
  }

  async updateNodeData(
    nodeId: string,
    data: Record<string, unknown>,
    sessionId: string,
  ): Promise<WorkflowGraphMutationResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    const node = graph.nodes.find((n) => n.id === nodeId);
    if (node) {
      node.data = { ...node.data, ...data };
    }
    graph.derived_graph = buildDerivedGraph(graph);
    return this.graphMutationResponse(graph, sessionId, [nodeId]);
  }

  async updateNodePosition(
    nodeId: string,
    position: { x: number; y: number },
    sessionId: string,
  ): Promise<WorkflowGraphMutationResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    const node = graph.nodes.find((n) => n.id === nodeId);
    if (node) {
      node.position = position;
    }
    graph.derived_graph = buildDerivedGraph(graph);
    return this.graphMutationResponse(graph, sessionId, []);
  }

  async getExecutionGraph(sessionId: string): Promise<WorkflowGraph> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    return structuredClone(graph);
  }

  async getUndoRedoState(_sessionId: string): Promise<UndoRedoState> {
    return { canUndo: false, canRedo: false, undoCount: 0 };
  }

  async undo(_sessionId: string): Promise<WorkflowGraphMutationResponse> {
    throw new Error('Undo not supported in mock mode');
  }

  async redo(_sessionId: string): Promise<WorkflowGraphMutationResponse> {
    throw new Error('Redo not supported in mock mode');
  }

  async saveWorkflow(name: string, graph: WorkflowGraph): Promise<string> {
    const now = new Date().toISOString();
    this.savedWorkflows.set(name, {
      graph: structuredClone(graph),
      metadata: { name, created: now, modified: now },
    });
    return `/mock/workflows/${name}.json`;
  }

  async loadWorkflow(path: string): Promise<WorkflowFile> {
    const name = path.replace(/.*\//, '').replace('.json', '');
    const saved = this.savedWorkflows.get(name);
    if (saved) {
      return {
        version: '1.0',
        metadata: saved.metadata,
        graph: structuredClone(saved.graph),
      };
    }
    return {
      version: '1.0',
      metadata: { name, created: new Date().toISOString(), modified: new Date().toISOString() },
      graph: { nodes: [], edges: [], derived_graph: buildDerivedGraph({ nodes: [], edges: [] }) },
    };
  }

  async listWorkflows(): Promise<WorkflowMetadata[]> {
    return Array.from(this.savedWorkflows.values()).map((w) => w.metadata);
  }

  async deleteWorkflow(name: string): Promise<void> {
    this.savedWorkflows.delete(name);
  }

  async createGroup(
    name: string,
    selectedNodeIds: string[],
    sessionId: string,
  ): Promise<WorkflowGraphMutationResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    const groupId = `group-${Date.now()}`;
    const selectedNodes = graph.nodes.filter((n) => selectedNodeIds.includes(n.id));
    const selectedSet = new Set(selectedNodeIds);

    const internalEdges = graph.edges.filter(
      (e) => selectedSet.has(e.source) && selectedSet.has(e.target)
    );
    const internalizedEdgeIds = internalEdges.map((e) => e.id);

    // Boundary edges: edges that cross the group boundary (one end inside, one outside)
    const boundaryEdges = graph.edges.filter(
      (e) => selectedSet.has(e.source) !== selectedSet.has(e.target)
    );
    const boundaryEdgeIds = boundaryEdges.map((e) => e.id);

    // Suggest input ports from boundary edges targeting selected nodes
    const suggestedInputs: PortMapping[] = boundaryEdges
      .filter((e) => selectedSet.has(e.target))
      .map((e) => ({
        internal_node_id: e.target,
        internal_port_id: e.target_handle || 'input',
        group_port_id: `in-${e.target}-${e.target_handle || 'input'}`,
        group_port_label: e.target_handle || 'Input',
        data_type: 'any' as PortDataType,
      }));

    // Suggest output ports from boundary edges sourced from selected nodes
    const suggestedOutputs: PortMapping[] = boundaryEdges
      .filter((e) => selectedSet.has(e.source))
      .map((e) => ({
        internal_node_id: e.source,
        internal_port_id: e.source_handle || 'output',
        group_port_id: `out-${e.source}-${e.source_handle || 'output'}`,
        group_port_label: e.source_handle || 'Output',
        data_type: 'any' as PortDataType,
      }));

    const group: NodeGroup = {
      id: groupId,
      name,
      nodes: selectedNodes,
      edges: internalEdges,
      exposed_inputs: suggestedInputs,
      exposed_outputs: suggestedOutputs,
      position: selectedNodes[0]?.position || { x: 0, y: 0 },
      collapsed: true,
    };

    graph.nodes = graph.nodes
      .filter((node) => !selectedSet.has(node.id))
      .concat({
        id: group.id,
        node_type: 'node-group',
        position: group.position,
        data: { label: group.name, group, isGroup: true },
      });
    graph.edges = graph.edges
      .filter((edge) => !internalizedEdgeIds.includes(edge.id))
      .map((edge) => {
        const input = suggestedInputs.find(
          (mapping) =>
            mapping.internal_node_id === edge.target &&
            mapping.internal_port_id === edge.target_handle,
        );
        if (input) {
          return { ...edge, target: group.id, target_handle: input.group_port_id };
        }
        const output = suggestedOutputs.find(
          (mapping) =>
            mapping.internal_node_id === edge.source &&
            mapping.internal_port_id === edge.source_handle,
        );
        if (output) {
          return { ...edge, source: group.id, source_handle: output.group_port_id };
        }
        return edge;
      });
    graph.derived_graph = buildDerivedGraph(graph);

    return this.graphMutationResponse(
      graph,
      sessionId,
      Array.from(new Set([...selectedNodeIds, ...boundaryEdgeIds])),
    );
  }

  async updateGroupPorts(
    groupId: string,
    exposedInputs: PortMapping[],
    exposedOutputs: PortMapping[],
    sessionId: string,
  ): Promise<WorkflowGraphMutationResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    graph.nodes = graph.nodes.map((node) => {
      if (node.id !== groupId) return node;
      const group = node.data.group as NodeGroup;
      const updatedGroup = {
        ...group,
        exposed_inputs: exposedInputs,
        exposed_outputs: exposedOutputs,
      };
      return {
        ...node,
        data: { ...node.data, group: updatedGroup, label: updatedGroup.name, isGroup: true },
      };
    });
    graph.derived_graph = buildDerivedGraph(graph);
    return this.graphMutationResponse(graph, sessionId, [groupId]);
  }

  async ungroup(groupId: string, sessionId: string): Promise<WorkflowGraphMutationResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    const groupNode = graph.nodes.find((node) => node.id === groupId);
    if (!groupNode) throw new Error(`Group not found: ${groupId}`);
    const group = groupNode.data.group as NodeGroup;

    graph.nodes = graph.nodes.filter((node) => node.id !== groupId).concat(group.nodes);
    graph.edges = graph.edges
      .map((edge) => {
        if (edge.target === groupId) {
          const mapping = group.exposed_inputs.find(
            (candidate) => candidate.group_port_id === edge.target_handle,
          );
          if (mapping) {
            return {
              ...edge,
              target: mapping.internal_node_id,
              target_handle: mapping.internal_port_id,
            };
          }
        }
        if (edge.source === groupId) {
          const mapping = group.exposed_outputs.find(
            (candidate) => candidate.group_port_id === edge.source_handle,
          );
          if (mapping) {
            return {
              ...edge,
              source: mapping.internal_node_id,
              source_handle: mapping.internal_port_id,
            };
          }
        }
        return edge;
      })
      .concat(group.edges);
    graph.derived_graph = buildDerivedGraph(graph);
    return this.graphMutationResponse(graph, sessionId, group.nodes.map((node) => node.id));
  }

  subscribeEvents(listener: (event: WorkflowEvent) => void): () => void {
    this.eventListeners.add(listener);
    return () => this.eventListeners.delete(listener);
  }

  // --- Internal ---

  private async simulateExecution(graph: WorkflowGraph): Promise<void> {
    const executionId = `mock-run-${Date.now()}`;
    this.emit({
      type: 'Started',
      data: {
        workflow_id: executionId,
        node_count: graph.nodes.length,
        execution_id: executionId,
      },
    });

    for (const node of graph.nodes) {
      this.emit({
        type: 'NodeStarted',
        data: {
          node_id: node.id,
          node_type: node.node_type,
          execution_id: executionId,
        },
      });
      await sleep(300);
      this.emit({
        type: 'NodeCompleted',
        data: {
          node_id: node.id,
          outputs: {},
          execution_id: executionId,
        },
      });
    }

    this.emit({ type: 'Completed', data: { outputs: {}, execution_id: executionId } });
  }

  private emit(event: WorkflowEvent): void {
    this.eventListeners.forEach((listener) => listener(event));
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

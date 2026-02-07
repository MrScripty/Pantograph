/**
 * MockWorkflowBackend — in-memory mock for testing and prototyping.
 *
 * Provides realistic-enough behavior for frontend development without
 * a real backend. Node definitions, validation, and execution are all mocked.
 */
import type { WorkflowBackend } from '../types/backend.js';
import type { UndoRedoState } from '../types/backend.js';
import type {
  NodeDefinition,
  PortDataType,
  WorkflowGraph,
  WorkflowFile,
  WorkflowMetadata,
  WorkflowEvent,
  GraphNode,
  GraphEdge,
} from '../types/workflow.js';
import type { NodeGroup, PortMapping, CreateGroupResult } from '../types/groups.js';

/** Default mock node definitions */
export const MOCK_NODE_DEFINITIONS: NodeDefinition[] = [
  {
    node_type: 'text-input',
    category: 'input',
    label: 'Text Input',
    description: 'User text input field',
    inputs: [{ id: 'text', label: 'Text', data_type: 'string', required: false, multiple: false }],
    outputs: [{ id: 'text', label: 'Text', data_type: 'string', required: true, multiple: false }],
    execution_mode: 'reactive',
  },
  {
    node_type: 'llm-inference',
    category: 'processing',
    label: 'LLM Inference',
    description: 'Text completion via LLM',
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
    inputs: [{ id: 'text', label: 'Text', data_type: 'string', required: true, multiple: false }],
    outputs: [],
    execution_mode: 'reactive',
  },
];

function mockValidateConnection(sourceType: string, targetType: string): boolean {
  if (targetType === 'any' || sourceType === 'any') return true;
  if (sourceType === targetType) return true;
  if (sourceType === 'string' && targetType === 'prompt') return true;
  if (sourceType === 'json' && targetType === 'string') return true;
  if (sourceType === 'document' && targetType === 'string') return true;
  return false;
}

export class MockWorkflowBackend implements WorkflowBackend {
  private eventListeners: Set<(event: WorkflowEvent) => void> = new Set();
  private savedWorkflows: Map<string, { graph: WorkflowGraph; metadata: WorkflowMetadata }> = new Map();
  private sessions: Map<string, WorkflowGraph> = new Map();
  private sessionCounter = 0;

  /** Optionally override mock node definitions */
  constructor(private nodeDefinitions: NodeDefinition[] = MOCK_NODE_DEFINITIONS) {}

  async getNodeDefinitions(): Promise<NodeDefinition[]> {
    return this.nodeDefinitions;
  }

  async validateConnection(sourceType: string, targetType: string): Promise<boolean> {
    return mockValidateConnection(sourceType, targetType);
  }

  async createSession(graph: WorkflowGraph): Promise<string> {
    const sessionId = `mock-session-${++this.sessionCounter}`;
    this.sessions.set(sessionId, structuredClone(graph));
    return sessionId;
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

  async addNode(node: GraphNode, sessionId: string): Promise<void> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    graph.nodes.push(node);
  }

  async addEdge(edge: GraphEdge, sessionId: string): Promise<WorkflowGraph> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    graph.edges.push(edge);
    return structuredClone(graph);
  }

  async removeEdge(edgeId: string, sessionId: string): Promise<WorkflowGraph> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    graph.edges = graph.edges.filter((e) => e.id !== edgeId);
    return structuredClone(graph);
  }

  async updateNodeData(
    nodeId: string,
    data: Record<string, unknown>,
    sessionId: string,
  ): Promise<void> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    const node = graph.nodes.find((n) => n.id === nodeId);
    if (node) {
      node.data = { ...node.data, ...data };
    }
  }

  async getExecutionGraph(sessionId: string): Promise<WorkflowGraph> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);
    return structuredClone(graph);
  }

  async getUndoRedoState(_sessionId: string): Promise<UndoRedoState> {
    return { canUndo: false, canRedo: false, undoCount: 0 };
  }

  async undo(_sessionId: string): Promise<WorkflowGraph> {
    throw new Error('Undo not supported in mock mode');
  }

  async redo(_sessionId: string): Promise<WorkflowGraph> {
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
      graph: { nodes: [], edges: [] },
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
    graph: WorkflowGraph,
  ): Promise<CreateGroupResult> {
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
      exposed_inputs: [],
      exposed_outputs: [],
      position: selectedNodes[0]?.position || { x: 0, y: 0 },
      collapsed: false,
    };

    return {
      group,
      internalized_edge_ids: internalizedEdgeIds,
      boundary_edge_ids: boundaryEdgeIds,
      suggested_inputs: suggestedInputs,
      suggested_outputs: suggestedOutputs,
    };
  }

  async updateGroupPorts(
    group: NodeGroup,
    exposedInputs: PortMapping[],
    exposedOutputs: PortMapping[],
  ): Promise<NodeGroup> {
    return { ...group, exposed_inputs: exposedInputs, exposed_outputs: exposedOutputs };
  }

  subscribeEvents(listener: (event: WorkflowEvent) => void): () => void {
    this.eventListeners.add(listener);
    return () => this.eventListeners.delete(listener);
  }

  // --- Internal ---

  private async simulateExecution(graph: WorkflowGraph): Promise<void> {
    this.emit({ type: 'Started', data: { workflow_id: `mock-${Date.now()}`, node_count: graph.nodes.length } });

    for (const node of graph.nodes) {
      this.emit({ type: 'NodeStarted', data: { node_id: node.id, node_type: node.node_type } });
      await sleep(300);
      this.emit({ type: 'NodeCompleted', data: { node_id: node.id, outputs: {} } });
    }

    this.emit({ type: 'Completed', data: { outputs: {} } });
  }

  private emit(event: WorkflowEvent): void {
    this.eventListeners.forEach((listener) => listener(event));
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

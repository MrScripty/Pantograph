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
import type { NodeGroup, PortMapping, CreateGroupResult } from '../types/groups.js';
import { isPortTypeCompatible } from '../portTypeCompatibility.js';
import { buildDerivedGraph, computeGraphFingerprint } from '../graphRevision.js';

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

function cloneGraphWithoutEdge(graph: WorkflowGraph, edgeId: string): WorkflowGraph {
  return {
    ...structuredClone(graph),
    edges: graph.edges.filter((edge) => edge.id !== edgeId),
  };
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

  private findDefinition(nodeType: string | undefined): NodeDefinition | undefined {
    return this.nodeDefinitions.find((definition) => definition.node_type === nodeType);
  }

  private resolveEdgeInsertBridge(
    graph: WorkflowGraph,
    edgeId: string,
    nodeType: string,
  ): EdgeInsertionPreviewResponse {
    const currentRevision = computeGraphFingerprint(graph);
    const edge = graph.edges.find((candidate) => candidate.id === edgeId);
    if (!edge) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'unknown_edge',
          message: `edge '${edgeId}' was not found`,
        },
      };
    }

    const sourceNode = graph.nodes.find((node) => node.id === edge.source);
    const targetNode = graph.nodes.find((node) => node.id === edge.target);
    const sourceDef = this.findDefinition(sourceNode?.node_type);
    const targetDef = this.findDefinition(targetNode?.node_type);
    const insertDef = this.findDefinition(nodeType);
    const sourcePort = sourceDef?.outputs.find((port) => port.id === edge.source_handle);
    const targetPort = targetDef?.inputs.find((port) => port.id === edge.target_handle);

    if (!sourcePort) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'unknown_source_anchor',
          message: `source anchor '${edge.source}.${edge.source_handle}' was not found`,
        },
      };
    }
    if (!targetPort) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'unknown_target_anchor',
          message: `target anchor '${edge.target}.${edge.target_handle}' was not found`,
        },
      };
    }
    if (!insertDef) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'unknown_insert_node_type',
          message: `insertable node type '${nodeType}' is unknown`,
        },
      };
    }

    const graphWithoutEdge = cloneGraphWithoutEdge(graph, edgeId);
    for (const inputPort of insertDef.inputs) {
      if (!isPortTypeCompatible(sourcePort.data_type, inputPort.data_type)) {
        continue;
      }

      for (const outputPort of insertDef.outputs) {
        if (!isPortTypeCompatible(outputPort.data_type, targetPort.data_type)) {
          continue;
        }

        const targetOccupied = graphWithoutEdge.edges.some(
          (candidate) =>
            candidate.target === edge.target && candidate.target_handle === edge.target_handle,
        );
        if (!targetPort.multiple && targetOccupied) {
          continue;
        }

        return {
          accepted: true,
          graph_revision: currentRevision,
          bridge: {
            input_port_id: inputPort.id,
            output_port_id: outputPort.id,
          },
        };
      }
    }

    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'no_compatible_insert_path',
        message: `node type '${nodeType}' has no valid path between edge '${edgeId}'`,
      },
    };
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

    const sourceNode = graph.nodes.find((node) => node.id === sourceAnchor.node_id);
    if (!sourceNode) throw new Error(`Source node not found: ${sourceAnchor.node_id}`);
    const sourceDef = this.nodeDefinitions.find((def) => def.node_type === sourceNode.node_type);
    const sourcePort = sourceDef?.outputs.find((port) => port.id === sourceAnchor.port_id);
    if (!sourcePort) throw new Error(`Source anchor not found: ${sourceAnchor.node_id}.${sourceAnchor.port_id}`);

    const compatibleNodes = graph.nodes
      .filter((node) => node.id !== sourceAnchor.node_id)
      .map((node) => {
        const definition = this.nodeDefinitions.find((def) => def.node_type === node.node_type);
        if (!definition) return null;

        const anchors = definition.inputs
          .filter((port) => {
            if (!isPortTypeCompatible(sourcePort.data_type, port.data_type)) return false;
            if (!port.multiple) {
              return !graph.edges.some(
                (edge) => edge.target === node.id && edge.target_handle === port.id
              );
            }
            return true;
          })
          .map((port) => ({
            port_id: port.id,
            port_label: port.label,
            data_type: port.data_type,
            multiple: port.multiple,
          }));

        if (anchors.length === 0) return null;

        return {
          node_id: node.id,
          node_type: node.node_type,
          node_label: String(node.data.label ?? definition.label),
          position: node.position,
          anchors,
        };
      })
      .filter((node): node is NonNullable<typeof node> => node !== null);

    const insertableNodeTypes = this.nodeDefinitions
      .map((definition) => {
        const matchingInputPortIds = definition.inputs
          .filter((port) => isPortTypeCompatible(sourcePort.data_type, port.data_type))
          .map((port) => port.id);
        if (matchingInputPortIds.length === 0) return null;
        return {
          node_type: definition.node_type,
          category: definition.category,
          label: definition.label,
          description: definition.description,
          matching_input_port_ids: matchingInputPortIds,
        };
      })
      .filter((node): node is NonNullable<typeof node> => node !== null);

    const currentRevision = computeGraphFingerprint(graph);
    return {
      graph_revision: currentRevision,
      revision_matches: !graphRevision || graphRevision === currentRevision,
      source_anchor: sourceAnchor,
      compatible_nodes: compatibleNodes,
      insertable_node_types: insertableNodeTypes,
    };
  }

  async connectAnchors(
    sourceAnchor: ConnectionAnchor,
    targetAnchor: ConnectionAnchor,
    sessionId: string,
    graphRevision: string,
  ): Promise<ConnectionCommitResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);

    const currentRevision = computeGraphFingerprint(graph);
    if (graphRevision !== currentRevision) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'stale_revision',
          message: `graph revision '${graphRevision}' is stale`,
        },
      };
    }

    const sourceNode = graph.nodes.find((node) => node.id === sourceAnchor.node_id);
    const targetNode = graph.nodes.find((node) => node.id === targetAnchor.node_id);
    const sourceDef = this.nodeDefinitions.find((def) => def.node_type === sourceNode?.node_type);
    const targetDef = this.nodeDefinitions.find((def) => def.node_type === targetNode?.node_type);
    const sourcePort = sourceDef?.outputs.find((port) => port.id === sourceAnchor.port_id);
    const targetPort = targetDef?.inputs.find((port) => port.id === targetAnchor.port_id);

    if (!sourcePort) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'unknown_source_anchor',
          message: `source anchor '${sourceAnchor.node_id}.${sourceAnchor.port_id}' was not found`,
        },
      };
    }
    if (!targetPort) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'unknown_target_anchor',
          message: `target anchor '${targetAnchor.node_id}.${targetAnchor.port_id}' was not found`,
        },
      };
    }
    if (!isPortTypeCompatible(sourcePort.data_type, targetPort.data_type)) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'incompatible_types',
          message: `${sourcePort.data_type} cannot connect to ${targetPort.data_type}`,
        },
      };
    }

    const edge: GraphEdge = {
      id: `${sourceAnchor.node_id}-${sourceAnchor.port_id}-${targetAnchor.node_id}-${targetAnchor.port_id}`,
      source: sourceAnchor.node_id,
      source_handle: sourceAnchor.port_id,
      target: targetAnchor.node_id,
      target_handle: targetAnchor.port_id,
    };
    graph.edges.push(edge);
    graph.derived_graph = buildDerivedGraph(graph);
    return {
      accepted: true,
      graph_revision: graph.derived_graph.graph_fingerprint,
      graph: structuredClone(graph),
    };
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

    const currentRevision = computeGraphFingerprint(graph);
    if (graphRevision !== currentRevision) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'stale_revision',
          message: `graph revision '${graphRevision}' is stale`,
        },
      };
    }

    const sourceNode = graph.nodes.find((node) => node.id === sourceAnchor.node_id);
    const sourceDef = this.nodeDefinitions.find((def) => def.node_type === sourceNode?.node_type);
    const sourcePort = sourceDef?.outputs.find((port) => port.id === sourceAnchor.port_id);
    if (!sourcePort) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'unknown_source_anchor',
          message: `source anchor '${sourceAnchor.node_id}.${sourceAnchor.port_id}' was not found`,
        },
      };
    }

    const insertDef = this.nodeDefinitions.find((def) => def.node_type === nodeType);
    if (!insertDef) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'unknown_insert_node_type',
          message: `insertable node type '${nodeType}' is unknown`,
        },
      };
    }

    const targetPort =
      insertDef.inputs.find(
        (port) =>
          preferredInputPortId &&
          port.id === preferredInputPortId &&
          isPortTypeCompatible(sourcePort.data_type, port.data_type)
      ) ??
      insertDef.inputs
        .filter((port) => isPortTypeCompatible(sourcePort.data_type, port.data_type))
        .sort((left, right) => left.label.localeCompare(right.label) || left.id.localeCompare(right.id))[0];

    if (!targetPort) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'no_compatible_insert_input',
          message: `node type '${nodeType}' has no compatible input for ${sourcePort.data_type}`,
        },
      };
    }

    const insertedNodeId = `${nodeType}-${Date.now()}`;
    graph.nodes.push({
      id: insertedNodeId,
      node_type: nodeType,
      position: positionHint.position,
      data: {
        label: insertDef.label,
        ...Object.fromEntries(insertDef.inputs.map((input) => [input.id, null])),
      },
    });
    graph.edges.push({
      id: `${sourceAnchor.node_id}-${sourceAnchor.port_id}-${insertedNodeId}-${targetPort.id}`,
      source: sourceAnchor.node_id,
      source_handle: sourceAnchor.port_id,
      target: insertedNodeId,
      target_handle: targetPort.id,
    });
    graph.derived_graph = buildDerivedGraph(graph);

    return {
      accepted: true,
      graph_revision: graph.derived_graph.graph_fingerprint,
      inserted_node_id: insertedNodeId,
      graph: structuredClone(graph),
    };
  }

  async previewNodeInsertOnEdge(
    edgeId: string,
    nodeType: string,
    sessionId: string,
    graphRevision: string,
  ): Promise<EdgeInsertionPreviewResponse> {
    const graph = this.sessions.get(sessionId);
    if (!graph) throw new Error(`Session not found: ${sessionId}`);

    const currentRevision = computeGraphFingerprint(graph);
    if (graphRevision !== currentRevision) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'stale_revision',
          message: `graph revision '${graphRevision}' is stale`,
        },
      };
    }

    return this.resolveEdgeInsertBridge(graph, edgeId, nodeType);
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

    const currentRevision = computeGraphFingerprint(graph);
    if (graphRevision !== currentRevision) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'stale_revision',
          message: `graph revision '${graphRevision}' is stale`,
        },
      };
    }

    const preview = this.resolveEdgeInsertBridge(graph, edgeId, nodeType);
    if (!preview.accepted || !preview.bridge) {
      return {
        accepted: false,
        graph_revision: preview.graph_revision,
        rejection: preview.rejection,
      };
    }

    const edge = graph.edges.find((candidate) => candidate.id === edgeId);
    if (!edge) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'unknown_edge',
          message: `edge '${edgeId}' was not found`,
        },
      };
    }

    const insertDef = this.findDefinition(nodeType);
    if (!insertDef) {
      return {
        accepted: false,
        graph_revision: currentRevision,
        rejection: {
          reason: 'unknown_insert_node_type',
          message: `insertable node type '${nodeType}' is unknown`,
        },
      };
    }

    const insertedNodeId = `${nodeType}-${Date.now()}`;
    graph.edges = graph.edges.filter((candidate) => candidate.id !== edgeId);
    graph.nodes.push({
      id: insertedNodeId,
      node_type: nodeType,
      position: positionHint.position,
      data: {
        label: insertDef.label,
        ...Object.fromEntries(insertDef.inputs.map((input) => [input.id, null])),
      },
    });
    graph.edges.push({
      id: `${edge.source}-${edge.source_handle}-${insertedNodeId}-${preview.bridge.input_port_id}`,
      source: edge.source,
      source_handle: edge.source_handle,
      target: insertedNodeId,
      target_handle: preview.bridge.input_port_id,
    });
    graph.edges.push({
      id: `${insertedNodeId}-${preview.bridge.output_port_id}-${edge.target}-${edge.target_handle}`,
      source: insertedNodeId,
      source_handle: preview.bridge.output_port_id,
      target: edge.target,
      target_handle: edge.target_handle,
    });
    graph.derived_graph = buildDerivedGraph(graph);

    return {
      accepted: true,
      graph_revision: graph.derived_graph.graph_fingerprint,
      inserted_node_id: insertedNodeId,
      bridge: preview.bridge,
      graph: structuredClone(graph),
    };
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

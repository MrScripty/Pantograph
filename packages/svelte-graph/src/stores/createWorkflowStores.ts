/**
 * Workflow Store Factory — creates per-instance workflow state
 *
 * This is the main assembler that creates all workflow sub-stores
 * and returns a unified API. Replaces the global workflowStore.
 */
import { writable, derived, get } from 'svelte/store';
import type { Node, Edge } from '@xyflow/svelte';
import type {
  WorkflowGraph,
  WorkflowMetadata,
  NodeExecutionState,
  NodeExecutionInfo,
  ConnectionIntentState,
  NodeDefinition,
  GraphNode,
} from '../types/workflow.js';
import type { NodeGroup, PortMapping } from '../types/groups.js';
import type { ViewportState } from '../types/view.js';
import type { WorkflowBackend } from '../types/backend.js';
import {
  appendNodeStreamContent,
  clearNodeRuntimeDataKeysInNodes,
  clearNodeStreamContent,
  setNodeStreamContent,
  updateNodeRuntimeDataInNodes,
} from './runtimeData.ts';
import { buildDerivedGraph } from '../graphRevision.ts';
import { applyWorkflowGraphMutationResponse } from './workflowGraphMutationResponse.ts';
import type { WorkflowGraphMutationResponse } from '../types/workflow.js';
import {
  extractWorkflowNodeGroups,
  findWorkflowGroupContainingNodeIds,
  getWorkflowConnectedNodes,
  getWorkflowNodesBounds,
} from './workflowStoreGraphQueries.ts';
import {
  materializeWorkflowGraphSnapshot,
  projectWorkflowGraphStoreState,
} from './workflowStoreMaterialization.ts';
import { buildDefaultWorkflowGraphState } from './defaultWorkflowGraph.ts';
import { edgeToGraphEdge } from '../workflowConnections.ts';

interface InferenceParamSchema {
  key: string;
  label: string;
  param_type: 'Number' | 'Integer' | 'String' | 'Boolean';
  default: unknown;
  description?: string;
  constraints?: {
    min?: number;
    max?: number;
    allowed_values?: unknown[];
  };
}

export type WorkflowGraphMutationResultStatus = 'applied' | 'failed' | 'skipped' | 'stale';

export interface WorkflowGraphMutationResult {
  action: string;
  error?: unknown;
  response?: WorkflowGraphMutationResponse;
  sessionId: string | null;
  status: WorkflowGraphMutationResultStatus;
}

interface SyncEdgesFromBackendOptions {
  markDirty?: boolean;
  sessionId?: string;
}

export interface WorkflowStores {
  // Writable stores
  nodes: ReturnType<typeof writable<Node[]>>;
  edges: ReturnType<typeof writable<Edge[]>>;
  nodeDefinitions: ReturnType<typeof writable<NodeDefinition[]>>;
  workflowMetadata: ReturnType<typeof writable<WorkflowMetadata | null>>;
  isDirty: ReturnType<typeof writable<boolean>>;
  isExecuting: ReturnType<typeof writable<boolean>>;
  isEditing: ReturnType<typeof writable<boolean>>;
  nodeExecutionStates: ReturnType<typeof writable<Map<string, NodeExecutionInfo>>>;
  currentViewport: ReturnType<typeof writable<ViewportState>>;
  nodeGroups: ReturnType<typeof writable<Map<string, NodeGroup>>>;
  selectedNodeIds: ReturnType<typeof writable<string[]>>;
  connectionIntent: ReturnType<typeof writable<ConnectionIntentState | null>>;

  // Derived stores
  workflowGraph: ReturnType<typeof derived>;
  nodeDefinitionsByCategory: ReturnType<typeof derived>;

  // Actions — nodes
  addNode: (definition: NodeDefinition, position: { x: number; y: number }) => Promise<WorkflowGraphMutationResult>;
  removeNode: (nodeId: string) => Promise<WorkflowGraphMutationResult>;
  updateNodePosition: (
    nodeId: string,
    position: { x: number; y: number },
  ) => Promise<WorkflowGraphMutationResult>;
  updateNodeData: (
    nodeId: string,
    data: Record<string, unknown>,
  ) => Promise<WorkflowGraphMutationResult>;
  updateNodeRuntimeData: (nodeId: string, data: Record<string, unknown>) => void;
  clearNodeRuntimeData: (keys: string[]) => void;
  getNodeById: (nodeId: string) => Node | undefined;
  isNodeGroup: (nodeId: string) => boolean;
  getConnectedNodes: (nodeId: string) => { inputs: Node[]; outputs: Node[] };
  getNodesBounds: (nodeIds: string[]) => { x: number; y: number; width: number; height: number } | null;

  // Actions — edges
  addEdge: (edge: Edge) => Promise<WorkflowGraphMutationResult>;
  removeEdge: (edgeId: string) => Promise<WorkflowGraphMutationResult>;
  syncEdgesFromBackend: (
    backendGraph: WorkflowGraph,
    options?: SyncEdgesFromBackendOptions,
  ) => boolean;

  // Actions — execution
  setNodeExecutionState: (nodeId: string, state: NodeExecutionState, message?: string) => void;
  getNodeExecutionInfo: (nodeId: string) => NodeExecutionInfo | undefined;
  resetExecutionStates: () => void;

  // Actions — streaming
  appendStreamContent: (nodeId: string, chunk: string) => void;
  setStreamContent: (nodeId: string, content: string) => void;
  clearStreamContent: () => void;

  // Actions — workflow
  loadWorkflow: (graph: WorkflowGraph, metadata?: WorkflowMetadata) => void;
  clearWorkflow: () => void;
  loadDefaultWorkflow: (definitions: NodeDefinition[]) => void;
  updateViewport: (viewport: ViewportState) => void;
  setConnectionIntent: (intent: ConnectionIntentState | null) => void;
  clearConnectionIntent: () => void;
  setActiveSessionId: (sessionId: string | null) => void;

  // Compatibility no-ops while graph canonicalization is backend-owned.
  syncInferencePorts: (sourceNodeId: string, inferenceSettings: InferenceParamSchema[]) => void;
  syncExpandPorts: (sourceNodeId: string, inferenceSettings: InferenceParamSchema[]) => void;
  autoConnectExpandToInference: (expandNodeId: string, inferenceSettings: InferenceParamSchema[]) => void;

  // Actions — groups
  createGroup: (name: string, nodeIds: string[]) => Promise<NodeGroup | null>;
  ungroupNodes: (groupId: string) => Promise<boolean>;
  updateGroupPorts: (groupId: string, exposedInputs: PortMapping[], exposedOutputs: PortMapping[]) => Promise<boolean>;
  getGroupById: (groupId: string) => NodeGroup | undefined;
  collapseGroup: () => void;
}

/**
 * Create per-instance workflow stores.
 *
 * @param backend - The workflow backend implementation
 * @param viewStores - Optional view stores (for group stack wiring). Pass after creating view stores.
 */
export function createWorkflowStores(
  backend: WorkflowBackend,
  viewStores?: {
    groupStack: ReturnType<typeof writable<string[]>>;
    tabOutOfGroup: () => Promise<void>;
  },
): WorkflowStores {
  // --- Writable stores ---
  const nodes = writable<Node[]>([]);
  const edges = writable<Edge[]>([]);
  const nodeDefinitions = writable<NodeDefinition[]>([]);
  const workflowMetadata = writable<WorkflowMetadata | null>(null);
  const isDirty = writable<boolean>(false);
  const isExecuting = writable<boolean>(false);
  const isEditing = writable<boolean>(true);
  const derivedGraph = writable<WorkflowGraph['derived_graph']>(undefined);
  const nodeExecutionStates = writable<Map<string, NodeExecutionInfo>>(new Map());
  const currentViewport = writable<ViewportState>({ x: 0, y: 0, zoom: 1 });
  const nodeGroups = writable<Map<string, NodeGroup>>(new Map());
  const selectedNodeIds = writable<string[]>([]);
  const connectionIntent = writable<ConnectionIntentState | null>(null);
  let activeSessionId: string | null = null;

  // --- Derived stores ---
  const workflowGraph = derived(
    [nodes, edges, derivedGraph],
    ([$nodes, $edges, $derivedGraph]): WorkflowGraph =>
      projectWorkflowGraphStoreState({
        nodes: $nodes,
        edges: $edges,
        derivedGraph: $derivedGraph,
      }),
  );

  const nodeDefinitionsByCategory = derived(nodeDefinitions, ($defs) => {
    const grouped = new Map<string, NodeDefinition[]>();
    for (const def of $defs) {
      const list = grouped.get(def.category) || [];
      list.push(def);
      grouped.set(def.category, list);
    }
    return grouped;
  });

  function setActiveSessionId(sessionId: string | null) {
    activeSessionId = sessionId;
  }

  function isActiveSession(sessionId: string): boolean {
    return activeSessionId === sessionId;
  }

  function materializeWorkflowGraph(graph: WorkflowGraph) {
    const definitions = get(nodeDefinitions);
    const selectedIds = get(selectedNodeIds);

    return materializeWorkflowGraphSnapshot({
      graph,
      definitions,
      selectedNodeIds: selectedIds,
    });
  }

  function applyWorkflowGraph(
    graph: WorkflowGraph,
    options?: {
      metadata?: WorkflowMetadata | null;
      markDirty?: boolean;
    },
  ) {
    const { graphNodes, graphEdges, graph: nextGraph } = materializeWorkflowGraph(graph);
    nodes.set(graphNodes);
    edges.set(graphEdges);
    nodeGroups.set(extractWorkflowNodeGroups(graphNodes));
    if (typeof options?.metadata !== 'undefined') {
      workflowMetadata.set(options.metadata);
    }
    connectionIntent.set(null);
    derivedGraph.set(buildDerivedGraph(nextGraph));
    isDirty.set(options?.markDirty ?? true);
  }

  function applyBackendMutationResponse(
    sessionId: string,
    response: WorkflowGraphMutationResponse,
  ): boolean {
    if (!isActiveSession(sessionId)) {
      return false;
    }

    applyWorkflowGraph(response.graph, { markDirty: true });
    applyWorkflowGraphMutationResponse(response, {
      setNodeExecutionState,
    });
    return true;
  }

  async function syncGraphMutationFromBackend(
    action: string,
    mutate: (sessionId: string) => Promise<WorkflowGraphMutationResponse>,
  ): Promise<WorkflowGraphMutationResult> {
    if (!activeSessionId) {
      console.warn(`[workflowStores] Ignoring ${action} without an active session`);
      return { action, sessionId: null, status: 'skipped' };
    }

    const requestSessionId = activeSessionId;

    try {
      const response = await mutate(requestSessionId);
      if (!applyBackendMutationResponse(requestSessionId, response)) {
        return { action, response, sessionId: requestSessionId, status: 'stale' };
      }
      return { action, response, sessionId: requestSessionId, status: 'applied' };
    } catch (error) {
      if (!isActiveSession(requestSessionId)) {
        return { action, error, sessionId: requestSessionId, status: 'stale' };
      }

        console.error(`[workflowStores] Failed to ${action}:`, error);
      return { action, error, sessionId: requestSessionId, status: 'failed' };
    }
  }

  // --- Node actions ---

  function addNode(
    definition: NodeDefinition,
    position: { x: number; y: number },
  ): Promise<WorkflowGraphMutationResult> {
    const id = `${definition.node_type}-${Date.now()}`;
    const newNode: GraphNode = {
      id,
      node_type: definition.node_type,
      position,
      data: {
        label: definition.label,
        definition,
        ...Object.fromEntries(definition.inputs.map((input) => [input.id, null])),
      },
    };
    selectedNodeIds.set([id]);
    return syncGraphMutationFromBackend('add node', (sessionId) =>
      backend.addNode(newNode, sessionId)
    );
  }

  function removeNode(nodeId: string): Promise<WorkflowGraphMutationResult> {
    selectedNodeIds.update((ids) => ids.filter((id) => id !== nodeId));
    return syncGraphMutationFromBackend('remove node', (sessionId) =>
      backend.removeNode(nodeId, sessionId)
    );
  }

  function updateNodePosition(
    nodeId: string,
    position: { x: number; y: number },
  ): Promise<WorkflowGraphMutationResult> {
    return syncGraphMutationFromBackend('update node position', (sessionId) =>
      backend.updateNodePosition(nodeId, position, sessionId)
    );
  }

  function updateNodeData(
    nodeId: string,
    data: Record<string, unknown>,
  ): Promise<WorkflowGraphMutationResult> {
    return syncGraphMutationFromBackend('update node data', (sessionId) =>
      backend.updateNodeData(nodeId, data, sessionId)
    );
  }

  function updateNodeRuntimeData(nodeId: string, data: Record<string, unknown>) {
    nodes.update((n) => updateNodeRuntimeDataInNodes(n, nodeId, data));
  }

  function clearNodeRuntimeData(keys: string[]) {
    if (keys.length === 0) return;

    nodes.update((n) => clearNodeRuntimeDataKeysInNodes(n, keys));
  }

  function getNodeById(nodeId: string): Node | undefined {
    return get(nodes).find((n) => n.id === nodeId);
  }

  function isNodeGroupFn(nodeId: string): boolean {
    const node = getNodeById(nodeId);
    if (!node) return false;
    return node.data?.isGroup === true || node.type === 'node-group';
  }

  function getConnectedNodes(nodeId: string): { inputs: Node[]; outputs: Node[] } {
    return getWorkflowConnectedNodes(get(nodes), get(edges), nodeId);
  }

  function getNodesBounds(nodeIds: string[]) {
    return getWorkflowNodesBounds(get(nodes), nodeIds);
  }

  // --- Edge actions ---

  function addEdgeFn(edge: Edge): Promise<WorkflowGraphMutationResult> {
    return syncGraphMutationFromBackend('add edge', (sessionId) =>
      backend.addEdge(edgeToGraphEdge(edge), sessionId),
    );
  }

  function removeEdgeFn(edgeId: string): Promise<WorkflowGraphMutationResult> {
    return syncGraphMutationFromBackend('remove edge', (sessionId) =>
      backend.removeEdge(edgeId, sessionId)
    );
  }

  function syncEdgesFromBackend(
    backendGraph: WorkflowGraph,
    options?: SyncEdgesFromBackendOptions,
  ): boolean {
    if (options?.sessionId && !isActiveSession(options.sessionId)) {
      return false;
    }

    applyWorkflowGraph(backendGraph, { markDirty: options?.markDirty ?? true });
    return true;
  }

  // --- Execution actions ---

  function setNodeExecutionState(nodeId: string, state: NodeExecutionState, message?: string) {
    nodeExecutionStates.update((map) => {
      const newMap = new Map(map);
      newMap.set(nodeId, { state, message });
      return newMap;
    });
  }

  function getNodeExecutionInfo(nodeId: string): NodeExecutionInfo | undefined {
    return get(nodeExecutionStates).get(nodeId);
  }

  function resetExecutionStates() {
    nodeExecutionStates.set(new Map());
  }

  // --- Streaming actions ---

  function appendStreamContent(nodeId: string, chunk: string) {
    nodes.update((n) => appendNodeStreamContent(n, nodeId, chunk));
  }

  function setStreamContent(nodeId: string, content: string) {
    nodes.update((n) => setNodeStreamContent(n, nodeId, content));
  }

  function clearStreamContent() {
    nodes.update(clearNodeStreamContent);
  }

  // --- Workflow actions ---

  function loadWorkflowFn(graph: WorkflowGraph, metadata?: WorkflowMetadata) {
    selectedNodeIds.set([]);
    applyWorkflowGraph(graph, {
      metadata: metadata || null,
      markDirty: false,
    });
  }

  function clearWorkflow() {
    nodes.set([]);
    edges.set([]);
    nodeGroups.set(new Map());
    workflowMetadata.set(null);
    selectedNodeIds.set([]);
    connectionIntent.set(null);
    derivedGraph.set(
      buildDerivedGraph({
        nodes: [],
        edges: [],
      })
    );
    isDirty.set(false);
    resetExecutionStates();
  }

  function loadDefaultWorkflow(definitions: NodeDefinition[]) {
    selectedNodeIds.set([]);
    const defaultWorkflow = buildDefaultWorkflowGraphState(definitions);
    nodes.set(defaultWorkflow.nodes);
    nodeGroups.set(new Map());
    edges.set(defaultWorkflow.edges);
    connectionIntent.set(null);
    derivedGraph.set(buildDerivedGraph(defaultWorkflow.graph));
    isDirty.set(false);
  }

  function updateViewport(viewport: ViewportState) {
    currentViewport.set(viewport);
  }

  function setConnectionIntent(intent: ConnectionIntentState | null) {
    connectionIntent.set(intent);
  }

  function clearConnectionIntent() {
    connectionIntent.set(null);
  }

  function syncInferencePorts(
    _sourceNodeId: string,
    _inferenceSettings: InferenceParamSchema[],
  ) {
    // Backend-owned graph canonicalization now applies inference port changes.
  }

  function syncExpandPorts(
    _sourceNodeId: string,
    _inferenceSettings: InferenceParamSchema[],
  ): void {
    // Backend-owned graph canonicalization now applies expand-settings changes.
  }

  function autoConnectExpandToInference(
    _expandNodeId: string,
    _inferenceSettings: InferenceParamSchema[],
  ): void {
    // Backend-owned graph canonicalization now applies expand passthrough edges.
  }

  // --- Group actions ---

  async function createGroup(name: string, nodeIds: string[]): Promise<NodeGroup | null> {
    if (nodeIds.length < 2) {
      console.warn('[workflowStores] Cannot create group with less than 2 nodes');
      return null;
    }
    if (!activeSessionId) {
      console.warn('[workflowStores] Cannot create group without an active session');
      return null;
    }

    try {
      const requestSessionId = activeSessionId;
      const response = await backend.createGroup(name, nodeIds, requestSessionId);
      if (!applyBackendMutationResponse(requestSessionId, response)) {
        return null;
      }
      return findWorkflowGroupContainingNodeIds(get(nodeGroups), nodeIds);
    } catch (error) {
      console.error('[workflowStores] Failed to create group:', error);
      return null;
    }
  }

  async function ungroupNodes(groupId: string): Promise<boolean> {
    if (!activeSessionId) {
      console.warn('[workflowStores] Cannot ungroup without an active session');
      return false;
    }

    try {
      const requestSessionId = activeSessionId;
      const response = await backend.ungroup(groupId, requestSessionId);
      return applyBackendMutationResponse(requestSessionId, response);
    } catch (error) {
      console.error('[workflowStores] Failed to ungroup:', error);
      return false;
    }
  }

  async function updateGroupPortsFn(
    groupId: string,
    exposedInputs: PortMapping[],
    exposedOutputs: PortMapping[],
  ): Promise<boolean> {
    if (!activeSessionId) {
      console.warn('[workflowStores] Cannot update group ports without an active session');
      return false;
    }

    try {
      const requestSessionId = activeSessionId;
      const response = await backend.updateGroupPorts(
        groupId,
        exposedInputs,
        exposedOutputs,
        requestSessionId,
      );
      return applyBackendMutationResponse(requestSessionId, response);
    } catch (error) {
      console.error('[workflowStores] Failed to update group ports:', error);
      return false;
    }
  }

  function getGroupById(groupId: string): NodeGroup | undefined {
    return get(nodeGroups).get(groupId);
  }

  function collapseGroup(): void {
    viewStores?.tabOutOfGroup();
  }

  return {
    // Stores
    nodes, edges, nodeDefinitions, workflowMetadata, isDirty, isExecuting,
    isEditing, nodeExecutionStates, currentViewport, nodeGroups, selectedNodeIds,
    connectionIntent,
    workflowGraph, nodeDefinitionsByCategory,
    // Node actions
    addNode, removeNode, updateNodePosition, updateNodeData, updateNodeRuntimeData,
    clearNodeRuntimeData,
    getNodeById, isNodeGroup: isNodeGroupFn, getConnectedNodes, getNodesBounds,
    // Edge actions
    addEdge: addEdgeFn, removeEdge: removeEdgeFn, syncEdgesFromBackend,
    // Execution actions
    setNodeExecutionState, getNodeExecutionInfo, resetExecutionStates,
    // Streaming actions
    appendStreamContent, setStreamContent, clearStreamContent,
    // Workflow actions
    loadWorkflow: loadWorkflowFn, clearWorkflow, loadDefaultWorkflow, updateViewport,
    setConnectionIntent, clearConnectionIntent, setActiveSessionId,
    // Compatibility no-ops
    syncInferencePorts, syncExpandPorts, autoConnectExpandToInference,
    // Group actions
    createGroup, ungroupNodes, updateGroupPorts: updateGroupPortsFn,
    getGroupById, collapseGroup,
  };
}

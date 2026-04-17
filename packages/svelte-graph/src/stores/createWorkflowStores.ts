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
import { removeNodeDataKeys } from './runtimeData.js';
import { buildDerivedGraph } from '../graphRevision.js';
import { resolveNodeDefinitionOverlay } from './definitionOverlay.ts';
import { applySelectedNodeIds } from '../workflowSelection.js';

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
  addNode: (definition: NodeDefinition, position: { x: number; y: number }) => void;
  removeNode: (nodeId: string) => void;
  updateNodePosition: (nodeId: string, position: { x: number; y: number }) => void;
  updateNodeData: (nodeId: string, data: Record<string, unknown>) => void;
  updateNodeRuntimeData: (nodeId: string, data: Record<string, unknown>) => void;
  clearNodeRuntimeData: (keys: string[]) => void;
  getNodeById: (nodeId: string) => Node | undefined;
  isNodeGroup: (nodeId: string) => boolean;
  getConnectedNodes: (nodeId: string) => { inputs: Node[]; outputs: Node[] };
  getNodesBounds: (nodeIds: string[]) => { x: number; y: number; width: number; height: number } | null;

  // Actions — edges
  addEdge: (edge: Edge) => void;
  removeEdge: (edgeId: string) => void;
  syncEdgesFromBackend: (backendGraph: WorkflowGraph) => void;

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
    ([$nodes, $edges, $derivedGraph]): WorkflowGraph => ({
      nodes: $nodes.map((n) => ({
        id: n.id,
        node_type: n.type || 'unknown',
        position: n.position,
        data: n.data,
      })),
      edges: $edges.map((e) => ({
        id: e.id,
        source: e.source,
        source_handle: e.sourceHandle || 'output',
        target: e.target,
        target_handle: e.targetHandle || 'input',
      })),
      ...(typeof $derivedGraph === 'undefined' ? {} : { derived_graph: $derivedGraph }),
    })
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

  function materializeWorkflowGraph(graph: WorkflowGraph) {
    const definitions = get(nodeDefinitions);
    const selectedIds = get(selectedNodeIds);

    const graphNodes = applySelectedNodeIds(
      graph.nodes.map((n) => {
        const nodeType = n.node_type;
        const nodeData = { ...n.data };

        const definition = resolveNodeDefinitionOverlay(nodeType, nodeData, definitions);
        return {
          id: n.id,
          type: nodeType,
          position: n.position,
          data: { ...nodeData, definition },
        };
      }),
      selectedIds,
    );

    const graphEdges: Edge[] = graph.edges.map((e) => {
      return {
        id: e.id,
        source: e.source,
        sourceHandle: e.source_handle,
        target: e.target,
        targetHandle: e.target_handle,
      };
    });

    return { graphNodes, graphEdges, graph };
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
    if (typeof options?.metadata !== 'undefined') {
      workflowMetadata.set(options.metadata);
    }
    connectionIntent.set(null);
    derivedGraph.set(buildDerivedGraph(nextGraph));
    isDirty.set(options?.markDirty ?? true);
  }

  function syncGraphMutationFromBackend(
    action: string,
    mutate: (sessionId: string) => Promise<WorkflowGraph>,
  ) {
    if (!activeSessionId) {
      console.warn(`[workflowStores] Ignoring ${action} without an active session`);
      return;
    }

    void mutate(activeSessionId)
      .then((graph) => {
        applyWorkflowGraph(graph, { markDirty: true });
      })
      .catch((error) => {
        console.error(`[workflowStores] Failed to ${action}:`, error);
      });
  }

  function markGraphModified() {
    isDirty.set(true);
    connectionIntent.set(null);
    derivedGraph.set(
      buildDerivedGraph({
        nodes: get(nodes).map((node) => ({
          id: node.id,
          node_type: node.type || 'unknown',
          position: node.position,
          data: node.data,
        })),
        edges: get(edges).map((edge) => ({
          id: edge.id,
          source: edge.source,
          source_handle: edge.sourceHandle || 'output',
          target: edge.target,
          target_handle: edge.targetHandle || 'input',
        })),
      })
    );
  }

  // --- Node actions ---

  function addNode(definition: NodeDefinition, position: { x: number; y: number }) {
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
    syncGraphMutationFromBackend('add node', (sessionId) => backend.addNode(newNode, sessionId));
  }

  function removeNode(nodeId: string) {
    selectedNodeIds.update((ids) => ids.filter((id) => id !== nodeId));
    syncGraphMutationFromBackend('remove node', (sessionId) => backend.removeNode(nodeId, sessionId));
  }

  function updateNodePosition(nodeId: string, position: { x: number; y: number }) {
    syncGraphMutationFromBackend('update node position', (sessionId) =>
      backend.updateNodePosition(nodeId, position, sessionId)
    );
  }

  function updateNodeData(nodeId: string, data: Record<string, unknown>) {
    syncGraphMutationFromBackend('update node data', (sessionId) =>
      backend.updateNodeData(nodeId, data, sessionId)
    );
  }

  function updateNodeRuntimeData(nodeId: string, data: Record<string, unknown>) {
    nodes.update((n) =>
      n.map((node) =>
        node.id === nodeId ? { ...node, data: { ...node.data, ...data } } : node
      )
    );
  }

  function clearNodeRuntimeData(keys: string[]) {
    if (keys.length === 0) return;

    const runtimeKeys = new Set(keys);
    nodes.update((n) =>
      n.map((node) => {
        const { changed, data } = removeNodeDataKeys(node.data, runtimeKeys);
        return changed ? { ...node, data } : node;
      })
    );
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
    const currentEdges = get(edges);
    const currentNodes = get(nodes);
    const inputNodeIds = currentEdges.filter((e) => e.target === nodeId).map((e) => e.source);
    const outputNodeIds = currentEdges.filter((e) => e.source === nodeId).map((e) => e.target);
    return {
      inputs: currentNodes.filter((n) => inputNodeIds.includes(n.id)),
      outputs: currentNodes.filter((n) => outputNodeIds.includes(n.id)),
    };
  }

  function getNodesBounds(nodeIds: string[]) {
    const currentNodes = get(nodes);
    const targetNodes = currentNodes.filter((n) => nodeIds.includes(n.id));
    if (targetNodes.length === 0) return null;

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const node of targetNodes) {
      const width = (node.measured?.width || node.width || 200) as number;
      const height = (node.measured?.height || node.height || 100) as number;
      minX = Math.min(minX, node.position.x);
      minY = Math.min(minY, node.position.y);
      maxX = Math.max(maxX, node.position.x + width);
      maxY = Math.max(maxY, node.position.y + height);
    }
    return { x: minX, y: minY, width: maxX - minX, height: maxY - minY };
  }

  // --- Edge actions ---

  function addEdgeFn(edge: Edge) {
    syncGraphMutationFromBackend('add edge', (sessionId) =>
      backend.addEdge(
        {
          id: edge.id,
          source: edge.source,
          source_handle: edge.sourceHandle || 'output',
          target: edge.target,
          target_handle: edge.targetHandle || 'input',
        },
        sessionId,
      ),
    );
  }

  function removeEdgeFn(edgeId: string) {
    syncGraphMutationFromBackend('remove edge', (sessionId) =>
      backend.removeEdge(edgeId, sessionId)
    );
  }

  function syncEdgesFromBackend(backendGraph: WorkflowGraph) {
    applyWorkflowGraph(backendGraph, { markDirty: true });
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
    nodes.update((n) =>
      n.map((node) =>
        node.id === nodeId
          ? { ...node, data: { ...node.data, streamContent: (node.data.streamContent || '') + chunk } }
          : node
      )
    );
  }

  function setStreamContent(nodeId: string, content: string) {
    nodes.update((n) =>
      n.map((node) =>
        node.id === nodeId
          ? { ...node, data: { ...node.data, streamContent: content } }
          : node
      )
    );
  }

  function clearStreamContent() {
    nodes.update((n) =>
      n.map((node) =>
        node.data.streamContent
          ? { ...node, data: { ...node.data, streamContent: '' } }
          : node
      )
    );
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
    const textInputDef = definitions.find((d) => d.node_type === 'text-input');
    const llmDef = definitions.find((d) => d.node_type === 'llm-inference');
    const outputDef = definitions.find((d) => d.node_type === 'text-output');

    nodes.set([
      { id: 'user-input', type: 'text-input', position: { x: 50, y: 150 }, data: { label: 'User Input', text: '', definition: textInputDef } },
      { id: 'llm', type: 'llm-inference', position: { x: 350, y: 150 }, data: { label: 'LLM Inference', definition: llmDef } },
      { id: 'output', type: 'text-output', position: { x: 650, y: 150 }, data: { label: 'Output', text: '', definition: outputDef } },
    ]);
    edges.set([
      { id: 'input-to-llm', source: 'user-input', sourceHandle: 'text', target: 'llm', targetHandle: 'prompt' },
      { id: 'llm-to-output', source: 'llm', sourceHandle: 'response', target: 'output', targetHandle: 'text' },
    ]);
    connectionIntent.set(null);
    derivedGraph.set(
      buildDerivedGraph({
        nodes: [
          { id: 'user-input', node_type: 'text-input', position: { x: 50, y: 150 }, data: { label: 'User Input', text: '', definition: textInputDef } },
          { id: 'llm', node_type: 'llm-inference', position: { x: 350, y: 150 }, data: { label: 'LLM Inference', definition: llmDef } },
          { id: 'output', node_type: 'text-output', position: { x: 650, y: 150 }, data: { label: 'Output', text: '', definition: outputDef } },
        ],
        edges: [
          { id: 'input-to-llm', source: 'user-input', source_handle: 'text', target: 'llm', target_handle: 'prompt' },
          { id: 'llm-to-output', source: 'llm', source_handle: 'response', target: 'output', target_handle: 'text' },
        ],
      })
    );
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

    try {
      const graph = get(workflowGraph) as WorkflowGraph;
      const result = await backend.createGroup(name, nodeIds, graph);

      nodes.update((n) => n.filter((node) => !nodeIds.includes(node.id)));

      const internalizedSet = new Set(result.internalized_edge_ids);
      edges.update((e) => e.filter((edge) => !internalizedSet.has(edge.id)));

      const groupNode: Node = {
        id: result.group.id,
        type: 'node-group',
        position: result.group.position,
        data: { label: result.group.name, group: result.group, isGroup: true },
      };
      nodes.update((n) => [...n, groupNode]);

      edges.update((e) =>
        e.map((edge) => {
          const inputMapping = result.group.exposed_inputs.find(
            (m) => m.internal_node_id === edge.target && m.internal_port_id === edge.targetHandle
          );
          if (inputMapping) {
            return { ...edge, target: result.group.id, targetHandle: inputMapping.group_port_id };
          }
          const outputMapping = result.group.exposed_outputs.find(
            (m) => m.internal_node_id === edge.source && m.internal_port_id === edge.sourceHandle
          );
          if (outputMapping) {
            return { ...edge, source: result.group.id, sourceHandle: outputMapping.group_port_id };
          }
          return edge;
        })
      );

      nodeGroups.update((groups) => {
        const newGroups = new Map(groups);
        newGroups.set(result.group.id, result.group);
        return newGroups;
      });

      markGraphModified();
      return result.group;
    } catch (error) {
      console.error('[workflowStores] Failed to create group:', error);
      return null;
    }
  }

  async function ungroupNodes(groupId: string): Promise<boolean> {
    const groups = get(nodeGroups);
    const group = groups.get(groupId);
    if (!group) {
      console.warn('[workflowStores] Group not found:', groupId);
      return false;
    }

    try {
      const definitions = get(nodeDefinitions);
      nodes.update((n) => n.filter((node) => node.id !== groupId));

      const restoredNodes: Node[] = group.nodes.map((gn) => {
        const definition = definitions.find((d) => d.node_type === gn.node_type);
        return { id: gn.id, type: gn.node_type, position: gn.position, data: { ...gn.data, definition } };
      });
      nodes.update((n) => [...n, ...restoredNodes]);

      const restoredEdges: Edge[] = group.edges.map((ge) => ({
        id: ge.id, source: ge.source, sourceHandle: ge.source_handle,
        target: ge.target, targetHandle: ge.target_handle,
      }));
      edges.update((e) => [...e, ...restoredEdges]);

      edges.update((e) =>
        e.map((edge) => {
          if (edge.target === groupId) {
            const mapping = group.exposed_inputs.find((m) => m.group_port_id === edge.targetHandle);
            if (mapping) return { ...edge, target: mapping.internal_node_id, targetHandle: mapping.internal_port_id };
          }
          if (edge.source === groupId) {
            const mapping = group.exposed_outputs.find((m) => m.group_port_id === edge.sourceHandle);
            if (mapping) return { ...edge, source: mapping.internal_node_id, sourceHandle: mapping.internal_port_id };
          }
          return edge;
        })
      );

      nodeGroups.update((groups) => {
        const newGroups = new Map(groups);
        newGroups.delete(groupId);
        return newGroups;
      });

      markGraphModified();
      return true;
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
    const groups = get(nodeGroups);
    const group = groups.get(groupId);
    if (!group) {
      console.warn('[workflowStores] Group not found:', groupId);
      return false;
    }

    try {
      const updatedGroup = await backend.updateGroupPorts(group, exposedInputs, exposedOutputs);

      nodeGroups.update((groups) => {
        const newGroups = new Map(groups);
        newGroups.set(groupId, updatedGroup);
        return newGroups;
      });

      nodes.update((n) =>
        n.map((node) =>
          node.id === groupId
            ? { ...node, data: { ...node.data, group: updatedGroup } }
            : node
        )
      );

      markGraphModified();
      return true;
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

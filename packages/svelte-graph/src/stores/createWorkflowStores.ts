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
  NodeDefinition,
  PortDefinition,
  PortDataType,
} from '../types/workflow.js';
import type { NodeGroup, PortMapping, CreateGroupResult } from '../types/groups.js';
import type { ViewportState } from '../types/view.js';
import type { WorkflowBackend } from '../types/backend.js';

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

  // Derived stores
  workflowGraph: ReturnType<typeof derived>;
  nodeDefinitionsByCategory: ReturnType<typeof derived>;

  // Actions — nodes
  addNode: (definition: NodeDefinition, position: { x: number; y: number }) => void;
  removeNode: (nodeId: string) => void;
  updateNodePosition: (nodeId: string, position: { x: number; y: number }) => void;
  updateNodeData: (nodeId: string, data: Record<string, unknown>) => void;
  getNodeById: (nodeId: string) => Node | undefined;
  isNodeGroup: (nodeId: string) => boolean;
  getConnectedNodes: (nodeId: string) => { inputs: Node[]; outputs: Node[] };
  getNodesBounds: (nodeIds: string[]) => { x: number; y: number; width: number; height: number } | null;

  // Actions — edges
  addEdge: (edge: Edge) => void;
  removeEdge: (edgeId: string) => void;
  syncEdgesFromBackend: (backendGraph: WorkflowGraph) => void;

  // Actions — execution
  setNodeExecutionState: (nodeId: string, state: NodeExecutionState, errorMessage?: string) => void;
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

  // Actions — inference settings
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

/** Schema for a model-specific inference parameter (from pumas-library). */
export interface InferenceParamSchema {
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
  const nodeExecutionStates = writable<Map<string, NodeExecutionInfo>>(new Map());
  const currentViewport = writable<ViewportState>({ x: 0, y: 0, zoom: 1 });
  const nodeGroups = writable<Map<string, NodeGroup>>(new Map());
  const selectedNodeIds = writable<string[]>([]);

  // --- Derived stores ---
  const workflowGraph = derived(
    [nodes, edges],
    ([$nodes, $edges]): WorkflowGraph => ({
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

  // --- Node actions ---

  function addNode(definition: NodeDefinition, position: { x: number; y: number }) {
    const id = `${definition.node_type}-${Date.now()}`;
    const newNode: Node = {
      id,
      type: definition.node_type,
      position,
      data: {
        label: definition.label,
        definition,
        ...Object.fromEntries(definition.inputs.map((input) => [input.id, null])),
      },
    };
    nodes.update((n) => [...n, newNode]);
    isDirty.set(true);
  }

  function removeNode(nodeId: string) {
    nodes.update((n) => n.filter((node) => node.id !== nodeId));
    edges.update((e) => e.filter((edge) => edge.source !== nodeId && edge.target !== nodeId));
    isDirty.set(true);
  }

  function updateNodePosition(nodeId: string, position: { x: number; y: number }) {
    nodes.update((n) =>
      n.map((node) => (node.id === nodeId ? { ...node, position } : node))
    );
    isDirty.set(true);
  }

  function updateNodeData(nodeId: string, data: Record<string, unknown>) {
    nodes.update((n) =>
      n.map((node) =>
        node.id === nodeId ? { ...node, data: { ...node.data, ...data } } : node
      )
    );
    isDirty.set(true);
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
    let added = false;
    edges.update((e) => {
      const exists = e.some(
        (existing) =>
          existing.source === edge.source &&
          existing.sourceHandle === edge.sourceHandle &&
          existing.target === edge.target &&
          existing.targetHandle === edge.targetHandle
      );
      if (exists) return e;
      added = true;
      return [...e, edge];
    });
    if (!added) return;
    isDirty.set(true);

    // Auto-sync when connecting an inference_settings edge
    if (edge.sourceHandle === 'inference_settings') {
      const sourceNode = getNodeById(edge.source);
      const settings = sourceNode?.data?.inference_settings as InferenceParamSchema[] | undefined;
      if (settings && settings.length > 0) {
        syncExpandPorts(edge.source, settings);
        syncInferencePorts(edge.source, settings);
      }
    }
  }

  function removeEdgeFn(edgeId: string) {
    edges.update((e) => e.filter((edge) => edge.id !== edgeId));
    isDirty.set(true);
  }

  function syncEdgesFromBackend(backendGraph: WorkflowGraph) {
    const newEdges: Edge[] = backendGraph.edges.map((e) => ({
      id: e.id,
      source: e.source,
      sourceHandle: e.source_handle,
      target: e.target,
      targetHandle: e.target_handle,
    }));
    edges.set(newEdges);
    isDirty.set(true);
  }

  // --- Execution actions ---

  function setNodeExecutionState(nodeId: string, state: NodeExecutionState, errorMessage?: string) {
    nodeExecutionStates.update((map) => {
      const newMap = new Map(map);
      newMap.set(nodeId, { state, errorMessage });
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
    const definitions = get(nodeDefinitions);
    const migratedNodeIds = new Set<string>();

    nodes.set(
      graph.nodes.map((n) => {
        let nodeType = n.node_type;
        let nodeData = { ...n.data };

        // Migration: system-prompt -> text-input
        if (nodeType === 'system-prompt') {
          nodeType = 'text-input';
          migratedNodeIds.add(n.id);
          if ('prompt' in nodeData && !('text' in nodeData)) {
            nodeData.text = nodeData.prompt;
            delete nodeData.prompt;
          }
        }

        const definition = definitions.find((d) => d.node_type === nodeType);
        return {
          id: n.id,
          type: nodeType,
          position: n.position,
          data: { ...nodeData, definition },
        };
      })
    );

    edges.set(
      graph.edges.map((e) => {
        let sourceHandle = e.source_handle;
        let targetHandle = e.target_handle;
        if (migratedNodeIds.has(e.source) && sourceHandle === 'prompt') sourceHandle = 'text';
        if (migratedNodeIds.has(e.target) && targetHandle === 'prompt') targetHandle = 'text';
        return {
          id: e.id,
          source: e.source,
          sourceHandle,
          target: e.target,
          targetHandle,
        };
      })
    );

    workflowMetadata.set(metadata || null);
    isDirty.set(false);
  }

  function clearWorkflow() {
    nodes.set([]);
    edges.set([]);
    workflowMetadata.set(null);
    isDirty.set(false);
    resetExecutionStates();
  }

  function loadDefaultWorkflow(definitions: NodeDefinition[]) {
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
    isDirty.set(false);
  }

  function updateViewport(viewport: ViewportState) {
    currentViewport.set(viewport);
  }

  // --- Inference settings actions ---

  /**
   * Map a pumas-library param_type string to a Pantograph PortDataType.
   */
  function paramTypeToPortDataType(paramType: string): PortDataType {
    switch (paramType) {
      case 'Number':
      case 'Integer':
        return 'number';
      case 'String':
        return 'string';
      case 'Boolean':
        return 'boolean';
      default:
        return 'any';
    }
  }

  /**
   * Find downstream target node IDs connected via a specific source handle.
   */
  function findConnectedTargets(sourceId: string, sourceHandle: string): string[] {
    return get(edges)
      .filter((e) => e.source === sourceId && e.sourceHandle === sourceHandle)
      .map((e) => e.target);
  }

  /**
   * Sync model-derived inference ports to downstream inference nodes.
   *
   * When a puma-lib or model-provider node's model selection changes,
   * this function finds connected inference nodes and updates their
   * definition inputs to include model-specific parameter ports.
   *
   * Ports not in the base NodeDefinition are considered model-derived
   * and are stripped before appending the new set.
   */
  function syncInferencePorts(sourceNodeId: string, inferenceSettings: InferenceParamSchema[]) {
    // Find inference nodes connected downstream via model_path or inference_settings
    const downstreamIds = new Set([
      ...findConnectedTargets(sourceNodeId, 'model_path'),
      ...findConnectedTargets(sourceNodeId, 'inference_settings'),
    ]);

    const defs = get(nodeDefinitions);

    for (const nodeId of downstreamIds) {
      const node = getNodeById(nodeId);
      if (!node?.data?.definition) continue;

      const nodeDef = node.data.definition as NodeDefinition;

      // Get the base definition for this node type (static ports only)
      const baseDef = defs.find((d) => d.node_type === nodeDef.node_type);
      if (!baseDef) continue;

      const basePortIds = new Set(baseDef.inputs.map((p) => p.id));

      // Strip any previously-appended model-derived ports
      const staticPorts = nodeDef.inputs.filter((p: PortDefinition) => basePortIds.has(p.id));

      // Convert inference_settings schema to PortDefinitions
      const modelPorts: PortDefinition[] = inferenceSettings.map((s) => ({
        id: s.key,
        label: s.label,
        data_type: paramTypeToPortDataType(s.param_type),
        required: false,
        multiple: false,
      }));

      // Update definition with static + model-derived ports
      updateNodeData(nodeId, {
        definition: {
          ...nodeDef,
          inputs: [...staticPorts, ...modelPorts],
        },
      });
    }
  }

  /**
   * Sync dynamic output ports on downstream expand-settings nodes from inference schema.
   *
   * Called with the source node ID (puma-lib / model-provider). Finds connected
   * expand-settings nodes via the inference_settings handle, strips old dynamic
   * output ports, and appends new ones from the schema.
   */
  function syncExpandPorts(sourceNodeId: string, inferenceSettings: InferenceParamSchema[]): void {
    const expandIds = findConnectedTargets(sourceNodeId, 'inference_settings');
    const defs = get(nodeDefinitions);
    const baseDef = defs.find((d) => d.node_type === 'expand-settings');
    if (!baseDef) return;

    const basePortIds = new Set(baseDef.outputs.map((p) => p.id));

    const modelPorts: PortDefinition[] = inferenceSettings.map((s) => ({
      id: s.key,
      label: s.label,
      data_type: paramTypeToPortDataType(s.param_type),
      required: false,
      multiple: false,
    }));

    for (const expandId of expandIds) {
      const node = getNodeById(expandId);
      if (!node?.data?.definition) continue;

      const nodeDef = node.data.definition as NodeDefinition;
      // Only operate on expand-settings nodes
      if (nodeDef.node_type !== 'expand-settings') continue;

      // Strip model-derived ports, keep static ports from base definition
      const staticPorts = nodeDef.outputs.filter((p: PortDefinition) => basePortIds.has(p.id));

      updateNodeData(expandId, {
        definition: { ...nodeDef, outputs: [...staticPorts, ...modelPorts] },
        inference_settings: inferenceSettings,
      });

      // Also sync inference ports on nodes downstream of this expand node,
      // and auto-connect the expand outputs to their inputs
      syncInferencePorts(expandId, inferenceSettings);
      autoConnectExpandToInference(expandId, inferenceSettings);
    }
  }

  /**
   * Auto-create edges from expand-settings output ports to downstream inference input ports.
   *
   * Finds inference nodes connected via the expand node's inference_settings
   * output and creates edges for each matching parameter port.
   */
  function autoConnectExpandToInference(
    expandNodeId: string,
    inferenceSettings: InferenceParamSchema[]
  ): void {
    const downstreamIds = findConnectedTargets(expandNodeId, 'inference_settings');

    for (const targetId of downstreamIds) {
      for (const param of inferenceSettings) {
        addEdgeFn({
          id: `${expandNodeId}-${param.key}-${targetId}-${param.key}`,
          source: expandNodeId,
          sourceHandle: param.key,
          target: targetId,
          targetHandle: param.key,
        });
      }
    }
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

      isDirty.set(true);
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

      isDirty.set(true);
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

      isDirty.set(true);
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
    workflowGraph, nodeDefinitionsByCategory,
    // Node actions
    addNode, removeNode, updateNodePosition, updateNodeData,
    getNodeById, isNodeGroup: isNodeGroupFn, getConnectedNodes, getNodesBounds,
    // Edge actions
    addEdge: addEdgeFn, removeEdge: removeEdgeFn, syncEdgesFromBackend,
    // Execution actions
    setNodeExecutionState, getNodeExecutionInfo, resetExecutionStates,
    // Streaming actions
    appendStreamContent, setStreamContent, clearStreamContent,
    // Workflow actions
    loadWorkflow: loadWorkflowFn, clearWorkflow, loadDefaultWorkflow, updateViewport,
    // Inference settings actions
    syncInferencePorts, syncExpandPorts, autoConnectExpandToInference,
    // Group actions
    createGroup, ungroupNodes, updateGroupPorts: updateGroupPortsFn,
    getGroupById, collapseGroup,
  };
}

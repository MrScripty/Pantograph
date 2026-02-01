import { writable, derived, get } from 'svelte/store';
import type { Node, Edge } from '@xyflow/svelte';
import type {
  WorkflowGraph,
  WorkflowMetadata,
  NodeExecutionState,
  NodeExecutionInfo,
  NodeDefinition,
} from '../services/workflow/types';
import type { NodeGroup, PortMapping, CreateGroupResult } from '../services/workflow/groupTypes';
import { invoke } from '@tauri-apps/api/core';

// Re-export view-related stores and functions for backward compatibility
// (NodeGroupNode.svelte imports these from workflowStore)
export {
  groupStack,
  tabIntoGroup,
  tabOutOfGroup,
} from './viewStore';

// Create expandedGroupId as a derived store from groupStack
import { groupStack as groupStackStore } from './viewStore';

/** Currently expanded group ID (last item in groupStack, or null if not in a group) */
export const expandedGroupId = derived(groupStackStore, ($stack) =>
  $stack.length > 0 ? $stack[$stack.length - 1] : null
);

// --- Types ---

export interface ViewportState {
  x: number;
  y: number;
  zoom: number;
}

// --- State ---

export const nodes = writable<Node[]>([]);
export const edges = writable<Edge[]>([]);
export const nodeDefinitions = writable<NodeDefinition[]>([]);
export const workflowMetadata = writable<WorkflowMetadata | null>(null);
export const isDirty = writable<boolean>(false);
export const isExecuting = writable<boolean>(false);
export const isEditing = writable<boolean>(true);
export const nodeExecutionStates = writable<Map<string, NodeExecutionInfo>>(new Map());

/** Current viewport state (for saving/restoring during zoom transitions) */
export const currentViewport = writable<ViewportState>({ x: 0, y: 0, zoom: 1 });

// --- Derived ---

export const workflowGraph = derived(
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

export const nodeDefinitionsByCategory = derived(nodeDefinitions, ($defs) => {
  const grouped = new Map<string, NodeDefinition[]>();
  for (const def of $defs) {
    const list = grouped.get(def.category) || [];
    list.push(def);
    grouped.set(def.category, list);
  }
  return grouped;
});

// --- Actions ---

export function addNode(definition: NodeDefinition, position: { x: number; y: number }) {
  const id = `${definition.node_type}-${Date.now()}`;

  const newNode: Node = {
    id,
    type: definition.node_type,
    position,
    data: {
      label: definition.label,
      definition,
      // Initialize empty values for inputs
      ...Object.fromEntries(definition.inputs.map((input) => [input.id, null])),
    },
  };

  nodes.update((n) => [...n, newNode]);
  isDirty.set(true);
}

export function removeNode(nodeId: string) {
  nodes.update((n) => n.filter((node) => node.id !== nodeId));
  edges.update((e) => e.filter((edge) => edge.source !== nodeId && edge.target !== nodeId));
  isDirty.set(true);
}

export function addEdge(edge: Edge) {
  // Prevent duplicate edges
  edges.update((e) => {
    const exists = e.some(
      (existing) =>
        existing.source === edge.source &&
        existing.sourceHandle === edge.sourceHandle &&
        existing.target === edge.target &&
        existing.targetHandle === edge.targetHandle
    );
    if (exists) return e;
    return [...e, edge];
  });
  isDirty.set(true);
}

export function removeEdge(edgeId: string) {
  edges.update((e) => e.filter((edge) => edge.id !== edgeId));
  isDirty.set(true);
}

export function updateNodePosition(nodeId: string, position: { x: number; y: number }) {
  nodes.update((n) =>
    n.map((node) => (node.id === nodeId ? { ...node, position } : node))
  );
  isDirty.set(true);
}

export function updateNodeData(nodeId: string, data: Record<string, unknown>) {
  nodes.update((n) =>
    n.map((node) =>
      node.id === nodeId ? { ...node, data: { ...node.data, ...data } } : node
    )
  );
  isDirty.set(true);
}

export function setNodeExecutionState(nodeId: string, state: NodeExecutionState, errorMessage?: string) {
  nodeExecutionStates.update((map) => {
    const newMap = new Map(map);
    newMap.set(nodeId, { state, errorMessage });
    return newMap;
  });
}

/** Get the execution info for a node (state + error message) */
export function getNodeExecutionInfo(nodeId: string): NodeExecutionInfo | undefined {
  return get(nodeExecutionStates).get(nodeId);
}

export function resetExecutionStates() {
  nodeExecutionStates.set(new Map());
}

export function loadWorkflow(graph: WorkflowGraph, metadata?: WorkflowMetadata) {
  // Get current node definitions to attach to loaded nodes
  const definitions = get(nodeDefinitions);

  // Track which nodes were migrated from system-prompt
  const migratedNodeIds = new Set<string>();

  // Convert GraphNodes to SvelteFlow Nodes
  // Migrate system-prompt nodes to text-input
  nodes.set(
    graph.nodes.map((n) => {
      let nodeType = n.node_type;
      let nodeData = { ...n.data };

      // Migration: system-prompt -> text-input
      if (nodeType === 'system-prompt') {
        nodeType = 'text-input';
        migratedNodeIds.add(n.id);
        // Migrate 'prompt' field to 'text' if present
        if ('prompt' in nodeData && !('text' in nodeData)) {
          nodeData.text = nodeData.prompt;
          delete nodeData.prompt;
        }
      }

      // Look up the definition for this node type
      const definition = definitions.find((d) => d.node_type === nodeType);

      return {
        id: n.id,
        type: nodeType,
        position: n.position,
        data: {
          ...nodeData,
          definition, // Attach definition so BaseNode can render inputs/outputs
        },
      };
    })
  );

  // Convert GraphEdges to SvelteFlow Edges
  // Migrate 'prompt' handles to 'text' for migrated nodes
  edges.set(
    graph.edges.map((e) => {
      let sourceHandle = e.source_handle;
      let targetHandle = e.target_handle;

      // If the source was a migrated system-prompt node, change 'prompt' to 'text'
      if (migratedNodeIds.has(e.source) && sourceHandle === 'prompt') {
        sourceHandle = 'text';
      }
      // If the target was a migrated system-prompt node, change 'prompt' to 'text'
      if (migratedNodeIds.has(e.target) && targetHandle === 'prompt') {
        targetHandle = 'text';
      }

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

export function clearWorkflow() {
  nodes.set([]);
  edges.set([]);
  workflowMetadata.set(null);
  isDirty.set(false);
  resetExecutionStates();
}

/**
 * Sync the edges from a backend graph response.
 * This is used when edge operations return the updated graph from the backend.
 * We only update edges, not nodes, since edge operations don't change nodes.
 */
export function syncEdgesFromBackend(backendGraph: { nodes: unknown[]; edges: Array<{ id: string; source: string; source_handle: string; target: string; target_handle: string }> }) {
  // Convert backend edge format to SvelteFlow format
  // Note: type, selectable, focusable come from defaultEdgeOptions in WorkflowGraph.svelte
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

// --- Default Workflow ---

export function loadDefaultWorkflow(definitions: NodeDefinition[]) {
  const textInputDef = definitions.find((d) => d.node_type === 'text-input');
  const llmDef = definitions.find((d) => d.node_type === 'llm-inference');
  const outputDef = definitions.find((d) => d.node_type === 'text-output');

  const defaultNodes: Node[] = [
    {
      id: 'user-input',
      type: 'text-input',
      position: { x: 50, y: 150 },
      data: { label: 'User Input', text: '', definition: textInputDef },
    },
    {
      id: 'llm',
      type: 'llm-inference',
      position: { x: 350, y: 150 },
      data: { label: 'LLM Inference', definition: llmDef },
    },
    {
      id: 'output',
      type: 'text-output',
      position: { x: 650, y: 150 },
      data: { label: 'Output', text: '', definition: outputDef },
    },
  ];

  const defaultEdges: Edge[] = [
    {
      id: 'input-to-llm',
      source: 'user-input',
      sourceHandle: 'text',
      target: 'llm',
      targetHandle: 'prompt',
    },
    {
      id: 'llm-to-output',
      source: 'llm',
      sourceHandle: 'response',
      target: 'output',
      targetHandle: 'text',
    },
  ];

  nodes.set(defaultNodes);
  edges.set(defaultEdges);
  isDirty.set(false);
}

// --- Viewport Management ---

/**
 * Update the current viewport state
 * Called by WorkflowGraph when viewport changes
 */
export function updateViewport(viewport: ViewportState) {
  currentViewport.set(viewport);
}

/**
 * Get a node by ID
 */
export function getNodeById(nodeId: string): Node | undefined {
  return get(nodes).find((n) => n.id === nodeId);
}

/**
 * Check if a node is a group node (for zoom navigation)
 * This will be enhanced by Workstream B when NodeGroup is implemented
 */
export function isNodeGroup(nodeId: string): boolean {
  const node = getNodeById(nodeId);
  if (!node) return false;

  // Check for group markers (will be set by Workstream B)
  return node.data?.isGroup === true || node.type === 'node-group';
}

/**
 * Get connected nodes (inputs and outputs) for a given node
 * Useful for understanding node context during zoom transitions
 */
export function getConnectedNodes(nodeId: string): { inputs: Node[]; outputs: Node[] } {
  const currentEdges = get(edges);
  const currentNodes = get(nodes);

  const inputNodeIds = currentEdges
    .filter((e) => e.target === nodeId)
    .map((e) => e.source);

  const outputNodeIds = currentEdges
    .filter((e) => e.source === nodeId)
    .map((e) => e.target);

  return {
    inputs: currentNodes.filter((n) => inputNodeIds.includes(n.id)),
    outputs: currentNodes.filter((n) => outputNodeIds.includes(n.id)),
  };
}

/**
 * Get the bounds of a set of nodes (for calculating zoom targets)
 */
export function getNodesBounds(nodeIds: string[]): {
  x: number;
  y: number;
  width: number;
  height: number;
} | null {
  const currentNodes = get(nodes);
  const targetNodes = currentNodes.filter((n) => nodeIds.includes(n.id));

  if (targetNodes.length === 0) return null;

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const node of targetNodes) {
    const width = (node.measured?.width || node.width || 200) as number;
    const height = (node.measured?.height || node.height || 100) as number;

    minX = Math.min(minX, node.position.x);
    minY = Math.min(minY, node.position.y);
    maxX = Math.max(maxX, node.position.x + width);
    maxY = Math.max(maxY, node.position.y + height);
  }

  return {
    x: minX,
    y: minY,
    width: maxX - minX,
    height: maxY - minY,
  };
}

// --- Node Group State ---

/** Map of group ID to NodeGroup data */
export const nodeGroups = writable<Map<string, NodeGroup>>(new Map());

/** Currently selected nodes (for group creation) */
export const selectedNodeIds = writable<string[]>([]);

// --- Node Group Actions ---

/**
 * Create a node group from selected nodes
 * Calls the backend to create the group and updates local state
 */
export async function createGroup(name: string, nodeIds: string[]): Promise<NodeGroup | null> {
  if (nodeIds.length < 2) {
    console.warn('[workflowStore] Cannot create group with less than 2 nodes');
    return null;
  }

  try {
    // Get current graph state
    const graph = get(workflowGraph);

    // Call backend to create group
    const result = await invoke<CreateGroupResult>('create_node_group', {
      name,
      selectedNodeIds: nodeIds,
      graph,
    });

    // Remove grouped nodes from main graph
    nodes.update((n) => n.filter((node) => !nodeIds.includes(node.id)));

    // Remove internalized edges from main graph
    const internalizedSet = new Set(result.internalized_edge_ids);
    edges.update((e) => e.filter((edge) => !internalizedSet.has(edge.id)));

    // Add the group node to the graph
    const groupNode: Node = {
      id: result.group.id,
      type: 'node-group',
      position: result.group.position,
      data: {
        label: result.group.name,
        group: result.group,
        isGroup: true,
      },
    };
    nodes.update((n) => [...n, groupNode]);

    // Update boundary edges to connect to group node instead of internal nodes
    edges.update((e) =>
      e.map((edge) => {
        // Update edges that target nodes inside the group
        const inputMapping = result.group.exposed_inputs.find(
          (m) => m.internal_node_id === edge.target && m.internal_port_id === edge.targetHandle
        );
        if (inputMapping) {
          return {
            ...edge,
            target: result.group.id,
            targetHandle: inputMapping.group_port_id,
          };
        }

        // Update edges that source from nodes inside the group
        const outputMapping = result.group.exposed_outputs.find(
          (m) => m.internal_node_id === edge.source && m.internal_port_id === edge.sourceHandle
        );
        if (outputMapping) {
          return {
            ...edge,
            source: result.group.id,
            sourceHandle: outputMapping.group_port_id,
          };
        }

        return edge;
      })
    );

    // Store the group
    nodeGroups.update((groups) => {
      const newGroups = new Map(groups);
      newGroups.set(result.group.id, result.group);
      return newGroups;
    });

    isDirty.set(true);
    return result.group;
  } catch (error) {
    console.error('[workflowStore] Failed to create group:', error);
    return null;
  }
}

/**
 * Ungroup nodes - dissolve a group and restore its nodes to the main graph
 */
export async function ungroupNodes(groupId: string): Promise<boolean> {
  const groups = get(nodeGroups);
  const group = groups.get(groupId);

  if (!group) {
    console.warn('[workflowStore] Group not found:', groupId);
    return false;
  }

  try {
    // Get node definitions
    const definitions = get(nodeDefinitions);

    // Remove the group node
    nodes.update((n) => n.filter((node) => node.id !== groupId));

    // Add the internal nodes back to the graph
    const restoredNodes: Node[] = group.nodes.map((gn) => {
      const definition = definitions.find((d) => d.node_type === gn.node_type);
      return {
        id: gn.id,
        type: gn.node_type,
        position: gn.position,
        data: {
          ...gn.data,
          definition,
        },
      };
    });
    nodes.update((n) => [...n, ...restoredNodes]);

    // Add internal edges back
    const restoredEdges: Edge[] = group.edges.map((ge) => ({
      id: ge.id,
      source: ge.source,
      sourceHandle: ge.source_handle,
      target: ge.target,
      targetHandle: ge.target_handle,
    }));
    edges.update((e) => [...e, ...restoredEdges]);

    // Update boundary edges to connect to internal nodes instead of group
    edges.update((e) =>
      e.map((edge) => {
        if (edge.target === groupId) {
          // Find the input mapping for this port
          const mapping = group.exposed_inputs.find(
            (m) => m.group_port_id === edge.targetHandle
          );
          if (mapping) {
            return {
              ...edge,
              target: mapping.internal_node_id,
              targetHandle: mapping.internal_port_id,
            };
          }
        }

        if (edge.source === groupId) {
          // Find the output mapping for this port
          const mapping = group.exposed_outputs.find(
            (m) => m.group_port_id === edge.sourceHandle
          );
          if (mapping) {
            return {
              ...edge,
              source: mapping.internal_node_id,
              sourceHandle: mapping.internal_port_id,
            };
          }
        }

        return edge;
      })
    );

    // Remove from groups store
    nodeGroups.update((groups) => {
      const newGroups = new Map(groups);
      newGroups.delete(groupId);
      return newGroups;
    });

    isDirty.set(true);
    return true;
  } catch (error) {
    console.error('[workflowStore] Failed to ungroup:', error);
    return false;
  }
}

/**
 * Update the port mappings for a group
 */
export async function updateGroupPorts(
  groupId: string,
  exposedInputs: PortMapping[],
  exposedOutputs: PortMapping[]
): Promise<boolean> {
  const groups = get(nodeGroups);
  const group = groups.get(groupId);

  if (!group) {
    console.warn('[workflowStore] Group not found:', groupId);
    return false;
  }

  try {
    // Call backend to validate and update
    const updatedGroup = await invoke<NodeGroup>('update_group_ports', {
      group,
      exposedInputs,
      exposedOutputs,
    });

    // Update local state
    nodeGroups.update((groups) => {
      const newGroups = new Map(groups);
      newGroups.set(groupId, updatedGroup);
      return newGroups;
    });

    // Update the node data
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
    console.error('[workflowStore] Failed to update group ports:', error);
    return false;
  }
}

/**
 * Get a group by ID
 */
export function getGroupById(groupId: string): NodeGroup | undefined {
  return get(nodeGroups).get(groupId);
}

/**
 * Collapse a group (save changes from editing and close)
 * Called when exiting group editing mode
 */
export function collapseGroup(): void {
  // This is handled by viewStore.tabOutOfGroup
  // This function is here for compatibility with NodeGroupEditor
  import('./viewStore').then(({ tabOutOfGroup }) => {
    tabOutOfGroup();
  });
}

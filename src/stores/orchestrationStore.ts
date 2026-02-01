/**
 * Orchestration Store
 *
 * Manages state for orchestration graphs - the high-level control flow
 * between data graphs in a two-level workflow system.
 */

import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// Types
// ============================================================================

export type OrchestrationNodeType =
  | 'start'
  | 'end'
  | 'condition'
  | 'loop'
  | 'data_graph'
  | 'merge';

export interface OrchestrationNode {
  id: string;
  nodeType: OrchestrationNodeType;
  position: [number, number];
  config: Record<string, unknown>;
}

export interface OrchestrationEdge {
  id: string;
  source: string;
  sourceHandle: string;
  target: string;
  targetHandle: string;
}

export interface OrchestrationGraph {
  id: string;
  name: string;
  description: string;
  nodes: OrchestrationNode[];
  edges: OrchestrationEdge[];
  dataGraphs: Record<string, string>;
}

export interface OrchestrationGraphMetadata {
  id: string;
  name: string;
  description: string;
  nodeCount: number;
}

export interface OrchestrationNodeTypeInfo {
  nodeType: string;
  label: string;
  description: string;
  inputHandles: string[];
  outputHandles: string[];
  category: string;
}

export interface OrchestrationResult {
  success: boolean;
  outputs: Record<string, unknown>;
  error?: string;
  nodesExecuted: number;
  executionTimeMs: number;
}

export interface ConditionConfig {
  conditionKey: string;
  expectedValue?: unknown;
}

export interface LoopConfig {
  maxIterations: number;
  exitConditionKey?: string;
  iterationKey: string;
}

export interface DataGraphConfig {
  dataGraphId: string;
  inputMappings: Record<string, string>;
  outputMappings: Record<string, string>;
}

// ============================================================================
// Stores
// ============================================================================

/** Currently loaded orchestration graph */
export const currentOrchestration = writable<OrchestrationGraph | null>(null);

/** List of available orchestrations (metadata only) */
export const orchestrationList = writable<OrchestrationGraphMetadata[]>([]);

/** Currently selected node ID */
export const selectedOrchestrationNodeId = writable<string | null>(null);

/** Is an orchestration currently executing? */
export const isOrchestrationRunning = writable<boolean>(false);

/** Last execution result */
export const lastOrchestrationResult = writable<OrchestrationResult | null>(null);

/** Available node type definitions */
export const orchestrationNodeTypes = writable<OrchestrationNodeTypeInfo[]>([]);

/** Is the orchestration dirty (has unsaved changes)? */
export const isOrchestrationDirty = writable<boolean>(false);

// ============================================================================
// Derived Stores
// ============================================================================

/** Get the nodes from the current orchestration */
export const orchestrationNodes = derived(currentOrchestration, ($orch) =>
  $orch?.nodes ?? []
);

/** Get the edges from the current orchestration */
export const orchestrationEdges = derived(currentOrchestration, ($orch) =>
  $orch?.edges ?? []
);

/** Get the selected node */
export const selectedOrchestrationNode = derived(
  [currentOrchestration, selectedOrchestrationNodeId],
  ([$orch, $selectedId]) => {
    if (!$orch || !$selectedId) return null;
    return $orch.nodes.find((n) => n.id === $selectedId) ?? null;
  }
);

/** Convert orchestration nodes to SvelteFlow format */
export const orchestrationFlowNodes = derived(orchestrationNodes, ($nodes) =>
  $nodes.map((node) => ({
    id: node.id,
    type: node.nodeType,
    position: { x: node.position[0], y: node.position[1] },
    data: {
      label: getNodeTypeLabel(node.nodeType),
      config: node.config,
    },
  }))
);

/** Convert orchestration edges to SvelteFlow format */
export const orchestrationFlowEdges = derived(orchestrationEdges, ($edges) =>
  $edges.map((edge) => ({
    id: edge.id,
    source: edge.source,
    sourceHandle: edge.sourceHandle,
    target: edge.target,
    targetHandle: edge.targetHandle,
  }))
);

// ============================================================================
// Helper Functions
// ============================================================================

function getNodeTypeLabel(nodeType: OrchestrationNodeType): string {
  const labels: Record<OrchestrationNodeType, string> = {
    start: 'Start',
    end: 'End',
    condition: 'Condition',
    loop: 'Loop',
    data_graph: 'Data Graph',
    merge: 'Merge',
  };
  return labels[nodeType] ?? nodeType;
}

// ============================================================================
// Actions
// ============================================================================

/**
 * Load the list of available orchestrations
 */
export async function loadOrchestrationList(): Promise<void> {
  try {
    const list = await invoke<OrchestrationGraphMetadata[]>('list_orchestrations');
    orchestrationList.set(list);
  } catch (error) {
    console.error('Failed to load orchestration list:', error);
    throw error;
  }
}

/**
 * Load the available node type definitions
 */
export async function loadOrchestrationNodeTypes(): Promise<void> {
  try {
    const types = await invoke<OrchestrationNodeTypeInfo[]>(
      'get_orchestration_node_types'
    );
    orchestrationNodeTypes.set(types);
  } catch (error) {
    console.error('Failed to load orchestration node types:', error);
    throw error;
  }
}

/**
 * Create a new orchestration graph
 */
export async function createOrchestration(
  name: string,
  description?: string
): Promise<OrchestrationGraph> {
  try {
    const graph = await invoke<OrchestrationGraph>('create_orchestration', {
      name,
      description,
    });
    currentOrchestration.set(graph);
    isOrchestrationDirty.set(false);
    await loadOrchestrationList();
    return graph;
  } catch (error) {
    console.error('Failed to create orchestration:', error);
    throw error;
  }
}

/**
 * Load an orchestration graph by ID
 */
export async function loadOrchestration(id: string): Promise<void> {
  try {
    console.log(`[orchestrationStore] Loading orchestration: ${id}`);
    const graph = await invoke<OrchestrationGraph>('get_orchestration', { id });
    console.log(`[orchestrationStore] Loaded orchestration:`, graph);
    currentOrchestration.set(graph);
    console.log(`[orchestrationStore] Set currentOrchestration store`);
    selectedOrchestrationNodeId.set(null);
    isOrchestrationDirty.set(false);
  } catch (error) {
    console.error('[orchestrationStore] Failed to load orchestration:', error);
    throw error;
  }
}

/**
 * Save the current orchestration graph
 */
export async function saveOrchestration(): Promise<void> {
  const orch = get(currentOrchestration);
  if (!orch) {
    throw new Error('No orchestration to save');
  }

  try {
    await invoke('save_orchestration', { graph: orch });
    isOrchestrationDirty.set(false);
    await loadOrchestrationList();
  } catch (error) {
    console.error('Failed to save orchestration:', error);
    throw error;
  }
}

/**
 * Delete an orchestration graph
 */
export async function deleteOrchestration(id: string): Promise<void> {
  try {
    await invoke('delete_orchestration', { id });
    const current = get(currentOrchestration);
    if (current?.id === id) {
      currentOrchestration.set(null);
    }
    await loadOrchestrationList();
  } catch (error) {
    console.error('Failed to delete orchestration:', error);
    throw error;
  }
}

/**
 * Add a node to the current orchestration
 */
export async function addOrchestrationNode(
  nodeType: OrchestrationNodeType,
  position: [number, number],
  config: Record<string, unknown> = {}
): Promise<void> {
  const orch = get(currentOrchestration);
  if (!orch) {
    throw new Error('No orchestration loaded');
  }

  const node: OrchestrationNode = {
    id: `node-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    nodeType,
    position,
    config,
  };

  try {
    const updated = await invoke<OrchestrationGraph>('add_orchestration_node', {
      orchestrationId: orch.id,
      node,
    });
    currentOrchestration.set(updated);
    isOrchestrationDirty.set(true);
  } catch (error) {
    console.error('Failed to add orchestration node:', error);
    throw error;
  }
}

/**
 * Remove a node from the current orchestration
 */
export async function removeOrchestrationNode(nodeId: string): Promise<void> {
  const orch = get(currentOrchestration);
  if (!orch) {
    throw new Error('No orchestration loaded');
  }

  try {
    const updated = await invoke<OrchestrationGraph>('remove_orchestration_node', {
      orchestrationId: orch.id,
      nodeId,
    });
    currentOrchestration.set(updated);
    if (get(selectedOrchestrationNodeId) === nodeId) {
      selectedOrchestrationNodeId.set(null);
    }
    isOrchestrationDirty.set(true);
  } catch (error) {
    console.error('Failed to remove orchestration node:', error);
    throw error;
  }
}

/**
 * Add an edge to the current orchestration
 */
export async function addOrchestrationEdge(
  source: string,
  sourceHandle: string,
  target: string,
  targetHandle: string
): Promise<void> {
  const orch = get(currentOrchestration);
  if (!orch) {
    throw new Error('No orchestration loaded');
  }

  const edge: OrchestrationEdge = {
    id: `edge-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    source,
    sourceHandle,
    target,
    targetHandle,
  };

  try {
    const updated = await invoke<OrchestrationGraph>('add_orchestration_edge', {
      orchestrationId: orch.id,
      edge,
    });
    currentOrchestration.set(updated);
    isOrchestrationDirty.set(true);
  } catch (error) {
    console.error('Failed to add orchestration edge:', error);
    throw error;
  }
}

/**
 * Remove an edge from the current orchestration
 */
export async function removeOrchestrationEdge(edgeId: string): Promise<void> {
  const orch = get(currentOrchestration);
  if (!orch) {
    throw new Error('No orchestration loaded');
  }

  try {
    const updated = await invoke<OrchestrationGraph>('remove_orchestration_edge', {
      orchestrationId: orch.id,
      edgeId,
    });
    currentOrchestration.set(updated);
    isOrchestrationDirty.set(true);
  } catch (error) {
    console.error('Failed to remove orchestration edge:', error);
    throw error;
  }
}

/**
 * Update a node's configuration
 */
export async function updateOrchestrationNodeConfig(
  nodeId: string,
  config: Record<string, unknown>
): Promise<void> {
  const orch = get(currentOrchestration);
  if (!orch) {
    throw new Error('No orchestration loaded');
  }

  try {
    const updated = await invoke<OrchestrationGraph>('update_orchestration_node', {
      orchestrationId: orch.id,
      nodeId,
      config,
    });
    currentOrchestration.set(updated);
    isOrchestrationDirty.set(true);
  } catch (error) {
    console.error('Failed to update orchestration node:', error);
    throw error;
  }
}

/**
 * Update a node's position
 */
export async function updateOrchestrationNodePosition(
  nodeId: string,
  x: number,
  y: number
): Promise<void> {
  const orch = get(currentOrchestration);
  if (!orch) {
    throw new Error('No orchestration loaded');
  }

  try {
    await invoke('update_orchestration_node_position', {
      orchestrationId: orch.id,
      nodeId,
      x,
      y,
    });

    // Update local state
    currentOrchestration.update((o) => {
      if (!o) return o;
      return {
        ...o,
        nodes: o.nodes.map((n) =>
          n.id === nodeId ? { ...n, position: [x, y] as [number, number] } : n
        ),
      };
    });
    isOrchestrationDirty.set(true);
  } catch (error) {
    console.error('Failed to update orchestration node position:', error);
    throw error;
  }
}

/**
 * Associate a data graph with a DataGraph node
 */
export async function setDataGraphForNode(
  nodeId: string,
  dataGraphId: string
): Promise<void> {
  const orch = get(currentOrchestration);
  if (!orch) {
    throw new Error('No orchestration loaded');
  }

  try {
    await invoke('set_orchestration_data_graph', {
      orchestrationId: orch.id,
      nodeId,
      dataGraphId,
    });

    // Update local state
    currentOrchestration.update((o) => {
      if (!o) return o;
      return {
        ...o,
        dataGraphs: { ...o.dataGraphs, [nodeId]: dataGraphId },
      };
    });
    isOrchestrationDirty.set(true);
  } catch (error) {
    console.error('Failed to set data graph for node:', error);
    throw error;
  }
}

/**
 * Execute the current orchestration
 */
export async function executeOrchestration(
  initialData: Record<string, unknown> = {}
): Promise<OrchestrationResult> {
  const orch = get(currentOrchestration);
  if (!orch) {
    throw new Error('No orchestration loaded');
  }

  isOrchestrationRunning.set(true);
  lastOrchestrationResult.set(null);

  try {
    const result = await invoke<OrchestrationResult>('execute_orchestration', {
      orchestrationId: orch.id,
      initialData,
    });
    lastOrchestrationResult.set(result);
    return result;
  } catch (error) {
    console.error('Failed to execute orchestration:', error);
    const errorResult: OrchestrationResult = {
      success: false,
      outputs: {},
      error: String(error),
      nodesExecuted: 0,
      executionTimeMs: 0,
    };
    lastOrchestrationResult.set(errorResult);
    throw error;
  } finally {
    isOrchestrationRunning.set(false);
  }
}

/**
 * Select a node in the orchestration
 */
export function selectOrchestrationNode(nodeId: string | null): void {
  selectedOrchestrationNodeId.set(nodeId);
}

/**
 * Clear the current orchestration
 */
export function clearOrchestration(): void {
  currentOrchestration.set(null);
  selectedOrchestrationNodeId.set(null);
  isOrchestrationDirty.set(false);
  lastOrchestrationResult.set(null);
}

/**
 * Graph Session Store — thin re-export layer with Pantograph-specific additions.
 *
 * Re-exports session stores from the singleton and adds system graph support
 * (SYSTEM_GRAPHS, loadSystemGraph) which is Pantograph-specific.
 */
import { sessionStores, backend, workflowStores } from './storeInstances';

// Re-export types
export type { GraphType, GraphInfo } from '@pantograph/svelte-graph';

// --- Re-export writable stores ---
export const currentGraphId = sessionStores.currentGraphId;
export const currentGraphType = sessionStores.currentGraphType;
export const currentGraphName = sessionStores.currentGraphName;
export const availableWorkflows = sessionStores.availableWorkflows;
export const currentSessionId = sessionStores.currentSessionId;

// --- Re-export derived stores ---
export const isReadOnly = sessionStores.isReadOnly;
export const currentGraphInfo = sessionStores.currentGraphInfo;

// --- Re-export actions ---
export const refreshWorkflowList = sessionStores.refreshWorkflowList;
export const loadWorkflowByName = sessionStores.loadWorkflowByName;
export const createNewWorkflow = sessionStores.createNewWorkflow;
export const saveLastGraph = sessionStores.saveLastGraph;

// --- Pantograph-specific: System graphs ---

import type { GraphInfo, GraphType } from '@pantograph/svelte-graph';

export const SYSTEM_GRAPHS: GraphInfo[] = [
  {
    id: 'app-architecture',
    name: 'App Architecture',
    type: 'system',
    description: 'Internal application architecture visualization',
  },
];

/**
 * Load a system graph (e.g., app-architecture)
 */
export function loadSystemGraph(graphId: string): boolean {
  const systemGraph = SYSTEM_GRAPHS.find((g) => g.id === graphId);
  if (!systemGraph) {
    console.error(`Unknown system graph: ${graphId}`);
    return false;
  }

  currentGraphId.set(graphId);
  currentGraphType.set('system');
  currentGraphName.set(systemGraph.name);

  saveLastGraph(graphId, 'system');
  return true;
}

// --- Override: loadLastGraph with system graph support ---

const LAST_GRAPH_KEY = 'pantograph.lastGraph';

function getLastGraph(): { id: string; type: GraphType } | null {
  try {
    const stored = localStorage.getItem(LAST_GRAPH_KEY);
    return stored ? JSON.parse(stored) : null;
  } catch {
    return null;
  }
}

/**
 * Load the last opened graph, or fall back to default.
 * Extends the package's loadLastGraph to handle system graphs.
 */
export async function loadLastGraph(): Promise<void> {
  const last = getLastGraph();
  if (last?.type === 'system') {
    // Load definitions so they're available when user switches to a workflow
    const defs = await backend.getNodeDefinitions();
    workflowStores.nodeDefinitions.set(defs);
    await sessionStores.refreshWorkflowList();
    loadSystemGraph(last.id);
    return;
  }

  // Delegate to factory for workflow graphs (handles definitions, fallback, etc.)
  await sessionStores.loadLastGraph();
}

/**
 * Switch to a different graph (workflow or system)
 */
export async function switchGraph(graphId: string, type: GraphType): Promise<boolean> {
  if (type === 'system') {
    return loadSystemGraph(graphId);
  }
  return sessionStores.loadWorkflowByName(graphId);
}

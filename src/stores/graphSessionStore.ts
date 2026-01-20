import { writable, derived, get } from 'svelte/store';
import { workflowService } from '../services/workflow/WorkflowService';
import { loadWorkflow, clearWorkflow, nodeDefinitions } from './workflowStore';
import type { WorkflowMetadata } from '../services/workflow/types';

// --- Types ---

export type GraphType = 'workflow' | 'system';

export interface GraphInfo {
  id: string;
  name: string;
  type: GraphType;
  description?: string;
  path?: string; // File path for workflows
}

// --- Constants ---

const LAST_GRAPH_KEY = 'pantograph.lastGraph';
const DEFAULT_GRAPH_ID = 'coding-agent';

// System graphs (read-only)
export const SYSTEM_GRAPHS: GraphInfo[] = [
  {
    id: 'app-architecture',
    name: 'App Architecture',
    type: 'system',
    description: 'Internal application architecture visualization',
  },
];

// --- State ---

export const currentGraphId = writable<string | null>(null);
export const currentGraphType = writable<GraphType>('workflow');
export const currentGraphName = writable<string>('Untitled');
export const availableWorkflows = writable<WorkflowMetadata[]>([]);

// --- Derived ---

export const isReadOnly = derived(currentGraphType, ($type) => $type === 'system');

export const currentGraphInfo = derived(
  [currentGraphId, currentGraphType, currentGraphName],
  ([$id, $type, $name]): GraphInfo | null => {
    if (!$id) return null;

    // Check if it's a system graph
    const systemGraph = SYSTEM_GRAPHS.find((g) => g.id === $id);
    if (systemGraph) return systemGraph;

    // It's a workflow
    return {
      id: $id,
      name: $name,
      type: $type,
    };
  }
);

// --- Actions ---

/**
 * Load available workflows from the backend
 */
export async function refreshWorkflowList(): Promise<void> {
  try {
    const workflows = await workflowService.listWorkflows();
    availableWorkflows.set(workflows);
  } catch (error) {
    console.error('Failed to load workflow list:', error);
    availableWorkflows.set([]);
  }
}

/**
 * Load a workflow by name (filename stem without .json extension)
 */
export async function loadWorkflowByName(name: string): Promise<boolean> {
  console.log(`[graphSessionStore] Loading workflow: "${name}"`);
  try {
    // Ensure node definitions are loaded (needed for loadWorkflow to attach them)
    if (get(nodeDefinitions).length === 0) {
      const definitions = await workflowService.getNodeDefinitions();
      nodeDefinitions.set(definitions);
    }

    const path = `.pantograph/workflows/${name}.json`;
    console.log(`[graphSessionStore] Loading from path: ${path}`);
    const file = await workflowService.loadWorkflow(path);
    console.log(`[graphSessionStore] Loaded workflow with ${file.graph.nodes.length} nodes`);

    loadWorkflow(file.graph, file.metadata);

    currentGraphId.set(name);
    currentGraphType.set('workflow');
    currentGraphName.set(file.metadata.name);

    saveLastGraph(name, 'workflow');

    console.log(`[graphSessionStore] Workflow "${name}" loaded successfully`);
    return true;
  } catch (error) {
    console.error(`[graphSessionStore] Failed to load workflow "${name}":`, error);
    return false;
  }
}

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

  // The actual graph data will be loaded by the component
  // based on currentGraphId being 'app-architecture'

  return true;
}

/**
 * Create a new empty workflow
 */
export function createNewWorkflow(): void {
  clearWorkflow();

  const newId = `workflow-${Date.now()}`;
  currentGraphId.set(newId);
  currentGraphType.set('workflow');
  currentGraphName.set('Untitled Workflow');
}

/**
 * Save last opened graph to localStorage
 */
function saveLastGraph(id: string, type: GraphType): void {
  try {
    localStorage.setItem(LAST_GRAPH_KEY, JSON.stringify({ id, type }));
  } catch {
    // localStorage might not be available
  }
}

/**
 * Get last opened graph from localStorage
 */
function getLastGraph(): { id: string; type: GraphType } | null {
  try {
    const stored = localStorage.getItem(LAST_GRAPH_KEY);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch {
    // localStorage might not be available or corrupted
  }
  return null;
}

/**
 * Load the last opened graph, or fall back to default
 */
export async function loadLastGraph(): Promise<void> {
  // Load node definitions first so they're available for loadWorkflow
  const definitions = await workflowService.getNodeDefinitions();
  nodeDefinitions.set(definitions);

  await refreshWorkflowList();

  const lastGraph = getLastGraph();

  if (lastGraph) {
    if (lastGraph.type === 'system') {
      loadSystemGraph(lastGraph.id);
      return;
    }

    // Try to load the last workflow
    const success = await loadWorkflowByName(lastGraph.id);
    if (success) return;
  }

  // Fall back to default coding-agent workflow
  const success = await loadWorkflowByName(DEFAULT_GRAPH_ID);
  if (success) return;

  // If coding-agent doesn't exist, create a new empty workflow
  createNewWorkflow();
}

/**
 * Switch to a different graph
 */
export async function switchGraph(graphId: string, type: GraphType): Promise<boolean> {
  console.log(`[graphSessionStore] switchGraph called: graphId="${graphId}", type="${type}"`);
  if (type === 'system') {
    return loadSystemGraph(graphId);
  }
  return loadWorkflowByName(graphId);
}

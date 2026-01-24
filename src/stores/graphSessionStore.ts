import { writable, derived, get } from 'svelte/store';
import { workflowService } from '../services/workflow/WorkflowService';
import { loadWorkflow, clearWorkflow, nodeDefinitions } from './workflowStore';
import { loadOrchestration } from './orchestrationStore';
import { setOrchestrationContext } from './viewStore';
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
/** The current backend session ID for the loaded workflow */
export const currentSessionId = writable<string | null>(null);

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
 * Also creates a backend session for editing with undo/redo support.
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

    // Load workflow into frontend stores
    loadWorkflow(file.graph, file.metadata);

    // Load associated orchestration if specified (enables zoom-out navigation)
    console.log(`[graphSessionStore] Checking for orchestrationId in metadata:`, file.metadata);
    if (file.metadata.orchestrationId) {
      try {
        console.log(`[graphSessionStore] Loading orchestration: ${file.metadata.orchestrationId}`);
        await loadOrchestration(file.metadata.orchestrationId);
        setOrchestrationContext(file.metadata.orchestrationId);
        console.log(`[graphSessionStore] Loaded associated orchestration: ${file.metadata.orchestrationId}`);
      } catch (error) {
        console.warn('[graphSessionStore] Failed to load associated orchestration:', error);
        // Continue without orchestration - workflow still usable, just no zoom-out
      }
    } else {
      console.log('[graphSessionStore] No orchestrationId in workflow metadata');
    }

    // Create a backend session for this workflow
    // This enables editing operations to go through the backend with undo/redo
    const sessionId = await workflowService.createSession(file.graph);
    currentSessionId.set(sessionId);
    console.log(`[graphSessionStore] Created backend session: ${sessionId}`);

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
 * Also creates a backend session for editing with undo/redo support.
 */
export async function createNewWorkflow(): Promise<void> {
  clearWorkflow();

  const newId = `workflow-${Date.now()}`;
  currentGraphId.set(newId);
  currentGraphType.set('workflow');
  currentGraphName.set('Untitled Workflow');

  // Create a backend session for the empty workflow
  const emptyGraph = { nodes: [], edges: [] };
  const sessionId = await workflowService.createSession(emptyGraph);
  currentSessionId.set(sessionId);
  console.log(`[graphSessionStore] Created backend session for new workflow: ${sessionId}`);
}

/**
 * Save last opened graph to localStorage
 */
export function saveLastGraph(id: string, type: GraphType): void {
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
  await createNewWorkflow();
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

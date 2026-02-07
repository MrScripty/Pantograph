/**
 * Session Store Factory — creates per-instance graph session state
 *
 * Manages which graph is currently loaded, backend sessions, and workflow list.
 */
import { writable, derived, get } from 'svelte/store';
import type { WorkflowMetadata } from '../types/workflow.js';
import type { WorkflowBackend } from '../types/backend.js';
import type { WorkflowStores } from './createWorkflowStores.js';
import type { ViewStores } from './createViewStores.js';

// --- Types ---

export type GraphType = 'workflow' | 'system';

export interface GraphInfo {
  id: string;
  name: string;
  type: GraphType;
  description?: string;
  path?: string;
}

export interface SessionStoreOptions {
  /** Default graph ID to load on startup */
  defaultGraphId?: string;
  /** localStorage key for last-opened graph (omit to disable) */
  storageKey?: string;
  /** Hook called after a workflow is loaded (e.g., for loading orchestration context) */
  onWorkflowLoaded?: (metadata: WorkflowMetadata) => Promise<void>;
}

export interface SessionStores {
  // Writable stores
  currentGraphId: ReturnType<typeof writable<string | null>>;
  currentGraphType: ReturnType<typeof writable<GraphType>>;
  currentGraphName: ReturnType<typeof writable<string>>;
  availableWorkflows: ReturnType<typeof writable<WorkflowMetadata[]>>;
  currentSessionId: ReturnType<typeof writable<string | null>>;

  // Derived stores
  isReadOnly: ReturnType<typeof derived>;
  currentGraphInfo: ReturnType<typeof derived>;

  // Actions
  refreshWorkflowList: () => Promise<void>;
  loadWorkflowByName: (name: string) => Promise<boolean>;
  createNewWorkflow: () => Promise<void>;
  saveLastGraph: (id: string, type: GraphType) => void;
  loadLastGraph: () => Promise<void>;
  switchGraph: (graphId: string, type: GraphType) => Promise<boolean>;
}

export function createSessionStores(
  backend: WorkflowBackend,
  workflowStores: WorkflowStores,
  _viewStores: ViewStores,
  options?: SessionStoreOptions,
): SessionStores {
  const defaultGraphId = options?.defaultGraphId ?? 'coding-agent';
  const storageKey = options?.storageKey;

  // --- State ---
  const currentGraphId = writable<string | null>(null);
  const currentGraphType = writable<GraphType>('workflow');
  const currentGraphName = writable<string>('Untitled');
  const availableWorkflows = writable<WorkflowMetadata[]>([]);
  const currentSessionId = writable<string | null>(null);

  // --- Derived ---
  const isReadOnly = derived(currentGraphType, ($type) => $type === 'system');

  const currentGraphInfo = derived(
    [currentGraphId, currentGraphType, currentGraphName],
    ([$id, $type, $name]): GraphInfo | null => {
      if (!$id) return null;
      return { id: $id, name: $name, type: $type };
    }
  );

  // --- Actions ---

  async function refreshWorkflowList(): Promise<void> {
    try {
      const workflows = await backend.listWorkflows();
      availableWorkflows.set(workflows);
    } catch (error) {
      console.error('Failed to load workflow list:', error);
      availableWorkflows.set([]);
    }
  }

  async function loadWorkflowByName(name: string): Promise<boolean> {
    try {
      // Ensure node definitions are loaded
      if (get(workflowStores.nodeDefinitions).length === 0) {
        const definitions = await backend.getNodeDefinitions();
        workflowStores.nodeDefinitions.set(definitions);
      }

      const path = `.pantograph/workflows/${name}.json`;
      const file = await backend.loadWorkflow(path);

      workflowStores.loadWorkflow(file.graph, file.metadata);

      // Call optional hook for consumer-specific post-load behavior
      if (options?.onWorkflowLoaded && file.metadata) {
        try {
          await options.onWorkflowLoaded(file.metadata);
        } catch (error) {
          console.warn('[sessionStores] onWorkflowLoaded hook failed:', error);
        }
      }

      // Create a backend session for this workflow
      const sessionId = await backend.createSession(file.graph);
      currentSessionId.set(sessionId);

      currentGraphId.set(name);
      currentGraphType.set('workflow');
      currentGraphName.set(file.metadata.name);

      saveLastGraph(name, 'workflow');
      return true;
    } catch (error) {
      console.error(`[sessionStores] Failed to load workflow "${name}":`, error);
      return false;
    }
  }

  async function createNewWorkflow(): Promise<void> {
    workflowStores.clearWorkflow();

    const newId = `workflow-${Date.now()}`;
    currentGraphId.set(newId);
    currentGraphType.set('workflow');
    currentGraphName.set('Untitled Workflow');

    const emptyGraph = { nodes: [], edges: [] };
    const sessionId = await backend.createSession(emptyGraph);
    currentSessionId.set(sessionId);
  }

  function saveLastGraph(id: string, type: GraphType): void {
    if (!storageKey) return;
    try {
      localStorage.setItem(storageKey, JSON.stringify({ id, type }));
    } catch {
      // localStorage might not be available
    }
  }

  function getLastGraph(): { id: string; type: GraphType } | null {
    if (!storageKey) return null;
    try {
      const stored = localStorage.getItem(storageKey);
      if (stored) return JSON.parse(stored);
    } catch {
      // localStorage might not be available or corrupted
    }
    return null;
  }

  async function loadLastGraph(): Promise<void> {
    // Load node definitions first
    const definitions = await backend.getNodeDefinitions();
    workflowStores.nodeDefinitions.set(definitions);

    await refreshWorkflowList();

    const lastGraph = getLastGraph();
    if (lastGraph) {
      if (lastGraph.type === 'workflow') {
        const success = await loadWorkflowByName(lastGraph.id);
        if (success) return;
      }
      // System graphs are consumer-specific — skip here
    }

    // Fall back to default workflow
    const success = await loadWorkflowByName(defaultGraphId);
    if (success) return;

    // If default doesn't exist, create a new empty workflow
    await createNewWorkflow();
  }

  async function switchGraph(graphId: string, type: GraphType): Promise<boolean> {
    if (type === 'workflow') {
      return loadWorkflowByName(graphId);
    }
    // System graphs are consumer-specific — return false
    return false;
  }

  return {
    currentGraphId, currentGraphType, currentGraphName, availableWorkflows, currentSessionId,
    isReadOnly, currentGraphInfo,
    refreshWorkflowList, loadWorkflowByName, createNewWorkflow,
    saveLastGraph, loadLastGraph, switchGraph,
  };
}

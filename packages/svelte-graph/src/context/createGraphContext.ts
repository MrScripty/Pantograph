/**
 * Creates a graph context and sets it in Svelte's component context.
 *
 * Must be called during component initialization (top-level <script>, not in onMount).
 * Child components retrieve the context via useGraphContext().
 */
import { setContext } from 'svelte';
import { GRAPH_CONTEXT_KEY } from './keys.js';
import type { GraphContext } from './types.js';
import type { WorkflowBackend } from '../types/backend.js';
import type { NodeTypeRegistry } from '../types/registry.js';
import { createWorkflowStores } from '../stores/createWorkflowStores.js';
import { createViewStores, type ViewStoreOptions } from '../stores/createViewStores.js';
import { createSessionStores, type SessionStoreOptions } from '../stores/createSessionStores.js';

export interface GraphContextOptions {
  view?: ViewStoreOptions;
  session?: SessionStoreOptions;
}

/**
 * Create a graph context with backend, registry, and all stores.
 * Sets the context for all child components in the subtree.
 */
export function createGraphContext(
  backend: WorkflowBackend,
  registry: NodeTypeRegistry,
  options?: GraphContextOptions,
): GraphContext {
  const viewStores = createViewStores(options?.view);
  const workflowStores = createWorkflowStores(backend, {
    groupStack: viewStores.groupStack,
    tabOutOfGroup: viewStores.tabOutOfGroup,
  });
  const sessionStores = createSessionStores(backend, workflowStores, viewStores, options?.session);

  const context: GraphContext = {
    backend,
    registry,
    stores: {
      workflow: workflowStores,
      view: viewStores,
      session: sessionStores,
    },
  };

  setContext(GRAPH_CONTEXT_KEY, context);
  return context;
}

/**
 * Create a graph context from pre-existing stores.
 * Use this when stores are created separately (e.g., for global wrapper compatibility).
 */
export function createGraphContextFromStores(
  backend: WorkflowBackend,
  registry: NodeTypeRegistry,
  stores: GraphContext['stores'],
): GraphContext {
  const context: GraphContext = { backend, registry, stores };
  setContext(GRAPH_CONTEXT_KEY, context);
  return context;
}

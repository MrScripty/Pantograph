/**
 * Singleton store instances for Pantograph.
 *
 * Creates the TauriWorkflowBackend and all store factories once.
 * Both the Svelte context (for package components) and the global
 * store wrappers (for Pantograph-specific node components) point
 * to these same instances.
 */
import { TauriWorkflowBackend } from '../backends/TauriWorkflowBackend';
import { PANTOGRAPH_NODE_REGISTRY } from '../registry/pantographNodeTypes';
import {
  createWorkflowStores,
  createViewStores,
  createSessionStores,
} from '@pantograph/svelte-graph';
import { loadOrchestration } from './orchestrationStore';
import { workflowService } from '../services/workflow/WorkflowService';

// --- Singleton backend ---

export const backend = new TauriWorkflowBackend();
export const registry = PANTOGRAPH_NODE_REGISTRY;

// --- Singleton stores ---

export const viewStores = createViewStores({
  storageKey: 'pantograph.viewState',
});

export const workflowStores = createWorkflowStores(backend, {
  groupStack: viewStores.groupStack,
  tabOutOfGroup: viewStores.tabOutOfGroup,
});

export const sessionStores = createSessionStores(backend, workflowStores, viewStores, {
  defaultGraphId: 'coding-agent',
  storageKey: 'pantograph.lastGraph',
  onWorkflowLoaded: async (metadata) => {
    if (metadata.orchestrationId) {
      try {
        await loadOrchestration(metadata.orchestrationId);
        viewStores.setOrchestrationContext(metadata.orchestrationId);
      } catch (error) {
        console.warn('[storeInstances] Failed to load orchestration:', error);
      }
    }
  },
});

// Enable auto-persistence for view state
viewStores.enablePersistence();

// Sync session IDs to the legacy workflowService so existing components
// that call workflowService.addEdge() etc. continue to work.
sessionStores.currentSessionId.subscribe((id) => {
  workflowService.setCurrentExecutionId(id);
});

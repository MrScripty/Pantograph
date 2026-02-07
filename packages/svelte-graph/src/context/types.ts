// Context types for the graph editor

import type { WorkflowBackend } from '../types/backend.js';
import type { NodeTypeRegistry } from '../types/registry.js';
import type { WorkflowStores } from '../stores/createWorkflowStores.js';
import type { ViewStores } from '../stores/createViewStores.js';
import type { SessionStores } from '../stores/createSessionStores.js';

/** The full graph context available to all child components */
export interface GraphContext {
  backend: WorkflowBackend;
  registry: NodeTypeRegistry;
  stores: {
    workflow: WorkflowStores;
    view: ViewStores;
    session: SessionStores;
  };
}

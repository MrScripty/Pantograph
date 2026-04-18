import test from 'node:test';
import assert from 'node:assert/strict';
import { get } from 'svelte/store';
import { writable } from 'svelte/store';

import { createSessionStores } from './createSessionStores.ts';
import type {
  NodeDefinition,
  WorkflowBackend,
  WorkflowGraph,
  WorkflowMetadata,
  WorkflowSessionHandle,
} from '../index.ts';
import type { ViewStores } from './createViewStores.ts';
import type { WorkflowStores } from './createWorkflowStores.ts';

function createBackendStub(): WorkflowBackend {
  let sessionCounter = 0;
  const definitions: NodeDefinition[] = [];

  return {
    async getNodeDefinitions() {
      return definitions;
    },
    async validateConnection() {
      return true;
    },
    async createSession(_graph: WorkflowGraph) {
      sessionCounter += 1;
      return {
        session_id: `stub-session-${sessionCounter}`,
        session_kind: 'edit',
      } satisfies WorkflowSessionHandle;
    },
    async runSession() {},
    async removeSession() {},
    async executeWorkflow() {},
    async addNode() {
      return { graph: { nodes: [], edges: [] } };
    },
    async removeNode() {
      return { graph: { nodes: [], edges: [] } };
    },
    async addEdge() {
      return { graph: { nodes: [], edges: [] } };
    },
    async getConnectionCandidates() {
      throw new Error('not implemented');
    },
    async connectAnchors() {
      throw new Error('not implemented');
    },
    async insertNodeAndConnect() {
      throw new Error('not implemented');
    },
    async previewNodeInsertOnEdge() {
      throw new Error('not implemented');
    },
    async insertNodeOnEdge() {
      throw new Error('not implemented');
    },
    async removeEdge() {
      return { graph: { nodes: [], edges: [] } };
    },
    async updateNodeData() {
      return { graph: { nodes: [], edges: [] } };
    },
    async updateNodePosition() {
      return { graph: { nodes: [], edges: [] } };
    },
    async getExecutionGraph() {
      throw new Error('not implemented');
    },
    async getUndoRedoState() {
      return { canUndo: false, canRedo: false, undoCount: 0 };
    },
    async undo() {
      return { graph: { nodes: [], edges: [] } };
    },
    async redo() {
      return { graph: { nodes: [], edges: [] } };
    },
    async saveWorkflow() {
      return '';
    },
    async loadWorkflow() {
      return {
        version: '1.0',
        metadata: {
          name: 'stub',
          created: '',
          modified: '',
        },
        graph: { nodes: [], edges: [] },
      };
    },
    async listWorkflows() {
      return [] satisfies WorkflowMetadata[];
    },
    async deleteWorkflow() {},
    async createGroup() {
      throw new Error('not implemented');
    },
    async updateGroupPorts() {
      throw new Error('not implemented');
    },
    subscribeEvents() {
      return () => {};
    },
  };
}

test('createSessionStores tracks edit session kind for editor-owned sessions', async () => {
  const backend = createBackendStub();
  const workflowStores = {
    nodeDefinitions: writable<NodeDefinition[]>([]),
    setActiveSessionId() {},
    clearWorkflow() {},
    loadWorkflow() {},
  } as Pick<WorkflowStores, 'nodeDefinitions' | 'setActiveSessionId' | 'clearWorkflow' | 'loadWorkflow'> as WorkflowStores;
  const viewStores = {
    groupStack: writable<string[]>([]),
    async tabOutOfGroup() {},
  } as Pick<ViewStores, 'groupStack' | 'tabOutOfGroup'> as ViewStores;
  const sessionStores = createSessionStores(backend, workflowStores, viewStores);

  assert.equal(get(sessionStores.currentSessionKind), null);
  assert.equal(get(sessionStores.currentSessionId), null);

  await sessionStores.createNewWorkflow();

  assert.equal(get(sessionStores.currentSessionKind), 'edit');
  assert.match(get(sessionStores.currentSessionId) ?? '', /^stub-session-/);
});

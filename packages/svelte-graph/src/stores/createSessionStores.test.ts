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

function createBackendStub(overrides: Partial<WorkflowBackend> = {}): WorkflowBackend {
  let sessionCounter = 0;
  const definitions: NodeDefinition[] = [];

  const backend: WorkflowBackend = {
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
    async ungroup() {
      throw new Error('not implemented');
    },
    subscribeEvents() {
      return () => {};
    },
  };
  return { ...backend, ...overrides };
}

function createWorkflowStoresStub(
  onLoadWorkflow: (graph: WorkflowGraph, metadata?: WorkflowMetadata) => void = () => {},
): WorkflowStores {
  return {
    nodeDefinitions: writable<NodeDefinition[]>([]),
    setActiveSessionId() {},
    clearWorkflow() {},
    loadWorkflow: onLoadWorkflow,
  } as Pick<WorkflowStores, 'nodeDefinitions' | 'setActiveSessionId' | 'clearWorkflow' | 'loadWorkflow'> as WorkflowStores;
}

function createViewStoresStub(): ViewStores {
  return {
    groupStack: writable<string[]>([]),
    async tabOutOfGroup() {},
  } as Pick<ViewStores, 'groupStack' | 'tabOutOfGroup'> as ViewStores;
}

test('createSessionStores tracks edit session kind for editor-owned sessions', async () => {
  const backend = createBackendStub();
  const workflowStores = createWorkflowStoresStub();
  const viewStores = createViewStoresStub();
  const sessionStores = createSessionStores(backend, workflowStores, viewStores);

  assert.equal(get(sessionStores.currentSessionKind), null);
  assert.equal(get(sessionStores.currentSessionId), null);

  await sessionStores.createNewWorkflow();

  assert.equal(get(sessionStores.currentSessionKind), 'edit');
  assert.match(get(sessionStores.currentSessionId) ?? '', /^stub-session-/);
});

test('loadWorkflowByName renders the loaded file graph after creating an edit session', async () => {
  const loadedGraph = {
    nodes: [
      {
        id: 'loaded-node',
        node_type: 'text-input',
        position: { x: 1, y: 2 },
        data: {},
      },
    ],
    edges: [],
  } satisfies WorkflowGraph;
  let renderedGraph: WorkflowGraph | null = null;
  let createdSessionWorkflowId: string | null | undefined;
  const backend = createBackendStub({
    async createSession(_graph: WorkflowGraph, workflowId?: string | null) {
      createdSessionWorkflowId = workflowId;
      return {
        session_id: 'stub-session-1',
        session_kind: 'edit',
      } satisfies WorkflowSessionHandle;
    },
    async loadWorkflow(path: string) {
      assert.equal(path, '.pantograph/workflows/saved-flow.json');
      return {
        version: '1.0',
        metadata: {
          id: 'saved-flow',
          name: 'Saved Flow',
          created: '',
          modified: '',
        },
        graph: loadedGraph,
      };
    },
    async getExecutionGraph() {
      throw new Error('session graph refresh should not block initial render');
    },
  });
  const workflowStores = createWorkflowStoresStub((graph) => {
    renderedGraph = graph;
  });
  const sessionStores = createSessionStores(backend, workflowStores, createViewStoresStub());

  const loaded = await sessionStores.loadWorkflowByName('saved-flow');

  assert.equal(loaded, true);
  assert.equal(get(sessionStores.graphSessionError), null);
  assert.deepEqual(renderedGraph, loadedGraph);
  assert.equal(createdSessionWorkflowId, 'saved-flow');
  assert.equal(get(sessionStores.currentGraphId), 'saved-flow');
  assert.equal(get(sessionStores.currentGraphName), 'Saved Flow');
  assert.match(get(sessionStores.currentSessionId) ?? '', /^stub-session-/);
});

test('loadWorkflowByName exposes backend failures through graphSessionError', async () => {
  const backend = createBackendStub({
    async loadWorkflow() {
      throw new Error('workflow file missing');
    },
  });
  const sessionStores = createSessionStores(
    backend,
    createWorkflowStoresStub(),
    createViewStoresStub(),
  );

  const loaded = await sessionStores.loadWorkflowByName('missing-flow');

  assert.equal(loaded, false);
  assert.equal(
    get(sessionStores.graphSessionError),
    'Failed to load workflow "missing-flow": workflow file missing',
  );
});

test('deleteWorkflowByName deletes current workflow after backend confirmation', async () => {
  const calls: string[] = [];
  const backend = createBackendStub({
    async loadWorkflow() {
      return {
        version: '1.0',
        metadata: {
          id: 'saved-flow',
          name: 'Saved Flow',
          created: '',
          modified: '',
        },
        graph: { nodes: [], edges: [] },
      };
    },
    async listWorkflows() {
      calls.push('list');
      return [] satisfies WorkflowMetadata[];
    },
    async deleteWorkflow(name: string) {
      calls.push(`delete:${name}`);
    },
  });
  let cleared = false;
  const workflowStores = {
    ...createWorkflowStoresStub(),
    clearWorkflow() {
      cleared = true;
    },
  } as WorkflowStores;
  const sessionStores = createSessionStores(backend, workflowStores, createViewStoresStub());

  await sessionStores.loadWorkflowByName('saved-flow');
  const deleted = await sessionStores.deleteWorkflowByName('saved-flow');

  assert.equal(deleted, true);
  assert.deepEqual(calls, ['delete:saved-flow', 'list']);
  assert.equal(cleared, true);
  assert.equal(get(sessionStores.currentGraphName), 'Untitled Workflow');
  assert.match(get(sessionStores.currentSessionId) ?? '', /^stub-session-/);
  assert.equal(get(sessionStores.graphSessionError), null);
});

test('deleteWorkflowByName keeps current graph when backend deletion fails', async () => {
  const backend = createBackendStub({
    async deleteWorkflow() {
      throw new Error('delete denied');
    },
  });
  const sessionStores = createSessionStores(
    backend,
    createWorkflowStoresStub(),
    createViewStoresStub(),
  );

  const deleted = await sessionStores.deleteWorkflowByName('saved-flow');

  assert.equal(deleted, false);
  assert.equal(
    get(sessionStores.graphSessionError),
    'Failed to delete workflow "saved-flow": delete denied',
  );
});

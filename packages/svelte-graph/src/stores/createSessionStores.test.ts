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

interface Deferred<T> {
  promise: Promise<T>;
  reject: (error: unknown) => void;
  resolve: (value: T) => void;
}

function createDeferred<T>(): Deferred<T> {
  let resolve: (value: T) => void = () => {};
  let reject: (error: unknown) => void = () => {};
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });

  return { promise, reject, resolve };
}

function createBackendStub(overrides: Partial<WorkflowBackend> = {}): WorkflowBackend {
  let sessionCounter = 0;
  const definitions: NodeDefinition[] = [];
  const sessionGraphs = new Map<string, WorkflowGraph>();

  const backend: WorkflowBackend = {
    async getNodeDefinitions() {
      return definitions;
    },
    async validateConnection() {
      return true;
    },
    async createSession(_graph: WorkflowGraph) {
      sessionCounter += 1;
      sessionGraphs.set(`stub-session-${sessionCounter}`, _graph);
      return {
        session_id: `stub-session-${sessionCounter}`,
        session_kind: 'edit',
      } satisfies WorkflowSessionHandle;
    },
    async runSession() {
      return { workflow_run_id: 'stub-run-1' };
    },
    async removeSession() {},
    async addNode() {
      return { graph: { nodes: [], edges: [] } };
    },
    async removeNode() {
      return { graph: { nodes: [], edges: [] } };
    },
    async deleteSelection() {
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
    async removeEdges() {
      return { graph: { nodes: [], edges: [] } };
    },
    async updateNodeData() {
      return { graph: { nodes: [], edges: [] } };
    },
    async updateNodePosition() {
      return { graph: { nodes: [], edges: [] } };
    },
    async getExecutionGraph(sessionId: string) {
      return sessionGraphs.get(sessionId) ?? { nodes: [], edges: [] };
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
  const result = { ...backend, ...overrides };
  const createSession = result.createSession;
  result.createSession = async (graph, workflowId) => {
    const session = await createSession(graph, workflowId);
    sessionGraphs.set(session.session_id, graph);
    return session;
  };
  return result;
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

test('loadWorkflowByName renders the owner canonical graph and revision before activating the session', async () => {
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
  const ownerGraph: WorkflowGraph = {
    ...loadedGraph,
    nodes: loadedGraph.nodes.map((node) => ({ ...node, data: { emit_metadata: false } })),
    derived_graph: { schema_version: 1, graph_fingerprint: 'owner-revision', consumer_count_map: {} },
  };
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
    async getExecutionGraph(sessionId: string) {
      assert.equal(sessionId, 'stub-session-1');
      assert.equal(get(sessionStores.currentSessionId), null);
      return ownerGraph;
    },
  });
  const workflowStores = createWorkflowStoresStub((graph) => {
    renderedGraph = graph;
  });
  const sessionStores = createSessionStores(backend, workflowStores, createViewStoresStub());

  const loaded = await sessionStores.loadWorkflowByName('saved-flow');

  assert.equal(loaded, true);
  assert.equal(get(sessionStores.graphSessionError), null);
  assert.deepEqual(renderedGraph, ownerGraph);
  assert.equal(createdSessionWorkflowId, 'saved-flow');
  assert.equal(get(sessionStores.currentGraphId), 'saved-flow');
  assert.equal(get(sessionStores.currentGraphName), 'Saved Flow');
  assert.match(get(sessionStores.currentSessionId) ?? '', /^stub-session-/);
});

test('loadWorkflowByName uses loaded metadata id for session identity', async () => {
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
      assert.equal(path, '.pantograph/workflows/Display Flow.json');
      return {
        version: '1.0',
        metadata: {
          id: 'saved-flow',
          name: 'Display Flow',
          created: '',
          modified: '',
        },
        graph: { nodes: [], edges: [] },
      };
    },
  });
  const sessionStores = createSessionStores(
    backend,
    createWorkflowStoresStub(),
    createViewStoresStub(),
  );

  const loaded = await sessionStores.loadWorkflowByName('Display Flow');

  assert.equal(loaded, true);
  assert.equal(createdSessionWorkflowId, 'saved-flow');
  assert.equal(get(sessionStores.currentGraphId), 'saved-flow');
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

test('loadWorkflowByName ignores stale load responses after a newer workflow starts', async () => {
  const firstLoad = createDeferred<Awaited<ReturnType<WorkflowBackend['loadWorkflow']>>>();
  const secondGraph = {
    nodes: [
      {
        id: 'second-node',
        node_type: 'text-input',
        position: { x: 3, y: 4 },
        data: {},
      },
    ],
    edges: [],
  } satisfies WorkflowGraph;
  const renderedGraphs: WorkflowGraph[] = [];
  const createdSessionWorkflowIds: (string | null | undefined)[] = [];
  const backend = createBackendStub({
    async createSession(_graph: WorkflowGraph, workflowId?: string | null) {
      createdSessionWorkflowIds.push(workflowId);
      return {
        session_id: `session-${workflowId ?? 'untitled'}`,
        session_kind: 'edit',
      } satisfies WorkflowSessionHandle;
    },
    async loadWorkflow(path: string) {
      if (path.endsWith('/first-flow.json')) {
        return firstLoad.promise;
      }

      assert.equal(path, '.pantograph/workflows/second-flow.json');
      return {
        version: '1.0',
        metadata: {
          id: 'second-flow',
          name: 'Second Flow',
          created: '',
          modified: '',
        },
        graph: secondGraph,
      };
    },
  });
  const workflowStores = createWorkflowStoresStub((graph) => {
    renderedGraphs.push(graph);
  });
  const sessionStores = createSessionStores(backend, workflowStores, createViewStoresStub());

  const firstResult = sessionStores.loadWorkflowByName('first-flow');
  const secondResult = await sessionStores.loadWorkflowByName('second-flow');
  firstLoad.resolve({
    version: '1.0',
    metadata: {
      id: 'first-flow',
      name: 'First Flow',
      created: '',
      modified: '',
    },
    graph: { nodes: [], edges: [] },
  });

  assert.equal(secondResult, true);
  assert.equal(await firstResult, false);
  assert.deepEqual(renderedGraphs, [secondGraph]);
  assert.deepEqual(createdSessionWorkflowIds, ['second-flow']);
  assert.equal(get(sessionStores.currentGraphId), 'second-flow');
  assert.equal(get(sessionStores.currentGraphName), 'Second Flow');
  assert.equal(get(sessionStores.currentSessionId), 'session-second-flow');
});

test('loadWorkflowByName closes a session created by a stale load', async () => {
  const firstSessionStarted = createDeferred<void>();
  const firstSession = createDeferred<WorkflowSessionHandle>();
  const closedSessions: string[] = [];
  const renderedGraphs: WorkflowGraph[] = [];
  const backend = createBackendStub({
    async createSession(_graph: WorkflowGraph, workflowId?: string | null) {
      if (workflowId === 'first-flow') {
        firstSessionStarted.resolve(undefined);
        return firstSession.promise;
      }

      return {
        session_id: 'second-session',
        session_kind: 'edit',
      } satisfies WorkflowSessionHandle;
    },
    async removeSession(sessionId: string) {
      closedSessions.push(sessionId);
    },
    async loadWorkflow(path: string) {
      const workflowId = path.includes('first-flow') ? 'first-flow' : 'second-flow';
      return {
        version: '1.0',
        metadata: {
          id: workflowId,
          name: workflowId,
          created: '',
          modified: '',
        },
        graph: {
          nodes: [
            {
              id: workflowId,
              node_type: 'text-input',
              position: { x: 0, y: 0 },
              data: {},
            },
          ],
          edges: [],
        },
      };
    },
  });
  const workflowStores = createWorkflowStoresStub((graph) => {
    renderedGraphs.push(graph);
  });
  const sessionStores = createSessionStores(backend, workflowStores, createViewStoresStub());

  const firstResult = sessionStores.loadWorkflowByName('first-flow');
  await firstSessionStarted.promise;
  const secondResultPromise = sessionStores.loadWorkflowByName('second-flow');
  firstSession.resolve({
    session_id: 'first-session',
    session_kind: 'edit',
  });

  assert.equal(await secondResultPromise, true);
  assert.equal(await firstResult, false);
  assert.deepEqual(closedSessions, ['first-session']);
  assert.deepEqual(renderedGraphs.map((graph) => graph.nodes[0]?.id), ['second-flow']);
  assert.equal(get(sessionStores.currentSessionId), 'second-session');
});

test('loadWorkflowByName closes the previous active session after replacement', async () => {
  const closedSessions: string[] = [];
  const backend = createBackendStub({
    async createSession(_graph: WorkflowGraph, workflowId?: string | null) {
      return {
        session_id: `${workflowId}-session`,
        session_kind: 'edit',
      } satisfies WorkflowSessionHandle;
    },
    async removeSession(sessionId: string) {
      closedSessions.push(sessionId);
    },
    async loadWorkflow(path: string) {
      const workflowId = path.includes('first-flow') ? 'first-flow' : 'second-flow';
      return {
        version: '1.0',
        metadata: {
          id: workflowId,
          name: workflowId,
          created: '',
          modified: '',
        },
        graph: { nodes: [], edges: [] },
      };
    },
  });
  const sessionStores = createSessionStores(
    backend,
    createWorkflowStoresStub(),
    createViewStoresStub(),
  );

  assert.equal(await sessionStores.loadWorkflowByName('first-flow'), true);
  assert.equal(await sessionStores.loadWorkflowByName('second-flow'), true);

  assert.deepEqual(closedSessions, ['first-flow-session']);
  assert.equal(get(sessionStores.currentSessionId), 'second-flow-session');
});

test('late owner graph cannot replace a newer session and its orphan is closed', async () => {
  const started = createDeferred<void>();
  const firstGraph = createDeferred<WorkflowGraph>();
  const closed: string[] = [];
  const rendered: WorkflowGraph[] = [];
  const backend = createBackendStub({
    async createSession(_graph, workflowId) {
      return { session_id: workflowId!, session_kind: 'edit' };
    },
    async getExecutionGraph(sessionId) {
      if (sessionId === 'first') { started.resolve(); return firstGraph.promise; }
      return { nodes: [], edges: [], derived_graph: {
        schema_version: 1, graph_fingerprint: sessionId, consumer_count_map: {},
      } };
    },
    async removeSession(sessionId) { closed.push(sessionId); },
  });
  const stores = createSessionStores(backend, createWorkflowStoresStub((graph) => rendered.push(graph)), createViewStoresStub());
  const first = stores.loadWorkflowByName('first');
  await started.promise;
  assert.equal(get(stores.currentSessionId), null);
  assert.equal(await stores.loadWorkflowByName('second'), true);
  firstGraph.resolve({ nodes: [], edges: [] });
  assert.equal(await first, false);
  assert.deepEqual(closed, ['first']);
  assert.equal(get(stores.currentSessionId), 'second');
  assert.deepEqual(rendered.map((graph) => graph.derived_graph?.graph_fingerprint), ['second']);
});

test('failed owner graph read closes only the candidate and preserves the active session', async () => {
  const closed: string[] = [];
  let fail = false;
  const backend = createBackendStub({
    async getExecutionGraph() {
      if (fail) throw new Error('Owner snapshot unavailable');
      return { nodes: [], edges: [] };
    },
    async removeSession(sessionId) { closed.push(sessionId); },
  });
  const stores = createSessionStores(backend, createWorkflowStoresStub(), createViewStoresStub());
  assert.equal(await stores.loadWorkflowByName('active'), true);
  fail = true;
  assert.equal(await stores.loadWorkflowByName('failed'), false);
  assert.equal(get(stores.currentSessionId), 'stub-session-1');
  assert.equal(get(stores.currentGraphId), 'active');
  assert.match(get(stores.graphSessionError) ?? '', /Owner snapshot unavailable/);
  assert.deepEqual(closed, ['stub-session-2']);
});

test('cancelled owner read rejection cannot overwrite a newer graph error or session', async () => {
  const started = createDeferred<void>();
  const cancelled = createDeferred<WorkflowGraph>();
  const closed: string[] = [];
  const backend = createBackendStub({
    async getExecutionGraph(sessionId) {
      if (sessionId === 'stub-session-1') { started.resolve(); return cancelled.promise; }
      return { nodes: [], edges: [] };
    },
    async removeSession(sessionId) { closed.push(sessionId); },
  });
  const stores = createSessionStores(backend, createWorkflowStoresStub(), createViewStoresStub());
  const first = stores.loadWorkflowByName('cancelled');
  await started.promise;
  assert.equal(await stores.loadWorkflowByName('current'), true);
  cancelled.reject(new Error('late cancelled transport'));
  assert.equal(await first, false);
  assert.equal(get(stores.currentSessionId), 'stub-session-2');
  assert.equal(get(stores.graphSessionError), null);
  assert.deepEqual(closed, ['stub-session-1']);
});

test('new workflow uses the owner empty graph revision instead of the local fallback fingerprint', async () => {
  const ownerGraph: WorkflowGraph = { nodes: [], edges: [], derived_graph: {
    schema_version: 1, graph_fingerprint: 'owner-empty-revision', consumer_count_map: {},
  } };
  let rendered: WorkflowGraph | null = null;
  const backend = createBackendStub({ async getExecutionGraph() { return ownerGraph; } });
  const stores = createSessionStores(backend, createWorkflowStoresStub((graph) => { rendered = graph; }), createViewStoresStub());
  await stores.createNewWorkflow();
  assert.deepEqual(rendered, ownerGraph);
});

test('new workflow publishes owner graph before cleanup and cannot erase a newer load when cleanup completes', async () => {
  const cleanupStarted = createDeferred<void>();
  const cleanup = createDeferred<void>();
  let rendered: WorkflowGraph | null = null;
  const backend = createBackendStub({
    async getExecutionGraph(sessionId) {
      return { nodes: [], edges: [], derived_graph: {
        schema_version: 1, graph_fingerprint: sessionId, consumer_count_map: {},
      } };
    },
    async removeSession(sessionId) {
      if (sessionId === 'stub-session-1') { cleanupStarted.resolve(); await cleanup.promise; }
    },
  });
  const stores = createSessionStores(backend, createWorkflowStoresStub((graph) => { rendered = graph; }), createViewStoresStub());
  await stores.loadWorkflowByName('old');
  const creating = stores.createNewWorkflow();
  await cleanupStarted.promise;
  assert.equal(get(stores.currentSessionId), 'stub-session-2');
  assert.equal((rendered as WorkflowGraph | null)?.derived_graph?.graph_fingerprint, 'stub-session-2');
  await stores.loadWorkflowByName('newer');
  cleanup.resolve();
  await creating;
  assert.equal(get(stores.currentGraphId), 'newer');
  assert.equal(get(stores.currentSessionId), 'stub-session-3');
  assert.equal((rendered as WorkflowGraph | null)?.derived_graph?.graph_fingerprint, 'stub-session-3');
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

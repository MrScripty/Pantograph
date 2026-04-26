import test from 'node:test';
import assert from 'node:assert/strict';
import { get, writable } from 'svelte/store';

import { createWorkflowStores } from './createWorkflowStores.ts';
import type {
  GraphNode,
  NodeDefinition,
  WorkflowGraph,
  WorkflowGraphMutationResponse,
  WorkflowMetadata,
  WorkflowSessionHandle,
} from '../types/workflow.ts';
import type { WorkflowBackend } from '../types/backend.ts';
import type { NodeGroup } from '../types/groups.ts';

function flushAsyncWork(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
}

function createDeferred<T>(): Deferred<T> {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((promiseResolve) => {
    resolve = promiseResolve;
  });

  return { promise, resolve };
}

function createBackendStub(initialGraph: WorkflowGraph): WorkflowBackend {
  const nodeDefinitions: NodeDefinition[] = [
    {
      node_type: 'text-input',
      category: 'input',
      label: 'Text Input',
      description: 'Provides text input',
      io_binding_origin: 'client_session',
      inputs: [],
      outputs: [
        {
          id: 'text',
          label: 'Text',
          data_type: 'string',
          required: false,
          multiple: false,
        },
      ],
      execution_mode: 'manual',
    },
  ];

  let currentGraph = structuredClone(initialGraph);

  return {
    async getNodeDefinitions() {
      return nodeDefinitions;
    },
    async validateConnection() {
      return true;
    },
    async createSession(_graph: WorkflowGraph) {
      return {
        session_id: 'stub-session-1',
        session_kind: 'edit',
      } satisfies WorkflowSessionHandle;
    },
    async runSession() {
      return { workflow_run_id: 'stub-run-1' };
    },
    async removeSession() {},
    async addNode() {
      throw new Error('not implemented');
    },
    async removeNode() {
      throw new Error('not implemented');
    },
    async deleteSelection(nodeIds: string[], edgeIds: string[]) {
      const deletedNodeIds = new Set(nodeIds);
      const deletedEdgeIds = new Set(edgeIds);
      currentGraph = {
        ...currentGraph,
        nodes: currentGraph.nodes.filter((node) => !deletedNodeIds.has(node.id)),
        edges: currentGraph.edges.filter(
          (edge) =>
            !deletedEdgeIds.has(edge.id) &&
            !deletedNodeIds.has(edge.source) &&
            !deletedNodeIds.has(edge.target),
        ),
      };
      return {
        graph: structuredClone(currentGraph),
      };
    },
    async addEdge() {
      throw new Error('not implemented');
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
      throw new Error('not implemented');
    },
    async removeEdges(edgeIds: string[]) {
      const deletedEdgeIds = new Set(edgeIds);
      currentGraph = {
        ...currentGraph,
        edges: currentGraph.edges.filter((edge) => !deletedEdgeIds.has(edge.id)),
      };
      return {
        graph: structuredClone(currentGraph),
      };
    },
    async updateNodeData(nodeId: string, data: Record<string, unknown>) {
      currentGraph = {
        ...currentGraph,
        nodes: currentGraph.nodes.map((node) =>
          node.id === nodeId
            ? {
                ...node,
                data: { ...node.data, ...data },
              }
            : node,
        ),
      };

      return {
        graph: structuredClone(currentGraph),
        workflow_event: {
          type: 'GraphModified',
          data: {
            workflow_id: 'stub-session-1',
            dirty_tasks: [nodeId],
          },
        },
      };
    },
    async updateNodePosition() {
      throw new Error('not implemented');
    },
    async getExecutionGraph() {
      return structuredClone(currentGraph);
    },
    async getUndoRedoState() {
      return { canUndo: false, canRedo: false, undoCount: 0 };
    },
    async undo() {
      throw new Error('not implemented');
    },
    async redo() {
      throw new Error('not implemented');
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
        } satisfies WorkflowMetadata,
        graph: structuredClone(currentGraph),
      };
    },
    async listWorkflows() {
      return [] satisfies WorkflowMetadata[];
    },
    async deleteWorkflow() {},
    async createGroup(name: string, selectedNodeIds: string[]) {
      const selected = new Set(selectedNodeIds);
      const selectedNodes = currentGraph.nodes.filter((node) => selected.has(node.id));
      const group: NodeGroup = {
        id: 'group-from-backend',
        name,
        nodes: selectedNodes,
        edges: currentGraph.edges.filter(
          (edge) => selected.has(edge.source) && selected.has(edge.target),
        ),
        exposed_inputs: [
          {
            internal_node_id: selectedNodeIds[0],
            internal_port_id: 'text',
            group_port_id: `in-${selectedNodeIds[0]}-text`,
            group_port_label: 'text',
            data_type: 'any',
          },
        ],
        exposed_outputs: [],
        position: { x: 42, y: 24 },
        collapsed: true,
      };

      currentGraph = {
        nodes: [
          ...currentGraph.nodes.filter((node) => !selected.has(node.id)),
          {
            id: group.id,
            node_type: 'node-group',
            position: group.position,
            data: { label: group.name, group, isGroup: true },
          },
        ],
        edges: [
          {
            id: 'backend-owned-boundary',
            source: 'source',
            source_handle: 'text',
            target: group.id,
            target_handle: group.exposed_inputs[0].group_port_id,
          },
        ],
      };

      return {
        graph: structuredClone(currentGraph),
        workflow_event: {
          type: 'GraphModified',
          data: {
            workflow_id: 'stub-session-1',
            dirty_tasks: [group.id],
          },
        },
      };
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
}

test('createWorkflowStores applies backend graph-mutation responses to graph state and node execution state', async () => {
  const graph = {
    nodes: [
      {
        id: 'text-input-1',
        node_type: 'text-input',
        position: { x: 0, y: 0 },
        data: { text: 'draft' },
      },
    ],
    edges: [],
  } satisfies WorkflowGraph;
  const backend = createBackendStub(graph);
  const stores = createWorkflowStores(backend, {
    groupStack: writable<string[]>([]),
    async tabOutOfGroup() {},
  });

  stores.nodeDefinitions.set(await backend.getNodeDefinitions());

  const session = await backend.createSession(graph);
  stores.setActiveSessionId(session.session_id);
  stores.loadWorkflow(graph);
  stores.setNodeExecutionState('text-input-1', 'running');

  await stores.updateNodeData('text-input-1', { text: 'updated' });

  const updatedNode = (get(stores.workflowGraph) as WorkflowGraph).nodes.find(
    (node: GraphNode) => node.id === 'text-input-1',
  );
  assert.equal(updatedNode?.data.text, 'updated');
  assert.equal(
    get(stores.nodeExecutionStates).get('text-input-1')?.state,
    'idle',
  );
});

test('createWorkflowStores renders backend-owned group mutation responses', async () => {
  const graph = {
    nodes: [
      {
        id: 'source',
        node_type: 'text-input',
        position: { x: 0, y: 0 },
        data: { text: 'source' },
      },
      {
        id: 'a',
        node_type: 'text-input',
        position: { x: 100, y: 0 },
        data: { text: 'a' },
      },
      {
        id: 'b',
        node_type: 'text-input',
        position: { x: 200, y: 0 },
        data: { text: 'b' },
      },
    ],
    edges: [
      {
        id: 'source-to-a',
        source: 'source',
        source_handle: 'text',
        target: 'a',
        target_handle: 'text',
      },
      {
        id: 'a-to-b',
        source: 'a',
        source_handle: 'text',
        target: 'b',
        target_handle: 'text',
      },
    ],
  } satisfies WorkflowGraph;
  const backend = createBackendStub(graph);
  const stores = createWorkflowStores(backend, {
    groupStack: writable<string[]>([]),
    async tabOutOfGroup() {},
  });

  const session = await backend.createSession(graph);
  stores.setActiveSessionId(session.session_id);
  stores.loadWorkflow(graph);

  const group = await stores.createGroup('Backend Group', ['a', 'b']);
  await flushAsyncWork();

  assert.equal(group?.id, 'group-from-backend');
  assert.equal(get(stores.nodeGroups).get('group-from-backend')?.name, 'Backend Group');
  assert.deepEqual(
    (get(stores.workflowGraph) as WorkflowGraph).edges.map((edge) => edge.id),
    ['backend-owned-boundary'],
  );
});

test('createWorkflowStores ignores stale graph mutation responses', async () => {
  const graph = {
    nodes: [
      {
        id: 'text-input-1',
        node_type: 'text-input',
        position: { x: 0, y: 0 },
        data: { text: 'draft' },
      },
    ],
    edges: [],
  } satisfies WorkflowGraph;
  const mutation = createDeferred<WorkflowGraphMutationResponse>();
  const backend = createBackendStub(graph);
  const stores = createWorkflowStores(
    {
      ...backend,
      async updateNodeData() {
        return mutation.promise;
      },
    },
    {
      groupStack: writable<string[]>([]),
      async tabOutOfGroup() {},
    },
  );

  stores.loadWorkflow(graph);
  stores.setActiveSessionId('session-a');
  const result = stores.updateNodeData('text-input-1', { text: 'stale update' });
  stores.setActiveSessionId('session-b');
  mutation.resolve({
    graph: {
      nodes: [
        {
          id: 'text-input-1',
          node_type: 'text-input',
          position: { x: 0, y: 0 },
          data: { text: 'stale update' },
        },
      ],
      edges: [],
    },
  });

  assert.equal((await result).status, 'stale');
  const node = (get(stores.workflowGraph) as WorkflowGraph).nodes[0];
  assert.equal(node.data.text, 'draft');
});

test('createWorkflowStores reports stale add-node responses without applying them', async () => {
  const graph = { nodes: [], edges: [] } satisfies WorkflowGraph;
  const mutation = createDeferred<WorkflowGraphMutationResponse>();
  const backend = createBackendStub(graph);
  const stores = createWorkflowStores(
    {
      ...backend,
      async addNode() {
        return mutation.promise;
      },
    },
    {
      groupStack: writable<string[]>([]),
      async tabOutOfGroup() {},
    },
  );
  const definition = (await backend.getNodeDefinitions())[0];

  stores.loadWorkflow(graph);
  stores.setActiveSessionId('session-a');
  const result = stores.addNode(definition, { x: 5, y: 6 });
  stores.setActiveSessionId('session-b');
  mutation.resolve({
    graph: {
      nodes: [
        {
          id: 'stale-node',
          node_type: 'text-input',
          position: { x: 5, y: 6 },
          data: {},
        },
      ],
      edges: [],
    },
  });

  assert.equal((await result).status, 'stale');
  assert.deepEqual((get(stores.workflowGraph) as WorkflowGraph).nodes, []);
});

test('createWorkflowStores reports stale position responses without applying them', async () => {
  const graph = {
    nodes: [
      {
        id: 'text-input-1',
        node_type: 'text-input',
        position: { x: 0, y: 0 },
        data: {},
      },
    ],
    edges: [],
  } satisfies WorkflowGraph;
  const mutation = createDeferred<WorkflowGraphMutationResponse>();
  const backend = createBackendStub(graph);
  const stores = createWorkflowStores(
    {
      ...backend,
      async updateNodePosition() {
        return mutation.promise;
      },
    },
    {
      groupStack: writable<string[]>([]),
      async tabOutOfGroup() {},
    },
  );

  stores.loadWorkflow(graph);
  stores.setActiveSessionId('session-a');
  const result = stores.updateNodePosition('text-input-1', { x: 10, y: 20 });
  stores.setActiveSessionId('session-b');
  mutation.resolve({
    graph: {
      nodes: [
        {
          id: 'text-input-1',
          node_type: 'text-input',
          position: { x: 10, y: 20 },
          data: {},
        },
      ],
      edges: [],
    },
  });

  assert.equal((await result).status, 'stale');
  const node = (get(stores.workflowGraph) as WorkflowGraph).nodes[0];
  assert.deepEqual(node.position, { x: 0, y: 0 });
});

test('createWorkflowStores reports stale edge removal responses without applying them', async () => {
  const graph = {
    nodes: [],
    edges: [
      {
        id: 'edge-a',
        source: 'source',
        source_handle: 'text',
        target: 'target',
        target_handle: 'text',
      },
    ],
  } satisfies WorkflowGraph;
  const mutation = createDeferred<WorkflowGraphMutationResponse>();
  const backend = createBackendStub(graph);
  const stores = createWorkflowStores(
    {
      ...backend,
      async removeEdge() {
        return mutation.promise;
      },
    },
    {
      groupStack: writable<string[]>([]),
      async tabOutOfGroup() {},
    },
  );

  stores.loadWorkflow(graph);
  stores.setActiveSessionId('session-a');
  const result = stores.removeEdge('edge-a');
  stores.setActiveSessionId('session-b');
  mutation.resolve({
    graph: {
      nodes: [],
      edges: [],
    },
  });

  assert.equal((await result).status, 'stale');
  assert.deepEqual(
    (get(stores.workflowGraph) as WorkflowGraph).edges.map((edge) => edge.id),
    ['edge-a'],
  );
});

test('createWorkflowStores deletes selected nodes and edges with one backend mutation', async () => {
  const graph = {
    nodes: [
      {
        id: 'source',
        node_type: 'text-input',
        position: { x: 0, y: 0 },
        data: {},
      },
      {
        id: 'target',
        node_type: 'text-input',
        position: { x: 100, y: 0 },
        data: {},
      },
      {
        id: 'extra',
        node_type: 'text-input',
        position: { x: 200, y: 0 },
        data: {},
      },
    ],
    edges: [
      {
        id: 'source-target',
        source: 'source',
        source_handle: 'text',
        target: 'target',
        target_handle: 'text',
      },
      {
        id: 'target-extra',
        source: 'target',
        source_handle: 'text',
        target: 'extra',
        target_handle: 'text',
      },
    ],
  } satisfies WorkflowGraph;
  let deleteSelectionCalls = 0;
  const backend = createBackendStub(graph);
  const stores = createWorkflowStores(
    {
      ...backend,
      async deleteSelection(nodeIds: string[], edgeIds: string[], sessionId: string) {
        deleteSelectionCalls += 1;
        assert.deepEqual(nodeIds, ['target']);
        assert.deepEqual(edgeIds, ['source-target']);
        assert.equal(sessionId, 'session-a');
        return backend.deleteSelection(nodeIds, edgeIds, sessionId);
      },
    },
    {
      groupStack: writable<string[]>([]),
      async tabOutOfGroup() {},
    },
  );

  stores.loadWorkflow(graph);
  stores.setActiveSessionId('session-a');
  stores.selectedNodeIds.set(['target']);

  const result = await stores.deleteSelection(['target'], ['source-target']);

  assert.equal(result.status, 'applied');
  assert.equal(deleteSelectionCalls, 1);
  assert.deepEqual(
    (get(stores.workflowGraph) as WorkflowGraph).nodes.map((node) => node.id),
    ['source', 'extra'],
  );
  assert.deepEqual((get(stores.workflowGraph) as WorkflowGraph).edges, []);
  assert.deepEqual(get(stores.selectedNodeIds), []);
});

test('createWorkflowStores ignores stale group mutation responses', async () => {
  const graph = {
    nodes: [
      {
        id: 'a',
        node_type: 'text-input',
        position: { x: 0, y: 0 },
        data: { text: 'a' },
      },
      {
        id: 'b',
        node_type: 'text-input',
        position: { x: 100, y: 0 },
        data: { text: 'b' },
      },
    ],
    edges: [],
  } satisfies WorkflowGraph;
  const groupMutation = createDeferred<WorkflowGraphMutationResponse>();
  const backend = createBackendStub(graph);
  const stores = createWorkflowStores(
    {
      ...backend,
      async createGroup() {
        return groupMutation.promise;
      },
    },
    {
      groupStack: writable<string[]>([]),
      async tabOutOfGroup() {},
    },
  );

  stores.loadWorkflow(graph);
  stores.setActiveSessionId('session-a');
  const result = stores.createGroup('Stale Group', ['a', 'b']);
  stores.setActiveSessionId('session-b');
  groupMutation.resolve({
    graph: {
      nodes: [
        {
          id: 'stale-group',
          node_type: 'node-group',
          position: { x: 0, y: 0 },
          data: { label: 'Stale Group', isGroup: true },
        },
      ],
      edges: [],
    },
  });

  assert.equal(await result, null);
  assert.equal(get(stores.nodeGroups).has('stale-group'), false);
  assert.deepEqual(
    (get(stores.workflowGraph) as WorkflowGraph).nodes.map((node) => node.id),
    ['a', 'b'],
  );
});

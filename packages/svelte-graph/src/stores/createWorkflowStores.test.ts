import test from 'node:test';
import assert from 'node:assert/strict';
import { get, writable } from 'svelte/store';

import { createWorkflowStores } from './createWorkflowStores.ts';
import type {
  GraphNode,
  NodeDefinition,
  WorkflowGraph,
  WorkflowMetadata,
  WorkflowSessionHandle,
} from '../types/workflow.ts';
import type { WorkflowBackend } from '../types/backend.ts';

function flushAsyncWork(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
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
    async runSession() {},
    async removeSession() {},
    async executeWorkflow() {},
    async addNode() {
      throw new Error('not implemented');
    },
    async removeNode() {
      throw new Error('not implemented');
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
            execution_id: 'stub-session-1',
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

  stores.updateNodeData('text-input-1', { text: 'updated' });
  await flushAsyncWork();

  const updatedNode = (get(stores.workflowGraph) as WorkflowGraph).nodes.find(
    (node: GraphNode) => node.id === 'text-input-1',
  );
  assert.equal(updatedNode?.data.text, 'updated');
  assert.equal(
    get(stores.nodeExecutionStates).get('text-input-1')?.state,
    'idle',
  );
});

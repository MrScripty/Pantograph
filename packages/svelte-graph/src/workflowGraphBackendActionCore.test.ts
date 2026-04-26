import test from 'node:test';
import assert from 'node:assert/strict';

import type { Edge } from '@xyflow/svelte';

import {
  applyAcceptedWorkflowGraphMutation,
  commitWorkflowConnectionCore,
  commitWorkflowReconnectCore,
  removeWorkflowGraphEdgesCore,
} from './workflowGraphBackendActionCore.ts';
import type {
  ConnectionCommitResponse,
  WorkflowGraph,
} from './types/workflow.ts';

const graph: WorkflowGraph = {
  nodes: [],
  edges: [],
  derived_graph: {
    schema_version: 1,
    graph_fingerprint: 'revision-after-removal',
    consumer_count_map: {},
  },
};

test('applyAcceptedWorkflowGraphMutation syncs accepted backend graphs', () => {
  let syncedGraph: WorkflowGraph | null = null;

  const applied = applyAcceptedWorkflowGraphMutation(
    {
      accepted: true,
      graph_revision: 'revision-a',
      graph,
    },
    {
      setNodeExecutionState: () => undefined,
      syncGraph: (nextGraph) => {
        syncedGraph = nextGraph;
        return true;
      },
    },
  );

  assert.equal(applied, true);
  assert.equal(syncedGraph, graph);
});

test('commitWorkflowConnectionCore preserves rejected connection intent', async () => {
  let requestedRevision: string | null = null;
  const response: ConnectionCommitResponse = {
    accepted: false,
    graph_revision: 'revision-rejected',
    rejection: {
      reason: 'incompatible_types',
      message: 'Ports are not compatible',
    },
  };

  const result = await commitWorkflowConnectionCore({
    applyAcceptedMutation: () => {
      throw new Error('Rejected responses must not be applied');
    },
    connectAnchors: async (_sourceAnchor, _targetAnchor, graphRevision) => {
      requestedRevision = graphRevision;
      return response;
    },
    connection: {
      source: 'source',
      sourceHandle: 'out',
      target: 'target',
      targetHandle: 'in',
    },
    currentGraphRevision: 'current-revision',
    currentIntent: {
      sourceAnchor: { node_id: 'source', port_id: 'out' },
      graphRevision: 'intent-revision',
      compatibleNodeIds: ['target'],
      compatibleTargetKeys: ['target:in'],
      insertableNodeTypes: [],
    },
  });

  assert.equal(requestedRevision, 'intent-revision');
  assert.equal(result.response, response);
  assert.deepEqual(result.intent?.rejection, response.rejection);
  assert.deepEqual(result.intent?.compatibleNodeIds, ['target']);
});

test('removeWorkflowGraphEdgesCore ignores empty edge batches', async () => {
  let called = false;

  await removeWorkflowGraphEdgesCore({
    edgeIds: [],
    errorMessage: 'remove failed',
    removeEdges: async () => {
      called = true;
      return graph;
    },
    syncGraph: () => {
      throw new Error('Empty edge batches must not sync');
    },
  });

  assert.equal(called, false);
});

test('commitWorkflowReconnectCore restores old edge on rejected reconnect', async () => {
  const oldEdge: Edge = {
    id: 'edge-a',
    source: 'source',
    sourceHandle: 'out',
    target: 'target',
    targetHandle: 'in',
  };
  let restoredEdgeId: string | null = null;
  let requestedRevision: string | null = null;

  const result = await commitWorkflowReconnectCore({
    anchors: {
      sourceAnchor: { node_id: 'source', port_id: 'out' },
      targetAnchor: { node_id: 'other-target', port_id: 'in' },
    },
    applyAcceptedMutation: () => {
      throw new Error('Rejected reconnects must not be applied');
    },
    connectAnchors: async (_sourceAnchor, _targetAnchor, graphRevision) => {
      requestedRevision = graphRevision;
      return {
        accepted: false,
        graph_revision: 'revision-rejected',
        rejection: {
          reason: 'cycle_detected',
          message: 'Reconnect would create a cycle',
        },
      };
    },
    fallbackRevision: 'fallback-revision',
    oldEdge,
    removeEdge: async () => graph,
    restoreEdge: async (edge) => {
      restoredEdgeId = edge.id;
      return graph;
    },
    syncGraph: () => true,
  });

  assert.equal(requestedRevision, 'revision-after-removal');
  assert.equal(restoredEdgeId, 'edge-a');
  assert.equal(result.type, 'rejected');
  if (result.type === 'rejected') {
    assert.equal(result.graphRevision, 'revision-rejected');
    assert.deepEqual(result.sourceAnchor, { node_id: 'source', port_id: 'out' });
  }
});

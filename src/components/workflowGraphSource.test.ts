import test from 'node:test';
import assert from 'node:assert/strict';

import { resolveWorkflowGraphSource } from './workflowGraphSource.ts';

const workflowNodes = [{ id: 'workflow-node' }];
const workflowEdges = [{ id: 'workflow-edge' }];
const architectureGraph = {
  nodes: [{ id: 'architecture-node' }],
  edges: [{ id: 'architecture-edge' }],
};

test('resolveWorkflowGraphSource selects the architecture graph when requested', () => {
  assert.deepEqual(
    resolveWorkflowGraphSource({
      currentGraphType: 'system',
      currentGraphId: 'app-architecture',
      architectureGraph,
      workflowNodes,
      workflowEdges,
    }),
    {
      type: 'architecture',
      nodes: architectureGraph.nodes,
      edges: architectureGraph.edges,
    },
  );
});

test('resolveWorkflowGraphSource waits when the requested architecture graph is not loaded', () => {
  assert.deepEqual(
    resolveWorkflowGraphSource({
      currentGraphType: 'system',
      currentGraphId: 'app-architecture',
      architectureGraph: null,
      workflowNodes,
      workflowEdges,
    }),
    {
      type: 'architecture-pending',
    },
  );
});

test('resolveWorkflowGraphSource selects workflow store data for ordinary graphs', () => {
  assert.deepEqual(
    resolveWorkflowGraphSource({
      currentGraphType: 'workflow',
      currentGraphId: 'main',
      architectureGraph,
      workflowNodes,
      workflowEdges,
    }),
    {
      type: 'workflow',
      nodes: workflowNodes,
      edges: workflowEdges,
    },
  );
});

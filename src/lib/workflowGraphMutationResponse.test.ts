import test from 'node:test';
import assert from 'node:assert/strict';

import { parseWorkflowGraphMutationResponse } from './workflowGraphMutationResponse.ts';

test('parseWorkflowGraphMutationResponse accepts graph payloads with additive graph-modified events', () => {
  const response = parseWorkflowGraphMutationResponse({
    graph: { nodes: [], edges: [] },
    workflow_event: {
      type: 'GraphModified',
      data: {
        workflow_id: 'session-1',
        execution_id: 'session-1',
        dirty_tasks: ['node-a'],
      },
    },
  });

  assert.deepEqual(response.graph, { nodes: [], edges: [] });
  assert.equal(response.workflow_event?.type, 'GraphModified');
});

test('parseWorkflowGraphMutationResponse rejects payloads without graph data', () => {
  assert.throws(
    () => parseWorkflowGraphMutationResponse({ workflow_event: null }),
    /missing graph payload/,
  );
});

test('parseWorkflowGraphMutationResponse rejects malformed workflow events', () => {
  assert.throws(
    () =>
      parseWorkflowGraphMutationResponse({
        graph: { nodes: [], edges: [] },
        workflow_event: { type: 'Cancelled', data: {} },
      }),
    /invalid workflow_event payload/,
  );
});

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
        dirty_tasks: ['node-a'],
      },
    },
    workflow_session_state: {
      contract_version: 1,
      residency: 'active',
    },
  });

  assert.deepEqual(response.graph, { nodes: [], edges: [] });
  assert.equal(response.workflow_event?.type, 'GraphModified');
  assert.equal(response.workflow_session_state?.contract_version, 1);
});

test('parseWorkflowGraphMutationResponse normalizes backend graph-modified events', () => {
  const response = parseWorkflowGraphMutationResponse({
    graph: { nodes: [], edges: [] },
    workflow_event: {
      type: 'graphModified',
      workflowId: 'workflow-1',
      executionId: 'session-1',
      dirtyTasks: ['node-a'],
      memoryImpact: null,
      occurredAtMs: 123,
    },
    workflow_execution_session_state: {
      contract_version: 1,
      residency: 'active',
    },
  });

  assert.equal(response.workflow_event?.type, 'GraphModified');
  assert.deepEqual(response.workflow_event?.data, {
    workflow_id: 'workflow-1',
    execution_id: 'session-1',
    dirty_tasks: ['node-a'],
    memory_impact: null,
    occurred_at_ms: 123,
  });
  assert.equal(response.workflow_session_state?.contract_version, 1);
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

test('parseWorkflowGraphMutationResponse rejects malformed workflow session state', () => {
  assert.throws(
    () =>
      parseWorkflowGraphMutationResponse({
        graph: { nodes: [], edges: [] },
        workflow_session_state: { contract_version: 'v1', residency: 'active' },
      }),
    /invalid workflow_session_state payload/,
  );
});

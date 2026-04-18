import test from 'node:test';
import assert from 'node:assert/strict';

import { applyWorkflowGraphMutationResponse } from './workflowGraphMutationResponse.ts';
import type { WorkflowGraphMutationResponse } from '../types/workflow.ts';

test('applyWorkflowGraphMutationResponse replays backend-owned dirty tasks into idle execution state', () => {
  const calls: Array<{ nodeId: string; state: string }> = [];
  const response: WorkflowGraphMutationResponse = {
    graph: { nodes: [], edges: [] },
    workflow_event: {
      type: 'GraphModified',
      data: {
        workflow_id: 'session-1',
        execution_id: 'session-1',
        dirty_tasks: ['node-a', 'node-b'],
      },
    },
  };

  const handled = applyWorkflowGraphMutationResponse(response, {
    setNodeExecutionState(nodeId, state) {
      calls.push({ nodeId, state });
    },
  });

  assert.equal(handled, true);
  assert.deepEqual(calls, [
    { nodeId: 'node-a', state: 'idle' },
    { nodeId: 'node-b', state: 'idle' },
  ]);
});

test('applyWorkflowGraphMutationResponse ignores responses without backend workflow events', () => {
  const calls: Array<{ nodeId: string; state: string }> = [];
  const response: WorkflowGraphMutationResponse = {
    graph: { nodes: [], edges: [] },
  };

  const handled = applyWorkflowGraphMutationResponse(response, {
    setNodeExecutionState(nodeId, state) {
      calls.push({ nodeId, state });
    },
  });

  assert.equal(handled, false);
  assert.deepEqual(calls, []);
});

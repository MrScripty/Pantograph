import test from 'node:test';
import assert from 'node:assert/strict';
import type { Edge } from '@xyflow/svelte';

import { applyWorkflowExecutionEvent } from './workflowExecutionEvents.ts';
import type { NodeExecutionState, WorkflowEvent } from '../types/workflow.ts';

function createWorkflowActions() {
  const stateCalls: Array<{ nodeId: string; state: NodeExecutionState; message?: string }> = [];
  const updateCalls: Array<{ nodeId: string; data: Record<string, unknown> }> = [];

  return {
    workflow: {
      setNodeExecutionState(nodeId: string, state: NodeExecutionState, message?: string) {
        stateCalls.push({ nodeId, state, message });
      },
      updateNodeData(nodeId: string, data: Record<string, unknown>) {
        updateCalls.push({ nodeId, data });
      },
    },
    stateCalls,
    updateCalls,
  };
}

function applyEvent(
  event: WorkflowEvent,
  options?: {
    activeExecutionId?: string | null;
    waitingForInput?: boolean;
    edges?: Edge[];
  },
) {
  const actions = createWorkflowActions();
  const result = applyWorkflowExecutionEvent({
    event,
    activeExecutionId: options?.activeExecutionId ?? null,
    waitingForInput: options?.waitingForInput ?? false,
    edges: options?.edges ?? [],
    workflow: actions.workflow,
  });

  return {
    ...actions,
    result,
  };
}

test('applyWorkflowExecutionEvent marks node starts as running and claims execution id', () => {
  const { result, stateCalls } = applyEvent({
    type: 'NodeStarted',
    data: {
      node_id: 'node-a',
      node_type: 'llm',
      execution_id: 'run-1',
    },
  });

  assert.deepEqual(stateCalls, [{ nodeId: 'node-a', state: 'running', message: undefined }]);
  assert.equal(result.activeExecutionId, 'run-1');
  assert.equal(result.waitingForInput, false);
  assert.equal(result.handled, true);
  assert.equal(result.shouldCleanup, false);
});

test('applyWorkflowExecutionEvent ignores events for a different claimed execution', () => {
  const { result, stateCalls } = applyEvent(
    {
      type: 'NodeStarted',
      data: {
        node_id: 'node-b',
        node_type: 'llm',
        execution_id: 'run-2',
      },
    },
    { activeExecutionId: 'run-1' },
  );

  assert.deepEqual(stateCalls, []);
  assert.equal(result.activeExecutionId, 'run-1');
  assert.equal(result.handled, false);
  assert.equal(result.shouldCleanup, false);
});

test('applyWorkflowExecutionEvent propagates completed outputs to connected targets', () => {
  const { result, stateCalls, updateCalls } = applyEvent(
    {
      type: 'NodeCompleted',
      data: {
        node_id: 'producer',
        outputs: {
          image: 'blob-1',
        },
        execution_id: 'run-1',
      },
    },
    {
      edges: [
        {
          id: 'edge-1',
          source: 'producer',
          sourceHandle: 'image',
          target: 'consumer',
          targetHandle: 'input_image',
        } as Edge,
      ],
    },
  );

  assert.deepEqual(stateCalls, [{ nodeId: 'producer', state: 'success', message: undefined }]);
  assert.deepEqual(updateCalls, [
    {
      nodeId: 'consumer',
      data: {
        input_image: 'blob-1',
      },
    },
  ]);
  assert.equal(result.handled, true);
  assert.equal(result.shouldCleanup, false);
});

test('applyWorkflowExecutionEvent marks waiting nodes and keeps waiting state true', () => {
  const { result, stateCalls } = applyEvent({
    type: 'WaitingForInput',
    data: {
      node_id: 'input-node',
      message: 'Need user confirmation',
      execution_id: 'run-1',
    },
  });

  assert.deepEqual(stateCalls, [
    {
      nodeId: 'input-node',
      state: 'waiting',
      message: 'Need user confirmation',
    },
  ]);
  assert.equal(result.waitingForInput, true);
  assert.equal(result.handled, true);
  assert.equal(result.shouldCleanup, false);
});

test('applyWorkflowExecutionEvent requests cleanup for cancelled runs', () => {
  const { result } = applyEvent(
    {
      type: 'Cancelled',
      data: {
        error: 'Stopped by user',
        execution_id: 'run-1',
      },
    },
    {
      activeExecutionId: 'run-1',
      waitingForInput: true,
    },
  );

  assert.equal(result.activeExecutionId, 'run-1');
  assert.equal(result.waitingForInput, false);
  assert.equal(result.handled, true);
  assert.equal(result.shouldCleanup, true);
});

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
    activeWorkflowRunId?: string | null;
    waitingForInput?: boolean;
    edges?: Edge[];
  },
) {
  const actions = createWorkflowActions();
  const result = applyWorkflowExecutionEvent({
    event,
    activeWorkflowRunId: options?.activeWorkflowRunId ?? null,
    waitingForInput: options?.waitingForInput ?? false,
    edges: options?.edges ?? [],
    workflow: actions.workflow,
  });

  return {
    ...actions,
    result,
  };
}

test('applyWorkflowExecutionEvent marks node starts as running and claims workflow run id', () => {
  const { result, stateCalls } = applyEvent({
    type: 'NodeStarted',
    data: {
      node_id: 'node-a',
      node_type: 'llm',
      workflow_run_id: 'run-1',
    },
  });

  assert.deepEqual(stateCalls, [{ nodeId: 'node-a', state: 'running', message: undefined }]);
  assert.equal(result.activeWorkflowRunId, 'run-1');
  assert.equal(result.waitingForInput, false);
  assert.equal(result.handled, true);
  assert.equal(result.shouldCleanup, false);
});

test('applyWorkflowExecutionEvent ignores events for a different claimed workflow run', () => {
  const { result, stateCalls } = applyEvent(
    {
      type: 'NodeStarted',
      data: {
        node_id: 'node-b',
        node_type: 'llm',
        workflow_run_id: 'run-2',
      },
    },
    { activeWorkflowRunId: 'run-1' },
  );

  assert.deepEqual(stateCalls, []);
  assert.equal(result.activeWorkflowRunId, 'run-1');
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
        workflow_run_id: 'run-1',
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
      workflow_run_id: 'run-1',
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
        workflow_run_id: 'run-1',
      },
    },
    {
      activeWorkflowRunId: 'run-1',
      waitingForInput: true,
    },
  );

  assert.equal(result.activeWorkflowRunId, 'run-1');
  assert.equal(result.waitingForInput, false);
  assert.equal(result.handled, true);
  assert.equal(result.shouldCleanup, true);
});

test('applyWorkflowExecutionEvent marks incremental rerun tasks as running and clears waiting state', () => {
  const { result, stateCalls } = applyEvent(
    {
      type: 'IncrementalExecutionStarted',
      data: {
        task_ids: ['node-a', 'node-b'],
        workflow_run_id: 'run-1',
      },
    },
    {
      activeWorkflowRunId: 'run-1',
      waitingForInput: true,
    },
  );

  assert.deepEqual(stateCalls, [
    { nodeId: 'node-a', state: 'running', message: undefined },
    { nodeId: 'node-b', state: 'running', message: undefined },
  ]);
  assert.equal(result.activeWorkflowRunId, 'run-1');
  assert.equal(result.waitingForInput, false);
  assert.equal(result.handled, true);
  assert.equal(result.shouldCleanup, false);
});

test('applyWorkflowExecutionEvent replays graph-modified dirty tasks into idle state without clearing waiting input state', () => {
  const { result, stateCalls } = applyEvent(
    {
      type: 'GraphModified',
      data: {
        workflow_id: 'wf-1',
        workflow_run_id: 'run-1',
        dirty_tasks: ['node-a', 'node-b'],
      },
    },
    {
      activeWorkflowRunId: 'run-1',
      waitingForInput: true,
    },
  );

  assert.deepEqual(stateCalls, [
    { nodeId: 'node-a', state: 'idle', message: undefined },
    { nodeId: 'node-b', state: 'idle', message: undefined },
  ]);
  assert.equal(result.activeWorkflowRunId, 'run-1');
  assert.equal(result.waitingForInput, true);
  assert.equal(result.handled, true);
  assert.equal(result.shouldCleanup, false);
});

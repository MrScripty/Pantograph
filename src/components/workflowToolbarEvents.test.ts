import test from 'node:test';
import assert from 'node:assert/strict';
import type { Edge } from '@xyflow/svelte';

import { applyWorkflowToolbarEvent } from './workflowToolbarEvents.ts';
import type { NodeExecutionState, WorkflowEvent } from '../services/workflow/types.ts';

function createWorkflowActions() {
  const stateCalls: Array<{ nodeId: string; state: NodeExecutionState; message?: string }> = [];
  const runtimeDataCalls: Array<{ nodeId: string; data: Record<string, unknown> }> = [];
  const appendCalls: Array<{ nodeId: string; chunk: string }> = [];
  const replaceCalls: Array<{ nodeId: string; content: string }> = [];

  return {
    workflow: {
      setNodeExecutionState(nodeId: string, state: NodeExecutionState, message?: string) {
        stateCalls.push({ nodeId, state, message });
      },
      updateNodeRuntimeData(nodeId: string, data: Record<string, unknown>) {
        runtimeDataCalls.push({ nodeId, data });
      },
      appendStreamContent(nodeId: string, chunk: string) {
        appendCalls.push({ nodeId, chunk });
      },
      setStreamContent(nodeId: string, content: string) {
        replaceCalls.push({ nodeId, content });
      },
    },
    stateCalls,
    runtimeDataCalls,
    appendCalls,
    replaceCalls,
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
  const result = applyWorkflowToolbarEvent({
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

test('applyWorkflowToolbarEvent marks incremental rerun tasks as running and clears waiting state', () => {
  const { result, stateCalls } = applyEvent(
    {
      type: 'IncrementalExecutionStarted',
      data: {
        task_ids: ['node-a', 'node-b'],
        execution_id: 'run-1',
      },
    },
    {
      activeExecutionId: 'run-1',
      waitingForInput: true,
    },
  );

  assert.deepEqual(stateCalls, [
    { nodeId: 'node-a', state: 'running', message: undefined },
    { nodeId: 'node-b', state: 'running', message: undefined },
  ]);
  assert.equal(result.waitingForInput, false);
  assert.equal(result.handled, true);
});

test('applyWorkflowToolbarEvent replays graph-modified dirty tasks into idle state without clearing waiting input state', () => {
  const { result, stateCalls } = applyEvent(
    {
      type: 'GraphModified',
      data: {
        workflow_id: 'wf-1',
        execution_id: 'run-1',
        dirty_tasks: ['node-a', 'node-b'],
      },
    },
    {
      activeExecutionId: 'run-1',
      waitingForInput: true,
    },
  );

  assert.deepEqual(stateCalls, [
    { nodeId: 'node-a', state: 'idle', message: undefined },
    { nodeId: 'node-b', state: 'idle', message: undefined },
  ]);
  assert.equal(result.waitingForInput, true);
  assert.equal(result.handled, true);
});

test('applyWorkflowToolbarEvent marks waiting nodes and keeps waiting state true', () => {
  const { result, stateCalls } = applyEvent(
    {
      type: 'WaitingForInput',
      data: {
        node_id: 'input-node',
        message: 'Need user confirmation',
        execution_id: 'run-1',
      },
    },
    {
      activeExecutionId: 'run-1',
    },
  );

  assert.deepEqual(stateCalls, [
    {
      nodeId: 'input-node',
      state: 'waiting',
      message: 'Need user confirmation',
    },
  ]);
  assert.equal(result.activeExecutionId, 'run-1');
  assert.equal(result.waitingForInput, true);
  assert.equal(result.handled, true);
  assert.equal(result.shouldCleanup, false);
});

test('applyWorkflowToolbarEvent requests cleanup for cancelled runs', () => {
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

test('applyWorkflowToolbarEvent mirrors completed outputs into node and downstream runtime data', () => {
  const { runtimeDataCalls, stateCalls } = applyEvent(
    {
      type: 'NodeCompleted',
      data: {
        node_id: 'producer',
        outputs: {
          audio: 'base64-audio',
          text: 'hello',
        },
        execution_id: 'run-1',
      },
    },
    {
      edges: [
        {
          id: 'edge-audio',
          source: 'producer',
          sourceHandle: 'audio',
          target: 'audio-target',
          targetHandle: 'audio',
        } as Edge,
        {
          id: 'edge-text',
          source: 'producer',
          sourceHandle: 'text',
          target: 'text-target',
          targetHandle: 'prompt',
        } as Edge,
      ],
    },
  );

  assert.deepEqual(stateCalls, [
    { nodeId: 'producer', state: 'success', message: undefined },
  ]);
  assert.deepEqual(runtimeDataCalls, [
    {
      nodeId: 'producer',
      data: {
        audio: 'base64-audio',
        text: 'hello',
      },
    },
    {
      nodeId: 'audio-target',
      data: {
        audio: 'base64-audio',
      },
    },
    {
      nodeId: 'text-target',
      data: {
        prompt: 'hello',
      },
    },
  ]);
});

test('applyWorkflowToolbarEvent forwards text stream chunks to connected targets', () => {
  const { appendCalls, replaceCalls } = applyEvent(
    {
      type: 'NodeStream',
      data: {
        node_id: 'producer',
        port: 'text',
        chunk: {
          mode: 'replace',
          text: 'hello',
        },
        execution_id: 'run-1',
      },
    },
    {
      edges: [
        {
          id: 'edge-text',
          source: 'producer',
          sourceHandle: 'text',
          target: 'text-target',
          targetHandle: 'stream',
        } as Edge,
      ],
    },
  );

  assert.deepEqual(appendCalls, []);
  assert.deepEqual(replaceCalls, [{ nodeId: 'text-target', content: 'hello' }]);
});

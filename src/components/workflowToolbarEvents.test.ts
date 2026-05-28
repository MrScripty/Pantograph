import test from 'node:test';
import assert from 'node:assert/strict';
import type { Edge } from '@xyflow/svelte';

import {
  applyWorkflowToolbarEvent,
  isCurrentWorkflowSubmitFailure,
  isNumericWorkflowSemanticVersion,
  isWorkflowSemanticVersionConflictError,
  nextWorkflowPatchSemanticVersion,
  workflowSubmitDisabledReason,
  workflowValidationRefreshKey,
} from './workflowToolbarEvents.ts';
import type { NodeExecutionState, WorkflowEvent } from '../services/workflow/types.ts';

function createWorkflowActions() {
  const stateCalls: Array<{ nodeId: string; state: NodeExecutionState; message?: string }> = [];
  const runtimeDataCalls: Array<{ nodeId: string; data: Record<string, unknown> }> = [];
  const appendCalls: Array<{ nodeId: string; chunk: string; sequence?: number | null }> = [];
  const replaceCalls: Array<{ nodeId: string; content: string; sequence?: number | null }> = [];

  return {
    workflow: {
      setNodeExecutionState(nodeId: string, state: NodeExecutionState, message?: string) {
        stateCalls.push({ nodeId, state, message });
      },
      updateNodeRuntimeData(nodeId: string, data: Record<string, unknown>) {
        runtimeDataCalls.push({ nodeId, data });
      },
      appendStreamContent(nodeId: string, chunk: string, sequence?: number | null) {
        appendCalls.push({ nodeId, chunk, sequence });
      },
      setStreamContent(nodeId: string, content: string, sequence?: number | null) {
        replaceCalls.push({ nodeId, content, sequence });
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
    activeWorkflowRunId?: string | null;
    waitingForInput?: boolean;
    edges?: Edge[];
  },
) {
  const actions = createWorkflowActions();
  const result = applyWorkflowToolbarEvent({
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

test('applyWorkflowToolbarEvent marks incremental rerun tasks as running and clears waiting state', () => {
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
  assert.equal(result.waitingForInput, false);
  assert.equal(result.handled, true);
});

test('isCurrentWorkflowSubmitFailure rejects stale async submit failures', () => {
  assert.equal(
    isCurrentWorkflowSubmitFailure({
      submittedWorkflowId: 'workflow-a',
      currentGraphId: 'workflow-a',
      currentGraphType: 'workflow',
    }),
    true,
  );

  assert.equal(
    isCurrentWorkflowSubmitFailure({
      submittedWorkflowId: 'workflow-a',
      currentGraphId: 'workflow-b',
      currentGraphType: 'workflow',
    }),
    false,
  );

  assert.equal(
    isCurrentWorkflowSubmitFailure({
      submittedWorkflowId: 'workflow-a',
      currentGraphId: 'workflow-a',
      currentGraphType: 'node_group',
    }),
    false,
  );
});

test('isNumericWorkflowSemanticVersion accepts numeric major minor patch versions', () => {
  assert.equal(isNumericWorkflowSemanticVersion('0.1.0'), true);
  assert.equal(isNumericWorkflowSemanticVersion('12.3.405'), true);
  assert.equal(isNumericWorkflowSemanticVersion('0.1'), false);
  assert.equal(isNumericWorkflowSemanticVersion('0.1.0-beta'), false);
  assert.equal(isNumericWorkflowSemanticVersion('0.1.x'), false);
});

test('nextWorkflowPatchSemanticVersion increments valid patch versions', () => {
  assert.equal(nextWorkflowPatchSemanticVersion('0.1.0'), '0.1.1');
  assert.equal(nextWorkflowPatchSemanticVersion('2.4.9'), '2.4.10');
  assert.equal(nextWorkflowPatchSemanticVersion('bad'), '0.1.0');
});

test('workflowSubmitDisabledReason explains every disabled submit gate', () => {
  const enabled = {
    isExecuting: false,
    isReadOnly: false,
    isDirty: false,
    hasSavedWorkflow: true,
    hasWorkflowId: true,
    semanticVersionInvalid: false,
    submitGate: { allowed: true },
  };

  assert.equal(workflowSubmitDisabledReason(enabled), null);
  assert.equal(
    workflowSubmitDisabledReason({ ...enabled, isDirty: true }),
    'Save workflow changes before submitting',
  );
  assert.equal(
    workflowSubmitDisabledReason({ ...enabled, hasSavedWorkflow: false }),
    'Save the workflow before submitting',
  );
  assert.equal(
    workflowSubmitDisabledReason({ ...enabled, semanticVersionInvalid: true }),
    'Workflow version must use numeric major.minor.patch format',
  );
  assert.equal(
    workflowSubmitDisabledReason({ ...enabled, isReadOnly: true, isDirty: true }),
    'Cannot submit a read-only graph',
  );
  assert.equal(
    workflowSubmitDisabledReason({ ...enabled, submitGate: null }),
    'Workflow validation summary unavailable',
  );
  assert.equal(
    workflowSubmitDisabledReason({
      ...enabled,
      submitGate: {
        allowed: false,
        reason_code: 'validation_pending',
        message: 'Inference validation is still pending',
      },
    }),
    'Inference validation is still pending',
  );
});

test('workflowValidationRefreshKey allows dirty draft validation to stay separate from submit gating', () => {
  const refreshKey = workflowValidationRefreshKey({
    currentGraphType: 'workflow',
    graphSessionId: 'graph-session-1',
    graphRevision: 'graph-revision-1',
  });

  assert.equal(refreshKey, 'graph-session-1:graph-revision-1');
  assert.equal(
    workflowSubmitDisabledReason({
      isExecuting: false,
      isReadOnly: false,
      isDirty: true,
      hasSavedWorkflow: true,
      hasWorkflowId: true,
      semanticVersionInvalid: false,
      submitGate: { allowed: true },
    }),
    'Save workflow changes before submitting',
  );
});

test('workflowValidationRefreshKey rejects non-workflow or incomplete validation context', () => {
  assert.equal(
    workflowValidationRefreshKey({
      currentGraphType: 'system',
      graphSessionId: 'graph-session-1',
      graphRevision: 'graph-revision-1',
    }),
    null,
  );
  assert.equal(
    workflowValidationRefreshKey({
      currentGraphType: 'workflow',
      graphSessionId: null,
      graphRevision: 'graph-revision-1',
    }),
    null,
  );
  assert.equal(
    workflowValidationRefreshKey({
      currentGraphType: 'workflow',
      graphSessionId: 'graph-session-1',
      graphRevision: null,
    }),
    null,
  );
});

test('isWorkflowSemanticVersionConflictError detects attribution conflicts', () => {
  assert.equal(
    isWorkflowSemanticVersionConflictError({
      code: 'internal_error',
      message: 'workflow semantic version already points at a different execution fingerprint',
    }),
    true,
  );
  assert.equal(
    isWorkflowSemanticVersionConflictError({
      code: 'invalid_request',
      message: 'workflow semantic version already points at a different execution fingerprint',
    }),
    true,
  );
  assert.equal(
    isWorkflowSemanticVersionConflictError({
      code: 'internal_error',
      message: 'workflow semantic version is invalid',
    }),
    false,
  );
});

test('applyWorkflowToolbarEvent replays graph-modified dirty tasks into idle state without clearing waiting input state', () => {
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
        workflow_run_id: 'run-1',
      },
    },
    {
      activeWorkflowRunId: 'run-1',
    },
  );

  assert.deepEqual(stateCalls, [
    {
      nodeId: 'input-node',
      state: 'waiting',
      message: 'Need user confirmation',
    },
  ]);
  assert.equal(result.activeWorkflowRunId, 'run-1');
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
        workflow_run_id: 'run-1',
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
          sequence: 7,
        },
        workflow_run_id: 'run-1',
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
  assert.deepEqual(replaceCalls, [{ nodeId: 'text-target', content: 'hello', sequence: 7 }]);
});

test('applyWorkflowToolbarEvent treats response as the canonical text generation stream port', () => {
  const edge = {
    id: 'edge-response',
    source: 'llm',
    sourceHandle: 'response',
    target: 'text-output',
    targetHandle: 'text',
  } as Edge;
  const stream = applyEvent(
    {
      type: 'NodeStream',
      data: {
        node_id: 'llm',
        port: 'response',
        chunk: 'partial text',
        workflow_run_id: 'run-1',
      },
    },
    {
      edges: [edge],
    },
  );
  const completed = applyEvent(
    {
      type: 'NodeCompleted',
      data: {
        node_id: 'llm',
        outputs: {
          response: 'final text',
        },
        workflow_run_id: 'run-1',
      },
    },
    {
      edges: [edge],
    },
  );

  assert.deepEqual(stream.appendCalls, [
    { nodeId: 'text-output', chunk: 'partial text', sequence: null },
  ]);
  assert.deepEqual(stream.runtimeDataCalls, []);
  assert.deepEqual(completed.runtimeDataCalls, [
    {
      nodeId: 'llm',
      data: {
        response: 'final text',
      },
    },
    {
      nodeId: 'text-output',
      data: {
        text: 'final text',
      },
    },
  ]);
});

test('applyWorkflowToolbarEvent ignores stale text stream chunks from another active run', () => {
  const stream = applyEvent(
    {
      type: 'NodeStream',
      data: {
        node_id: 'llm',
        port: 'response',
        chunk: 'stale text',
        workflow_run_id: 'run-2',
      },
    },
    {
      activeWorkflowRunId: 'run-1',
      edges: [
        {
          id: 'edge-response',
          source: 'llm',
          sourceHandle: 'response',
          target: 'text-output',
          targetHandle: 'text',
        } as Edge,
      ],
    },
  );

  assert.deepEqual(stream.appendCalls, []);
  assert.deepEqual(stream.replaceCalls, []);
  assert.deepEqual(stream.runtimeDataCalls, []);
  assert.equal(stream.result.activeWorkflowRunId, 'run-1');
  assert.equal(stream.result.handled, false);
});

test('applyWorkflowToolbarEvent forwards audio stream references without inline base64', () => {
  const descriptor = {
    artifact_id: 'artifact-audio',
    lifecycle_state: 'retained',
  };
  const chunk = {
    artifact_id: 'artifact-audio',
    stream_handle: 'artifact-stream://artifact-audio',
    media_type: 'audio/ogg',
    sequence: 3,
    byte_length: 4096,
    available_byte_length: 2048,
    byte_range_start: 1024,
    byte_range_end_exclusive: 2048,
    lifecycle_state: 'streaming',
    is_final: false,
    descriptor,
  };

  const { runtimeDataCalls, appendCalls, replaceCalls } = applyEvent(
    {
      type: 'NodeStream',
      data: {
        node_id: 'producer',
        port: 'audio',
        chunk,
        workflow_run_id: 'run-1',
      },
    },
    {
      edges: [
        {
          id: 'edge-audio',
          source: 'producer',
          sourceHandle: 'audio',
          target: 'audio-target',
          targetHandle: 'stream',
        } as Edge,
      ],
    },
  );

  assert.deepEqual(appendCalls, []);
  assert.deepEqual(replaceCalls, []);
  assert.deepEqual(runtimeDataCalls, [
    {
      nodeId: 'audio-target',
      data: {
        stream: chunk,
        audio_mime: 'audio/ogg',
        stream_sequence: 3,
        stream_is_final: false,
        stream_artifact_id: 'artifact-audio',
        stream_handle: 'artifact-stream://artifact-audio',
        stream_byte_length: 4096,
        stream_available_byte_length: 2048,
        stream_byte_range_start: 1024,
        stream_byte_range_end_exclusive: 2048,
        stream_lifecycle_state: 'streaming',
        stream_descriptor: descriptor,
      },
    },
  ]);
});

test('applyWorkflowToolbarEvent keeps inline audio_base64 stream handling as fallback', () => {
  const chunk = {
    audio_base64: 'UklGRg==',
    mime_type: 'audio/wav',
    sequence: 1,
    is_final: true,
  };

  const { runtimeDataCalls } = applyEvent(
    {
      type: 'NodeStream',
      data: {
        node_id: 'producer',
        port: 'audio',
        chunk,
        workflow_run_id: 'run-1',
      },
    },
    {
      edges: [
        {
          id: 'edge-audio',
          source: 'producer',
          sourceHandle: 'audio',
          target: 'audio-target',
          targetHandle: 'stream',
        } as Edge,
      ],
    },
  );

  assert.deepEqual(runtimeDataCalls, [
    {
      nodeId: 'audio-target',
      data: {
        stream: chunk,
        audio_mime: 'audio/wav',
        stream_sequence: 1,
        stream_is_final: true,
        stream_artifact_id: null,
        stream_handle: null,
        stream_byte_length: null,
        stream_available_byte_length: null,
        stream_byte_range_start: null,
        stream_byte_range_end_exclusive: null,
        stream_lifecycle_state: null,
        stream_descriptor: undefined,
      },
    },
  ]);
});

test('applyWorkflowToolbarEvent reads audio stream metadata from a final descriptor', () => {
  const descriptor = {
    artifact_id: 'artifact-final-audio',
    stream_handle: 'artifact-stream://artifact-final-audio',
    byte_length: 8192,
    lifecycle_state: 'retained',
    format: {
      media_type: 'audio/flac',
    },
  };

  const { runtimeDataCalls } = applyEvent(
    {
      type: 'NodeStream',
      data: {
        node_id: 'producer',
        port: 'audio',
        chunk: {
          descriptor,
          is_final: true,
        },
        workflow_run_id: 'run-1',
      },
    },
    {
      edges: [
        {
          id: 'edge-audio',
          source: 'producer',
          sourceHandle: 'audio',
          target: 'audio-target',
          targetHandle: 'stream',
        } as Edge,
      ],
    },
  );

  assert.deepEqual(runtimeDataCalls, [
    {
      nodeId: 'audio-target',
      data: {
        stream: {
          descriptor,
          is_final: true,
        },
        audio_mime: 'audio/flac',
        stream_sequence: null,
        stream_is_final: true,
        stream_artifact_id: 'artifact-final-audio',
        stream_handle: 'artifact-stream://artifact-final-audio',
        stream_byte_length: 8192,
        stream_available_byte_length: null,
        stream_byte_range_start: null,
        stream_byte_range_end_exclusive: null,
        stream_lifecycle_state: 'retained',
        stream_descriptor: descriptor,
      },
    },
  ]);
});

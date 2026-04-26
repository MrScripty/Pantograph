import test from 'node:test';
import assert from 'node:assert/strict';

import type { Edge } from '@xyflow/svelte';

import {
  resolveWorkflowReconnectResultDecision,
  resolveWorkflowReconnectStartDecision,
} from './workflowGraphReconnect.ts';
import type { ConnectionIntentState } from './types/workflow.ts';

const edge: Edge = {
  id: 'edge-a',
  source: 'source',
  sourceHandle: 'out',
  target: 'target',
  targetHandle: 'in',
};

test('resolveWorkflowReconnectStartDecision starts editable reconnects', () => {
  assert.deepEqual(
    resolveWorkflowReconnectStartDecision({
      canEdit: true,
      edge,
      handleType: 'source',
    }),
    {
      sourceAnchor: { node_id: 'source', port_id: 'out' },
      type: 'start',
    },
  );
});

test('resolveWorkflowReconnectStartDecision ignores readonly reconnects', () => {
  assert.deepEqual(
    resolveWorkflowReconnectStartDecision({
      canEdit: false,
      edge,
      handleType: 'target',
    }),
    { type: 'ignore' },
  );
});

test('resolveWorkflowReconnectStartDecision clears incomplete reconnect anchors', () => {
  assert.deepEqual(
    resolveWorkflowReconnectStartDecision({
      canEdit: true,
      edge: { ...edge, sourceHandle: null },
      handleType: 'source',
    }),
    { type: 'clear' },
  );
});

test('resolveWorkflowReconnectResultDecision maps commit outcomes', () => {
  assert.deepEqual(
    resolveWorkflowReconnectResultDecision({ type: 'accepted' }),
    { type: 'clear' },
  );
  assert.deepEqual(
    resolveWorkflowReconnectResultDecision({ type: 'stale' }),
    { type: 'clear' },
  );

  const intent: ConnectionIntentState = {
    sourceAnchor: { node_id: 'source', port_id: 'out' },
    graphRevision: 'revision-a',
    compatibleNodeIds: [],
    compatibleTargetKeys: [],
    insertableNodeTypes: [],
    rejection: {
      reason: 'incompatible_types',
      message: 'Ports are not compatible',
    },
  };
  assert.deepEqual(
    resolveWorkflowReconnectResultDecision({ type: 'rejected', intent }),
    {
      intent,
      message: 'Ports are not compatible',
      type: 'set-intent',
    },
  );

  const error = new Error('connect failed');
  assert.deepEqual(
    resolveWorkflowReconnectResultDecision({ type: 'failed', error }),
    {
      error,
      type: 'log-failure',
    },
  );
});

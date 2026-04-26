import test from 'node:test';
import assert from 'node:assert/strict';

import {
  resolveWorkflowDeleteSelectionRequest,
  resolveWorkflowEdgeRemovalRequest,
} from './workflowGraphDeletion.ts';

test('resolveWorkflowDeleteSelectionRequest maps editable selection ids', () => {
  assert.deepEqual(
    resolveWorkflowDeleteSelectionRequest({
      canEdit: true,
      nodes: [
        { id: 'node-a', position: { x: 0, y: 0 }, data: {} },
        { id: 'node-b', position: { x: 0, y: 0 }, data: {} },
      ],
      edges: [
        { id: 'edge-a', source: 'node-a', target: 'node-b' },
      ],
    }),
    {
      nodeIds: ['node-a', 'node-b'],
      edgeIds: ['edge-a'],
    },
  );
});

test('resolveWorkflowDeleteSelectionRequest ignores readonly deletes', () => {
  assert.equal(
    resolveWorkflowDeleteSelectionRequest({
      canEdit: false,
      nodes: [{ id: 'node-a', position: { x: 0, y: 0 }, data: {} }],
      edges: [],
    }),
    null,
  );
});

test('resolveWorkflowEdgeRemovalRequest requires session and edge ids', () => {
  assert.equal(
    resolveWorkflowEdgeRemovalRequest({ edgeIds: ['edge-a'], sessionId: null }),
    null,
  );
  assert.equal(
    resolveWorkflowEdgeRemovalRequest({ edgeIds: [], sessionId: 'session-a' }),
    null,
  );
  assert.deepEqual(
    resolveWorkflowEdgeRemovalRequest({
      edgeIds: ['edge-a'],
      sessionId: 'session-a',
    }),
    {
      edgeIds: ['edge-a'],
      sessionId: 'session-a',
    },
  );
});

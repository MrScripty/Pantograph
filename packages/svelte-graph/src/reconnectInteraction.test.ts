import test from 'node:test';
import assert from 'node:assert/strict';

import {
  RECONNECT_ANCHOR_INSET_PX,
  insetReconnectAnchorPosition,
  resolveReconnectSourceAnchor,
} from './reconnectInteraction.ts';

test('resolveReconnectSourceAnchor supports dragging from either edge endpoint', () => {
  const edge = {
    source: 'llm-node',
    sourceHandle: 'text',
  };

  assert.deepEqual(resolveReconnectSourceAnchor(edge, 'source'), {
    node_id: 'llm-node',
    port_id: 'text',
  });
  assert.deepEqual(resolveReconnectSourceAnchor(edge, 'target'), {
    node_id: 'llm-node',
    port_id: 'text',
  });
});

test('resolveReconnectSourceAnchor returns null when the edge has no source handle', () => {
  assert.equal(
    resolveReconnectSourceAnchor(
      {
        source: 'llm-node',
        sourceHandle: null,
      },
      'source',
    ),
    null,
  );
});

test('insetReconnectAnchorPosition offsets source and target anchors inward', () => {
  const edge = {
    source: 'src',
    sourceHandle: 'out',
    sourceX: 10,
    sourceY: 20,
    targetX: 110,
    targetY: 20,
  };

  assert.deepEqual(insetReconnectAnchorPosition(edge, 'source'), {
    x: 10 + RECONNECT_ANCHOR_INSET_PX,
    y: 20,
  });
  assert.deepEqual(insetReconnectAnchorPosition(edge, 'target'), {
    x: 110 - RECONNECT_ANCHOR_INSET_PX,
    y: 20,
  });
});

test('insetReconnectAnchorPosition clamps the inset for short edges', () => {
  const edge = {
    source: 'src',
    sourceHandle: 'out',
    sourceX: 0,
    sourceY: 0,
    targetX: 10,
    targetY: 0,
  };

  assert.deepEqual(insetReconnectAnchorPosition(edge, 'source', 20), {
    x: 5,
    y: 0,
  });
  assert.deepEqual(insetReconnectAnchorPosition(edge, 'target', 20), {
    x: 5,
    y: 0,
  });
});

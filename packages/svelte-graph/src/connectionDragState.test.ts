import test from 'node:test';
import assert from 'node:assert/strict';

import {
  clearConnectionDragState,
  createConnectionDragState,
  markConnectionDragFinalizing,
  shouldRemoveReconnectedEdge,
  startConnectionDrag,
  startReconnectDrag,
  supportsInsertFromConnectionDrag,
} from './connectionDragState.ts';

test('startConnectionDrag creates a non-finalizing connect mode state', () => {
  assert.deepEqual(startConnectionDrag(), {
    mode: 'connect',
    reconnectingEdgeId: null,
    reconnectingSourceAnchor: null,
    finalizing: false,
  });
});

test('startReconnectDrag records reconnect ownership', () => {
  const state = startReconnectDrag('edge-1', {
    node_id: 'source-node',
    port_id: 'audio_stream',
  });

  assert.equal(state.mode, 'reconnect');
  assert.equal(state.reconnectingEdgeId, 'edge-1');
  assert.deepEqual(state.reconnectingSourceAnchor, {
    node_id: 'source-node',
    port_id: 'audio_stream',
  });
});

test('supportsInsertFromConnectionDrag blocks reconnect mode', () => {
  assert.equal(supportsInsertFromConnectionDrag(startConnectionDrag()), true);
  assert.equal(
    supportsInsertFromConnectionDrag(
      startReconnectDrag('edge-1', {
        node_id: 'source-node',
        port_id: 'audio_stream',
      }),
    ),
    false,
  );
});

test('shouldRemoveReconnectedEdge only removes unfinished invalid reconnects', () => {
  const reconnectState = startReconnectDrag('edge-1', {
    node_id: 'source-node',
    port_id: 'audio_stream',
  });

  assert.equal(
    shouldRemoveReconnectedEdge(reconnectState, {
      isValid: false,
    }),
    'edge-1',
  );

  assert.equal(
    shouldRemoveReconnectedEdge(markConnectionDragFinalizing(reconnectState), {
      isValid: false,
    }),
    null,
  );

  assert.equal(
    shouldRemoveReconnectedEdge(reconnectState, {
      isValid: true,
    }),
    null,
  );
});

test('clearConnectionDragState resets to idle', () => {
  assert.deepEqual(clearConnectionDragState(), createConnectionDragState());
});

import test from 'node:test';
import assert from 'node:assert/strict';

import {
  createConnectionDragState,
  startConnectionDrag,
  startReconnectDrag,
} from './connectionDragState.ts';
import {
  createHorseshoeDragSessionState,
  startHorseshoeDrag,
} from './horseshoeDragSession.ts';
import { buildWorkflowHorseshoeOpenContext } from './workflowHorseshoeOpenContext.ts';

test('buildWorkflowHorseshoeOpenContext projects connect drags into supported open context', () => {
  assert.deepEqual(
    buildWorkflowHorseshoeOpenContext({
      canEdit: true,
      session: startHorseshoeDrag({ x: 10, y: 20 }),
      connectionDragState: startConnectionDrag(),
      hasConnectionIntent: true,
      insertableCount: 2,
    }),
    {
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: true,
      insertableCount: 2,
      anchorPosition: { x: 10, y: 20 },
    },
  );
});

test('buildWorkflowHorseshoeOpenContext marks reconnect drags as unsupported for insert', () => {
  assert.deepEqual(
    buildWorkflowHorseshoeOpenContext({
      canEdit: true,
      session: startHorseshoeDrag({ x: 10, y: 20 }),
      connectionDragState: startReconnectDrag('edge-a', {
        node_id: 'source-a',
        port_id: 'out',
      }),
      hasConnectionIntent: true,
      insertableCount: 2,
    }).supportsInsert,
    false,
  );
});

test('buildWorkflowHorseshoeOpenContext preserves idle context without intent or anchor', () => {
  assert.deepEqual(
    buildWorkflowHorseshoeOpenContext({
      canEdit: false,
      session: createHorseshoeDragSessionState(),
      connectionDragState: createConnectionDragState(),
      hasConnectionIntent: false,
      insertableCount: 0,
    }),
    {
      canEdit: false,
      connectionDragActive: false,
      supportsInsert: true,
      hasConnectionIntent: false,
      insertableCount: 0,
      anchorPosition: null,
    },
  );
});

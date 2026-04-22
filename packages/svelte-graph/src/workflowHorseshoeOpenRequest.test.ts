import test from 'node:test';
import assert from 'node:assert/strict';

import { startConnectionDrag, startReconnectDrag } from './connectionDragState.ts';
import { startHorseshoeDrag } from './horseshoeDragSession.ts';
import { requestWorkflowHorseshoeOpen } from './workflowHorseshoeOpenRequest.ts';

const openContext = {
  canEdit: true,
  connectionDragActive: true,
  supportsInsert: true,
  hasConnectionIntent: true,
  insertableCount: 2,
  anchorPosition: { x: 10, y: 20 },
};

test('requestWorkflowHorseshoeOpen returns trace and open session for ready connect drags', () => {
  const result = requestWorkflowHorseshoeOpen({
    session: startHorseshoeDrag({ x: 10, y: 20 }),
    connectionDragState: startConnectionDrag(),
    openContext,
  });

  assert.equal(result.trace, 'request-open:drag:connect:intent:2-insertables:anchor');
  assert.equal(result.session.displayState, 'open');
  assert.equal(result.session.openRequested, false);
  assert.equal(result.session.blockedReason, null);
});

test('requestWorkflowHorseshoeOpen records reconnect mode and blocked session state', () => {
  const result = requestWorkflowHorseshoeOpen({
    session: startHorseshoeDrag({ x: 10, y: 20 }),
    connectionDragState: startReconnectDrag('edge-a', {
      node_id: 'source-a',
      port_id: 'out',
    }),
    openContext: {
      ...openContext,
      supportsInsert: false,
    },
  });

  assert.equal(result.trace, 'request-open:drag:reconnect:intent:2-insertables:anchor');
  assert.equal(result.session.displayState, 'blocked');
  assert.equal(result.session.blockedReason, 'insert_not_supported');
});

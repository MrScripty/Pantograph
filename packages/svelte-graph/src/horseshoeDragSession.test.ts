import test from 'node:test';
import assert from 'node:assert/strict';

import {
  clearHorseshoeDragSession,
  createHorseshoeDragSessionState,
  requestHorseshoeDisplay,
  startHorseshoeDrag,
  syncHorseshoeDisplay,
  updateHorseshoeAnchor,
} from './horseshoeDragSession.ts';

test('requestHorseshoeDisplay enters pending while candidates are loading', () => {
  const state = requestHorseshoeDisplay(
    startHorseshoeDrag({ x: 10, y: 20 }),
    {
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: false,
      insertableCount: 0,
      anchorPosition: { x: 10, y: 20 },
    },
  );

  assert.equal(state.displayState, 'pending');
  assert.equal(state.blockedReason, 'candidates_pending');
  assert.equal(state.openRequested, true);
});

test('syncHorseshoeDisplay opens once candidates arrive', () => {
  const pendingState = requestHorseshoeDisplay(
    startHorseshoeDrag({ x: 10, y: 20 }),
    {
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: false,
      insertableCount: 0,
      anchorPosition: { x: 10, y: 20 },
    },
  );

  const opened = syncHorseshoeDisplay(pendingState, {
    canEdit: true,
    connectionDragActive: true,
    supportsInsert: true,
    hasConnectionIntent: true,
    insertableCount: 3,
    anchorPosition: { x: 10, y: 20 },
  });

  assert.equal(opened.displayState, 'open');
  assert.equal(opened.blockedReason, null);
  assert.equal(opened.openRequested, false);
});

test('missing anchor keeps request alive until anchor is restored', () => {
  const blocked = requestHorseshoeDisplay(
    startHorseshoeDrag(null),
    {
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: true,
      insertableCount: 2,
      anchorPosition: null,
    },
  );

  assert.equal(blocked.displayState, 'blocked');
  assert.equal(blocked.blockedReason, 'missing_anchor_position');
  assert.equal(blocked.openRequested, true);

  const reopened = syncHorseshoeDisplay(
    updateHorseshoeAnchor(blocked, { x: 40, y: 80 }),
    {
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: true,
      insertableCount: 2,
      anchorPosition: { x: 40, y: 80 },
    },
  );

  assert.equal(reopened.displayState, 'open');
  assert.equal(reopened.openRequested, false);
});

test('no insertable nodes blocks and stops retrying', () => {
  const state = requestHorseshoeDisplay(
    startHorseshoeDrag({ x: 10, y: 20 }),
    {
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: true,
      insertableCount: 0,
      anchorPosition: { x: 10, y: 20 },
    },
  );

  assert.equal(state.displayState, 'blocked');
  assert.equal(state.blockedReason, 'no_insertable_nodes');
  assert.equal(state.openRequested, false);
});

test('clearHorseshoeDragSession resets to idle state', () => {
  assert.deepEqual(clearHorseshoeDragSession(), createHorseshoeDragSessionState());
});

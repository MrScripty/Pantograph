import test from 'node:test';
import assert from 'node:assert/strict';

import {
  formatHorseshoeBlockedReason,
  isSpaceKey,
  resolveHorseshoeOpenRequest,
  resolveHorseshoeSpaceKeyAction,
  shouldUpdateHorseshoeAnchorFromPointer,
} from './horseshoeInvocation.ts';

test('isSpaceKey accepts runtime space variants', () => {
  assert.equal(isSpaceKey({ code: 'Space', key: 'x' }), true);
  assert.equal(isSpaceKey({ code: 'KeyS', key: ' ' }), true);
  assert.equal(isSpaceKey({ code: 'KeyS', key: 'Space' }), true);
  assert.equal(isSpaceKey({ code: 'KeyS', key: 'Spacebar' }), true);
  assert.equal(isSpaceKey({ code: 'KeyS', key: 'Enter' }), false);
});

test('resolveHorseshoeOpenRequest queues while candidates are still loading', () => {
  assert.deepEqual(
    resolveHorseshoeOpenRequest({
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: false,
      insertableCount: 0,
      anchorPosition: { x: 10, y: 20 },
    }),
    {
      action: 'queue',
      reason: 'candidates_pending',
    },
  );
});

test('resolveHorseshoeOpenRequest blocks for explicit reasons', () => {
  assert.deepEqual(
    resolveHorseshoeOpenRequest({
      canEdit: false,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: true,
      insertableCount: 1,
      anchorPosition: { x: 10, y: 20 },
    }),
    {
      action: 'blocked',
      reason: 'not_editable',
    },
  );

  assert.deepEqual(
    resolveHorseshoeOpenRequest({
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: false,
      hasConnectionIntent: true,
      insertableCount: 1,
      anchorPosition: { x: 10, y: 20 },
    }),
    {
      action: 'blocked',
      reason: 'insert_not_supported',
    },
  );

  assert.deepEqual(
    resolveHorseshoeOpenRequest({
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: true,
      insertableCount: 0,
      anchorPosition: { x: 10, y: 20 },
    }),
    {
      action: 'blocked',
      reason: 'no_insertable_nodes',
    },
  );

  assert.deepEqual(
    resolveHorseshoeOpenRequest({
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: true,
      insertableCount: 2,
      anchorPosition: null,
    }),
    {
      action: 'blocked',
      reason: 'missing_anchor_position',
    },
  );
});

test('resolveHorseshoeOpenRequest opens when drag state is ready', () => {
  assert.deepEqual(
    resolveHorseshoeOpenRequest({
      canEdit: true,
      connectionDragActive: true,
      supportsInsert: true,
      hasConnectionIntent: true,
      insertableCount: 3,
      anchorPosition: { x: 10, y: 20 },
    }),
    {
      action: 'open',
      reason: null,
    },
  );
});

test('resolveHorseshoeSpaceKeyAction opens first and confirms second', () => {
  assert.equal(
    resolveHorseshoeSpaceKeyAction({
      displayState: 'hidden',
      dragActive: true,
      pending: false,
      hasSelection: false,
    }),
    'open',
  );

  assert.equal(
    resolveHorseshoeSpaceKeyAction({
      displayState: 'open',
      dragActive: true,
      pending: false,
      hasSelection: true,
    }),
    'confirm',
  );
});

test('resolveHorseshoeSpaceKeyAction ignores unavailable confirmations', () => {
  assert.equal(
    resolveHorseshoeSpaceKeyAction({
      displayState: 'open',
      dragActive: true,
      pending: true,
      hasSelection: true,
    }),
    'noop',
  );

  assert.equal(
    resolveHorseshoeSpaceKeyAction({
      displayState: 'hidden',
      dragActive: false,
      pending: false,
      hasSelection: false,
    }),
    'noop',
  );
});

test('shouldUpdateHorseshoeAnchorFromPointer freezes the menu once open', () => {
  assert.equal(shouldUpdateHorseshoeAnchorFromPointer('hidden'), true);
  assert.equal(shouldUpdateHorseshoeAnchorFromPointer('pending'), true);
  assert.equal(shouldUpdateHorseshoeAnchorFromPointer('blocked'), true);
  assert.equal(shouldUpdateHorseshoeAnchorFromPointer('open'), false);
});

test('formatHorseshoeBlockedReason returns actionable diagnostics', () => {
  assert.match(
    formatHorseshoeBlockedReason('candidates_pending'),
    /still loading/i,
  );
  assert.match(
    formatHorseshoeBlockedReason('insert_not_supported'),
    /output handle/i,
  );
});

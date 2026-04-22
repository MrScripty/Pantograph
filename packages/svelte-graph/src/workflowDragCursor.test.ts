import test from 'node:test';
import assert from 'node:assert/strict';

import { createHorseshoeDragSessionState, startHorseshoeDrag } from './horseshoeDragSession.ts';
import { resolveWorkflowDragCursorUpdate } from './workflowDragCursor.ts';

test('resolveWorkflowDragCursorUpdate updates the anchor while the menu is hidden', () => {
  assert.deepEqual(
    resolveWorkflowDragCursorUpdate({
      pointerPosition: { x: 40, y: 60 },
      session: startHorseshoeDrag({ x: 10, y: 20 }),
      insertableNodeTypes: [],
      selectedIndex: 0,
    }),
    {
      type: 'update-anchor',
      session: {
        dragActive: true,
        openRequested: false,
        displayState: 'hidden',
        blockedReason: null,
        anchorPosition: { x: 40, y: 60 },
      },
    },
  );
});

test('resolveWorkflowDragCursorUpdate selects the nearest visible item while the menu is open', () => {
  assert.deepEqual(
    resolveWorkflowDragCursorUpdate({
      pointerPosition: { x: 0, y: -126 },
      session: {
        ...startHorseshoeDrag({ x: 0, y: 0 }),
        displayState: 'open',
      },
      insertableNodeTypes: ['candidate-a'],
      selectedIndex: 0,
    }),
    {
      type: 'select-index',
      selectedIndex: 0,
    },
  );
});

test('resolveWorkflowDragCursorUpdate no-ops without a pointer position', () => {
  assert.deepEqual(
    resolveWorkflowDragCursorUpdate({
      pointerPosition: null,
      session: startHorseshoeDrag({ x: 10, y: 20 }),
      insertableNodeTypes: [],
      selectedIndex: 0,
    }),
    {
      type: 'noop',
    },
  );
});

test('resolveWorkflowDragCursorUpdate no-ops for open menus without an anchor', () => {
  assert.deepEqual(
    resolveWorkflowDragCursorUpdate({
      pointerPosition: { x: 0, y: -126 },
      session: {
        ...createHorseshoeDragSessionState(),
        displayState: 'open',
      },
      insertableNodeTypes: ['candidate-a'],
      selectedIndex: 0,
    }),
    {
      type: 'noop',
    },
  );
});

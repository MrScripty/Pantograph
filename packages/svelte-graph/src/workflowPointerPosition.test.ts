import test from 'node:test';
import assert from 'node:assert/strict';

import {
  resolveWorkflowPointerClientPosition,
  resolveWorkflowRelativePointerPosition,
} from './workflowPointerPosition.ts';

test('resolveWorkflowPointerClientPosition reads mouse client coordinates', () => {
  assert.deepEqual(
    resolveWorkflowPointerClientPosition({
      clientX: 120,
      clientY: 80,
    }),
    {
      clientX: 120,
      clientY: 80,
    },
  );
});

test('resolveWorkflowPointerClientPosition prefers active touch coordinates', () => {
  assert.deepEqual(
    resolveWorkflowPointerClientPosition({
      touches: [{ clientX: 30, clientY: 40 }],
      changedTouches: [{ clientX: 50, clientY: 60 }],
    }),
    {
      clientX: 30,
      clientY: 40,
    },
  );
});

test('resolveWorkflowPointerClientPosition falls back to changed touch coordinates', () => {
  assert.deepEqual(
    resolveWorkflowPointerClientPosition({
      touches: [],
      changedTouches: [{ clientX: 50, clientY: 60 }],
    }),
    {
      clientX: 50,
      clientY: 60,
    },
  );
});

test('resolveWorkflowPointerClientPosition returns null without touch coordinates', () => {
  assert.equal(
    resolveWorkflowPointerClientPosition({
      touches: [],
      changedTouches: [],
    }),
    null,
  );
});

test('resolveWorkflowRelativePointerPosition projects client coordinates through bounds', () => {
  assert.deepEqual(
    resolveWorkflowRelativePointerPosition({
      clientPosition: { clientX: 120, clientY: 80 },
      containerBounds: { left: 20, top: 30 },
    }),
    {
      x: 100,
      y: 50,
    },
  );
});

test('resolveWorkflowRelativePointerPosition returns null without coordinates or bounds', () => {
  assert.equal(
    resolveWorkflowRelativePointerPosition({
      clientPosition: null,
      containerBounds: { left: 20, top: 30 },
    }),
    null,
  );
  assert.equal(
    resolveWorkflowRelativePointerPosition({
      clientPosition: { clientX: 120, clientY: 80 },
      containerBounds: null,
    }),
    null,
  );
});

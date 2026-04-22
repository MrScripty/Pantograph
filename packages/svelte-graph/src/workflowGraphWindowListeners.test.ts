import test from 'node:test';
import assert from 'node:assert/strict';

import {
  WORKFLOW_PALETTE_DRAG_END_EVENT,
  WORKFLOW_PALETTE_DRAG_START_EVENT,
} from './paletteDragState.ts';
import {
  registerWorkflowGraphWindowListeners,
  type WorkflowGraphWindowListenerTarget,
} from './workflowGraphWindowListeners.ts';

interface ListenerRecord {
  listener: EventListenerOrEventListenerObject;
  options?: boolean | AddEventListenerOptions | EventListenerOptions;
  type: string;
}

function createListenerTarget() {
  const added: ListenerRecord[] = [];
  const removed: ListenerRecord[] = [];
  const target: WorkflowGraphWindowListenerTarget = {
    addEventListener(type, listener, options) {
      added.push({ type, listener, options });
    },
    removeEventListener(type, listener, options) {
      removed.push({ type, listener, options });
    },
  };

  return { added, removed, target };
}

test('registerWorkflowGraphWindowListeners registers and removes shared graph window listeners', () => {
  const keyDown = () => {};
  const paletteStart = () => {};
  const paletteEnd = () => {};
  const { added, removed, target } = createListenerTarget();

  const removeListeners = registerWorkflowGraphWindowListeners(target, {
    onKeyDown: keyDown,
    onPaletteDragEnd: paletteEnd,
    onPaletteDragStart: paletteStart,
  });

  assert.deepEqual(
    added.map(({ type, options }) => ({ type, options })),
    [
      { type: 'keydown', options: true },
      { type: WORKFLOW_PALETTE_DRAG_START_EVENT, options: undefined },
      { type: WORKFLOW_PALETTE_DRAG_END_EVENT, options: undefined },
      { type: 'blur', options: undefined },
    ],
  );

  removeListeners();

  assert.deepEqual(
    removed.map(({ type, options }) => ({ type, options })),
    added.map(({ type, options }) => ({ type, options })),
  );
  assert.deepEqual(
    removed.map(({ listener }) => listener),
    added.map(({ listener }) => listener),
  );
});

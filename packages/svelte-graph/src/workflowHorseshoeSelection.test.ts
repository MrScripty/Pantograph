import test from 'node:test';
import assert from 'node:assert/strict';

import { startHorseshoeDrag } from './horseshoeDragSession.ts';
import {
  createHorseshoeInsertFeedbackState,
  startHorseshoeInsertFeedback,
} from './horseshoeInsertFeedback.ts';
import {
  normalizeWorkflowHorseshoeSelectedIndex,
  resolveWorkflowHorseshoeSelectionSnapshot,
} from './workflowHorseshoeSelection.ts';

const items = [
  { node_type: 'a' },
  { node_type: 'b' },
];

test('resolveWorkflowHorseshoeSelectionSnapshot returns selected candidate and keyboard context', () => {
  assert.deepEqual(
    resolveWorkflowHorseshoeSelectionSnapshot({
      session: {
        ...startHorseshoeDrag({ x: 1, y: 2 }),
        displayState: 'open',
      },
      feedback: createHorseshoeInsertFeedbackState(),
      items,
      selectedIndex: 1,
    }),
    {
      keyboardContext: {
        displayState: 'open',
        dragActive: true,
        pending: false,
        hasSelection: true,
      },
      selectedCandidate: { node_type: 'b' },
    },
  );
});

test('resolveWorkflowHorseshoeSelectionSnapshot reports no selection outside item bounds', () => {
  assert.deepEqual(
    resolveWorkflowHorseshoeSelectionSnapshot({
      session: {
        ...startHorseshoeDrag(null),
        displayState: 'pending',
      },
      feedback: startHorseshoeInsertFeedback(),
      items,
      selectedIndex: 5,
    }),
    {
      keyboardContext: {
        displayState: 'pending',
        dragActive: true,
        pending: true,
        hasSelection: false,
      },
      selectedCandidate: null,
    },
  );
});

test('resolveWorkflowHorseshoeSelectionSnapshot accepts missing item lists', () => {
  assert.equal(
    resolveWorkflowHorseshoeSelectionSnapshot({
      session: startHorseshoeDrag(null),
      feedback: createHorseshoeInsertFeedbackState(),
      items: undefined,
      selectedIndex: 0,
    }).selectedCandidate,
    null,
  );
});

test('normalizeWorkflowHorseshoeSelectedIndex clamps selected index to available items', () => {
  assert.equal(
    normalizeWorkflowHorseshoeSelectedIndex({
      selectedIndex: -1,
      itemCount: 2,
    }),
    0,
  );
  assert.equal(
    normalizeWorkflowHorseshoeSelectedIndex({
      selectedIndex: 5,
      itemCount: 2,
    }),
    1,
  );
  assert.equal(
    normalizeWorkflowHorseshoeSelectedIndex({
      selectedIndex: 5,
      itemCount: 0,
    }),
    0,
  );
});

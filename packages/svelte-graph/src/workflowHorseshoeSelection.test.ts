import test from 'node:test';
import assert from 'node:assert/strict';

import { startHorseshoeDrag } from './horseshoeDragSession.ts';
import {
  createHorseshoeInsertFeedbackState,
  startHorseshoeInsertFeedback,
} from './horseshoeInsertFeedback.ts';
import { resolveWorkflowHorseshoeSelectionSnapshot } from './workflowHorseshoeSelection.ts';

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

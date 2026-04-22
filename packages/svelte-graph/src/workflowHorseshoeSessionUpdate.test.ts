import test from 'node:test';
import assert from 'node:assert/strict';

import { createHorseshoeInsertFeedbackState, startHorseshoeInsertFeedback } from './horseshoeInsertFeedback.ts';
import {
  clearHorseshoeDragSession,
  createHorseshoeDragSessionState,
  startHorseshoeDrag,
  type HorseshoeDragSessionState,
} from './horseshoeDragSession.ts';
import { resolveWorkflowHorseshoeSessionUpdate } from './workflowHorseshoeSessionUpdate.ts';

function viewState(session: HorseshoeDragSessionState) {
  return {
    session,
    feedback: createHorseshoeInsertFeedbackState(),
    selectedIndex: 3,
    query: 'mo',
  };
}

test('resolveWorkflowHorseshoeSessionUpdate no-ops when the session object is unchanged', () => {
  const current = viewState(createHorseshoeDragSessionState());

  const update = resolveWorkflowHorseshoeSessionUpdate({
    current,
    nextSession: current.session,
  });

  assert.equal(update.changed, false);
  assert.equal(update.state, current);
  assert.equal(update.clearQueryResetTimer, false);
});

test('resolveWorkflowHorseshoeSessionUpdate resets query selection when the menu opens', () => {
  const update = resolveWorkflowHorseshoeSessionUpdate({
    current: viewState(startHorseshoeDrag({ x: 10, y: 20 })),
    nextSession: {
      ...startHorseshoeDrag({ x: 10, y: 20 }),
      displayState: 'open',
    },
  });

  assert.equal(update.changed, true);
  assert.equal(update.state.query, '');
  assert.equal(update.state.selectedIndex, 0);
  assert.equal(update.state.feedback.pending, false);
  assert.equal(update.clearQueryResetTimer, false);
  assert.equal(update.trace, 'session:open:idle:clear:anchor');
});

test('resolveWorkflowHorseshoeSessionUpdate clears feedback query and timer on hidden sessions', () => {
  const update = resolveWorkflowHorseshoeSessionUpdate({
    current: {
      ...viewState({
        ...startHorseshoeDrag({ x: 10, y: 20 }),
        displayState: 'open',
      }),
      feedback: startHorseshoeInsertFeedback(),
    },
    nextSession: clearHorseshoeDragSession(),
  });

  assert.equal(update.changed, true);
  assert.deepEqual(update.state.feedback, createHorseshoeInsertFeedbackState());
  assert.equal(update.state.query, '');
  assert.equal(update.state.selectedIndex, 0);
  assert.equal(update.clearQueryResetTimer, true);
  assert.equal(update.trace, 'session:hidden:idle:clear:no-anchor');
});

import test from 'node:test';
import assert from 'node:assert/strict';

import { createConnectionDragState } from './connectionDragState.ts';
import { createHorseshoeDragSessionState, startHorseshoeDrag } from './horseshoeDragSession.ts';
import { createHorseshoeInsertFeedbackState, startHorseshoeInsertFeedback } from './horseshoeInsertFeedback.ts';
import {
  clearWorkflowConnectionDragInteraction,
  shouldClearWorkflowConnectionInteractionAfterConnectEnd,
} from './workflowConnectionInteraction.ts';

test('clearWorkflowConnectionDragInteraction resets drag session and feedback state', () => {
  assert.deepEqual(clearWorkflowConnectionDragInteraction(), {
    connectionDragState: createConnectionDragState(),
    horseshoeSession: createHorseshoeDragSessionState(),
    feedback: createHorseshoeInsertFeedbackState(),
  });
});

test('shouldClearWorkflowConnectionInteractionAfterConnectEnd clears idle interactions', () => {
  assert.equal(
    shouldClearWorkflowConnectionInteractionAfterConnectEnd({
      session: createHorseshoeDragSessionState(),
      feedback: createHorseshoeInsertFeedbackState(),
    }),
    true,
  );
});

test('shouldClearWorkflowConnectionInteractionAfterConnectEnd preserves active horseshoe work', () => {
  assert.equal(
    shouldClearWorkflowConnectionInteractionAfterConnectEnd({
      session: {
        ...startHorseshoeDrag({ x: 10, y: 20 }),
        displayState: 'open',
      },
      feedback: createHorseshoeInsertFeedbackState(),
    }),
    false,
  );

  assert.equal(
    shouldClearWorkflowConnectionInteractionAfterConnectEnd({
      session: createHorseshoeDragSessionState(),
      feedback: startHorseshoeInsertFeedback(),
    }),
    false,
  );

  assert.equal(
    shouldClearWorkflowConnectionInteractionAfterConnectEnd({
      session: {
        ...startHorseshoeDrag({ x: 10, y: 20 }),
        openRequested: true,
        displayState: 'pending',
        blockedReason: 'candidates_pending',
      },
      feedback: createHorseshoeInsertFeedbackState(),
    }),
    false,
  );
});

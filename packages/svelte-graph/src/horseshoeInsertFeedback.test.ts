import test from 'node:test';
import assert from 'node:assert/strict';

import {
  clearHorseshoeInsertFeedback,
  createHorseshoeInsertFeedbackState,
  rejectHorseshoeInsertFeedback,
  resolveHorseshoeStatusLabel,
  startHorseshoeInsertFeedback,
} from './horseshoeInsertFeedback.ts';

test('startHorseshoeInsertFeedback enters pending and clears old rejection state', () => {
  assert.deepEqual(startHorseshoeInsertFeedback(), {
    pending: true,
    rejectionMessage: null,
  });
});

test('rejectHorseshoeInsertFeedback preserves a visible rejection message', () => {
  assert.deepEqual(
    rejectHorseshoeInsertFeedback({ message: 'graph revision is stale' }),
    {
      pending: false,
      rejectionMessage: 'graph revision is stale',
    },
  );
});

test('rejectHorseshoeInsertFeedback falls back to a retryable message', () => {
  assert.deepEqual(rejectHorseshoeInsertFeedback(), {
    pending: false,
    rejectionMessage: 'Insert failed. Try again.',
  });
});

test('resolveHorseshoeStatusLabel prioritizes pending then rejection feedback', () => {
  assert.equal(
    resolveHorseshoeStatusLabel({
      pending: true,
      rejectionMessage: null,
      displayState: 'open',
      blockedReason: null,
    }),
    'Inserting node...',
  );

  assert.equal(
    resolveHorseshoeStatusLabel({
      pending: false,
      rejectionMessage: 'graph revision is stale',
      displayState: 'open',
      blockedReason: null,
    }),
    'graph revision is stale',
  );
});

test('resolveHorseshoeStatusLabel falls back to display-state messaging', () => {
  assert.equal(
    resolveHorseshoeStatusLabel({
      pending: false,
      rejectionMessage: null,
      displayState: 'pending',
      blockedReason: null,
    }),
    'Loading compatible nodes...',
  );

  assert.match(
    resolveHorseshoeStatusLabel({
      pending: false,
      rejectionMessage: null,
      displayState: 'blocked',
      blockedReason: 'insert_not_supported',
    }) ?? '',
    /output handle/i,
  );
});

test('clearHorseshoeInsertFeedback resets to idle state', () => {
  assert.deepEqual(clearHorseshoeInsertFeedback(), createHorseshoeInsertFeedbackState());
});

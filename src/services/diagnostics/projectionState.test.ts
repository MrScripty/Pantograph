import test from 'node:test';
import assert from 'node:assert/strict';
import { mockProjectionState } from './projectionState.ts';

test('mockProjectionState includes projection health defaults', () => {
  const state = mockProjectionState({
    projection_name: 'run_detail',
    projection_version: 6,
    last_applied_event_seq: 12,
  });

  assert.equal(state.projection_name, 'run_detail');
  assert.equal(state.projection_version, 6);
  assert.equal(state.last_applied_event_seq, 12);
  assert.equal(state.status, 'current');
  assert.equal(state.rebuilt_at_ms, null);
  assert.equal(typeof state.updated_at_ms, 'number');
  assert.equal(state.last_error, null);
  assert.equal(state.last_error_at_ms, null);
  assert.equal(state.last_failed_event_seq, null);
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { formatProjectionFreshnessState } from './projectionFreshness.ts';

test('formatProjectionFreshnessState includes backend projection health errors', () => {
  assert.equal(formatProjectionFreshnessState(null), 'Projection unavailable');
  assert.equal(
    formatProjectionFreshnessState({
      projection_name: 'run_detail',
      projection_version: 6,
      last_applied_event_seq: 12,
      status: 'failed',
      rebuilt_at_ms: null,
      updated_at_ms: 20,
      last_error: 'projection table missing',
      last_error_at_ms: 21,
      last_failed_event_seq: 12,
    }),
    'Failed at seq 12: projection table missing',
  );
});

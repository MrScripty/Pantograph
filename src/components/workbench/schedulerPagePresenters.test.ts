import test from 'node:test';
import assert from 'node:assert/strict';

import type { RunListProjectionRecord } from '../../services/diagnostics/types.ts';
import {
  filterAndSortSchedulerRuns,
  formatSchedulerDuration,
  formatSchedulerTimestamp,
  schedulerStatusClass,
} from './schedulerPagePresenters.ts';

function run(overrides: Partial<RunListProjectionRecord>): RunListProjectionRecord {
  return {
    workflow_run_id: 'run-a',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-a',
    workflow_semantic_version: '1.0.0',
    status: 'queued',
    accepted_at_ms: 1,
    enqueued_at_ms: 2,
    started_at_ms: null,
    completed_at_ms: null,
    duration_ms: null,
    scheduler_policy_id: 'policy-a',
    retention_policy_id: 'retention-a',
    last_event_seq: 1,
    last_updated_at_ms: 10,
    ...overrides,
  };
}

test('scheduler timestamp and duration presenters keep pending states visible', () => {
  assert.equal(formatSchedulerTimestamp(null), 'Unscheduled');
  assert.equal(formatSchedulerDuration(null, 'queued'), 'Pending');
  assert.equal(formatSchedulerDuration(null, 'running'), 'Running');
  assert.equal(formatSchedulerDuration(500, 'completed'), '500 ms');
  assert.equal(formatSchedulerDuration(1_500, 'completed'), '1.5 s');
});

test('schedulerStatusClass maps run statuses to stable classes', () => {
  assert.match(schedulerStatusClass('completed'), /emerald/);
  assert.match(schedulerStatusClass('running'), /cyan/);
  assert.match(schedulerStatusClass('queued'), /amber/);
  assert.match(schedulerStatusClass('failed'), /red/);
  assert.match(schedulerStatusClass('cancelled'), /neutral/);
});

test('filterAndSortSchedulerRuns filters by status and search text', () => {
  const runs = [
    run({ workflow_run_id: 'run-a', workflow_id: 'caption', status: 'queued' }),
    run({ workflow_run_id: 'run-b', workflow_id: 'render', status: 'completed' }),
    run({ workflow_run_id: 'run-c', workflow_id: 'caption', status: 'failed' }),
  ];

  const filtered = filterAndSortSchedulerRuns(runs, {
    search: 'caption',
    status: 'queued',
    sort: 'workflow_asc',
  });

  assert.deepEqual(filtered.map((item) => item.workflow_run_id), ['run-a']);
});

test('filterAndSortSchedulerRuns sorts by operational fields', () => {
  const runs = [
    run({ workflow_run_id: 'run-a', workflow_id: 'b', duration_ms: 1_000, last_updated_at_ms: 10 }),
    run({ workflow_run_id: 'run-b', workflow_id: 'a', duration_ms: 3_000, last_updated_at_ms: 30 }),
    run({ workflow_run_id: 'run-c', workflow_id: 'c', duration_ms: null, last_updated_at_ms: 20 }),
  ];

  assert.deepEqual(
    filterAndSortSchedulerRuns(runs, {
      search: '',
      status: 'all',
      sort: 'last_updated_desc',
    }).map((item) => item.workflow_run_id),
    ['run-b', 'run-c', 'run-a'],
  );
  assert.deepEqual(
    filterAndSortSchedulerRuns(runs, {
      search: '',
      status: 'all',
      sort: 'duration_desc',
    }).map((item) => item.workflow_run_id),
    ['run-b', 'run-a', 'run-c'],
  );
});

import test from 'node:test';
import assert from 'node:assert/strict';

import type { RunListProjectionRecord } from '../../services/diagnostics/types.ts';
import {
  filterAndSortSchedulerRuns,
  formatSchedulerPolicyLabel,
  formatSchedulerPriority,
  formatSchedulerQueuePosition,
  formatSchedulerProjectionFreshness,
  formatSchedulerEstimateLabel,
  formatSchedulerReasonLabel,
  formatSchedulerRetentionLabel,
  formatSchedulerScopeLabel,
  formatSchedulerTimelineKind,
  formatSchedulerTimelineSource,
  formatSchedulerDuration,
  formatSchedulerTimestamp,
  schedulerPolicyFilterOptions,
  schedulerRetentionFilterOptions,
  schedulerTimelinePayloadLabel,
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
    client_id: 'client-a',
    client_session_id: 'session-a',
    bucket_id: 'bucket-a',
    scheduler_queue_position: null,
    scheduler_priority: null,
    estimate_confidence: null,
    estimated_queue_wait_ms: null,
    estimated_duration_ms: null,
    scheduler_reason: null,
    last_event_seq: 1,
    last_updated_at_ms: 10,
    ...overrides,
  };
}

test('scheduler timestamp and duration presenters keep pending states visible', () => {
  assert.equal(formatSchedulerTimestamp(null), 'Unscheduled');
  assert.equal(formatSchedulerDuration(null, 'queued'), 'Pending');
  assert.equal(formatSchedulerDuration(null, 'delayed'), 'Pending');
  assert.equal(formatSchedulerDuration(null, 'running'), 'Running');
  assert.equal(formatSchedulerDuration(500, 'completed'), '500 ms');
  assert.equal(formatSchedulerDuration(1_500, 'completed'), '1.5 s');
});

test('schedulerStatusClass maps run statuses to stable classes', () => {
  assert.match(schedulerStatusClass('completed'), /emerald/);
  assert.match(schedulerStatusClass('running'), /cyan/);
  assert.match(schedulerStatusClass('queued'), /amber/);
  assert.match(schedulerStatusClass('delayed'), /orange/);
  assert.match(schedulerStatusClass('failed'), /red/);
  assert.match(schedulerStatusClass('cancelled'), /neutral/);
});

test('scheduler policy presenters keep missing dense table facts explicit', () => {
  assert.equal(formatSchedulerPolicyLabel('policy-high'), 'policy-high');
  assert.equal(formatSchedulerPolicyLabel(''), 'Unassigned');
  assert.equal(formatSchedulerPolicyLabel('   '), 'Unassigned');
  assert.equal(formatSchedulerPolicyLabel(null), 'Unassigned');
  assert.equal(formatSchedulerRetentionLabel('retention-short'), 'retention-short');
  assert.equal(formatSchedulerRetentionLabel(undefined), 'Unassigned');
  assert.equal(formatSchedulerScopeLabel('session-a'), 'session-a');
  assert.equal(formatSchedulerScopeLabel(''), 'Unassigned');
});

test('scheduler queue and estimate presenters keep unavailable facts explicit', () => {
  assert.equal(formatSchedulerQueuePosition(0), '0');
  assert.equal(formatSchedulerQueuePosition(null), 'Unassigned');
  assert.equal(formatSchedulerPriority(5), '5');
  assert.equal(formatSchedulerPriority(undefined), 'Default');
  assert.equal(
    formatSchedulerEstimateLabel(
      run({
        estimate_confidence: 'low',
        estimated_queue_wait_ms: 1_500,
        estimated_duration_ms: 2_500,
      }),
    ),
    'wait 1.5 s / run 2.5 s (low)',
  );
  assert.equal(formatSchedulerEstimateLabel(run({ estimate_confidence: 'low' })), 'low confidence');
  assert.equal(formatSchedulerEstimateLabel(run({})), 'Unavailable');
  assert.equal(formatSchedulerReasonLabel('warm_session_reused'), 'warm_session_reused');
  assert.equal(formatSchedulerReasonLabel(''), 'Unavailable');
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
    schedulerPolicy: 'all',
    retentionPolicy: 'all',
    sort: 'workflow_asc',
  });

  assert.deepEqual(filtered.map((item) => item.workflow_run_id), ['run-a']);
});

test('filterAndSortSchedulerRuns searches client scope facts', () => {
  const runs = [
    run({ workflow_run_id: 'run-a', client_session_id: 'session-alpha', bucket_id: 'bucket-main' }),
    run({ workflow_run_id: 'run-b', client_session_id: 'session-beta', bucket_id: 'bucket-side' }),
  ];

  assert.deepEqual(
    filterAndSortSchedulerRuns(runs, {
      search: 'bucket-side',
      status: 'all',
      schedulerPolicy: 'all',
      retentionPolicy: 'all',
      sort: 'workflow_asc',
    }).map((item) => item.workflow_run_id),
    ['run-b'],
  );
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
      schedulerPolicy: 'all',
      retentionPolicy: 'all',
      sort: 'last_updated_desc',
    }).map((item) => item.workflow_run_id),
    ['run-b', 'run-c', 'run-a'],
  );
  assert.deepEqual(
    filterAndSortSchedulerRuns(runs, {
      search: '',
      status: 'all',
      schedulerPolicy: 'all',
      retentionPolicy: 'all',
      sort: 'duration_desc',
    }).map((item) => item.workflow_run_id),
    ['run-b', 'run-a', 'run-c'],
  );
});

test('scheduler policy filters use explicit projection labels', () => {
  const runs = [
    run({
      workflow_run_id: 'run-a',
      scheduler_policy_id: 'policy-b',
      retention_policy_id: 'retention-a',
    }),
    run({
      workflow_run_id: 'run-b',
      scheduler_policy_id: 'policy-a',
      retention_policy_id: null,
    }),
    run({
      workflow_run_id: 'run-c',
      scheduler_policy_id: '',
      retention_policy_id: 'retention-b',
    }),
  ];

  assert.deepEqual(schedulerPolicyFilterOptions(runs), ['Unassigned', 'policy-a', 'policy-b']);
  assert.deepEqual(schedulerRetentionFilterOptions(runs), ['Unassigned', 'retention-a', 'retention-b']);
  assert.deepEqual(
    filterAndSortSchedulerRuns(runs, {
      search: '',
      status: 'all',
      schedulerPolicy: 'policy-a',
      retentionPolicy: 'Unassigned',
      sort: 'workflow_asc',
    }).map((item) => item.workflow_run_id),
    ['run-b'],
  );
});

test('scheduler timeline presenters expose typed projection facts', () => {
  assert.equal(
    formatSchedulerProjectionFreshness({
      projection_name: 'scheduler_timeline',
      projection_version: 1,
      last_applied_event_seq: 12,
      status: 'current',
      rebuilt_at_ms: null,
      updated_at_ms: 20,
    }),
    'Current at seq 12',
  );
  assert.equal(
    formatSchedulerTimelineKind({
      event_kind: 'scheduler_queue_placement',
    }),
    'Scheduler Queue Placement',
  );
  assert.equal(
    formatSchedulerTimelineSource({
      source_component: 'workflow_service',
    }),
    'Workflow Service',
  );
  assert.equal(schedulerTimelinePayloadLabel({ payload_json: '{}' }), 'Metadata only');
  assert.equal(schedulerTimelinePayloadLabel({ payload_json: '{"queue":"default"}' }), 'Payload captured');
});

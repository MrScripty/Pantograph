import test from 'node:test';
import assert from 'node:assert/strict';

import type {
  IoArtifactRetentionSummaryRecord,
  RunListProjectionRecord,
  SchedulerTimelineProjectionRecord,
} from '../../services/diagnostics/types.ts';
import {
  buildSchedulerEstimateRows,
  buildSchedulerRetentionSummaryRows,
  buildSchedulerRunListQuery,
  filterSchedulerTimelineEvents,
  filterAndSortSchedulerRuns,
  formatSchedulerAcceptedDateLabel,
  formatSchedulerEstimateDuration,
  formatSchedulerPlacementLabel,
  formatSchedulerPolicyLabel,
  formatSchedulerPriority,
  formatSchedulerQueuePosition,
  formatSchedulerProjectionFreshness,
  formatSchedulerEstimateLabel,
  formatSchedulerReasonLabel,
  formatSchedulerRetentionLabel,
  formatSchedulerRetentionStateLabel,
  formatSchedulerScopeLabel,
  formatSchedulerStatusLabel,
  formatSchedulerTimelineKind,
  formatSchedulerTimelineSource,
  formatSchedulerDuration,
  formatSchedulerTimestamp,
  schedulerAcceptedDateFilterOptions,
  schedulerBucketFilterOptions,
  schedulerClientFilterOptions,
  schedulerClientSessionFilterOptions,
  schedulerRunSupportsAdminQueueControls,
  schedulerPolicyFilterOptions,
  schedulerRetentionFilterOptions,
  schedulerRunSupportsQueueControls,
  schedulerSelectedDeviceFilterOptions,
  schedulerSelectedNetworkNodeFilterOptions,
  schedulerSelectedRuntimeFilterOptions,
  schedulerTimelinePayloadLabel,
  schedulerTimelineKindFilterOptions,
  schedulerTimelineSourceFilterOptions,
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
    workflow_execution_session_id: 'exec-session-a',
    selected_runtime_id: 'runtime-a',
    selected_device_id: 'device-a',
    selected_network_node_id: 'network-node-a',
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

function timelineEvent(
  overrides: Partial<SchedulerTimelineProjectionRecord>,
): SchedulerTimelineProjectionRecord {
  return {
    event_seq: 1,
    event_id: 'event-a',
    event_kind: 'scheduler_queue_placement',
    source_component: 'scheduler',
    occurred_at_ms: 1,
    recorded_at_ms: 2,
    workflow_run_id: 'run-a',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-a',
    workflow_semantic_version: '1.0.0',
    scheduler_policy_id: 'policy-a',
    retention_policy_id: 'retention-a',
    summary: 'queued',
    detail: null,
    payload_json: '{}',
    ...overrides,
  };
}

test('scheduler timestamp and duration presenters keep pending states visible', () => {
  assert.equal(formatSchedulerTimestamp(null), 'Unscheduled');
  assert.equal(formatSchedulerDuration(null, 'future'), 'Pending');
  assert.equal(formatSchedulerDuration(null, 'scheduled'), 'Pending');
  assert.equal(formatSchedulerDuration(null, 'queued'), 'Pending');
  assert.equal(formatSchedulerDuration(null, 'delayed'), 'Pending');
  assert.equal(formatSchedulerDuration(null, 'running'), 'Running');
  assert.equal(formatSchedulerDuration(500, 'completed'), '500 ms');
  assert.equal(formatSchedulerDuration(1_500, 'completed'), '1.5 s');
});

test('schedulerStatusClass maps run statuses to stable classes', () => {
  assert.match(schedulerStatusClass('completed'), /emerald/);
  assert.match(schedulerStatusClass('running'), /cyan/);
  assert.match(schedulerStatusClass('future'), /amber/);
  assert.match(schedulerStatusClass('scheduled'), /amber/);
  assert.match(schedulerStatusClass('queued'), /amber/);
  assert.match(schedulerStatusClass('delayed'), /orange/);
  assert.match(schedulerStatusClass('failed'), /red/);
  assert.match(schedulerStatusClass('cancelled'), /neutral/);
  assert.equal(formatSchedulerStatusLabel('future'), 'Future');
  assert.equal(formatSchedulerStatusLabel('scheduled'), 'Scheduled');
  assert.equal(formatSchedulerStatusLabel('cancelled'), 'Cancelled');
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
  assert.equal(formatSchedulerPlacementLabel('runtime-a'), 'runtime-a');
  assert.equal(formatSchedulerPlacementLabel(''), 'Unassigned');
  assert.equal(formatSchedulerAcceptedDateLabel(86_400_000), '1970-01-02');
  assert.equal(formatSchedulerAcceptedDateLabel(null), 'Unassigned');
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
  assert.equal(formatSchedulerEstimateDuration(null), 'Unavailable');
  assert.equal(formatSchedulerEstimateDuration(750), '750 ms');
  assert.equal(formatSchedulerEstimateDuration(1_500), '1.5 s');
  assert.equal(formatSchedulerReasonLabel('warm_session_reused'), 'warm_session_reused');
  assert.equal(formatSchedulerReasonLabel(''), 'Unavailable');
});

test('buildSchedulerEstimateRows exposes selected-run estimate projection facts', () => {
  assert.deepEqual(buildSchedulerEstimateRows(null), [
    { label: 'Confidence', value: 'Unavailable' },
    { label: 'Queue Wait', value: 'Unavailable' },
    { label: 'Run Duration', value: 'Unavailable' },
    { label: 'Policy', value: 'Unassigned', mono: true },
    { label: 'Updated', value: 'Unavailable' },
  ]);

  const rows = buildSchedulerEstimateRows({
    workflow_run_id: 'run-a',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-a',
    workflow_semantic_version: '1.0.0',
    scheduler_policy_id: 'policy-a',
    latest_estimate_json: '{}',
    estimate_confidence: 'medium',
    estimated_queue_wait_ms: 1_500,
    estimated_duration_ms: 2_500,
    last_event_seq: 7,
    last_updated_at_ms: 86_400_000,
  });

  assert.equal(rows.find((row) => row.label === 'Confidence')?.value, 'medium');
  assert.equal(rows.find((row) => row.label === 'Queue Wait')?.value, '1.5 s');
  assert.equal(rows.find((row) => row.label === 'Run Duration')?.value, '2.5 s');
  assert.equal(rows.find((row) => row.label === 'Policy')?.value, 'policy-a');
  assert.equal(rows.find((row) => row.label === 'Updated')?.value, new Date(86_400_000).toLocaleString());
});

test('buildSchedulerRetentionSummaryRows formats typed retention projection counts', () => {
  const rows = buildSchedulerRetentionSummaryRows([
    { retention_state: 'expired', artifact_count: 2 },
    { retention_state: 'retained', artifact_count: 5 },
    { retention_state: 'metadata_only', artifact_count: 5 },
    { retention_state: 'too_large', artifact_count: 1 },
  ] satisfies IoArtifactRetentionSummaryRecord[]);

  assert.deepEqual(rows, [
    { label: 'Metadata retained only', count: 5 },
    { label: 'Payload retained', count: 5 },
    { label: 'Payload expired', count: 2 },
    { label: 'Too large to retain', count: 1 },
  ]);
  assert.equal(formatSchedulerRetentionStateLabel('deleted'), 'Payload deleted');
});

test('scheduler queue controls require queued run execution-session facts', () => {
  assert.equal(schedulerRunSupportsQueueControls(run({ status: 'queued' })), true);
  assert.equal(schedulerRunSupportsQueueControls(run({ status: 'delayed' })), true);
  assert.equal(schedulerRunSupportsQueueControls(run({ status: 'running' })), false);
  assert.equal(
    schedulerRunSupportsQueueControls(
      run({ status: 'queued', workflow_execution_session_id: null }),
    ),
    false,
  );
  assert.equal(
    schedulerRunSupportsAdminQueueControls(
      run({ status: 'queued', workflow_execution_session_id: null }),
    ),
    true,
  );
  assert.equal(schedulerRunSupportsAdminQueueControls(run({ status: 'delayed' })), true);
  assert.equal(schedulerRunSupportsAdminQueueControls(run({ status: 'running' })), false);
});

test('filterAndSortSchedulerRuns filters by status and search text', () => {
  const runs = [
    run({ workflow_run_id: 'run-a', workflow_id: 'caption', status: 'queued' }),
    run({ workflow_run_id: 'run-b', workflow_id: 'render', status: 'completed', client_id: 'client-b' }),
    run({ workflow_run_id: 'run-c', workflow_id: 'caption', status: 'failed' }),
  ];

  const filtered = filterAndSortSchedulerRuns(runs, {
    search: 'caption',
    status: 'queued',
    schedulerPolicy: 'all',
    retentionPolicy: 'all',
    client: 'all',
    clientSession: 'all',
    bucket: 'all',
    selectedRuntime: 'all',
    selectedDevice: 'all',
    selectedNetworkNode: 'all',
    acceptedDate: 'all',
    sort: 'workflow_asc',
  });

  assert.deepEqual(filtered.map((item) => item.workflow_run_id), ['run-a']);
});

test('filterAndSortSchedulerRuns searches client scope facts', () => {
  const runs = [
    run({ workflow_run_id: 'run-a', client_session_id: 'session-alpha', bucket_id: 'bucket-main' }),
    run({
      workflow_run_id: 'run-b',
      client_session_id: 'session-beta',
      bucket_id: 'bucket-side',
      workflow_execution_session_id: 'exec-side',
    }),
  ];

  assert.deepEqual(
    filterAndSortSchedulerRuns(runs, {
      search: 'bucket-side',
      status: 'all',
      schedulerPolicy: 'all',
      retentionPolicy: 'all',
      client: 'all',
      clientSession: 'all',
      bucket: 'all',
      selectedRuntime: 'all',
      selectedDevice: 'all',
      selectedNetworkNode: 'all',
      acceptedDate: 'all',
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
      client: 'all',
      clientSession: 'all',
      bucket: 'all',
      selectedRuntime: 'all',
      selectedDevice: 'all',
      selectedNetworkNode: 'all',
      acceptedDate: 'all',
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
      client: 'all',
      clientSession: 'all',
      bucket: 'all',
      selectedRuntime: 'all',
      selectedDevice: 'all',
      selectedNetworkNode: 'all',
      acceptedDate: 'all',
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
      accepted_at_ms: 86_400_000,
    }),
    run({
      workflow_run_id: 'run-b',
      scheduler_policy_id: 'policy-a',
      retention_policy_id: null,
      client_id: 'client-b',
      client_session_id: 'session-b',
      bucket_id: 'bucket-b',
      selected_runtime_id: 'runtime-b',
      selected_device_id: 'device-b',
      selected_network_node_id: 'network-node-b',
      accepted_at_ms: 172_800_000,
    }),
    run({
      workflow_run_id: 'run-c',
      scheduler_policy_id: '',
      retention_policy_id: 'retention-b',
      client_id: null,
      client_session_id: null,
      bucket_id: null,
      selected_runtime_id: null,
      selected_device_id: null,
      selected_network_node_id: null,
      accepted_at_ms: null,
    }),
  ];

  assert.deepEqual(schedulerPolicyFilterOptions(runs), ['Unassigned', 'policy-a', 'policy-b']);
  assert.deepEqual(schedulerRetentionFilterOptions(runs), ['Unassigned', 'retention-a', 'retention-b']);
  assert.deepEqual(schedulerClientFilterOptions(runs), ['Unassigned', 'client-a', 'client-b']);
  assert.deepEqual(schedulerClientSessionFilterOptions(runs), ['Unassigned', 'session-a', 'session-b']);
  assert.deepEqual(schedulerBucketFilterOptions(runs), ['Unassigned', 'bucket-a', 'bucket-b']);
  assert.deepEqual(schedulerSelectedRuntimeFilterOptions(runs), ['Unassigned', 'runtime-a', 'runtime-b']);
  assert.deepEqual(schedulerSelectedDeviceFilterOptions(runs), ['Unassigned', 'device-a', 'device-b']);
  assert.deepEqual(schedulerSelectedNetworkNodeFilterOptions(runs), [
    'Unassigned',
    'network-node-a',
    'network-node-b',
  ]);
  assert.deepEqual(schedulerAcceptedDateFilterOptions(runs), ['Unassigned', '1970-01-02', '1970-01-03']);
  assert.deepEqual(
    filterAndSortSchedulerRuns(runs, {
      search: '',
      status: 'all',
      schedulerPolicy: 'policy-a',
      retentionPolicy: 'Unassigned',
      client: 'client-b',
      clientSession: 'session-b',
      bucket: 'bucket-b',
      selectedRuntime: 'runtime-b',
      selectedDevice: 'device-b',
      selectedNetworkNode: 'network-node-b',
      acceptedDate: '1970-01-03',
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

test('scheduler timeline filters use typed event kind and source fields', () => {
  const events = [
    timelineEvent({ event_id: 'event-queue', event_kind: 'scheduler_queue_placement', source_component: 'scheduler' }),
    timelineEvent({ event_id: 'event-model', event_kind: 'scheduler_model_lifecycle_changed', source_component: 'workflow_service' }),
    timelineEvent({ event_id: 'event-started', event_kind: 'run_started', source_component: 'runtime' }),
  ];

  assert.deepEqual(schedulerTimelineKindFilterOptions(events), [
    'run_started',
    'scheduler_model_lifecycle_changed',
    'scheduler_queue_placement',
  ]);
  assert.deepEqual(schedulerTimelineSourceFilterOptions(events), [
    'runtime',
    'scheduler',
    'workflow_service',
  ]);
  assert.deepEqual(
    filterSchedulerTimelineEvents(events, {
      eventKind: 'scheduler_model_lifecycle_changed',
      sourceComponent: 'workflow_service',
    }).map((event) => event.event_id),
    ['event-model'],
  );
});

test('buildSchedulerRunListQuery sends backend-supported filters only', () => {
  assert.deepEqual(
    buildSchedulerRunListQuery(
      {
        search: 'caption',
        status: 'queued',
        schedulerPolicy: 'policy-a',
        retentionPolicy: 'retention-a',
        client: 'client-a',
        clientSession: 'session-a',
        bucket: 'bucket-a',
        selectedRuntime: 'runtime-a',
        selectedDevice: 'device-a',
        selectedNetworkNode: 'network-node-a',
        acceptedDate: '1970-01-02',
        sort: 'workflow_asc',
      },
      250,
    ),
    {
      limit: 250,
      status: 'queued',
      scheduler_policy_id: 'policy-a',
      retention_policy_id: 'retention-a',
      client_id: 'client-a',
      client_session_id: 'session-a',
      bucket_id: 'bucket-a',
      selected_runtime_id: 'runtime-a',
      selected_device_id: 'device-a',
      selected_network_node_id: 'network-node-a',
      accepted_at_from_ms: 86_400_000,
      accepted_at_to_ms: 172_799_999,
    },
  );
  assert.deepEqual(
    buildSchedulerRunListQuery(
      {
        search: 'caption',
        status: 'all',
        schedulerPolicy: 'Unassigned',
        retentionPolicy: 'all',
        client: 'Unassigned',
        clientSession: 'all',
        bucket: 'all',
        selectedRuntime: 'Unassigned',
        selectedDevice: 'all',
        selectedNetworkNode: 'all',
        acceptedDate: 'all',
        sort: 'workflow_asc',
      },
      50,
    ),
    { limit: 50 },
  );
});

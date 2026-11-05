import test from 'node:test';
import assert from 'node:assert/strict';

import type {
  IoArtifactRetentionSummaryRecord,
  RunListProjectionRecord,
  SchedulerTimelineProjectionRecord,
} from '../../services/diagnostics/types.ts';
import {
  buildSchedulerRetentionSummaryRows,
  buildSchedulerTimelineAttemptRows,
  filterSchedulerTimelineEvents,
  filterAndSortSchedulerRuns,
  formatSchedulerAcceptedDateLabel,
  formatSchedulerDevicePlacementLabel,
  formatSchedulerPlacementLabel,
  formatSchedulerModelCacheState,
  formatSchedulerPolicyLabel,
  formatSchedulerProjectionFreshness,
  formatSchedulerRetentionLabel,
  formatSchedulerRetentionStateLabel,
  formatSchedulerRuntimePlacementLabel,
  formatSchedulerScopeLabel,
  formatSchedulerStatusLabel,
  formatSchedulerTimelineKind,
  formatSchedulerTimelineSource,
  formatSchedulerTimestamp,
  schedulerAcceptedDateFilterOptions,
  schedulerBucketFilterOptions,
  schedulerClientFilterOptions,
  schedulerClientSessionFilterOptions,
  schedulerRunSupportsAdminQueueControls,
  schedulerRunSupportsDependencyReadinessResume,
  schedulerPolicyFilterOptions,
  schedulerRetentionFilterOptions,
  schedulerRunSupportsQueueControls,
  schedulerSelectedBackendFilterOptions,
  schedulerSelectedDeviceClassFilterOptions,
  schedulerSelectedDeviceFilterOptions,
  schedulerSelectedNetworkNodeFilterOptions,
  schedulerSelectedRuntimeFilterOptions,
  schedulerSelectedRuntimeVariantFilterOptions,
  schedulerTimelinePayloadLabel,
  schedulerTimelineKindFilterOptions,
  schedulerTimelineSourceFilterOptions,
  buildSchedulerFilterFields,
  buildSchedulerQueueSummary,
  buildSchedulerStatusSegments,
  filterSchedulerRunsBySegment,
  formatSchedulerClockTime,
  formatSchedulerRowTime,
  groupSchedulerRuns,
  schedulerAdminControlsUnavailableReason,
  schedulerAppliedFilterChips,
  schedulerDetailFreshnessText,
  schedulerEstimateSentence,
  schedulerFooterState,
  schedulerQueueControlsUnavailableReason,
  schedulerResumeUnavailableReason,
  schedulerRetentionPolicyText,
  schedulerRetentionText,
  schedulerRunDetailChips,
  schedulerRunDetailLine,
  schedulerRunPlacementText,
  schedulerRunPolicyText,
  schedulerRunProgressText,
  schedulerStatusSegmentForRun,
  schedulerTimelineAttemptText,
} from './schedulerPagePresenters.ts';
import {
  DEFAULT_SCHEDULER_RUN_FILTERS,
  SCHEDULER_SECONDARY_FILTER_KEYS,
  withSchedulerRunFilters,
} from '../../stores/schedulerRunListStore.ts';
import { groupRunsForContextPicker } from './shell/runContextPicker.ts';

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
    selected_runtime_variant_id: 'runtime_a.cuda',
    selected_backend_key: 'llama_cpp',
    selected_device_class: 'cuda',
    selected_device_id: 'device-a',
    selected_network_node_id: 'network-node-a',
    scheduler_queue_position: null,
    scheduler_priority: null,
    estimate_confidence: null,
    estimated_queue_wait_ms: null,
    estimated_duration_ms: null,
    model_cache_state: null,
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

test('scheduler timestamp and status presenters keep pending states visible', () => {
  assert.equal(formatSchedulerTimestamp(null), 'Unscheduled');
  assert.equal(formatSchedulerStatusLabel('future'), 'Future');
  assert.equal(formatSchedulerStatusLabel('scheduled'), 'Scheduled');
  assert.equal(formatSchedulerStatusLabel('cancelled'), 'Cancelled');
  assert.equal(formatSchedulerModelCacheState('cache_miss'), 'Model cache miss');
  assert.equal(formatSchedulerModelCacheState(null), 'Unavailable');
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
  assert.equal(formatSchedulerRuntimePlacementLabel(run({})), 'runtime-a / runtime_a.cuda');
  assert.equal(
    formatSchedulerRuntimePlacementLabel(run({ selected_runtime_variant_id: null })),
    'runtime-a',
  );
  assert.equal(
    formatSchedulerRuntimePlacementLabel(run({ selected_runtime_id: null })),
    'runtime_a.cuda',
  );
  assert.equal(formatSchedulerDevicePlacementLabel(run({})), 'cuda / device-a');
  assert.equal(formatSchedulerDevicePlacementLabel(run({ selected_device_id: null })), 'cuda');
  assert.equal(
    formatSchedulerDevicePlacementLabel(run({ selected_device_class: null })),
    'device-a',
  );
  assert.equal(formatSchedulerAcceptedDateLabel(86_400_000), '1970-01-02');
  assert.equal(formatSchedulerAcceptedDateLabel(null), 'Unassigned');
});

test('scheduler dependency-readiness resume presenter uses backend typed state', () => {
  assert.equal(
    schedulerRunSupportsDependencyReadinessResume(
      run({
        status: 'running',
        scheduler_reason: 'runtime_dependency_readiness_pending',
        workflow_execution_session_resume_state: 'dependency_readiness_pending',
      }),
    ),
    true,
  );
  assert.equal(
    schedulerRunSupportsDependencyReadinessResume(
      run({
        workflow_execution_session_id: null,
        workflow_execution_session_resume_state: 'dependency_readiness_pending',
      }),
    ),
    false,
  );
  assert.equal(
    schedulerRunSupportsDependencyReadinessResume(
      run({
        status: 'running',
        scheduler_reason: 'runtime_dependency_readiness_pending',
        workflow_execution_session_resume_state: null,
      }),
    ),
    false,
  );
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
    statusSegment: 'all',
    status: 'queued',
    schedulerPolicy: 'all',
    retentionPolicy: 'all',
    client: 'all',
    clientSession: 'all',
    bucket: 'all',
    selectedRuntime: 'all',
    selectedRuntimeVariant: 'all',
    selectedBackend: 'all',
    selectedDeviceClass: 'all',
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
      statusSegment: 'all',
      status: 'all',
      schedulerPolicy: 'all',
      retentionPolicy: 'all',
      client: 'all',
      clientSession: 'all',
      bucket: 'all',
      selectedRuntime: 'all',
      selectedRuntimeVariant: 'all',
      selectedBackend: 'all',
      selectedDeviceClass: 'all',
      selectedDevice: 'all',
      selectedNetworkNode: 'all',
      acceptedDate: 'all',
      sort: 'workflow_asc',
    }).map((item) => item.workflow_run_id),
    ['run-b'],
  );
});

test('filterAndSortSchedulerRuns searches selected runtime variant facts', () => {
  const runs = [
    run({ workflow_run_id: 'run-a', selected_runtime_variant_id: 'llama_cpp.cpu' }),
    run({ workflow_run_id: 'run-b', selected_runtime_variant_id: 'llama_cpp.cuda' }),
  ];

  assert.deepEqual(
    filterAndSortSchedulerRuns(runs, {
      search: 'llama_cpp.cuda',
      statusSegment: 'all',
      status: 'all',
      schedulerPolicy: 'all',
      retentionPolicy: 'all',
      client: 'all',
      clientSession: 'all',
      bucket: 'all',
      selectedRuntime: 'all',
      selectedRuntimeVariant: 'all',
      selectedBackend: 'all',
      selectedDeviceClass: 'all',
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
      statusSegment: 'all',
      status: 'all',
      schedulerPolicy: 'all',
      retentionPolicy: 'all',
      client: 'all',
      clientSession: 'all',
      bucket: 'all',
      selectedRuntime: 'all',
      selectedRuntimeVariant: 'all',
      selectedBackend: 'all',
      selectedDeviceClass: 'all',
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
      statusSegment: 'all',
      status: 'all',
      schedulerPolicy: 'all',
      retentionPolicy: 'all',
      client: 'all',
      clientSession: 'all',
      bucket: 'all',
      selectedRuntime: 'all',
      selectedRuntimeVariant: 'all',
      selectedBackend: 'all',
      selectedDeviceClass: 'all',
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
      selected_runtime_variant_id: 'runtime_b.metal',
      selected_backend_key: 'pytorch',
      selected_device_class: 'metal',
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
      selected_runtime_variant_id: null,
      selected_backend_key: null,
      selected_device_class: null,
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
  assert.deepEqual(schedulerSelectedRuntimeVariantFilterOptions(runs), [
    'Unassigned',
    'runtime_a.cuda',
    'runtime_b.metal',
  ]);
  assert.deepEqual(schedulerSelectedBackendFilterOptions(runs), ['Unassigned', 'llama_cpp', 'pytorch']);
  assert.deepEqual(schedulerSelectedDeviceClassFilterOptions(runs), ['Unassigned', 'cuda', 'metal']);
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
      statusSegment: 'all',
      status: 'all',
      schedulerPolicy: 'policy-a',
      retentionPolicy: 'Unassigned',
      client: 'client-b',
      clientSession: 'session-b',
      bucket: 'bucket-b',
      selectedRuntime: 'runtime-b',
      selectedRuntimeVariant: 'runtime_b.metal',
      selectedBackend: 'pytorch',
      selectedDeviceClass: 'metal',
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

test('scheduler timeline attempt rows use typed projection facts', () => {
  assert.deepEqual(buildSchedulerTimelineAttemptRows(timelineEvent({})), []);
  assert.deepEqual(
    buildSchedulerTimelineAttemptRows(
      timelineEvent({
        event_kind: 'scheduler_task_attempt_lifecycle_changed',
        scheduler_task_id: 'task-runtime-image-alpha',
        scheduler_attempt_id: 'attempt-runtime-image-alpha-001',
        scheduler_attempt_execution_class: 'runtime',
        scheduler_attempt_transition: 'redispatched',
        scheduler_attempt_started_at_ms: 1_000,
        scheduler_attempt_ended_at_ms: 2_250,
        scheduler_attempt_duration_ms: 1_250,
        scheduler_attempt_runtime_id: 'pytorch',
        scheduler_attempt_runtime_variant_id: 'pytorch.cuda',
        scheduler_attempt_backend_key: 'pytorch',
        scheduler_attempt_device_class: 'cuda',
        scheduler_attempt_device_id: 'cuda:0',
        scheduler_attempt_network_node_id: 'local-node-alpha',
        scheduler_attempt_reservation_id: 'reservation-runtime-image-alpha',
      }),
    ),
    [
      { label: 'Task', value: 'task-runtime-image-alpha' },
      { label: 'Attempt', value: 'attempt-runtime-image-alpha-001' },
      { label: 'Transition', value: 'Redispatched' },
      { label: 'Class', value: 'Runtime' },
      { label: 'Started', value: new Date(1_000).toLocaleString() },
      { label: 'Ended', value: new Date(2_250).toLocaleString() },
      { label: 'Duration', value: '1.3 s' },
      { label: 'Runtime', value: 'pytorch' },
      { label: 'Variant', value: 'pytorch.cuda' },
      { label: 'Backend', value: 'pytorch' },
      { label: 'Device Class', value: 'cuda' },
      { label: 'Device', value: 'cuda:0' },
      { label: 'Network Node', value: 'local-node-alpha' },
      { label: 'Reservation', value: 'reservation-runtime-image-alpha' },
    ],
  );
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


// --- Redesigned layout -------------------------------------------------------

const ALL_STATUSES: RunListProjectionRecord['status'][] = [
  'accepted',
  'future',
  'scheduled',
  'queued',
  'delayed',
  'running',
  'completed',
  'failed',
  'cancelled',
];

function localMs(hours: number, minutes: number, seconds = 0, day = 3): number {
  return new Date(2026, 10, day, hours, minutes, seconds).getTime();
}

test('status segments agree with the picker groups and have no memory-wait bucket (WR-004)', () => {
  const nowMs = localMs(20, 0);
  for (const status of ALL_STATUSES) {
    const record = run({ workflow_run_id: `run-${status}`, status, completed_at_ms: localMs(19, 0) });
    const [group] = groupRunsForContextPicker([record], { query: '', selectedRunId: null, nowMs });
    const segment = schedulerStatusSegmentForRun(record);
    if (group.id === 'running') assert.equal(segment, 'active', status);
    else if (group.id === 'queued') assert.equal(segment, 'queued', status);
    else assert.equal(segment, status === 'failed' ? 'failed' : 'done', status);
  }

  const pool = ALL_STATUSES.map((status) => run({ workflow_run_id: `run-${status}`, status }));
  assert.deepEqual(
    buildSchedulerStatusSegments(pool).map((option) => [option.value, option.label, option.count]),
    [
      ['all', 'All', 9],
      ['active', 'Active', 1],
      ['queued', 'Queued', 5],
      ['done', 'Done', 2],
      ['failed', 'Failed', 1],
    ],
  );
  assert.deepEqual(
    filterSchedulerRunsBySegment(pool, 'done').map((record) => record.status),
    ['completed', 'cancelled'],
  );
  assert.equal(filterSchedulerRunsBySegment(pool, 'all').length, 9);
});

test('groupSchedulerRuns uses the picker grouping and keeps the Sort order inside groups', () => {
  const nowMs = localMs(20, 0);
  const sorted = [
    run({ workflow_run_id: 'run-q3', status: 'queued', scheduler_queue_position: 3, last_updated_at_ms: localMs(19, 59) }),
    run({
      workflow_run_id: 'run-delayed',
      status: 'delayed',
      scheduler_reason: 'needs 8 GB on cuda:0',
      selected_runtime_id: null,
      selected_device_id: null,
      last_updated_at_ms: localMs(19, 58),
    }),
    run({
      workflow_run_id: 'run-live',
      status: 'running',
      started_at_ms: localMs(19, 30),
      last_updated_at_ms: localMs(19, 57),
    }),
    run({ workflow_run_id: 'run-q1', status: 'queued', scheduler_queue_position: 1, last_updated_at_ms: localMs(19, 56) }),
    run({
      workflow_run_id: 'run-old',
      status: 'completed',
      completed_at_ms: localMs(18, 0, 0, 2),
      last_updated_at_ms: localMs(18, 0, 0, 2),
    }),
  ];

  const groups = groupSchedulerRuns(sorted, { selectedRunId: 'run-q1', nowMs });

  assert.deepEqual(
    groups.map((group) => [group.title, group.rows.map((row) => row.runId)]),
    [
      ['Running', ['run-live']],
      ['Queued', ['run-q3', 'run-delayed', 'run-q1']],
      ['Finished earlier', ['run-old']],
    ],
  );
  const queued = groups[1].rows;
  assert.equal(queued[1].contextSuffix, ', needs 8 GB on cuda:0');
  assert.equal(queued[1].detail, null);
  assert.equal(queued[0].contextSuffix, ', #3 in queue');
  assert.equal(queued[0].detail, 'runtime-a on device-a');
  assert.equal(queued[0].time, '19:59');
  assert.equal(queued[2].selected, true);
  assert.equal(queued[0].selected, false);
  assert.equal(groups[0].rows[0].detail, 'Started 19:30');
  assert.equal(groups[2].rows[0].time, 'Nov 2');
});

test('row time and detail presenters stay short', () => {
  const startOfToday = localMs(0, 0);
  assert.equal(formatSchedulerRowTime(localMs(9, 5), startOfToday), '09:05');
  assert.equal(formatSchedulerRowTime(localMs(9, 5, 0, 1), startOfToday), 'Nov 1');
  assert.equal(formatSchedulerClockTime(localMs(9, 5, 7)), '09:05:07');
  assert.equal(schedulerRunDetailLine(run({ status: 'running', started_at_ms: null })), null);
  assert.equal(
    schedulerRunDetailLine(run({ status: 'queued', selected_device_id: null })),
    'runtime-a, device not assigned yet',
  );
  assert.equal(
    schedulerRunDetailLine(run({ status: 'queued', selected_runtime_id: null, selected_device_id: null })),
    null,
  );
});

test('buildSchedulerQueueSummary reads run-list estimates and degrades to a count', () => {
  const loaded = { loaded: true, loading: false, unreachable: false };
  assert.equal(
    buildSchedulerQueueSummary([run({ status: 'queued' }), run({ status: 'delayed' })], loaded),
    '2 runs waiting',
  );
  assert.equal(
    buildSchedulerQueueSummary(
      [
        run({ status: 'running', selected_device_id: 'cuda:0' }),
        run({ status: 'queued', estimated_duration_ms: 600_000, selected_device_id: 'cuda:1' }),
        run({ status: 'queued', estimated_duration_ms: 420_000, selected_device_id: null }),
        run({ status: 'completed', estimated_duration_ms: 999_000, selected_device_id: 'cuda:9' }),
      ],
      loaded,
    ),
    '2 runs waiting, about 17 min of queued work on 2 devices',
  );
  assert.equal(
    buildSchedulerQueueSummary(
      [
        run({ status: 'queued', estimated_duration_ms: 30_000, selected_device_id: null }),
        run({ status: 'queued', estimated_duration_ms: null, selected_device_id: null }),
      ],
      loaded,
    ),
    '2 runs waiting, under a minute of queued work (estimated for 1 of them)',
  );
  assert.equal(buildSchedulerQueueSummary([run({ status: 'queued' })], loaded), '1 run waiting');
  assert.equal(buildSchedulerQueueSummary([run({ status: 'running' })], loaded), 'No runs waiting, 1 running');
  assert.equal(buildSchedulerQueueSummary([], loaded), 'No runs waiting');
  assert.equal(
    buildSchedulerQueueSummary([], { loaded: false, loading: true, unreachable: false }),
    'Loading runs',
  );
  assert.equal(
    buildSchedulerQueueSummary([], { loaded: false, loading: false, unreachable: true }),
    'Run list unavailable, the engine did not answer',
  );
});

test('buildSchedulerFilterFields lists the fourteen disclosure selects with stable options', () => {
  const runs = [
    run({ workflow_run_id: 'run-a', scheduler_policy_id: 'fifo' }),
    run({ workflow_run_id: 'run-b', scheduler_policy_id: null }),
  ];
  const filters = withSchedulerRunFilters(DEFAULT_SCHEDULER_RUN_FILTERS, {
    selectedDevice: 'cuda:7',
    status: 'delayed',
  });
  const fields = buildSchedulerFilterFields(runs, filters);

  assert.deepEqual(
    fields.map((field) => field.key),
    SCHEDULER_SECONDARY_FILTER_KEYS,
  );
  assert.equal(new Set(fields.map((field) => field.id)).size, 14);
  const status = fields.find((field) => field.key === 'status');
  assert.equal(status?.value, 'delayed');
  assert.deepEqual(status?.options.slice(0, 2), [
    { value: 'all', label: 'All' },
    { value: 'accepted', label: 'Accepted' },
  ]);
  const sort = fields.find((field) => field.key === 'sort');
  assert.deepEqual(sort?.options.map((option) => option.label), ['Updated', 'Queued', 'Duration', 'Workflow', 'Status']);
  const policy = fields.find((field) => field.key === 'schedulerPolicy');
  assert.deepEqual(policy?.options.map((option) => option.value), ['all', 'Unassigned', 'fifo']);
  const device = fields.find((field) => field.key === 'selectedDevice');
  assert.deepEqual(device?.options.map((option) => option.value), ['all', 'device-a', 'cuda:7']);
});

test('schedulerAppliedFilterChips names each non-default disclosure filter', () => {
  assert.deepEqual(schedulerAppliedFilterChips(DEFAULT_SCHEDULER_RUN_FILTERS), []);
  const filters = withSchedulerRunFilters(DEFAULT_SCHEDULER_RUN_FILTERS, {
    search: 'ignored',
    statusSegment: 'failed',
    status: 'failed',
    sort: 'duration_desc',
    selectedNetworkNode: 'local',
  });
  assert.deepEqual(schedulerAppliedFilterChips(filters), [
    { key: 'status', label: 'Status: Failed' },
    { key: 'sort', label: 'Sort: Duration' },
    { key: 'selectedNetworkNode', label: 'Network node: local' },
  ]);
});

test('detail presenters describe progress, placement and retention from projection facts', () => {
  assert.equal(
    schedulerRunProgressText(run({ status: 'running', started_at_ms: localMs(20, 1, 40) })),
    'Running since 20:01:40',
  );
  assert.equal(
    schedulerRunProgressText(run({ status: 'queued', scheduler_queue_position: 2, scheduler_reason: 'model cached' })),
    'Queue position 2, model cached',
  );
  assert.equal(schedulerRunProgressText(run({ status: 'scheduled' })), 'Scheduled');
  assert.equal(schedulerRunProgressText(run({ status: 'delayed', scheduler_reason: 'waiting on cuda:0' })), 'Delayed: waiting on cuda:0');
  assert.equal(schedulerRunProgressText(run({ status: 'completed', duration_ms: 38_000 })), 'Finished in 38 s');
  assert.equal(
    schedulerRunProgressText(run({ status: 'failed', latest_error_message: 'out of memory' })),
    'Failed: out of memory',
  );
  assert.equal(schedulerRunProgressText(run({ status: 'cancelled', duration_ms: 70_000 })), 'Cancelled after 1m 10s');

  assert.equal(schedulerRunPlacementText(run({})), 'runtime-a / runtime_a.cuda on cuda / device-a');
  assert.equal(
    schedulerRunPlacementText(run({ selected_device_id: null, selected_device_class: null })),
    'runtime-a / runtime_a.cuda, device not assigned yet',
  );
  assert.equal(
    schedulerRunPlacementText(
      run({
        selected_runtime_id: null,
        selected_runtime_variant_id: null,
        selected_device_id: null,
        selected_device_class: null,
      }),
    ),
    'Not assigned to a device yet',
  );
  assert.equal(
    schedulerRunPolicyText(run({})),
    'Policy policy-a, bucket bucket-a, backend llama_cpp, network node network-node-a',
  );
  assert.equal(
    schedulerRunPolicyText(
      run({ scheduler_policy_id: null, bucket_id: 'default', selected_backend_key: null, selected_network_node_id: null }),
    ),
    'Bucket default',
  );
  assert.equal(schedulerRetentionPolicyText(run({})), 'Retention policy retention-a');
  assert.equal(schedulerRetentionPolicyText(run({ retention_policy_id: null })), 'No retention policy recorded');
  assert.equal(
    schedulerRetentionText(
      [
        { retention_state: 'metadata_only', artifact_count: 1 },
        { retention_state: 'retained', artifact_count: 3 },
      ],
      { loading: false, error: null },
    ),
    'Payload retained: 3, Metadata retained only: 1',
  );
  assert.equal(schedulerRetentionText([], { loading: false, error: null }), 'No retained artifacts projected');
  assert.equal(schedulerRetentionText([], { loading: true, error: null }), 'Loading the artifact summary');
  assert.equal(
    schedulerRetentionText([], { loading: false, error: 'Transport error: down' }),
    'Artifact summary unavailable: Transport error: down',
  );
  assert.deepEqual(schedulerRunDetailChips(run({ scheduler_priority: 5, scheduler_queue_position: 1 })), [
    'runtime-a on device-a',
    'priority 5',
    '#1 in queue',
  ]);
  assert.deepEqual(
    schedulerRunDetailChips(run({ selected_runtime_id: null, selected_device_id: null, scheduler_priority: 0 })),
    [],
  );
});

test('schedulerEstimateSentence reads the estimate query and says when it is missing', () => {
  const estimate = {
    workflow_run_id: 'run-a',
    workflow_id: 'workflow-a',
    estimate_confidence: 'high',
    estimated_queue_wait_ms: 40_000,
    estimated_duration_ms: 360_000,
    model_cache_state: 'cache_hit' as const,
    last_event_seq: 3,
    last_updated_at_ms: 1,
  };
  assert.equal(
    schedulerEstimateSentence(estimate, { loading: false, error: null }),
    'About 40 s queue wait, about 6m 00s to run, high confidence. Model cache hit.',
  );
  assert.equal(
    schedulerEstimateSentence(
      { ...estimate, estimated_queue_wait_ms: null, estimated_duration_ms: null, estimate_confidence: null, model_cache_state: 'unknown' },
      { loading: false, error: null },
    ),
    'No duration estimate.',
  );
  assert.equal(schedulerEstimateSentence(null, { loading: false, error: null }), 'No scheduler estimate for this run');
  assert.equal(schedulerEstimateSentence(null, { loading: true, error: null }), 'Loading the scheduler estimate');
  assert.equal(
    schedulerEstimateSentence(estimate, { loading: false, error: 'Internal error: boom' }),
    'Estimate unavailable: Internal error: boom',
  );
});

test('footer state and unavailable reasons keep every action reachable but honest', () => {
  assert.deepEqual(schedulerFooterState(null, null), { kind: 'none' });
  assert.deepEqual(schedulerFooterState('run-x', null), { kind: 'missing' });
  assert.deepEqual(schedulerFooterState('run-a', run({ status: 'failed' })), {
    kind: 'finished',
    workflowId: 'workflow-a',
  });
  const queued = run({ status: 'queued' });
  assert.deepEqual(schedulerFooterState('run-a', queued), { kind: 'live', run: queued });
  assert.equal(schedulerFooterState('run-a', run({ status: 'running' })).kind, 'live');

  assert.equal(schedulerQueueControlsUnavailableReason(queued), null);
  assert.match(schedulerQueueControlsUnavailableReason(run({ status: 'running' })) ?? '', /only while a run is queued/);
  assert.match(
    schedulerQueueControlsUnavailableReason(run({ status: 'queued', workflow_execution_session_id: null })) ?? '',
    /GUI admin actions under More actions/,
  );
  assert.equal(schedulerAdminControlsUnavailableReason(run({ status: 'delayed', workflow_execution_session_id: null })), null);
  assert.match(schedulerAdminControlsUnavailableReason(run({ status: 'running' })) ?? '', /queued or delayed/);
  assert.equal(
    schedulerResumeUnavailableReason(run({ workflow_execution_session_resume_state: 'dependency_readiness_pending' })),
    null,
  );
  assert.match(schedulerResumeUnavailableReason(run({})) ?? '', /dependency readiness/);
});

test('timeline helpers summarise attempts and projection freshness', () => {
  assert.equal(schedulerTimelineAttemptText(timelineEvent({})), null);
  assert.equal(
    schedulerTimelineAttemptText(
      timelineEvent({ scheduler_attempt_id: 'attempt-1', scheduler_attempt_device_id: 'cuda:0' }),
    ),
    'Attempt attempt-1 · Device cuda:0',
  );
  assert.equal(
    schedulerDetailFreshnessText({
      timeline: {
        projection_name: 'scheduler_timeline',
        projection_version: 1,
        last_applied_event_seq: 41,
        status: 'current',
        updated_at_ms: 1,
      },
      estimate: null,
      retention: null,
    }),
    'Timeline current at seq 41; estimate projection unavailable; artifacts projection unavailable',
  );
});

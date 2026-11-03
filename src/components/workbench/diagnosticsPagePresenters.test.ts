import test from 'node:test';
import assert from 'node:assert/strict';

import type {
  IoArtifactRetentionSummaryRecord,
  NodeStatusProjectionRecord,
  RunDetailProjectionRecord,
  RunListProjectionRecord,
  SchedulerTimelineProjectionRecord,
} from '../../services/diagnostics/types.ts';
import {
  DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
  DEFAULT_DIAGNOSTICS_EXECUTION_FILTERS,
  buildDiagnosticsClientLabel,
  buildDiagnosticsComparisonFilterChips,
  buildDiagnosticsComparisonShortcuts,
  buildDiagnosticsComparisonSummary,
  buildDiagnosticsDimensionLine,
  buildDiagnosticsErrorCard,
  buildDiagnosticsExecutionFilterChips,
  buildDiagnosticsFactSections,
  buildDiagnosticsLevelOptions,
  buildDiagnosticsLogLines,
  buildDiagnosticsNodeErrorsOutsideTimeline,
  buildDiagnosticsNodeStatusBars,
  buildDiagnosticsPlacementLabel,
  buildDiagnosticsProjectionRows,
  buildDiagnosticsRetentionRows,
  createDiagnosticsRefreshGate,
  diagnosticsSentenceCase,
  diagnosticsFactLayoutLabels,
  diagnosticsFocusKey,
  diagnosticsRecentFromDate,
  explainDiagnosticsError,
  filterDiagnosticsLogLines,
  formatDiagnosticsClock,
  formatDiagnosticsElapsed,
  formatDiagnosticsLogTime,
  groupDiagnosticsExecutionFacetRows,
  resolveDiagnosticsFocus,
  buildDiagnosticsExecutionFacetRows,
  buildDiagnosticsExecutionFilterOptions,
  buildDiagnosticsFacetSummary,
  buildDiagnosticsFactRows,
  buildDiagnosticsRetentionSummaryRows,
  buildDiagnosticsComparisonFilterOptions,
  buildDiagnosticsRunErrorSummary,
  filterDiagnosticsExecutionNodes,
  filterDiagnosticsComparisonRuns,
  formatDiagnosticErrorPhase,
  formatDiagnosticErrorSeverity,
  formatDiagnosticEventKind,
  formatDiagnosticSourceComponent,
  formatDiagnosticsDuration,
  formatDiagnosticsModelCacheState,
  formatDiagnosticsProjectionFreshness,
  formatDiagnosticsStatusLabel,
  hasActiveDiagnosticsComparisonFilters,
  hasTimelinePayload,
} from './diagnosticsPagePresenters.ts';

function createRunDetail(): RunDetailProjectionRecord {
  return {
    workflow_run_id: 'run-1',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-1',
    workflow_semantic_version: '1.2.3',
    status: 'running',
    accepted_at_ms: 1,
    enqueued_at_ms: 2,
    started_at_ms: 3,
    completed_at_ms: null,
    duration_ms: null,
    scheduler_policy_id: 'policy-a',
    retention_policy_id: 'retention-a',
    selected_runtime_id: 'runtime-a',
    selected_runtime_variant_id: 'runtime_a.cuda',
    selected_backend_key: 'llama_cpp',
    selected_model_id: 'pumas://models/tiny-gguf',
    selected_task_id: 'text_generation',
    selected_device_class: 'cuda',
    selected_device_id: 'device-a',
    selected_network_node_id: 'network-a',
    scheduler_queue_position: 1,
    scheduler_priority: 5,
    estimate_confidence: 'low',
    estimated_queue_wait_ms: 1_500,
    estimated_duration_ms: 2_500,
    model_cache_state: 'cache_hit',
    scheduler_reason: 'warm_session_reused',
    latest_error_event_id: null,
    latest_error_severity: null,
    latest_error_phase: null,
    latest_error_code: null,
    latest_error_message: null,
    fatal_error_event_id: null,
    error_count: 0,
    warning_count: 0,
    last_event_seq: 9,
    last_updated_at_ms: 10,
    client_id: 'client-a',
    client_session_id: 'session-a',
    workflow_execution_session_id: 'exec-session-a',
    bucket_id: 'bucket-a',
    workflow_run_snapshot_id: 'snapshot-a',
    workflow_presentation_revision_id: 'presentation-a',
    latest_estimate_json: null,
    latest_queue_placement_json: null,
    started_payload_json: null,
    terminal_payload_json: null,
    terminal_error: null,
    timeline_event_count: 4,
  };
}

function createRunListPeer(overrides: Partial<RunListProjectionRecord>): RunListProjectionRecord {
  return {
    workflow_run_id: 'run-peer',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-1',
    workflow_semantic_version: '1.2.3',
    status: 'completed',
    scheduler_policy_id: 'policy-a',
    retention_policy_id: 'retention-a',
    selected_runtime_id: 'runtime-a',
    selected_runtime_variant_id: 'runtime_a.cuda',
    selected_backend_key: 'llama_cpp',
    selected_model_id: 'pumas://models/tiny-gguf',
    selected_task_id: 'text_generation',
    selected_device_class: 'cuda',
    selected_device_id: 'device-a',
    selected_network_node_id: 'network-a',
    accepted_at_ms: 86_400_000,
    client_id: 'client-a',
    client_session_id: 'session-a',
    bucket_id: 'bucket-a',
    latest_error_event_id: null,
    latest_error_severity: null,
    latest_error_phase: null,
    latest_error_code: null,
    latest_error_message: null,
    fatal_error_event_id: null,
    error_count: 0,
    warning_count: 0,
    last_event_seq: 10,
    last_updated_at_ms: 20,
    ...overrides,
  };
}

function createNodeStatus(overrides: Partial<NodeStatusProjectionRecord>): NodeStatusProjectionRecord {
  return {
    workflow_run_id: 'run-1',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-1',
    workflow_semantic_version: '1.2.3',
    node_id: 'node-a',
    node_type: 'image.generate',
    node_version: '1.0.0',
    runtime_id: 'runtime-a',
    runtime_version: '2.0.0',
    task_id: 'image_generation',
    selected_backend_key: 'mock',
    model_id: 'model-a',
    model_version: '3.0.0',
    status: 'completed',
    started_at_ms: 1,
    completed_at_ms: 2,
    duration_ms: 1,
    error: null,
    error_event_id: null,
    error_severity: null,
    error_phase: null,
    error_code: null,
    last_event_seq: 9,
    last_updated_at_ms: 10,
    ...overrides,
  };
}

test('formatDiagnosticsDuration exposes pending and running states', () => {
  assert.equal(formatDiagnosticsDuration(null, 'running'), 'Running');
  assert.equal(formatDiagnosticsDuration(null, 'future'), 'Pending');
  assert.equal(formatDiagnosticsDuration(null, 'scheduled'), 'Pending');
  assert.equal(formatDiagnosticsDuration(null, 'queued'), 'Pending');
  assert.equal(formatDiagnosticsDuration(null, 'delayed'), 'Pending');
  assert.equal(formatDiagnosticsDuration(500, 'completed'), '500 ms');
  assert.equal(formatDiagnosticsDuration(2_500, 'completed'), '2.5 s');
});

test('formatDiagnosticsProjectionFreshness keeps projection cursor visible', () => {
  assert.equal(formatDiagnosticsProjectionFreshness(null), 'Projection unavailable');
  assert.equal(
    formatDiagnosticsProjectionFreshness({
      projection_name: 'run_detail',
      projection_version: 1,
      last_applied_event_seq: 42,
      status: 'needs_rebuild',
      rebuilt_at_ms: null,
      updated_at_ms: 5,
    }),
    'Needs rebuild at seq 42',
  );
});

test('formatDiagnosticsStatusLabel names every run status', () => {
  assert.equal(formatDiagnosticsStatusLabel('future'), 'Future');
  assert.equal(formatDiagnosticsStatusLabel('scheduled'), 'Scheduled');
  assert.equal(formatDiagnosticsStatusLabel('cancelled'), 'Cancelled');
});

test('diagnostic error presenters expose severity and run summary fields', () => {
  const run = createRunDetail();
  const summary = buildDiagnosticsRunErrorSummary({
    ...run,
    latest_error_event_id: 'error-event-1',
    latest_error_severity: 'fatal',
    latest_error_phase: 'runtime_model_load',
    latest_error_code: 'runtime_model_load_failed',
    latest_error_message: 'llama.cpp exited before model load completed',
    fatal_error_event_id: 'error-event-1',
  });

  assert.equal(summary?.eventId, 'error-event-1');
  assert.equal(summary?.fatal, true);
  assert.equal(summary?.phase, 'runtime_model_load');
  assert.equal(summary?.code, 'runtime_model_load_failed');
  assert.equal(formatDiagnosticErrorSeverity('fatal'), 'Fatal');
  assert.equal(formatDiagnosticErrorSeverity(null), 'Unknown');
  assert.equal(formatDiagnosticErrorPhase('runtime_model_load'), 'Runtime model load');
});

test('buildDiagnosticsFactRows uses projection fields without ledger parsing', () => {
  const rows = buildDiagnosticsFactRows(createRunDetail());

  assert.equal(rows.find((row) => row.label === 'Workflow')?.value, 'workflow-a');
  assert.equal(rows.find((row) => row.label === 'Workflow Version')?.value, '1.2.3');
  assert.equal(rows.find((row) => row.label === 'Queue Position')?.value, '1');
  assert.equal(rows.find((row) => row.label === 'Execution Session')?.value, 'exec-session-a');
  assert.equal(rows.find((row) => row.label === 'Selected Runtime')?.value, 'runtime-a');
  assert.equal(rows.find((row) => row.label === 'Selected Runtime Variant')?.value, 'runtime_a.cuda');
  assert.equal(rows.find((row) => row.label === 'Selected Backend')?.value, 'llama_cpp');
  assert.equal(
    rows.find((row) => row.label === 'Selected Model')?.value,
    'pumas://models/tiny-gguf',
  );
  assert.equal(rows.find((row) => row.label === 'Selected Task')?.value, 'text_generation');
  assert.equal(rows.find((row) => row.label === 'Selected Device Class')?.value, 'cuda');
  assert.equal(rows.find((row) => row.label === 'Selected Device')?.value, 'device-a');
  assert.equal(rows.find((row) => row.label === 'Selected Network Node')?.value, 'network-a');
  assert.equal(rows.find((row) => row.label === 'Estimated Queue Wait')?.value, '1.5 s');
  assert.equal(rows.find((row) => row.label === 'Model Cache')?.value, 'Model cache hit');
  assert.equal(rows.find((row) => row.label === 'Scheduler Reason')?.value, 'warm_session_reused');
  assert.equal(rows.find((row) => row.label === 'Timeline Events')?.value, '4');
  assert.equal(formatDiagnosticsModelCacheState('cache_miss'), 'Model cache miss');
  assert.equal(formatDiagnosticsModelCacheState(null), 'Unavailable');
});

test('buildDiagnosticsFacetSummary exposes comparison-ready run-list facets', () => {
  const activeRun = createRunDetail();
  const summary = buildDiagnosticsFacetSummary(activeRun, [
    activeRun,
    {
      workflow_run_id: 'run-2',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-1',
      workflow_semantic_version: '1.2.3',
      status: 'completed',
      scheduler_policy_id: 'policy-a',
      retention_policy_id: 'retention-b',
      selected_runtime_id: 'runtime-a',
      selected_runtime_variant_id: 'runtime_a.cuda',
      selected_backend_key: 'llama_cpp',
      selected_device_class: 'cuda',
      selected_device_id: 'device-b',
      selected_network_node_id: 'network-a',
      last_event_seq: 10,
      last_updated_at_ms: 20,
    },
    {
      workflow_run_id: 'run-3',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-2',
      workflow_semantic_version: '2.0.0',
      status: 'completed',
      scheduler_policy_id: 'policy-b',
      retention_policy_id: 'retention-a',
      selected_runtime_id: 'runtime-b',
      selected_runtime_variant_id: 'runtime_b.metal',
      selected_backend_key: 'pytorch',
      selected_device_class: 'metal',
      selected_device_id: 'device-a',
      selected_network_node_id: 'network-b',
      last_event_seq: 11,
      last_updated_at_ms: 30,
    },
    {
      workflow_run_id: 'run-4',
      workflow_id: 'workflow-b',
      workflow_version_id: 'wfver-other',
      workflow_semantic_version: '9.0.0',
      status: 'completed',
      last_event_seq: 12,
      last_updated_at_ms: 40,
    },
  ]);

  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.count, 2);
  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.total, 3);
  assert.equal(summary.rows.find((row) => row.label === 'Scheduler Policy')?.count, 2);
  assert.equal(summary.rows.find((row) => row.label === 'Retention Policy')?.count, 2);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Runtime')?.count, 2);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Runtime Variant')?.count, 2);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Backend')?.count, 2);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Device Class')?.count, 2);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Device')?.count, 2);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Network Node')?.count, 2);
  assert.match(summary.mixedVersionWarning ?? '', /2 workflow versions/);
});

test('buildDiagnosticsFacetSummary includes the active run when the run list is paged', () => {
  const activeRun = createRunDetail();
  const summary = buildDiagnosticsFacetSummary(activeRun, [
    {
      workflow_run_id: 'run-2',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-2',
      workflow_semantic_version: '2.0.0',
      status: 'completed',
      scheduler_policy_id: 'policy-b',
      retention_policy_id: 'retention-b',
      last_event_seq: 11,
      last_updated_at_ms: 30,
    },
  ]);

  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.count, 1);
  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.total, 2);
});

test('buildDiagnosticsFacetSummary prefers backend projection facets when provided', () => {
  const activeRun = createRunDetail();
  const summary = buildDiagnosticsFacetSummary(activeRun, [activeRun], [
    { facet_kind: 'workflow_version', facet_value: '1.2.3', run_count: 12 },
    { facet_kind: 'workflow_version', facet_value: '2.0.0', run_count: 3 },
    { facet_kind: 'status', facet_value: 'completed', run_count: 10 },
    { facet_kind: 'status', facet_value: 'failed', run_count: 5 },
    { facet_kind: 'scheduler_policy', facet_value: 'policy-a', run_count: 15 },
    { facet_kind: 'retention_policy', facet_value: 'retention-a', run_count: 15 },
    { facet_kind: 'selected_runtime', facet_value: 'runtime-a', run_count: 14 },
    { facet_kind: 'selected_runtime', facet_value: 'runtime-b', run_count: 1 },
    { facet_kind: 'selected_runtime_variant', facet_value: 'runtime_a.cuda', run_count: 13 },
    { facet_kind: 'selected_runtime_variant', facet_value: 'runtime_b.metal', run_count: 2 },
    { facet_kind: 'selected_backend', facet_value: 'llama_cpp', run_count: 12 },
    { facet_kind: 'selected_backend', facet_value: 'pytorch', run_count: 3 },
    { facet_kind: 'selected_device_class', facet_value: 'cuda', run_count: 13 },
    { facet_kind: 'selected_device_class', facet_value: 'metal', run_count: 2 },
    { facet_kind: 'selected_device', facet_value: 'device-a', run_count: 15 },
    { facet_kind: 'selected_network_node', facet_value: 'network-a', run_count: 15 },
  ]);

  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.count, 12);
  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.total, 15);
  assert.equal(summary.rows.find((row) => row.label === 'Status')?.total, 15);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Runtime')?.count, 14);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Runtime')?.total, 15);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Runtime Variant')?.count, 13);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Runtime Variant')?.total, 15);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Backend')?.count, 12);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Backend')?.total, 15);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Device Class')?.count, 13);
  assert.equal(summary.rows.find((row) => row.label === 'Selected Device Class')?.total, 15);
  assert.match(summary.mixedVersionWarning ?? '', /2 workflow versions/);
});

test('buildDiagnosticsRetentionSummaryRows formats typed projection counts', () => {
  const rows = buildDiagnosticsRetentionSummaryRows([
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
});

test('buildDiagnosticsExecutionFacetRows groups selected-run node status facts', () => {
  const rows = buildDiagnosticsExecutionFacetRows([
    createNodeStatus({ node_id: 'node-a', node_version: '1.0.0', runtime_version: '2.0.0', model_id: 'model-a' }),
    createNodeStatus({ node_id: 'node-b', node_version: '1.0.0', runtime_version: '2.1.0', model_id: 'model-a' }),
    createNodeStatus({
      node_id: 'node-c',
      node_version: '',
      runtime_id: null,
      runtime_version: null,
      model_id: null,
      model_version: null,
      status: 'running',
    }),
  ]);

  assert.deepEqual(
    rows.filter((row) => row.label === 'Node Version'),
    [
      { label: 'Node Version', value: '1.0.0', count: 2 },
      { label: 'Node Version', value: 'Unversioned', count: 1 },
    ],
  );
  assert.deepEqual(
    rows.filter((row) => row.label === 'Runtime Version'),
    [
      { label: 'Runtime Version', value: '2.0.0', count: 1 },
      { label: 'Runtime Version', value: '2.1.0', count: 1 },
      { label: 'Runtime Version', value: 'Unversioned', count: 1 },
    ],
  );
  assert.deepEqual(
    rows.filter((row) => row.label === 'Model'),
    [
      { label: 'Model', value: 'model-a', count: 2 },
      { label: 'Model', value: 'Unassigned', count: 1 },
    ],
  );
  assert.deepEqual(
    rows.filter((row) => row.label === 'Node Status'),
    [
      { label: 'Node Status', value: 'completed', count: 2 },
      { label: 'Node Status', value: 'running', count: 1 },
    ],
  );
});

test('diagnostics execution filters expose and apply node runtime model values', () => {
  const nodes = [
    createNodeStatus({ node_id: 'node-a', node_version: '1.0.0', runtime_id: 'runtime-a', model_id: 'model-a' }),
    createNodeStatus({
      node_id: 'node-b',
      node_version: '1.1.0',
      runtime_id: 'runtime-b',
      runtime_version: '2.1.0',
      model_id: 'model-b',
      model_version: '3.1.0',
      status: 'running',
    }),
    createNodeStatus({
      node_id: 'node-c',
      node_version: null,
      runtime_id: null,
      runtime_version: null,
      model_id: null,
      model_version: null,
      status: 'waiting',
    }),
  ];

  const options = buildDiagnosticsExecutionFilterOptions(nodes);
  assert.deepEqual(options.statuses, ['completed', 'running', 'waiting']);
  assert.deepEqual(options.nodeVersions, ['1.0.0', '1.1.0', 'Unversioned']);
  assert.deepEqual(options.runtimes, ['runtime-a', 'runtime-b', 'Unassigned']);
  assert.deepEqual(options.runtimeVersions, ['2.0.0', '2.1.0', 'Unversioned']);
  assert.deepEqual(options.models, ['model-a', 'model-b', 'Unassigned']);
  assert.deepEqual(options.modelVersions, ['3.0.0', '3.1.0', 'Unversioned']);

  const filteredNodes = filterDiagnosticsExecutionNodes(nodes, {
    ...DEFAULT_DIAGNOSTICS_EXECUTION_FILTERS,
    runtime: 'runtime-b',
    modelVersion: '3.1.0',
  });

  assert.deepEqual(
    filteredNodes.map((node) => node.node_id),
    ['node-b'],
  );
});

test('diagnostics comparison filters expose available projection values', () => {
  const activeRun = createRunDetail();
  const options = buildDiagnosticsComparisonFilterOptions(activeRun, [
    activeRun,
    createRunListPeer({ workflow_run_id: 'run-2', status: 'completed', scheduler_policy_id: 'policy-b' }),
    createRunListPeer({
      workflow_run_id: 'run-3',
      workflow_version_id: 'wfver-2',
      workflow_semantic_version: '2.0.0',
      bucket_id: null,
      client_id: null,
      client_session_id: null,
      retention_policy_id: null,
      selected_runtime_id: null,
      selected_runtime_variant_id: null,
      selected_backend_key: null,
      selected_device_class: null,
      selected_device_id: null,
      selected_network_node_id: null,
      accepted_at_ms: null,
    }),
    createRunListPeer({ workflow_run_id: 'run-4', workflow_id: 'workflow-b', status: 'failed' }),
  ]);

  assert.deepEqual(options.workflowVersions, ['1.2.3', '2.0.0']);
  assert.deepEqual(options.statuses, ['completed', 'running']);
  assert.deepEqual(options.schedulerPolicies, ['policy-a', 'policy-b']);
  assert.deepEqual(options.retentionPolicies, ['retention-a', 'Unassigned']);
  assert.deepEqual(options.selectedRuntimes, ['runtime-a', 'Unassigned']);
  assert.deepEqual(options.selectedRuntimeVariants, ['runtime_a.cuda', 'Unassigned']);
  assert.deepEqual(options.selectedBackends, ['llama_cpp', 'Unassigned']);
  assert.deepEqual(options.selectedDeviceClasses, ['cuda', 'Unassigned']);
  assert.deepEqual(options.selectedDevices, ['device-a', 'Unassigned']);
  assert.deepEqual(options.selectedNetworkNodes, ['network-a', 'Unassigned']);
  assert.deepEqual(options.clients, ['client-a', 'Unassigned']);
  assert.deepEqual(options.clientSessions, ['session-a', 'Unassigned']);
  assert.deepEqual(options.buckets, ['bucket-a', 'Unassigned']);
  assert.deepEqual(options.acceptedDates, ['1970-01-01', '1970-01-02', 'Unassigned']);
});

test('diagnostics comparison filters keep selected run and filter peer rows', () => {
  const activeRun = createRunDetail();
  const filteredRuns = filterDiagnosticsComparisonRuns(
    activeRun,
    [
      activeRun,
      createRunListPeer({ workflow_run_id: 'run-2', status: 'completed', scheduler_policy_id: 'policy-b' }),
      createRunListPeer({
        workflow_run_id: 'run-3',
        workflow_version_id: 'wfver-2',
        workflow_semantic_version: '2.0.0',
        status: 'completed',
        scheduler_policy_id: 'policy-b',
        selected_runtime_id: 'runtime-b',
        selected_runtime_variant_id: 'runtime_b.metal',
        selected_backend_key: 'pytorch',
        selected_device_class: 'metal',
      }),
      createRunListPeer({ workflow_run_id: 'run-4', workflow_id: 'workflow-b', status: 'completed' }),
    ],
    {
      ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
      workflowVersion: '1.2.3',
      status: 'completed',
      schedulerPolicy: 'policy-b',
      selectedRuntime: 'runtime-a',
      selectedRuntimeVariant: 'runtime_a.cuda',
      selectedBackend: 'llama_cpp',
      selectedDeviceClass: 'cuda',
      acceptedDate: '1970-01-02',
    },
  );

  assert.deepEqual(
    filteredRuns.map((run) => run.workflow_run_id),
    ['run-1', 'run-2'],
  );
  assert.equal(hasActiveDiagnosticsComparisonFilters(DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS), false);
  assert.equal(
    hasActiveDiagnosticsComparisonFilters({
      ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
      status: 'completed',
    }),
    true,
  );
});

test('diagnostics comparison filters support accepted date ranges', () => {
  const activeRun = createRunDetail();
  const filteredRuns = filterDiagnosticsComparisonRuns(
    activeRun,
    [
      activeRun,
      createRunListPeer({ workflow_run_id: 'run-2', accepted_at_ms: Date.parse('2026-04-01T12:00:00.000Z') }),
      createRunListPeer({ workflow_run_id: 'run-3', accepted_at_ms: Date.parse('2026-04-10T12:00:00.000Z') }),
      createRunListPeer({ workflow_run_id: 'run-4', accepted_at_ms: null }),
    ],
    {
      ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
      acceptedFromDate: '2026-04-01',
      acceptedToDate: '2026-04-05',
    },
  );

  assert.deepEqual(
    filteredRuns.map((run) => run.workflow_run_id),
    ['run-1', 'run-2'],
  );
  assert.equal(
    hasActiveDiagnosticsComparisonFilters({
      ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
      acceptedFromDate: '2026-04-01',
    }),
    true,
  );
});

test('timeline label helpers format typed projection enums and payload presence', () => {
  assert.equal(formatDiagnosticEventKind('scheduler_queue_placement'), 'scheduler queue placement');
  assert.equal(
    formatDiagnosticEventKind('inference_execution_diagnostic_observed'),
    'inference execution diagnostic observed',
  );
  assert.equal(formatDiagnosticSourceComponent('node_execution'), 'node execution');
  assert.equal(hasTimelinePayload({ payload_json: '{}' }), false);
  assert.equal(hasTimelinePayload({ payload_json: '{"decision":"delay"}' }), true);
});

// --- Redesigned page presenters ----------------------------------------------

function createTimelineEvent(overrides: Partial<SchedulerTimelineProjectionRecord>): SchedulerTimelineProjectionRecord {
  return {
    event_seq: 1,
    event_id: 'event-1',
    event_kind: 'scheduler_queue_placement',
    source_component: 'scheduler',
    occurred_at_ms: new Date(2026, 10, 3, 19, 35, 2, 114).getTime(),
    recorded_at_ms: 1,
    workflow_run_id: 'run-1',
    workflow_id: 'workflow-a',
    summary: 'placed on cuda:1',
    detail: null,
    error_severity: null,
    error_phase: null,
    related_event_ids: [],
    payload_json: '{}',
    ...overrides,
  };
}

function createFailedRun(overrides: Partial<RunDetailProjectionRecord> = {}): RunDetailProjectionRecord {
  return {
    ...createRunDetail(),
    status: 'failed',
    started_at_ms: new Date(2026, 10, 3, 19, 35, 20).getTime(),
    completed_at_ms: new Date(2026, 10, 3, 19, 35, 42).getTime(),
    duration_ms: 22_000,
    latest_error_event_id: 'error-event-1',
    latest_error_severity: 'fatal',
    latest_error_phase: 'runtime_model_load',
    latest_error_code: 'runtime_model_load_failed',
    latest_error_message: 'CUDA out of memory on cuda:1',
    fatal_error_event_id: 'error-event-1',
    error_count: 1,
    warning_count: 0,
    ...overrides,
  };
}

const NOW = new Date(2026, 10, 3, 20, 0, 0).getTime();

test('time presenters format clock, elapsed and log times', () => {
  assert.equal(formatDiagnosticsClock(new Date(2026, 10, 3, 19, 35, 20).getTime(), NOW), '19:35:20');
  assert.equal(formatDiagnosticsClock(new Date(2026, 10, 1, 8, 5, 0).getTime(), NOW), '2026-11-01 08:05:00');
  assert.equal(formatDiagnosticsClock(null, NOW), 'Not recorded');
  assert.equal(formatDiagnosticsElapsed(640), '640 ms');
  assert.equal(formatDiagnosticsElapsed(2_500), '2.5 s');
  assert.equal(formatDiagnosticsElapsed(22_000), '22 s');
  assert.equal(formatDiagnosticsElapsed(109_000), '1m 49s');
  assert.equal(formatDiagnosticsElapsed(2 * 3_600_000 + 5 * 60_000), '2h 05m');
  assert.equal(formatDiagnosticsLogTime(new Date(2026, 10, 3, 19, 35, 2, 114).getTime()), '19:35:02.114');
});

test('buildDiagnosticsDimensionLine summarises status, time and errors', () => {
  const failedNode = createNodeStatus({ node_id: 'load_model', status: 'failed' });
  assert.equal(
    buildDiagnosticsDimensionLine(createFailedRun(), [createNodeStatus({}), failedNode], NOW),
    'run-1, failed 19:35:42 after 22 s, 1 error in load_model',
  );
  assert.equal(
    buildDiagnosticsDimensionLine({ ...createRunDetail(), started_at_ms: new Date(2026, 10, 3, 19, 50, 0).getTime() }, [], NOW),
    'run-1, running since 19:50:00',
  );
  assert.equal(
    buildDiagnosticsDimensionLine({ ...createRunDetail(), status: 'queued', scheduler_queue_position: 2 }, [], NOW),
    'run-1, queued at position 2',
  );
  assert.equal(
    buildDiagnosticsDimensionLine(
      { ...createRunDetail(), status: 'delayed', scheduler_reason: 'needs 8 GB on cuda:0', warning_count: 2 },
      [],
      NOW,
    ),
    'run-1, delayed: needs 8 GB on cuda:0, 2 warnings',
  );
});

test('placement and client labels say what is assigned', () => {
  const run = createRunDetail();
  assert.equal(buildDiagnosticsPlacementLabel(run), 'runtime-a on device-a');
  assert.equal(buildDiagnosticsPlacementLabel({ ...run, selected_device_id: null }), 'runtime-a, device not assigned yet');
  assert.equal(
    buildDiagnosticsPlacementLabel({ ...run, selected_device_id: null, selected_runtime_id: null }),
    'Not placed yet',
  );
  assert.equal(buildDiagnosticsClientLabel(run), 'from client-a, session session-a');
  assert.equal(buildDiagnosticsClientLabel({ ...run, client_id: null, client_session_id: null }), null);
});

test('buildDiagnosticsFactSections places every run-detail fact exactly once', () => {
  const run = createFailedRun({ terminal_error: 'worker exited with code 1' });
  const factLabels = buildDiagnosticsFactRows(run).map((row) => row.label);
  assert.deepEqual([...diagnosticsFactLayoutLabels()].sort(), [...factLabels].sort());

  const sections = buildDiagnosticsFactSections(run, NOW);
  const placed = [...sections.timing, ...sections.placement, ...sections.retention, ...sections.more];
  const factValues = buildDiagnosticsFactRows(run).map((row) => row.value);
  for (const value of factValues) {
    assert.ok(placed.some((row) => row.value === value), `fact value ${value} is placed`);
  }
  // Timing leads with the lifecycle times; the five lifecycle rows plus three fact rows.
  assert.deepEqual(
    sections.timing.map((row) => row.key),
    ['Accepted', 'Queued', 'Started', 'Finished', 'Duration', 'Estimated wait', 'Estimated duration', 'Estimate confidence'],
  );
  assert.equal(sections.timing.find((row) => row.key === 'Duration')?.value, '22 s');
  assert.deepEqual(
    sections.placement.map((row) => row.key),
    ['Runtime', 'Backend', 'Device', 'Model', 'Model cache', 'Policy', 'Bucket', 'Queue position', 'Priority', 'Reason'],
  );
  assert.equal(sections.placement.find((row) => row.key === 'Runtime')?.value, 'runtime-a');
  const running = { ...createRunDetail(), started_at_ms: NOW - 252_000 };
  assert.equal(buildDiagnosticsFactSections(running, NOW).timing.find((row) => row.key === 'Duration')?.value, 'Running for 4m 12s');
  assert.deepEqual(sections.retention, [{ key: 'Policy', value: 'retention-a', mono: true }]);
  assert.equal(sections.more.find((row) => row.key === 'Latest error event')?.value, 'error-event-1');
  assert.equal(sections.more.find((row) => row.key === 'Terminal error')?.value, 'worker exited with code 1');
  assert.equal(sections.more.some((row) => row.key === 'Fatal error event'), false);
  for (const section of Object.values(sections)) {
    const keys = section.map((row: { key: string }) => row.key);
    assert.equal(new Set(keys).size, keys.length, 'keys are unique within a section');
  }
});

test('retention and projection rows keep every count and cursor', () => {
  assert.deepEqual(
    buildDiagnosticsRetentionRows([
      { retention_state: 'metadata_only', artifact_count: 1 },
      { retention_state: 'retained', artifact_count: 3 },
    ]),
    [
      { key: 'Payload retained', value: '3 artifacts' },
      { key: 'Metadata retained only', value: '1 artifact' },
    ],
  );
  const state = {
    projection_name: 'run_detail',
    projection_version: 1,
    last_applied_event_seq: 42,
    status: 'current' as const,
    rebuilt_at_ms: null,
    updated_at_ms: 5,
  };
  assert.deepEqual(
    buildDiagnosticsProjectionRows({ runDetail: state, timeline: null, runList: state, nodeStatus: state, io: state }).map(
      (row) => `${row.key}: ${row.value}`,
    ),
    [
      'Run detail: Current at seq 42',
      'Scheduler timeline: Projection unavailable',
      'Run list: Current at seq 42',
      'Node status: Current at seq 42',
      'I/O retention: Current at seq 42',
    ],
  );
});

test('node status bars and breakdown group the node projection', () => {
  const nodes = [
    createNodeStatus({ node_id: 'a', status: 'completed' }),
    createNodeStatus({ node_id: 'b', status: 'failed', runtime_id: 'runtime-b' }),
    createNodeStatus({ node_id: 'c', status: 'queued' }),
    createNodeStatus({ node_id: 'd', status: 'queued' }),
  ];
  assert.deepEqual(buildDiagnosticsNodeStatusBars(nodes), [
    { status: 'queued', label: 'Queued', count: 2, percent: 50 },
    { status: 'completed', label: 'Completed', count: 1, percent: 25 },
    { status: 'failed', label: 'Failed', count: 1, percent: 25 },
  ]);
  assert.deepEqual(buildDiagnosticsNodeStatusBars([]), []);

  const groups = groupDiagnosticsExecutionFacetRows(buildDiagnosticsExecutionFacetRows(nodes));
  assert.equal(groups.some((group) => group.label === 'Node status'), false);
  assert.equal(diagnosticsSentenceCase('Selected Runtime Variant'), 'Selected runtime variant');
  assert.deepEqual(
    groups.find((group) => group.label === 'Runtime')?.rows.map((row) => `${row.value}:${row.count}`),
    ['runtime-a:3', 'runtime-b:1'],
  );
});

test('filter chips name each applied execution and comparison filter', () => {
  assert.deepEqual(buildDiagnosticsExecutionFilterChips(DEFAULT_DIAGNOSTICS_EXECUTION_FILTERS), []);
  assert.deepEqual(
    buildDiagnosticsExecutionFilterChips({ ...DEFAULT_DIAGNOSTICS_EXECUTION_FILTERS, runtime: 'runtime-a', status: 'failed' }),
    [
      { field: 'status', label: 'Node status failed' },
      { field: 'runtime', label: 'Runtime runtime-a' },
    ],
  );
  assert.deepEqual(
    buildDiagnosticsComparisonFilterChips({
      ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
      status: 'failed',
      selectedDevice: 'cuda:1',
      acceptedFromDate: '2026-11-01',
    }),
    [
      { field: 'status', label: 'Status Failed' },
      { field: 'selectedDevice', label: 'Device cuda:1' },
      { field: 'acceptedFromDate', label: 'Accepted from 2026-11-01' },
    ],
  );
});

test('comparison shortcuts toggle version, device and recent presets', () => {
  const run = createRunDetail();
  const shortcuts = buildDiagnosticsComparisonShortcuts(run, DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS, NOW);
  assert.deepEqual(
    shortcuts.map((shortcut) => [shortcut.label, shortcut.pressed]),
    [
      ['Only v1.2.3', false],
      ['Only device-a', false],
      ['Last 7 days', false],
    ],
  );
  const versionOn = shortcuts[0].toggled;
  assert.equal(versionOn.workflowVersion, '1.2.3');
  const again = buildDiagnosticsComparisonShortcuts(run, versionOn, NOW);
  assert.equal(again[0].pressed, true);
  assert.equal(again[0].toggled.workflowVersion, 'all');

  const recentOn = shortcuts[2].toggled;
  assert.equal(recentOn.acceptedFromDate, diagnosticsRecentFromDate(NOW));
  assert.equal(buildDiagnosticsComparisonShortcuts(run, recentOn, NOW)[2].pressed, true);
  assert.equal(
    buildDiagnosticsComparisonShortcuts({ ...run, selected_device_id: null }, DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS, NOW).some(
      (shortcut) => shortcut.id === 'device',
    ),
    false,
  );
});

test('buildDiagnosticsComparisonSummary compares failures and durations with peers', () => {
  const run = createFailedRun();
  const peers = [
    createRunListPeer({ workflow_run_id: 'p1', status: 'failed', latest_error_code: 'runtime_model_load_failed' }),
    createRunListPeer({ workflow_run_id: 'p2', status: 'completed', duration_ms: 100_000 }),
    createRunListPeer({ workflow_run_id: 'p3', status: 'completed', duration_ms: 118_000 }),
    createRunListPeer({ workflow_run_id: 'p4', status: 'completed', duration_ms: 109_000 }),
  ];
  const failed = buildDiagnosticsComparisonSummary(run, [run, ...peers]);
  assert.equal(failed.failed, '2 of 5');
  assert.equal(failed.median, '1m 49s');
  assert.equal(failed.note, '1 of the other 4 compared runs also failed, 1 with runtime_model_load_failed.');

  const completed = { ...createRunDetail(), status: 'completed' as const, duration_ms: 90_000 };
  assert.equal(
    buildDiagnosticsComparisonSummary(completed, [completed, ...peers]).note,
    'This run took 1m 30s; the median completed run took 1m 49s.',
  );
  const alone = buildDiagnosticsComparisonSummary(createRunDetail(), [createRunDetail()]);
  assert.equal(alone.median, 'None yet');
  assert.equal(alone.note, 'No other runs of workflow-a in the run list match these filters.');
});

test('buildDiagnosticsLogLines turns timeline events and node errors into log lines', () => {
  const events = [
    createTimelineEvent({ event_seq: 3, event_id: 'event-3', event_kind: 'diagnostic_error_occurred', source_component: 'node_execution', summary: 'failed', error_severity: null, error_phase: 'runtime_model_load' }),
    createTimelineEvent({ event_seq: 1, event_id: 'event-1', payload_json: '{"decision":"place"}' }),
    createTimelineEvent({ event_seq: 2, event_id: 'event-2', event_kind: 'run_started', source_component: 'runtime', summary: 'memory pressure', error_severity: 'warning' }),
  ];
  const nodes = [
    createNodeStatus({ node_id: 'load_model', status: 'failed', canonical_error_event_id: 'event-3', error_code: 'runtime_model_load_failed', error_severity: 'fatal' }),
  ];
  const lines = buildDiagnosticsLogLines(events, nodes);
  assert.deepEqual(lines.map((line) => line.seq), [1, 2, 3]);
  assert.equal(lines[0].kind, 'scheduler queue placement');
  assert.equal(lines[0].source, 'scheduler, seq 1, payload captured');
  assert.equal(lines[0].time, '19:35:02.114');
  assert.equal(lines[0].domId, 'diagnostics-event-1');
  assert.equal(lines[1].level, 'warn');
  assert.equal(lines[2].kind, 'load_model');
  assert.equal(lines[2].level, 'error');
  assert.equal(lines[2].nodeId, 'load_model');
  assert.equal(lines[2].message, 'failed, phase runtime model load, runtime_model_load_failed');

  assert.deepEqual(
    buildDiagnosticsLevelOptions(lines).map((option) => `${option.label} ${option.count}`),
    ['All 3', 'Warnings 1', 'Errors 1'],
  );
  const only = (level: 'all' | 'warn' | 'error', source: 'all' | 'scheduler' | 'runtime' | 'nodes', query = '') =>
    filterDiagnosticsLogLines(lines, { level, source, query }).map((line) => line.eventId);
  assert.deepEqual(only('error', 'all'), ['event-3']);
  assert.deepEqual(only('warn', 'all'), ['event-2']);
  assert.deepEqual(only('all', 'nodes'), ['event-3']);
  assert.deepEqual(only('all', 'runtime'), ['event-2']);
  assert.deepEqual(only('all', 'all', 'MEMORY'), ['event-2']);
  assert.deepEqual(only('all', 'all', 'event-1'), ['event-1']);
});

test('node errors stay reachable when the timeline lacks their event', () => {
  const nodes = [
    createNodeStatus({ node_id: 'in-timeline', status: 'failed', error_event_id: 'event-1', error: 'boom' }),
    createNodeStatus({ node_id: 'missing', status: 'failed', error_event_id: 'event-9', error: 'boom' }),
    createNodeStatus({ node_id: 'unlinked', status: 'failed', error_severity: 'error' }),
    createNodeStatus({ node_id: 'fine', status: 'completed' }),
  ];
  assert.deepEqual(
    buildDiagnosticsNodeErrorsOutsideTimeline(nodes, [createTimelineEvent({ event_id: 'event-1' })]).map((node) => node.node_id),
    ['missing', 'unlinked'],
  );
});

test('resolveDiagnosticsFocus selects the focused line and the level that shows it', () => {
  const events = [
    createTimelineEvent({ event_seq: 1, event_id: 'event-1' }),
    createTimelineEvent({ event_seq: 2, event_id: 'event-2', error_severity: 'fatal' }),
  ];
  const nodes = [createNodeStatus({ node_id: 'load_model', status: 'failed', error_event_id: 'event-2' })];
  const lines = buildDiagnosticsLogLines(events, nodes);
  const focus = { workflow_run_id: 'run-1', diagnostic_event_id: 'event-2', node_id: null, requested_at_ms: 7 };

  assert.deepEqual(resolveDiagnosticsFocus(focus, 'run-1', lines, nodes), {
    eventId: 'event-2',
    nodeId: 'load_model',
    lineDomId: 'diagnostics-event-2',
    level: 'error',
  });
  assert.equal(resolveDiagnosticsFocus(focus, 'run-2', lines, nodes), null);
  assert.equal(resolveDiagnosticsFocus(null, 'run-1', lines, nodes), null);
  assert.deepEqual(
    resolveDiagnosticsFocus({ ...focus, diagnostic_event_id: null, node_id: 'load_model' }, 'run-1', lines, nodes)?.lineDomId,
    'diagnostics-event-2',
  );
  assert.equal(resolveDiagnosticsFocus({ ...focus, diagnostic_event_id: 'event-1' }, 'run-1', lines, nodes)?.level, 'all');
  assert.deepEqual(resolveDiagnosticsFocus({ ...focus, diagnostic_event_id: 'event-9' }, 'run-1', lines, nodes), {
    eventId: 'event-9',
    nodeId: null,
    lineDomId: null,
    level: 'all',
  });
  assert.equal(diagnosticsFocusKey(focus), 'run-1:7');
  assert.notEqual(diagnosticsFocusKey({ ...focus, requested_at_ms: 8 }), diagnosticsFocusKey(focus));
});

test('explainDiagnosticsError explains known codes and restates unknown ones', () => {
  const known = explainDiagnosticsError({
    code: 'runtime_model_load_failed',
    message: 'CUDA out of memory on cuda:1',
    phase: 'runtime_model_load',
    nodeId: 'load_model',
  });
  assert.equal(known.title, 'Model failed to load in load_model');
  assert.equal(known.explanation, 'The runtime could not load the model for this run: CUDA out of memory on cuda:1.');
  assert.match(known.suggestion, /^Try: /);

  const unknown = explainDiagnosticsError({ code: 'something_new', message: 'disk quota exceeded.', phase: 'artifact' });
  assert.equal(unknown.title, 'Artifact failed');
  assert.equal(unknown.explanation, 'The run reported: disk quota exceeded.');
  assert.match(unknown.suggestion, /^Try: show the error in the log/);

  const bare = explainDiagnosticsError({ code: null, message: '  ', phase: 'unassigned' });
  assert.equal(bare.title, 'The run reported an error');
  assert.equal(bare.explanation, 'No error message was projected for this run.');
});

test('buildDiagnosticsErrorCard covers fatal errors, warnings and terminal-only errors', () => {
  const nodes = [createNodeStatus({ node_id: 'load_model', status: 'failed', canonical_error_event_id: 'error-event-1' })];
  const card = buildDiagnosticsErrorCard(createFailedRun(), nodes);
  assert.equal(card?.kind, 'error');
  assert.equal(card?.title, 'Model failed to load in load_model');
  assert.equal(card?.codeLine, 'runtime_model_load_failed, phase runtime model load, fatal');
  assert.equal(card?.eventId, 'error-event-1');
  assert.equal(card?.nodeId, 'load_model');

  const warning = buildDiagnosticsErrorCard(
    {
      ...createRunDetail(),
      latest_error_event_id: 'warn-1',
      latest_error_severity: 'warning',
      latest_error_code: 'projection_failed',
      latest_error_message: 'projection lagging',
    },
    [],
  );
  assert.equal(warning?.kind, 'warning');

  const terminalOnly = buildDiagnosticsErrorCard({ ...createRunDetail(), status: 'failed', terminal_error: 'worker exited' }, []);
  assert.equal(terminalOnly?.kind, 'error');
  assert.equal(terminalOnly?.eventId, null);
  assert.equal(terminalOnly?.explanation, 'The run reported: worker exited.');
  assert.equal(buildDiagnosticsErrorCard(createRunDetail(), []), null);
});

test('createDiagnosticsRefreshGate applies only the newest refresh and nothing after close', () => {
  const gate = createDiagnosticsRefreshGate();
  const first = gate.begin();
  const second = gate.begin();
  assert.equal(gate.isCurrent(first), false);
  assert.equal(gate.isCurrent(second), true);
  gate.close();
  assert.equal(gate.isCurrent(second), false);
  assert.equal(gate.isCurrent(gate.begin()), false);

  const other = createDiagnosticsRefreshGate();
  assert.equal(other.isCurrent(other.begin()), true, 'gates are scoped to their owner');
});

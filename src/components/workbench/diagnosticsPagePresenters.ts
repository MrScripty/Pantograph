import type {
  DiagnosticErrorSeverity,
  DiagnosticEventKind,
  DiagnosticEventSourceComponent,
  IoArtifactRetentionState,
  IoArtifactRetentionSummaryRecord,
  NodeExecutionProjectionStatus,
  NodeStatusProjectionRecord,
  ProjectionStateRecord,
  RunDetailProjectionRecord,
  RunListFacetKind,
  RunListFacetRecord,
  RunListProjectionRecord,
  SchedulerTimelineProjectionRecord,
} from '../../services/diagnostics/types';
import { formatProjectionFreshnessState } from './projectionFreshness.ts';
import type { LogLevel } from './ui/statusGlyph.ts';
import type { KeyValueRow, SegmentOption } from './ui/types.ts';

export interface DiagnosticsFactRow {
  label: string;
  value: string;
  mono: boolean;
}

export interface DiagnosticsFacetRow {
  label: string;
  value: string;
  count: number;
  total: number;
}

export interface DiagnosticsFacetSummary {
  rows: DiagnosticsFacetRow[];
  mixedVersionWarning: string | null;
}

export interface DiagnosticsRetentionSummaryRow {
  label: string;
  count: number;
}

export interface DiagnosticsExecutionFacetRow {
  label: string;
  value: string;
  count: number;
}

export interface DiagnosticsRunErrorSummary {
  eventId: string;
  severity: DiagnosticErrorSeverity;
  phase: string;
  code: string;
  message: string;
  fatal: boolean;
}

export interface DiagnosticsExecutionFilters {
  status: string;
  nodeVersion: string;
  runtime: string;
  runtimeVersion: string;
  model: string;
  modelVersion: string;
}

export interface DiagnosticsExecutionFilterOptions {
  statuses: NodeStatusProjectionRecord['status'][];
  nodeVersions: string[];
  runtimes: string[];
  runtimeVersions: string[];
  models: string[];
  modelVersions: string[];
}

export interface DiagnosticsComparisonFilters {
  workflowVersion: string;
  status: string;
  schedulerPolicy: string;
  retentionPolicy: string;
  selectedRuntime: string;
  selectedRuntimeVariant: string;
  selectedBackend: string;
  selectedDeviceClass: string;
  selectedDevice: string;
  selectedNetworkNode: string;
  client: string;
  clientSession: string;
  bucket: string;
  acceptedDate: string;
  acceptedFromDate: string;
  acceptedToDate: string;
}

export interface DiagnosticsComparisonFilterOptions {
  workflowVersions: string[];
  statuses: RunListProjectionRecord['status'][];
  schedulerPolicies: string[];
  retentionPolicies: string[];
  selectedRuntimes: string[];
  selectedRuntimeVariants: string[];
  selectedBackends: string[];
  selectedDeviceClasses: string[];
  selectedDevices: string[];
  selectedNetworkNodes: string[];
  clients: string[];
  clientSessions: string[];
  buckets: string[];
  acceptedDates: string[];
}

export const DIAGNOSTICS_FILTER_ALL = 'all';

export const DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS: DiagnosticsComparisonFilters = {
  workflowVersion: DIAGNOSTICS_FILTER_ALL,
  status: DIAGNOSTICS_FILTER_ALL,
  schedulerPolicy: DIAGNOSTICS_FILTER_ALL,
  retentionPolicy: DIAGNOSTICS_FILTER_ALL,
  selectedRuntime: DIAGNOSTICS_FILTER_ALL,
  selectedRuntimeVariant: DIAGNOSTICS_FILTER_ALL,
  selectedBackend: DIAGNOSTICS_FILTER_ALL,
  selectedDeviceClass: DIAGNOSTICS_FILTER_ALL,
  selectedDevice: DIAGNOSTICS_FILTER_ALL,
  selectedNetworkNode: DIAGNOSTICS_FILTER_ALL,
  client: DIAGNOSTICS_FILTER_ALL,
  clientSession: DIAGNOSTICS_FILTER_ALL,
  bucket: DIAGNOSTICS_FILTER_ALL,
  acceptedDate: DIAGNOSTICS_FILTER_ALL,
  acceptedFromDate: '',
  acceptedToDate: '',
};

export const DEFAULT_DIAGNOSTICS_EXECUTION_FILTERS: DiagnosticsExecutionFilters = {
  status: DIAGNOSTICS_FILTER_ALL,
  nodeVersion: DIAGNOSTICS_FILTER_ALL,
  runtime: DIAGNOSTICS_FILTER_ALL,
  runtimeVersion: DIAGNOSTICS_FILTER_ALL,
  model: DIAGNOSTICS_FILTER_ALL,
  modelVersion: DIAGNOSTICS_FILTER_ALL,
};

export const EMPTY_DIAGNOSTICS_COMPARISON_FILTER_OPTIONS: DiagnosticsComparisonFilterOptions = {
  workflowVersions: [],
  statuses: [],
  schedulerPolicies: [],
  retentionPolicies: [],
  selectedRuntimes: [],
  selectedRuntimeVariants: [],
  selectedBackends: [],
  selectedDeviceClasses: [],
  selectedDevices: [],
  selectedNetworkNodes: [],
  clients: [],
  clientSessions: [],
  buckets: [],
  acceptedDates: [],
};

export const EMPTY_DIAGNOSTICS_EXECUTION_FILTER_OPTIONS: DiagnosticsExecutionFilterOptions = {
  statuses: [],
  nodeVersions: [],
  runtimes: [],
  runtimeVersions: [],
  models: [],
  modelVersions: [],
};

export function formatDiagnosticsDuration(
  value: number | null | undefined,
  status: RunDetailProjectionRecord['status'],
): string {
  if (value === null || value === undefined) {
    if (status === 'running') {
      return 'Running';
    }
    if (
      status === 'future' ||
      status === 'scheduled' ||
      status === 'queued' ||
      status === 'accepted' ||
      status === 'delayed'
    ) {
      return 'Pending';
    }
    return 'Unavailable';
  }
  if (value < 1_000) {
    return `${Math.round(value)} ms`;
  }
  return `${(value / 1_000).toFixed(1)} s`;
}

export function formatDiagnosticsProjectionFreshness(state: ProjectionStateRecord | null): string {
  return formatProjectionFreshnessState(state);
}

export function formatDiagnosticsStatusLabel(status: RunDetailProjectionRecord['status']): string {
  switch (status) {
    case 'accepted':
      return 'Accepted';
    case 'future':
      return 'Future';
    case 'scheduled':
      return 'Scheduled';
    case 'queued':
      return 'Queued';
    case 'delayed':
      return 'Delayed';
    case 'running':
      return 'Running';
    case 'completed':
      return 'Completed';
    case 'failed':
      return 'Failed';
    case 'cancelled':
      return 'Cancelled';
  }
}

export function formatDiagnosticErrorSeverity(severity: DiagnosticErrorSeverity | null | undefined): string {
  switch (severity) {
    case 'fatal':
      return 'Fatal';
    case 'error':
      return 'Error';
    case 'warning':
      return 'Warning';
    case null:
    case undefined:
      return 'Unknown';
  }
}

export function buildDiagnosticsRunErrorSummary(
  run: RunDetailProjectionRecord,
): DiagnosticsRunErrorSummary | null {
  if (!run.latest_error_event_id && !run.latest_error_message && !run.fatal_error_event_id) {
    return null;
  }
  const severity = run.latest_error_severity ?? (run.fatal_error_event_id ? 'fatal' : 'error');
  return {
    eventId: run.latest_error_event_id ?? run.fatal_error_event_id ?? 'unassigned',
    severity,
    phase: run.latest_error_phase ?? 'unassigned',
    code: run.latest_error_code ?? 'unknown_error',
    message: run.latest_error_message ?? run.terminal_error ?? 'No diagnostic error message was projected.',
    fatal: Boolean(run.fatal_error_event_id) || severity === 'fatal',
  };
}

export function buildDiagnosticsFactRows(run: RunDetailProjectionRecord): DiagnosticsFactRow[] {
  return [
    { label: 'Workflow', value: run.workflow_id, mono: true },
    {
      label: 'Workflow Version',
      value: run.workflow_semantic_version ?? run.workflow_version_id ?? 'Unversioned',
      mono: true,
    },
    { label: 'Run Snapshot', value: run.workflow_run_snapshot_id ?? 'Unavailable', mono: true },
    {
      label: 'Presentation Revision',
      value: run.workflow_presentation_revision_id ?? 'Unavailable',
      mono: true,
    },
    { label: 'Client', value: run.client_id ?? 'Unavailable', mono: true },
    { label: 'Client Session', value: run.client_session_id ?? 'Unavailable', mono: true },
    {
      label: 'Execution Session',
      value: run.workflow_execution_session_id ?? 'Unavailable',
      mono: true,
    },
    { label: 'Bucket', value: run.bucket_id ?? 'Default', mono: true },
    { label: 'Scheduler Policy', value: run.scheduler_policy_id ?? 'Default', mono: true },
    { label: 'Retention Policy', value: run.retention_policy_id ?? 'Default', mono: true },
    { label: 'Selected Runtime', value: run.selected_runtime_id ?? 'Unassigned', mono: true },
    {
      label: 'Selected Runtime Variant',
      value: run.selected_runtime_variant_id ?? 'Unassigned',
      mono: true,
    },
    { label: 'Selected Backend', value: run.selected_backend_key ?? 'Unassigned', mono: true },
    { label: 'Selected Model', value: run.selected_model_id ?? 'Unassigned', mono: true },
    { label: 'Selected Task', value: run.selected_task_id ?? 'Unassigned', mono: true },
    { label: 'Selected Device Class', value: run.selected_device_class ?? 'Unassigned', mono: true },
    { label: 'Selected Device', value: run.selected_device_id ?? 'Unassigned', mono: true },
    {
      label: 'Selected Network Node',
      value: run.selected_network_node_id ?? 'Unassigned',
      mono: true,
    },
    {
      label: 'Queue Position',
      value:
        run.scheduler_queue_position === null || run.scheduler_queue_position === undefined
          ? 'Unassigned'
          : String(run.scheduler_queue_position),
      mono: false,
    },
    {
      label: 'Priority',
      value:
        run.scheduler_priority === null || run.scheduler_priority === undefined ? 'Default' : String(run.scheduler_priority),
      mono: false,
    },
    { label: 'Estimate Confidence', value: run.estimate_confidence ?? 'Unavailable', mono: false },
    {
      label: 'Estimated Queue Wait',
      value:
        run.estimated_queue_wait_ms === null || run.estimated_queue_wait_ms === undefined
          ? 'Unavailable'
          : formatDiagnosticsDuration(run.estimated_queue_wait_ms, run.status),
      mono: false,
    },
    {
      label: 'Estimated Duration',
      value:
        run.estimated_duration_ms === null || run.estimated_duration_ms === undefined
          ? 'Unavailable'
          : formatDiagnosticsDuration(run.estimated_duration_ms, run.status),
      mono: false,
    },
    { label: 'Model Cache', value: formatDiagnosticsModelCacheState(run.model_cache_state), mono: false },
    { label: 'Scheduler Reason', value: run.scheduler_reason ?? 'Unavailable', mono: false },
    { label: 'Timeline Events', value: String(run.timeline_event_count), mono: false },
    { label: 'Last Event Seq', value: String(run.last_event_seq), mono: false },
  ];
}

export function formatDiagnosticsModelCacheState(state: RunDetailProjectionRecord['model_cache_state']): string {
  switch (state) {
    case 'unknown':
      return 'Cache state unknown';
    case 'not_required':
      return 'Model not required';
    case 'cache_hit':
      return 'Model cache hit';
    case 'cache_miss':
      return 'Model cache miss';
    case 'load_requested':
      return 'Model load requested';
    case 'loaded':
      return 'Model loaded';
    case 'unload_requested':
      return 'Model unload requested';
    case 'unloaded':
      return 'Model unloaded';
    case 'failed':
      return 'Model cache failed';
    case null:
    case undefined:
      return 'Unavailable';
  }
}

export function buildDiagnosticsFacetSummary(
  activeRun: RunDetailProjectionRecord,
  runs: RunListProjectionRecord[],
  backendFacets: RunListFacetRecord[] = [],
): DiagnosticsFacetSummary {
  const scopedRuns = ensureActiveRunInScope(activeRun, runs);
  const total = scopedRuns.length;
  const rows = [
    buildDiagnosticsFacetRow(
      'Workflow Version',
      workflowVersionLabel(activeRun),
      scopedRuns,
      workflowVersionLabel,
      total,
      backendFacets,
      'workflow_version',
    ),
    buildDiagnosticsFacetRow('Status', activeRun.status, scopedRuns, (run) => run.status, total, backendFacets, 'status'),
    buildDiagnosticsFacetRow(
      'Scheduler Policy',
      optionalFacetLabel(activeRun.scheduler_policy_id),
      scopedRuns,
      (run) => optionalFacetLabel(run.scheduler_policy_id),
      total,
      backendFacets,
      'scheduler_policy',
    ),
    buildDiagnosticsFacetRow(
      'Retention Policy',
      optionalFacetLabel(activeRun.retention_policy_id),
      scopedRuns,
      (run) => optionalFacetLabel(run.retention_policy_id),
      total,
      backendFacets,
      'retention_policy',
    ),
    buildDiagnosticsFacetRow(
      'Selected Runtime',
      optionalFacetLabel(activeRun.selected_runtime_id),
      scopedRuns,
      (run) => optionalFacetLabel(run.selected_runtime_id),
      total,
      backendFacets,
      'selected_runtime',
    ),
    buildDiagnosticsFacetRow(
      'Selected Runtime Variant',
      optionalFacetLabel(activeRun.selected_runtime_variant_id),
      scopedRuns,
      (run) => optionalFacetLabel(run.selected_runtime_variant_id),
      total,
      backendFacets,
      'selected_runtime_variant',
    ),
    buildDiagnosticsFacetRow(
      'Selected Backend',
      optionalFacetLabel(activeRun.selected_backend_key),
      scopedRuns,
      (run) => optionalFacetLabel(run.selected_backend_key),
      total,
      backendFacets,
      'selected_backend',
    ),
    buildDiagnosticsFacetRow(
      'Selected Device Class',
      optionalFacetLabel(activeRun.selected_device_class),
      scopedRuns,
      (run) => optionalFacetLabel(run.selected_device_class),
      total,
      backendFacets,
      'selected_device_class',
    ),
    buildDiagnosticsFacetRow(
      'Selected Device',
      optionalFacetLabel(activeRun.selected_device_id),
      scopedRuns,
      (run) => optionalFacetLabel(run.selected_device_id),
      total,
      backendFacets,
      'selected_device',
    ),
    buildDiagnosticsFacetRow(
      'Selected Network Node',
      optionalFacetLabel(activeRun.selected_network_node_id),
      scopedRuns,
      (run) => optionalFacetLabel(run.selected_network_node_id),
      total,
      backendFacets,
      'selected_network_node',
    ),
    buildDiagnosticsFacetRow(
      'Client',
      optionalFacetLabel(activeRun.client_id),
      [activeRun],
      (run) => optionalFacetLabel(run.client_id),
      1,
    ),
    buildDiagnosticsFacetRow(
      'Client Session',
      optionalFacetLabel(activeRun.client_session_id),
      [activeRun],
      (run) => optionalFacetLabel(run.client_session_id),
      1,
    ),
    buildDiagnosticsFacetRow(
      'Bucket',
      optionalFacetLabel(activeRun.bucket_id),
      [activeRun],
      (run) => optionalFacetLabel(run.bucket_id),
      1,
    ),
  ];

  const backendWorkflowVersions = backendFacets.filter((facet) => facet.facet_kind === 'workflow_version');
  const workflowVersionCount =
    backendWorkflowVersions.length > 0 ? backendWorkflowVersions.length : new Set(scopedRuns.map(workflowVersionLabel)).size;
  const mixedVersionWarning =
    workflowVersionCount > 1
      ? `${activeRun.workflow_id} has ${workflowVersionCount} workflow versions in the current run-list projection.`
      : null;

  return { rows, mixedVersionWarning };
}

export function buildDiagnosticsRetentionSummaryRows(
  summary: IoArtifactRetentionSummaryRecord[],
): DiagnosticsRetentionSummaryRow[] {
  return summary
    .map((item) => ({
      label: formatDiagnosticsRetentionStateLabel(item.retention_state),
      count: item.artifact_count,
    }))
    .sort((left, right) => right.count - left.count || left.label.localeCompare(right.label));
}

export function buildDiagnosticsExecutionFacetRows(
  nodes: NodeStatusProjectionRecord[],
): DiagnosticsExecutionFacetRow[] {
  return [
    ...buildExecutionFacetRows('Node Status', nodes.map((node) => node.status), 'Unassigned'),
    ...buildExecutionFacetRows('Error Severity', nodes.map((node) => node.error_severity), 'None'),
    ...buildExecutionFacetRows('Error Phase', nodes.map((node) => node.error_phase), 'None'),
    ...buildExecutionFacetRows('Node Version', nodes.map((node) => node.node_version), 'Unversioned'),
    ...buildExecutionFacetRows('Runtime', nodes.map((node) => node.runtime_id), 'Unassigned'),
    ...buildExecutionFacetRows('Runtime Version', nodes.map((node) => node.runtime_version), 'Unversioned'),
    ...buildExecutionFacetRows('Model', nodes.map((node) => node.model_id), 'Unassigned'),
    ...buildExecutionFacetRows('Model Version', nodes.map((node) => node.model_version), 'Unversioned'),
  ];
}

export function buildDiagnosticsExecutionFilterOptions(
  nodes: NodeStatusProjectionRecord[],
): DiagnosticsExecutionFilterOptions {
  return {
    statuses: uniqueSorted(nodes.map((node) => node.status)) as NodeStatusProjectionRecord['status'][],
    nodeVersions: uniqueSorted(nodes.map((node) => versionFacetLabel(node.node_version))),
    runtimes: uniqueSorted(nodes.map((node) => optionalFacetLabel(node.runtime_id))),
    runtimeVersions: uniqueSorted(nodes.map((node) => versionFacetLabel(node.runtime_version))),
    models: uniqueSorted(nodes.map((node) => optionalFacetLabel(node.model_id))),
    modelVersions: uniqueSorted(nodes.map((node) => versionFacetLabel(node.model_version))),
  };
}

export function filterDiagnosticsExecutionNodes(
  nodes: NodeStatusProjectionRecord[],
  filters: DiagnosticsExecutionFilters,
): NodeStatusProjectionRecord[] {
  return nodes.filter(
    (node) =>
      filterMatches(node.status, filters.status) &&
      filterMatches(versionFacetLabel(node.node_version), filters.nodeVersion) &&
      filterMatches(optionalFacetLabel(node.runtime_id), filters.runtime) &&
      filterMatches(versionFacetLabel(node.runtime_version), filters.runtimeVersion) &&
      filterMatches(optionalFacetLabel(node.model_id), filters.model) &&
      filterMatches(versionFacetLabel(node.model_version), filters.modelVersion),
  );
}

export function formatDiagnosticsRetentionStateLabel(retentionState: IoArtifactRetentionState): string {
  switch (retentionState) {
    case 'retained':
      return 'Payload retained';
    case 'metadata_only':
      return 'Metadata retained only';
    case 'external':
      return 'External reference';
    case 'truncated':
      return 'Payload truncated';
    case 'too_large':
      return 'Too large to retain';
    case 'expired':
      return 'Payload expired';
    case 'deleted':
      return 'Payload deleted';
  }
}

export function buildDiagnosticsComparisonFilterOptions(
  activeRun: RunDetailProjectionRecord,
  runs: RunListProjectionRecord[],
): DiagnosticsComparisonFilterOptions {
  const scopedRuns = ensureActiveRunInScope(activeRun, runs);
  return {
    workflowVersions: uniqueSorted(scopedRuns.map(workflowVersionLabel)),
    statuses: uniqueSorted(scopedRuns.map((run) => run.status)) as RunListProjectionRecord['status'][],
    schedulerPolicies: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.scheduler_policy_id))),
    retentionPolicies: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.retention_policy_id))),
    selectedRuntimes: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.selected_runtime_id))),
    selectedRuntimeVariants: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.selected_runtime_variant_id))),
    selectedBackends: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.selected_backend_key))),
    selectedDeviceClasses: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.selected_device_class))),
    selectedDevices: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.selected_device_id))),
    selectedNetworkNodes: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.selected_network_node_id))),
    clients: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.client_id))),
    clientSessions: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.client_session_id))),
    buckets: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.bucket_id))),
    acceptedDates: uniqueSorted(scopedRuns.map((run) => acceptedDateLabel(run.accepted_at_ms))),
  };
}

export function filterDiagnosticsComparisonRuns(
  activeRun: RunDetailProjectionRecord,
  runs: RunListProjectionRecord[],
  filters: DiagnosticsComparisonFilters,
): RunListProjectionRecord[] {
  const peerRuns = runs.filter(
    (run) => run.workflow_id === activeRun.workflow_id && run.workflow_run_id !== activeRun.workflow_run_id,
  );
  const matchingPeers = peerRuns.filter((run) => diagnosticsRunMatchesFilters(run, filters));
  return [activeRun, ...matchingPeers];
}

export function hasActiveDiagnosticsComparisonFilters(filters: DiagnosticsComparisonFilters): boolean {
  return (
    filters.workflowVersion !== DIAGNOSTICS_FILTER_ALL ||
    filters.status !== DIAGNOSTICS_FILTER_ALL ||
    filters.schedulerPolicy !== DIAGNOSTICS_FILTER_ALL ||
    filters.retentionPolicy !== DIAGNOSTICS_FILTER_ALL ||
    filters.selectedRuntime !== DIAGNOSTICS_FILTER_ALL ||
    filters.selectedRuntimeVariant !== DIAGNOSTICS_FILTER_ALL ||
    filters.selectedBackend !== DIAGNOSTICS_FILTER_ALL ||
    filters.selectedDeviceClass !== DIAGNOSTICS_FILTER_ALL ||
    filters.selectedDevice !== DIAGNOSTICS_FILTER_ALL ||
    filters.selectedNetworkNode !== DIAGNOSTICS_FILTER_ALL ||
    filters.client !== DIAGNOSTICS_FILTER_ALL ||
    filters.clientSession !== DIAGNOSTICS_FILTER_ALL ||
    filters.bucket !== DIAGNOSTICS_FILTER_ALL ||
    filters.acceptedDate !== DIAGNOSTICS_FILTER_ALL ||
    filters.acceptedFromDate.trim().length > 0 ||
    filters.acceptedToDate.trim().length > 0
  );
}

function buildDiagnosticsFacetRow<T extends RunListProjectionRecord>(
  label: string,
  value: string,
  runs: T[],
  readValue: (run: T) => string,
  total: number,
  backendFacets: RunListFacetRecord[] = [],
  facetKind?: RunListFacetKind,
): DiagnosticsFacetRow {
  if (facetKind) {
    const matchingFacets = backendFacets.filter((facet) => facet.facet_kind === facetKind);
    const backendTotal = matchingFacets.reduce((sum, facet) => sum + facet.run_count, 0);
    if (backendTotal > 0) {
      return {
        label,
        value,
        count: matchingFacets.find((facet) => facet.facet_value === value)?.run_count ?? 0,
        total: backendTotal,
      };
    }
  }
  return {
    label,
    value,
    count: runs.filter((run) => readValue(run) === value).length,
    total,
  };
}

function ensureActiveRunInScope(
  activeRun: RunDetailProjectionRecord,
  runs: RunListProjectionRecord[],
): RunListProjectionRecord[] {
  const workflowRuns = runs.filter((run) => run.workflow_id === activeRun.workflow_id);
  const activeRunIsProjected = workflowRuns.some((run) => run.workflow_run_id === activeRun.workflow_run_id);
  return activeRunIsProjected ? workflowRuns : [activeRun, ...workflowRuns];
}

function diagnosticsRunMatchesFilters(
  run: RunListProjectionRecord,
  filters: DiagnosticsComparisonFilters,
): boolean {
  return (
    filterMatches(workflowVersionLabel(run), filters.workflowVersion) &&
    filterMatches(run.status, filters.status) &&
    filterMatches(optionalFacetLabel(run.scheduler_policy_id), filters.schedulerPolicy) &&
    filterMatches(optionalFacetLabel(run.retention_policy_id), filters.retentionPolicy) &&
    filterMatches(optionalFacetLabel(run.selected_runtime_id), filters.selectedRuntime) &&
    filterMatches(optionalFacetLabel(run.selected_runtime_variant_id), filters.selectedRuntimeVariant) &&
    filterMatches(optionalFacetLabel(run.selected_backend_key), filters.selectedBackend) &&
    filterMatches(optionalFacetLabel(run.selected_device_class), filters.selectedDeviceClass) &&
    filterMatches(optionalFacetLabel(run.selected_device_id), filters.selectedDevice) &&
    filterMatches(optionalFacetLabel(run.selected_network_node_id), filters.selectedNetworkNode) &&
    filterMatches(optionalFacetLabel(run.client_id), filters.client) &&
    filterMatches(optionalFacetLabel(run.client_session_id), filters.clientSession) &&
    filterMatches(optionalFacetLabel(run.bucket_id), filters.bucket) &&
    filterMatches(acceptedDateLabel(run.accepted_at_ms), filters.acceptedDate) &&
    acceptedDateRangeMatches(run.accepted_at_ms, filters.acceptedFromDate, filters.acceptedToDate)
  );
}

function filterMatches(value: string, filter: string): boolean {
  return filter === DIAGNOSTICS_FILTER_ALL || value === filter;
}

function uniqueSorted(values: string[]): string[] {
  return [...new Set(values)].sort((left, right) => left.localeCompare(right));
}

function buildExecutionFacetRows(
  label: string,
  values: Array<string | null | undefined>,
  fallback: string,
): DiagnosticsExecutionFacetRow[] {
  const counts = new Map<string, number>();
  for (const value of values) {
    const normalized = value && value.trim().length > 0 ? value : fallback;
    counts.set(normalized, (counts.get(normalized) ?? 0) + 1);
  }
  return [...counts.entries()]
    .map(([value, count]) => ({ label, value, count }))
    .sort((left, right) => right.count - left.count || left.value.localeCompare(right.value));
}

function workflowVersionLabel(run: RunListProjectionRecord): string {
  return run.workflow_semantic_version ?? run.workflow_version_id ?? 'Unversioned';
}

function optionalFacetLabel(value: string | null | undefined): string {
  return value && value.trim().length > 0 ? value : 'Unassigned';
}

function versionFacetLabel(value: string | null | undefined): string {
  return value && value.trim().length > 0 ? value : 'Unversioned';
}

function acceptedDateLabel(value: number | null | undefined): string {
  if (!value) {
    return 'Unassigned';
  }
  return new Date(value).toISOString().slice(0, 10);
}

function acceptedDateRangeMatches(
  value: number | null | undefined,
  fromDate: string,
  toDate: string,
): boolean {
  const fromMs = dateInputStartMs(fromDate);
  const toMs = dateInputEndMs(toDate);
  if (fromMs === null && toMs === null) {
    return true;
  }
  if (!value) {
    return false;
  }
  return (fromMs === null || value >= fromMs) && (toMs === null || value <= toMs);
}

function dateInputStartMs(value: string): number | null {
  const normalized = value.trim();
  if (normalized.length === 0) {
    return null;
  }
  const parsed = Date.parse(`${normalized}T00:00:00.000Z`);
  return Number.isFinite(parsed) ? parsed : null;
}

function dateInputEndMs(value: string): number | null {
  const normalized = value.trim();
  if (normalized.length === 0) {
    return null;
  }
  const parsed = Date.parse(`${normalized}T23:59:59.999Z`);
  return Number.isFinite(parsed) ? parsed : null;
}

/** Lower-case words for a projection enum, for example `node_execution` to "node execution". */
export function diagnosticsWords(value: string): string {
  return value.split('_').filter((part) => part.length > 0).join(' ');
}

export function formatDiagnosticEventKind(kind: DiagnosticEventKind): string {
  return diagnosticsWords(kind);
}

export function formatDiagnosticErrorPhase(phase: string | null | undefined): string {
  if (!phase || phase.trim().length === 0) {
    return 'Unassigned';
  }
  const words = diagnosticsWords(phase.trim());
  return words.charAt(0).toUpperCase() + words.slice(1);
}

export function formatDiagnosticSourceComponent(source: DiagnosticEventSourceComponent): string {
  return diagnosticsWords(source);
}

export function hasTimelinePayload(event: Pick<SchedulerTimelineProjectionRecord, 'payload_json'>): boolean {
  const payload = event.payload_json.trim();
  return payload.length > 0 && payload !== '{}' && payload !== 'null';
}

// ---------------------------------------------------------------------------
// Redesigned page projection (docs/design/workbench-redesign-spec.md,
// "Diagnostics"). Everything below is pure: it reads typed projection records
// and returns what the page renders.
// ---------------------------------------------------------------------------

const MS_PER_SECOND = 1_000;
const MS_PER_MINUTE = 60 * MS_PER_SECOND;
const MS_PER_HOUR = 60 * MS_PER_MINUTE;
const MS_PER_DAY = 24 * MS_PER_HOUR;

const NOT_RECORDED = 'Not recorded';

function pad2(value: number): string {
  return String(value).padStart(2, '0');
}

function localDateKey(date: Date): string {
  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}`;
}

function localClock(date: Date): string {
  return `${pad2(date.getHours())}:${pad2(date.getMinutes())}:${pad2(date.getSeconds())}`;
}

/**
 * Local wall-clock time of a projection timestamp: `HH:MM:SS` on the same
 * local day as `nowMs`, `YYYY-MM-DD HH:MM:SS` otherwise.
 */
export function formatDiagnosticsClock(value: number | null | undefined, nowMs: number): string {
  if (!value) {
    return NOT_RECORDED;
  }
  const date = new Date(value);
  const clock = localClock(date);
  return localDateKey(date) === localDateKey(new Date(nowMs)) ? clock : `${localDateKey(date)} ${clock}`;
}

/** Log timestamp with milliseconds, `HH:MM:SS.mmm` local time. */
export function formatDiagnosticsLogTime(value: number): string {
  const date = new Date(value);
  return `${localClock(date)}.${String(date.getMilliseconds()).padStart(3, '0')}`;
}

/** Elapsed time in the words the page uses: "640 ms", "2.5 s", "22 s", "1m 49s", "2h 05m". */
export function formatDiagnosticsElapsed(value: number): string {
  const ms = Math.max(0, value);
  if (ms < MS_PER_SECOND) {
    return `${Math.round(ms)} ms`;
  }
  if (ms < 10 * MS_PER_SECOND) {
    return `${(ms / MS_PER_SECOND).toFixed(1)} s`;
  }
  if (ms < MS_PER_MINUTE) {
    return `${Math.round(ms / MS_PER_SECOND)} s`;
  }
  const totalSeconds = Math.round(ms / MS_PER_SECOND);
  if (ms < MS_PER_HOUR) {
    return `${Math.floor(totalSeconds / 60)}m ${pad2(totalSeconds % 60)}s`;
  }
  const totalMinutes = Math.floor(totalSeconds / 60);
  return `${Math.floor(totalMinutes / 60)}h ${pad2(totalMinutes % 60)}m`;
}

const RUN_STATUS_WORD: Record<RunListProjectionRecord['status'], string> = {
  accepted: 'accepted',
  future: 'scheduled for later',
  scheduled: 'scheduled',
  queued: 'queued',
  delayed: 'delayed',
  running: 'running',
  completed: 'completed',
  failed: 'failed',
  cancelled: 'cancelled',
};

function isTerminalRunStatus(status: RunListProjectionRecord['status']): boolean {
  return status === 'completed' || status === 'failed' || status === 'cancelled';
}

function plural(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? '' : 's'}`;
}

function nodeErrorEventId(node: NodeStatusProjectionRecord): string | null {
  return node.canonical_error_event_id ?? node.error_event_id ?? null;
}

/** Whether a node row carries a diagnostic error (the old "Node Errors" table criterion). */
export function diagnosticsNodeHasError(node: NodeStatusProjectionRecord): boolean {
  return Boolean(nodeErrorEventId(node) || node.error || node.error_severity);
}

function firstFailedNodeId(nodes: NodeStatusProjectionRecord[]): string | null {
  return nodes.find((node) => node.status === 'failed')?.node_id ?? null;
}

/**
 * The page summary line, for example
 * "run_1, failed 19:35:42 after 22 s, 1 error in load_model".
 */
export function buildDiagnosticsDimensionLine(
  run: RunDetailProjectionRecord,
  nodes: NodeStatusProjectionRecord[],
  nowMs: number,
): string {
  const word = RUN_STATUS_WORD[run.status];
  let state: string;
  if (isTerminalRunStatus(run.status)) {
    const at = run.completed_at_ms ?? run.last_updated_at_ms;
    const after =
      run.duration_ms === null || run.duration_ms === undefined ? '' : ` after ${formatDiagnosticsElapsed(run.duration_ms)}`;
    state = `${word} ${formatDiagnosticsClock(at, nowMs)}${after}`;
  } else if (run.status === 'running') {
    state = run.started_at_ms ? `${word} since ${formatDiagnosticsClock(run.started_at_ms, nowMs)}` : word;
  } else if (run.status === 'delayed') {
    state = run.scheduler_reason ? `${word}: ${run.scheduler_reason}` : word;
  } else {
    const position = run.scheduler_queue_position;
    state = position === null || position === undefined ? word : `${word} at position ${position}`;
  }

  const parts = [run.workflow_run_id, state];
  const errors = run.error_count ?? 0;
  if (errors > 0) {
    const failedNode = firstFailedNodeId(nodes);
    parts.push(`${plural(errors, 'error')}${failedNode ? ` in ${failedNode}` : ''}`);
  }
  const warnings = run.warning_count ?? 0;
  if (warnings > 0) {
    parts.push(plural(warnings, 'warning'));
  }
  return parts.join(', ');
}

/** Placement chip text: "diffusers on cuda:1", or what is still missing. */
export function buildDiagnosticsPlacementLabel(run: RunListProjectionRecord): string {
  const runtime = run.selected_runtime_id;
  const device = run.selected_device_id;
  if (runtime && device) {
    return `${runtime} on ${device}`;
  }
  if (runtime) {
    return `${runtime}, device not assigned yet`;
  }
  if (device) {
    return `on ${device}, runtime not assigned yet`;
  }
  return 'Not placed yet';
}

/** Client chip text, or null when the run carries no client facts. */
export function buildDiagnosticsClientLabel(run: RunDetailProjectionRecord): string | null {
  if (run.client_id && run.client_session_id) {
    return `from ${run.client_id}, session ${run.client_session_id}`;
  }
  if (run.client_id) {
    return `from ${run.client_id}`;
  }
  if (run.client_session_id) {
    return `session ${run.client_session_id}`;
  }
  return null;
}

type DiagnosticsFactSection = 'timing' | 'placement' | 'retention' | 'more';

/**
 * Where each `buildDiagnosticsFactRows` fact goes in the redesigned left
 * panel, with its sentence-case key. Every fact row label must appear here
 * exactly once (enforced by test) so no fact the page showed is lost.
 */
const DIAGNOSTICS_FACT_LAYOUT: Record<string, { section: DiagnosticsFactSection; key: string }> = {
  'Estimated Queue Wait': { section: 'timing', key: 'Estimated wait' },
  'Estimated Duration': { section: 'timing', key: 'Estimated duration' },
  'Estimate Confidence': { section: 'timing', key: 'Estimate confidence' },
  'Selected Runtime': { section: 'placement', key: 'Runtime' },
  'Selected Backend': { section: 'placement', key: 'Backend' },
  'Selected Device': { section: 'placement', key: 'Device' },
  'Selected Model': { section: 'placement', key: 'Model' },
  'Model Cache': { section: 'placement', key: 'Model cache' },
  'Scheduler Policy': { section: 'placement', key: 'Policy' },
  Bucket: { section: 'placement', key: 'Bucket' },
  'Queue Position': { section: 'placement', key: 'Queue position' },
  Priority: { section: 'placement', key: 'Priority' },
  'Scheduler Reason': { section: 'placement', key: 'Reason' },
  'Retention Policy': { section: 'retention', key: 'Policy' },
  Workflow: { section: 'more', key: 'Workflow' },
  'Workflow Version': { section: 'more', key: 'Workflow version' },
  'Run Snapshot': { section: 'more', key: 'Run snapshot' },
  'Presentation Revision': { section: 'more', key: 'Presentation revision' },
  Client: { section: 'more', key: 'Client' },
  'Client Session': { section: 'more', key: 'Client session' },
  'Execution Session': { section: 'more', key: 'Execution session' },
  'Selected Runtime Variant': { section: 'more', key: 'Runtime variant' },
  'Selected Task': { section: 'more', key: 'Task' },
  'Selected Device Class': { section: 'more', key: 'Device class' },
  'Selected Network Node': { section: 'more', key: 'Network node' },
  'Timeline Events': { section: 'more', key: 'Timeline events' },
  'Last Event Seq': { section: 'more', key: 'Last event seq' },
};

export function diagnosticsFactLayoutLabels(): string[] {
  return Object.keys(DIAGNOSTICS_FACT_LAYOUT);
}

export interface DiagnosticsFactSections {
  timing: KeyValueRow[];
  placement: KeyValueRow[];
  retention: KeyValueRow[];
  more: KeyValueRow[];
}

/**
 * The run facts split into the left panel's Timing, Placement, Retention and
 * "More facts" key-value blocks. Timing leads with the lifecycle timestamps;
 * "More facts" ends with the run's error identifiers when it has any.
 */
export function buildDiagnosticsFactSections(run: RunDetailProjectionRecord, nowMs: number): DiagnosticsFactSections {
  const sections: DiagnosticsFactSections = {
    timing: [
      { key: 'Accepted', value: formatDiagnosticsClock(run.accepted_at_ms, nowMs), mono: true },
      { key: 'Queued', value: formatDiagnosticsClock(run.enqueued_at_ms, nowMs), mono: true },
      { key: 'Started', value: formatDiagnosticsClock(run.started_at_ms, nowMs), mono: true },
      { key: 'Finished', value: formatDiagnosticsClock(run.completed_at_ms, nowMs), mono: true },
      { key: 'Duration', value: formatDiagnosticsRunDuration(run, nowMs) },
    ],
    placement: [],
    retention: [],
    more: [],
  };
  // Layout order, not fact-row order, so each block reads in a sensible sequence.
  const factRows = new Map(buildDiagnosticsFactRows(run).map((row) => [row.label, row]));
  for (const [label, layout] of Object.entries(DIAGNOSTICS_FACT_LAYOUT)) {
    const row = factRows.get(label);
    if (!row) continue;
    factRows.delete(label);
    sections[layout.section].push({ key: layout.key, value: row.value, mono: row.mono });
  }
  for (const row of factRows.values()) {
    sections.more.push({ key: row.label, value: row.value, mono: row.mono });
  }
  const errorEventId = run.latest_error_event_id;
  if (errorEventId) {
    sections.more.push({ key: 'Latest error event', value: errorEventId, mono: true });
  }
  if (run.fatal_error_event_id && run.fatal_error_event_id !== errorEventId) {
    sections.more.push({ key: 'Fatal error event', value: run.fatal_error_event_id, mono: true });
  }
  if (run.terminal_error && run.terminal_error !== run.latest_error_message) {
    sections.more.push({ key: 'Terminal error', value: run.terminal_error });
  }
  return sections;
}

/** Duration of a finished run, or how long a running run has been running as of `nowMs`. */
export function formatDiagnosticsRunDuration(run: RunListProjectionRecord, nowMs: number): string {
  if (typeof run.duration_ms === 'number') {
    return formatDiagnosticsElapsed(run.duration_ms);
  }
  if (run.status === 'running' && run.started_at_ms) {
    return `Running for ${formatDiagnosticsElapsed(nowMs - run.started_at_ms)}`;
  }
  return formatDiagnosticsDuration(null, run.status);
}

/** Sentence case for projection labels written in title case, for example "Selected Runtime" to "Selected runtime". */
export function diagnosticsSentenceCase(label: string): string {
  return label.charAt(0) + label.slice(1).toLowerCase();
}

export function buildDiagnosticsRetentionRows(
  summary: IoArtifactRetentionSummaryRecord[],
): KeyValueRow[] {
  return buildDiagnosticsRetentionSummaryRows(summary).map((row) => ({
    key: row.label,
    value: plural(row.count, 'artifact'),
  }));
}

export interface DiagnosticsProjectionStates {
  runDetail: ProjectionStateRecord | null;
  timeline: ProjectionStateRecord | null;
  runList: ProjectionStateRecord | null;
  nodeStatus: ProjectionStateRecord | null;
  io: ProjectionStateRecord | null;
}

/** One freshness row per projection the page reads (ADR-014 guardrail). */
export function buildDiagnosticsProjectionRows(states: DiagnosticsProjectionStates): KeyValueRow[] {
  return [
    { key: 'Run detail', value: formatDiagnosticsProjectionFreshness(states.runDetail) },
    { key: 'Scheduler timeline', value: formatDiagnosticsProjectionFreshness(states.timeline) },
    { key: 'Run list', value: formatDiagnosticsProjectionFreshness(states.runList) },
    { key: 'Node status', value: formatDiagnosticsProjectionFreshness(states.nodeStatus) },
    { key: 'I/O retention', value: formatDiagnosticsProjectionFreshness(states.io) },
  ];
}

export interface DiagnosticsNodeStatusBar {
  status: NodeExecutionProjectionStatus;
  label: string;
  count: number;
  /** Share of all nodes, 0 to 100, rounded. */
  percent: number;
}

const NODE_STATUS_BAR_ORDER: Array<[NodeExecutionProjectionStatus, string]> = [
  ['running', 'Running'],
  ['waiting', 'Waiting'],
  ['queued', 'Queued'],
  ['completed', 'Completed'],
  ['failed', 'Failed'],
  ['cancelled', 'Cancelled'],
];

/** "Nodes by status" bars, in lifecycle order, omitting statuses with no nodes. */
export function buildDiagnosticsNodeStatusBars(nodes: NodeStatusProjectionRecord[]): DiagnosticsNodeStatusBar[] {
  const total = nodes.length;
  return NODE_STATUS_BAR_ORDER.map(([status, label]) => {
    const count = nodes.filter((node) => node.status === status).length;
    return { status, label, count, percent: total > 0 ? Math.round((count / total) * 100) : 0 };
  }).filter((bar) => bar.count > 0);
}

export interface DiagnosticsFacetGroup {
  label: string;
  rows: DiagnosticsExecutionFacetRow[];
}

/**
 * Execution facet rows grouped by facet for the "Breakdown" disclosure. Node
 * status is left out because the status bars already show it.
 */
export function groupDiagnosticsExecutionFacetRows(rows: DiagnosticsExecutionFacetRow[]): DiagnosticsFacetGroup[] {
  const groups: DiagnosticsFacetGroup[] = [];
  for (const row of rows) {
    if (row.label === 'Node Status') continue;
    const label = diagnosticsSentenceCase(row.label);
    const group = groups.find((candidate) => candidate.label === label);
    if (group) {
      group.rows.push(row);
    } else {
      groups.push({ label, rows: [row] });
    }
  }
  return groups;
}

// --- Filters: field descriptors and applied-filter chips --------------------

export interface DiagnosticsExecutionFilterField {
  field: keyof DiagnosticsExecutionFilters;
  label: string;
  options: keyof DiagnosticsExecutionFilterOptions;
}

export const DIAGNOSTICS_EXECUTION_FILTER_FIELDS: readonly DiagnosticsExecutionFilterField[] = [
  { field: 'status', label: 'Node status', options: 'statuses' },
  { field: 'nodeVersion', label: 'Node version', options: 'nodeVersions' },
  { field: 'runtime', label: 'Runtime', options: 'runtimes' },
  { field: 'runtimeVersion', label: 'Runtime version', options: 'runtimeVersions' },
  { field: 'model', label: 'Model', options: 'models' },
  { field: 'modelVersion', label: 'Model version', options: 'modelVersions' },
];

type DiagnosticsComparisonSelectField = Exclude<
  keyof DiagnosticsComparisonFilters,
  'acceptedFromDate' | 'acceptedToDate'
>;

export type DiagnosticsComparisonFilterField =
  | { input: 'select'; field: DiagnosticsComparisonSelectField; label: string; options: keyof DiagnosticsComparisonFilterOptions }
  | { input: 'date'; field: 'acceptedFromDate' | 'acceptedToDate'; label: string };

export const DIAGNOSTICS_COMPARISON_FILTER_FIELDS: readonly DiagnosticsComparisonFilterField[] = [
  { input: 'select', field: 'workflowVersion', label: 'Version', options: 'workflowVersions' },
  { input: 'select', field: 'status', label: 'Status', options: 'statuses' },
  { input: 'select', field: 'schedulerPolicy', label: 'Scheduler policy', options: 'schedulerPolicies' },
  { input: 'select', field: 'retentionPolicy', label: 'Retention policy', options: 'retentionPolicies' },
  { input: 'select', field: 'client', label: 'Client', options: 'clients' },
  { input: 'select', field: 'clientSession', label: 'Session', options: 'clientSessions' },
  { input: 'select', field: 'selectedRuntime', label: 'Runtime', options: 'selectedRuntimes' },
  { input: 'select', field: 'selectedRuntimeVariant', label: 'Runtime variant', options: 'selectedRuntimeVariants' },
  { input: 'select', field: 'selectedBackend', label: 'Backend', options: 'selectedBackends' },
  { input: 'select', field: 'selectedDeviceClass', label: 'Device class', options: 'selectedDeviceClasses' },
  { input: 'select', field: 'selectedDevice', label: 'Device', options: 'selectedDevices' },
  { input: 'select', field: 'selectedNetworkNode', label: 'Network node', options: 'selectedNetworkNodes' },
  { input: 'select', field: 'bucket', label: 'Bucket', options: 'buckets' },
  { input: 'select', field: 'acceptedDate', label: 'Accepted on', options: 'acceptedDates' },
  { input: 'date', field: 'acceptedFromDate', label: 'Accepted from' },
  { input: 'date', field: 'acceptedToDate', label: 'Accepted to' },
];

/** Display text of a comparison filter option; statuses read as words. */
export function formatDiagnosticsComparisonOption(field: keyof DiagnosticsComparisonFilters, value: string): string {
  if (field === 'status' && isRunListStatus(value)) {
    return formatDiagnosticsStatusLabel(value);
  }
  return value;
}

const RUN_LIST_STATUSES = new Set<string>(Object.keys(RUN_STATUS_WORD));

function isRunListStatus(value: string): value is RunListProjectionRecord['status'] {
  return RUN_LIST_STATUSES.has(value);
}

export interface DiagnosticsFilterChip<F extends string> {
  field: F;
  /** Applied filter in words, for example "Device cuda:0". */
  label: string;
}

export function buildDiagnosticsExecutionFilterChips(
  filters: DiagnosticsExecutionFilters,
): DiagnosticsFilterChip<keyof DiagnosticsExecutionFilters>[] {
  return DIAGNOSTICS_EXECUTION_FILTER_FIELDS.filter(({ field }) => filters[field] !== DIAGNOSTICS_FILTER_ALL).map(
    ({ field, label }) => ({ field, label: `${label} ${filters[field]}` }),
  );
}

export function buildDiagnosticsComparisonFilterChips(
  filters: DiagnosticsComparisonFilters,
): DiagnosticsFilterChip<keyof DiagnosticsComparisonFilters>[] {
  const chips: DiagnosticsFilterChip<keyof DiagnosticsComparisonFilters>[] = [];
  for (const descriptor of DIAGNOSTICS_COMPARISON_FILTER_FIELDS) {
    const value = filters[descriptor.field];
    const applied = descriptor.input === 'date' ? value.trim().length > 0 : value !== DIAGNOSTICS_FILTER_ALL;
    if (applied) {
      chips.push({
        field: descriptor.field,
        label: `${descriptor.label} ${formatDiagnosticsComparisonOption(descriptor.field, value)}`,
      });
    }
  }
  return chips;
}

/** The value a cleared filter field returns to. */
export function defaultDiagnosticsComparisonFilterValue(field: keyof DiagnosticsComparisonFilters): string {
  return DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS[field];
}

export interface DiagnosticsComparisonShortcut {
  id: 'version' | 'device' | 'recent';
  label: string;
  pressed: boolean;
  /** Filters after the shortcut is toggled. */
  toggled: DiagnosticsComparisonFilters;
}

const RECENT_COMPARISON_DAYS = 7;

/** UTC date (the comparison date filters compare in UTC) of the first day of the recent window. */
export function diagnosticsRecentFromDate(nowMs: number): string {
  return new Date(nowMs - (RECENT_COMPARISON_DAYS - 1) * MS_PER_DAY).toISOString().slice(0, 10);
}

/**
 * The board's quick pills ("Only v0.9.1", "Only cuda:1", "Last 7 days"),
 * expressed as presets over the existing comparison filters.
 */
export function buildDiagnosticsComparisonShortcuts(
  run: RunDetailProjectionRecord,
  filters: DiagnosticsComparisonFilters,
  nowMs: number,
): DiagnosticsComparisonShortcut[] {
  const shortcuts: DiagnosticsComparisonShortcut[] = [];
  const version = workflowVersionLabel(run);
  const versionPressed = filters.workflowVersion === version;
  shortcuts.push({
    id: 'version',
    label: version === 'Unversioned' ? 'Only unversioned' : `Only v${version}`,
    pressed: versionPressed,
    toggled: { ...filters, workflowVersion: versionPressed ? DIAGNOSTICS_FILTER_ALL : version },
  });
  if (run.selected_device_id) {
    const devicePressed = filters.selectedDevice === run.selected_device_id;
    shortcuts.push({
      id: 'device',
      label: `Only ${run.selected_device_id}`,
      pressed: devicePressed,
      toggled: { ...filters, selectedDevice: devicePressed ? DIAGNOSTICS_FILTER_ALL : run.selected_device_id },
    });
  }
  const from = diagnosticsRecentFromDate(nowMs);
  const recentPressed = filters.acceptedFromDate === from && filters.acceptedToDate.trim().length === 0;
  shortcuts.push({
    id: 'recent',
    label: `Last ${RECENT_COMPARISON_DAYS} days`,
    pressed: recentPressed,
    toggled: { ...filters, acceptedFromDate: recentPressed ? '' : from, acceptedToDate: '' },
  });
  return shortcuts;
}

// --- "Compared with earlier runs" -------------------------------------------

export interface DiagnosticsComparisonSummary {
  /** Big number, for example "3 of 14". */
  failed: string;
  failedLabel: string;
  /** Big number: median duration of the other completed runs, for example "1m 49s", or "None yet". */
  median: string;
  medianLabel: string;
  note: string;
}

function median(values: number[]): number | null {
  if (values.length === 0) return null;
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

/**
 * Comparison of the selected run with the run-list peers that pass the
 * comparison filters. `comparedRuns` is `filterDiagnosticsComparisonRuns`
 * output, so it starts with the selected run.
 */
export function buildDiagnosticsComparisonSummary(
  run: RunDetailProjectionRecord,
  comparedRuns: RunListProjectionRecord[],
): DiagnosticsComparisonSummary {
  const total = comparedRuns.length;
  const failedRuns = comparedRuns.filter((candidate) => candidate.status === 'failed');
  const peers = comparedRuns.filter((candidate) => candidate.workflow_run_id !== run.workflow_run_id);
  // The median describes the other runs, so this run is compared against it rather than with itself.
  const completedDurations = peers
    .filter((candidate) => candidate.status === 'completed' && typeof candidate.duration_ms === 'number')
    .map((candidate) => candidate.duration_ms as number);
  const medianMs = median(completedDurations);

  let note: string;
  if (peers.length === 0) {
    note = `No other runs of ${run.workflow_id} in the run list match these filters.`;
  } else if (run.status === 'failed') {
    const failedPeers = peers.filter((candidate) => candidate.status === 'failed');
    const code = run.latest_error_code;
    const sameCode = code ? failedPeers.filter((candidate) => candidate.latest_error_code === code).length : 0;
    if (failedPeers.length === 0) {
      note = `None of the other ${plural(peers.length, 'compared run')} failed.`;
    } else {
      note =
        `${failedPeers.length} of the other ${plural(peers.length, 'compared run')} also failed` +
        (sameCode > 0 ? `, ${sameCode} with ${code}.` : '.');
    }
  } else if (medianMs !== null && typeof run.duration_ms === 'number') {
    note = `This run took ${formatDiagnosticsElapsed(run.duration_ms)}; the median completed run took ${formatDiagnosticsElapsed(medianMs)}.`;
  } else {
    note = `${completedDurations.length} of the other ${plural(peers.length, 'compared run')} completed with a recorded duration.`;
  }

  return {
    failed: `${failedRuns.length} of ${total}`,
    failedLabel: `runs of ${run.workflow_id} failed`,
    median: medianMs === null ? 'None yet' : formatDiagnosticsElapsed(medianMs),
    medianLabel: 'median duration of the other completed runs',
    note,
  };
}

// --- Event log ----------------------------------------------------------------

export type DiagnosticsLevelFilter = 'all' | 'warn' | 'error';
export type DiagnosticsSourceFilter = 'all' | 'scheduler' | 'runtime' | 'nodes';

export const DIAGNOSTICS_SOURCE_OPTIONS: SegmentOption<DiagnosticsSourceFilter>[] = [
  { value: 'all', label: 'All sources' },
  { value: 'scheduler', label: 'Scheduler' },
  { value: 'runtime', label: 'Runtime' },
  { value: 'nodes', label: 'Nodes' },
];

const SOURCE_FILTER_COMPONENT: Record<Exclude<DiagnosticsSourceFilter, 'all'>, DiagnosticEventSourceComponent> = {
  scheduler: 'scheduler',
  runtime: 'runtime',
  nodes: 'node_execution',
};

/** Log level of a timeline event: error and fatal are errors, warning is a warning. */
export function diagnosticsSeverityLevel(severity: DiagnosticErrorSeverity | null | undefined): LogLevel {
  switch (severity) {
    case 'fatal':
    case 'error':
      return 'error';
    case 'warning':
      return 'warn';
    case null:
    case undefined:
      return 'info';
  }
}

export interface DiagnosticsLogLine {
  eventId: string;
  /** DOM id of the rendered line, used to scroll a focused event into view. */
  domId: string;
  seq: number;
  time: string;
  level: LogLevel;
  kind: string;
  message: string;
  source: string;
  sourceComponent: DiagnosticEventSourceComponent;
  /** Node whose error this event is, when the node status projection links them. */
  nodeId: string | null;
  searchText: string;
}

export function diagnosticsLogLineDomId(eventSeq: number): string {
  return `diagnostics-event-${eventSeq}`;
}

/**
 * Timeline events as log lines, in event order. A node's error appears as its
 * own line: the event the node status row points at takes the node id as its
 * kind, the node's error code in the message, and at least error level.
 */
export function buildDiagnosticsLogLines(
  events: SchedulerTimelineProjectionRecord[],
  nodes: NodeStatusProjectionRecord[],
): DiagnosticsLogLine[] {
  const nodeByErrorEvent = new Map<string, NodeStatusProjectionRecord>();
  for (const node of nodes) {
    if (node.canonical_error_event_id) nodeByErrorEvent.set(node.canonical_error_event_id, node);
    if (node.error_event_id && !nodeByErrorEvent.has(node.error_event_id)) nodeByErrorEvent.set(node.error_event_id, node);
  }
  return [...events]
    .sort((left, right) => left.event_seq - right.event_seq)
    .map((event) => {
      const node = nodeByErrorEvent.get(event.event_id) ?? null;
      const eventLevel = diagnosticsSeverityLevel(event.error_severity);
      const level = node && eventLevel === 'info' ? diagnosticsSeverityLevel(node.error_severity ?? 'error') : eventLevel;
      const phase = event.error_phase ?? node?.error_phase ?? null;
      const message = [
        event.summary,
        event.detail ?? null,
        phase ? `phase ${diagnosticsWords(phase)}` : null,
        node?.error_code ?? null,
      ]
        .filter((part): part is string => Boolean(part && part.trim().length > 0))
        .join(', ');
      const source = [
        formatDiagnosticSourceComponent(event.source_component),
        `seq ${event.event_seq}`,
        hasTimelinePayload(event) ? 'payload captured' : null,
      ]
        .filter((part): part is string => part !== null)
        .join(', ');
      const kind = node ? node.node_id : formatDiagnosticEventKind(event.event_kind);
      return {
        eventId: event.event_id,
        domId: diagnosticsLogLineDomId(event.event_seq),
        seq: event.event_seq,
        time: formatDiagnosticsLogTime(event.occurred_at_ms),
        level,
        kind,
        message,
        source,
        sourceComponent: event.source_component,
        nodeId: node?.node_id ?? null,
        searchText: `${kind} ${message} ${source} ${event.event_id}`.toLowerCase(),
      };
    });
}

export function buildDiagnosticsLevelOptions(lines: DiagnosticsLogLine[]): SegmentOption<DiagnosticsLevelFilter>[] {
  return [
    { value: 'all', label: 'All', count: lines.length },
    { value: 'warn', label: 'Warnings', count: lines.filter((line) => line.level === 'warn').length },
    { value: 'error', label: 'Errors', count: lines.filter((line) => line.level === 'error').length },
  ];
}

export interface DiagnosticsLogFilter {
  level: DiagnosticsLevelFilter;
  source: DiagnosticsSourceFilter;
  query: string;
}

export function filterDiagnosticsLogLines(lines: DiagnosticsLogLine[], filter: DiagnosticsLogFilter): DiagnosticsLogLine[] {
  const query = filter.query.trim().toLowerCase();
  const component = filter.source === 'all' ? null : SOURCE_FILTER_COMPONENT[filter.source];
  return lines.filter(
    (line) =>
      (filter.level === 'all' || line.level === filter.level) &&
      (component === null || line.sourceComponent === component) &&
      (query.length === 0 || line.searchText.includes(query)),
  );
}

/**
 * Nodes with an error whose error event is not in the timeline. These keep a
 * compact table under the error card so no node error is unreachable.
 */
export function buildDiagnosticsNodeErrorsOutsideTimeline(
  nodes: NodeStatusProjectionRecord[],
  events: SchedulerTimelineProjectionRecord[],
): NodeStatusProjectionRecord[] {
  const eventIds = new Set(events.map((event) => event.event_id));
  return nodes.filter((node) => {
    if (!diagnosticsNodeHasError(node)) return false;
    const ids = [node.canonical_error_event_id, node.error_event_id].filter((id): id is string => Boolean(id));
    return !ids.some((id) => eventIds.has(id));
  });
}

export function diagnosticsNodeErrorEventLabel(node: NodeStatusProjectionRecord): string {
  return nodeErrorEventId(node) ?? 'Not linked';
}

// --- Deep-link focus ------------------------------------------------------------

/** Structural copy of `DiagnosticsFocusTarget` (workbenchStore) so presenters stay store-free. */
export interface DiagnosticsFocusRequest {
  workflow_run_id: string;
  diagnostic_event_id?: string | null;
  node_id?: string | null;
  requested_at_ms: number;
}

export interface DiagnosticsFocusResolution {
  /** Event to mark selected; null when the focus names only a node with no linked event. */
  eventId: string | null;
  nodeId: string | null;
  /** DOM id of the line to scroll into view; null when the event is not in the timeline. */
  lineDomId: string | null;
  /** Level filter that shows the focused line. */
  level: DiagnosticsLevelFilter;
}

/** Identity of one focus request; the page applies each request once. */
export function diagnosticsFocusKey(focus: DiagnosticsFocusRequest): string {
  return `${focus.workflow_run_id}:${focus.requested_at_ms}`;
}

/**
 * Resolve the workbench diagnostics focus for the loaded run: which line to
 * select and scroll to, and which level filter shows it. Returns null when
 * there is no focus or it belongs to another run.
 */
export function resolveDiagnosticsFocus(
  focus: DiagnosticsFocusRequest | null,
  runId: string,
  lines: DiagnosticsLogLine[],
  nodes: NodeStatusProjectionRecord[],
): DiagnosticsFocusResolution | null {
  if (!focus || focus.workflow_run_id !== runId) {
    return null;
  }
  const focusedNode = focus.node_id ? nodes.find((node) => node.node_id === focus.node_id) : undefined;
  const eventId = focus.diagnostic_event_id ?? (focusedNode ? nodeErrorEventId(focusedNode) : null);
  const line =
    (eventId ? lines.find((candidate) => candidate.eventId === eventId) : undefined) ??
    (focus.node_id ? lines.find((candidate) => candidate.nodeId === focus.node_id) : undefined);
  return {
    eventId: line?.eventId ?? eventId,
    nodeId: focus.node_id ?? line?.nodeId ?? null,
    lineDomId: line?.domId ?? null,
    level: line && line.level !== 'info' ? line.level : 'all',
  };
}

// --- Error card -----------------------------------------------------------------

export interface DiagnosticsErrorExplanation {
  title: string;
  explanation: string;
  /** Starts with "Try:". */
  suggestion: string;
}

interface KnownDiagnosticsError {
  title: string;
  /** What happened, completed by the projected message. */
  lead: string;
  suggestion: string;
}

/**
 * Plain-language explanations for diagnostic error codes from the workflow
 * service error registry (`crates/pantograph-workflow-service/src/workflow/diagnostic_errors.rs`).
 * Codes not listed here use the neutral fallback, which restates the message.
 */
const KNOWN_DIAGNOSTICS_ERRORS: Record<string, KnownDiagnosticsError> = {
  runtime_model_load_failed: {
    title: 'Model failed to load',
    lead: 'The runtime could not load the model for this run',
    suggestion:
      'Try: check free memory on the device the run was placed on (Network shows it), or choose a smaller model, then run the workflow again.',
  },
  runtime_preflight_failed: {
    title: 'Runtime check failed before start',
    lead: 'The scheduler checked the runtime and model before starting and the check did not pass',
    suggestion: 'Try: confirm the runtime and model the workflow asks for are installed, then run it again.',
  },
  runtime_launch_failed: {
    title: 'Runtime did not start',
    lead: 'The runtime process for this run could not be started',
    suggestion: 'Try: check the runtime installation in Settings, then run the workflow again.',
  },
  scheduler_admission_failed: {
    title: 'Scheduler did not admit the run',
    lead: 'The scheduler could not admit this run',
    suggestion: 'Try: check the placement and device memory in Scheduler and Network, then submit the run again.',
  },
  model_dependency_failed: {
    title: 'Model dependency unavailable',
    lead: 'A dependency the model needs could not be resolved',
    suggestion: 'Try: open the model in Library and resolve its dependencies, then run the workflow again.',
  },
  node_execution_failed: {
    title: 'Node failed',
    lead: 'A node stopped with an error while the run executed',
    suggestion: "Try: look at the node's inputs in I/O Inspector, fix the input or the node's settings, then run it again.",
  },
  output_validation_failed: {
    title: 'Output did not match its port',
    lead: 'A node produced output that did not match its declared port type',
    suggestion: "Try: compare the node's output in I/O Inspector with the type its port declares.",
  },
  run_timeout: {
    title: 'Run timed out',
    lead: 'The run did not finish within its time limit',
    suggestion: 'Try: run it again when the device is less busy, or with a longer time limit.',
  },
};

const FALLBACK_DIAGNOSTICS_SUGGESTION = 'Try: show the error in the log to see the events that led up to it.';

function asSentence(text: string): string {
  const trimmed = text.trim();
  if (trimmed.length === 0) return trimmed;
  return /[.!?]$/.test(trimmed) ? trimmed : `${trimmed}.`;
}

/**
 * Title, one-sentence explanation and "Try:" advice for a diagnostic error,
 * from its code and message. Known codes get a specific explanation; any other
 * code gets a neutral sentence that restates the message.
 */
export function explainDiagnosticsError(input: {
  code: string | null | undefined;
  message: string | null | undefined;
  phase?: string | null;
  nodeId?: string | null;
}): DiagnosticsErrorExplanation {
  const message = input.message?.trim() ?? '';
  const where = input.nodeId ? ` in ${input.nodeId}` : '';
  const known = input.code ? KNOWN_DIAGNOSTICS_ERRORS[input.code] : undefined;
  if (known) {
    return {
      title: `${known.title}${where}`,
      explanation: asSentence(message.length > 0 ? `${known.lead}: ${message}` : known.lead),
      suggestion: known.suggestion,
    };
  }
  const phase = input.phase && input.phase !== 'unassigned' ? formatDiagnosticErrorPhase(input.phase) : null;
  return {
    title: phase ? `${phase} failed${where}` : `The run reported an error${where}`,
    explanation: message.length > 0 ? asSentence(`The run reported: ${message}`) : 'No error message was projected for this run.',
    suggestion: FALLBACK_DIAGNOSTICS_SUGGESTION,
  };
}

export interface DiagnosticsErrorCard extends DiagnosticsErrorExplanation {
  /** `error` renders the danger card; `warning` renders a muted line instead. */
  kind: 'error' | 'warning';
  /** Muted machine detail: code, phase and severity. */
  codeLine: string;
  eventId: string | null;
  nodeId: string | null;
}

/**
 * The run's latest or terminal error as an error card, or null when the run
 * has none. A warning on a run that did not fail is returned with kind
 * `warning` so the page can mention it without the danger treatment.
 */
export function buildDiagnosticsErrorCard(
  run: RunDetailProjectionRecord,
  nodes: NodeStatusProjectionRecord[],
): DiagnosticsErrorCard | null {
  const summary = buildDiagnosticsRunErrorSummary(run);
  if (!summary && !run.terminal_error) {
    return null;
  }
  const eventId = summary && summary.eventId !== 'unassigned' ? summary.eventId : null;
  const code = summary?.code ?? 'unknown_error';
  const phase = summary?.phase ?? 'unassigned';
  const severity: DiagnosticErrorSeverity = summary?.severity ?? 'fatal';
  const message = summary?.message ?? run.terminal_error ?? null;
  const node =
    (eventId ? nodes.find((candidate) => nodeErrorEventId(candidate) === eventId) : undefined) ??
    nodes.find((candidate) => candidate.status === 'failed');
  const nodeId = node?.node_id ?? null;
  const explanation = explainDiagnosticsError({ code, message, phase, nodeId });
  const codeParts = [code];
  if (phase !== 'unassigned') codeParts.push(`phase ${diagnosticsWords(phase)}`);
  codeParts.push(formatDiagnosticErrorSeverity(severity).toLowerCase());
  return {
    ...explanation,
    kind: severity === 'warning' && run.status !== 'failed' ? 'warning' : 'error',
    codeLine: codeParts.join(', '),
    eventId,
    nodeId,
  };
}

// --- Refresh ownership ------------------------------------------------------------

/**
 * Current-invocation authority for the page's projection refreshes. The page
 * owns one gate: each refresh takes a token, and only the newest token may
 * apply its results. `close()` (on destroy) makes every outstanding token stale.
 */
export interface DiagnosticsRefreshGate {
  begin(): number;
  isCurrent(token: number): boolean;
  close(): void;
}

export function createDiagnosticsRefreshGate(): DiagnosticsRefreshGate {
  let generation = 0;
  let closed = false;
  return {
    begin() {
      generation += 1;
      return generation;
    },
    isCurrent(token) {
      return !closed && token === generation;
    },
    close() {
      closed = true;
      generation += 1;
    },
  };
}

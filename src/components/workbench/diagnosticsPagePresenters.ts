import type {
  DiagnosticEventKind,
  DiagnosticEventSourceComponent,
  IoArtifactRetentionState,
  IoArtifactRetentionSummaryRecord,
  NodeStatusProjectionRecord,
  ProjectionStateRecord,
  RunDetailProjectionRecord,
  RunListFacetKind,
  RunListFacetRecord,
  RunListProjectionRecord,
  SchedulerTimelineProjectionRecord,
} from '../../services/diagnostics/types';

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

export function formatDiagnosticsTimestamp(value: number | null | undefined): string {
  if (!value) {
    return 'Unavailable';
  }
  return new Date(value).toLocaleString();
}

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
  if (!state) {
    return 'Projection unavailable';
  }
  const cursor = `seq ${state.last_applied_event_seq}`;
  switch (state.status) {
    case 'current':
      return `Current at ${cursor}`;
    case 'rebuilding':
      return `Rebuilding at ${cursor}`;
    case 'needs_rebuild':
      return `Needs rebuild at ${cursor}`;
    case 'failed':
      return `Failed at ${cursor}`;
  }
}

export function diagnosticsStatusClass(status: RunDetailProjectionRecord['status']): string {
  switch (status) {
    case 'completed':
      return 'border-emerald-700 bg-emerald-950/60 text-emerald-200';
    case 'running':
      return 'border-cyan-700 bg-cyan-950/60 text-cyan-200';
    case 'queued':
    case 'accepted':
    case 'future':
    case 'scheduled':
      return 'border-amber-700 bg-amber-950/60 text-amber-200';
    case 'delayed':
      return 'border-orange-700 bg-orange-950/60 text-orange-200';
    case 'failed':
      return 'border-red-700 bg-red-950/60 text-red-200';
    case 'cancelled':
      return 'border-neutral-700 bg-neutral-900 text-neutral-300';
  }
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

export function formatDiagnosticEventKind(kind: DiagnosticEventKind): string {
  return kind
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function formatDiagnosticSourceComponent(source: DiagnosticEventSourceComponent): string {
  return source
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function hasTimelinePayload(event: Pick<SchedulerTimelineProjectionRecord, 'payload_json'>): boolean {
  const payload = event.payload_json.trim();
  return payload.length > 0 && payload !== '{}' && payload !== 'null';
}

import type {
  DiagnosticEventKind,
  DiagnosticEventSourceComponent,
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

export interface DiagnosticsComparisonFilters {
  status: string;
  schedulerPolicy: string;
  retentionPolicy: string;
  client: string;
  clientSession: string;
  bucket: string;
  acceptedDate: string;
}

export interface DiagnosticsComparisonFilterOptions {
  statuses: string[];
  schedulerPolicies: string[];
  retentionPolicies: string[];
  clients: string[];
  clientSessions: string[];
  buckets: string[];
  acceptedDates: string[];
}

export const DIAGNOSTICS_FILTER_ALL = 'all';

export const DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS: DiagnosticsComparisonFilters = {
  status: DIAGNOSTICS_FILTER_ALL,
  schedulerPolicy: DIAGNOSTICS_FILTER_ALL,
  retentionPolicy: DIAGNOSTICS_FILTER_ALL,
  client: DIAGNOSTICS_FILTER_ALL,
  clientSession: DIAGNOSTICS_FILTER_ALL,
  bucket: DIAGNOSTICS_FILTER_ALL,
  acceptedDate: DIAGNOSTICS_FILTER_ALL,
};

export const EMPTY_DIAGNOSTICS_COMPARISON_FILTER_OPTIONS: DiagnosticsComparisonFilterOptions = {
  statuses: [],
  schedulerPolicies: [],
  retentionPolicies: [],
  clients: [],
  clientSessions: [],
  buckets: [],
  acceptedDates: [],
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
    if (status === 'queued' || status === 'accepted' || status === 'delayed') {
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
      return 'border-amber-700 bg-amber-950/60 text-amber-200';
    case 'delayed':
      return 'border-orange-700 bg-orange-950/60 text-orange-200';
    case 'failed':
      return 'border-red-700 bg-red-950/60 text-red-200';
    case 'cancelled':
      return 'border-neutral-700 bg-neutral-900 text-neutral-300';
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
    { label: 'Scheduler Reason', value: run.scheduler_reason ?? 'Unavailable', mono: false },
    { label: 'Timeline Events', value: String(run.timeline_event_count), mono: false },
    { label: 'Last Event Seq', value: String(run.last_event_seq), mono: false },
  ];
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

export function buildDiagnosticsComparisonFilterOptions(
  activeRun: RunDetailProjectionRecord,
  runs: RunListProjectionRecord[],
): DiagnosticsComparisonFilterOptions {
  const scopedRuns = ensureActiveRunInScope(activeRun, runs);
  return {
    statuses: uniqueSorted(scopedRuns.map((run) => run.status)),
    schedulerPolicies: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.scheduler_policy_id))),
    retentionPolicies: uniqueSorted(scopedRuns.map((run) => optionalFacetLabel(run.retention_policy_id))),
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
  return Object.values(filters).some((value) => value !== DIAGNOSTICS_FILTER_ALL);
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
    filterMatches(run.status, filters.status) &&
    filterMatches(optionalFacetLabel(run.scheduler_policy_id), filters.schedulerPolicy) &&
    filterMatches(optionalFacetLabel(run.retention_policy_id), filters.retentionPolicy) &&
    filterMatches(optionalFacetLabel(run.client_id), filters.client) &&
    filterMatches(optionalFacetLabel(run.client_session_id), filters.clientSession) &&
    filterMatches(optionalFacetLabel(run.bucket_id), filters.bucket) &&
    filterMatches(acceptedDateLabel(run.accepted_at_ms), filters.acceptedDate)
  );
}

function filterMatches(value: string, filter: string): boolean {
  return filter === DIAGNOSTICS_FILTER_ALL || value === filter;
}

function uniqueSorted(values: string[]): string[] {
  return [...new Set(values)].sort((left, right) => left.localeCompare(right));
}

function workflowVersionLabel(run: RunListProjectionRecord): string {
  return run.workflow_semantic_version ?? run.workflow_version_id ?? 'Unversioned';
}

function optionalFacetLabel(value: string | null | undefined): string {
  return value && value.trim().length > 0 ? value : 'Unassigned';
}

function acceptedDateLabel(value: number | null | undefined): string {
  if (!value) {
    return 'Unassigned';
  }
  return new Date(value).toISOString().slice(0, 10);
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

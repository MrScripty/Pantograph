import type {
  DiagnosticEventKind,
  DiagnosticEventSourceComponent,
  ProjectionStateRecord,
  RunDetailProjectionRecord,
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
    if (status === 'queued' || status === 'accepted') {
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
    { label: 'Bucket', value: run.bucket_id ?? 'Default', mono: true },
    { label: 'Scheduler Policy', value: run.scheduler_policy_id ?? 'Default', mono: true },
    { label: 'Retention Policy', value: run.retention_policy_id ?? 'Default', mono: true },
    { label: 'Timeline Events', value: String(run.timeline_event_count), mono: false },
    { label: 'Last Event Seq', value: String(run.last_event_seq), mono: false },
  ];
}

export function buildDiagnosticsFacetSummary(
  activeRun: RunDetailProjectionRecord,
  runs: RunListProjectionRecord[],
): DiagnosticsFacetSummary {
  const workflowRuns = runs.filter((run) => run.workflow_id === activeRun.workflow_id);
  const activeRunIsProjected = workflowRuns.some((run) => run.workflow_run_id === activeRun.workflow_run_id);
  const scopedRuns = activeRunIsProjected ? workflowRuns : [activeRun, ...workflowRuns];
  const total = scopedRuns.length;
  const rows = [
    buildDiagnosticsFacetRow('Workflow Version', workflowVersionLabel(activeRun), scopedRuns, workflowVersionLabel, total),
    buildDiagnosticsFacetRow('Status', activeRun.status, scopedRuns, (run) => run.status, total),
    buildDiagnosticsFacetRow(
      'Scheduler Policy',
      optionalFacetLabel(activeRun.scheduler_policy_id),
      scopedRuns,
      (run) => optionalFacetLabel(run.scheduler_policy_id),
      total,
    ),
    buildDiagnosticsFacetRow(
      'Retention Policy',
      optionalFacetLabel(activeRun.retention_policy_id),
      scopedRuns,
      (run) => optionalFacetLabel(run.retention_policy_id),
      total,
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

  const workflowVersions = new Set(scopedRuns.map(workflowVersionLabel));
  const mixedVersionWarning =
    workflowVersions.size > 1
      ? `${activeRun.workflow_id} has ${workflowVersions.size} workflow versions in the current run-list projection.`
      : null;

  return { rows, mixedVersionWarning };
}

function buildDiagnosticsFacetRow<T extends RunListProjectionRecord>(
  label: string,
  value: string,
  runs: T[],
  readValue: (run: T) => string,
  total: number,
): DiagnosticsFacetRow {
  return {
    label,
    value,
    count: runs.filter((run) => readValue(run) === value).length,
    total,
  };
}

function workflowVersionLabel(run: RunListProjectionRecord): string {
  return run.workflow_semantic_version ?? run.workflow_version_id ?? 'Unversioned';
}

function optionalFacetLabel(value: string | null | undefined): string {
  return value && value.trim().length > 0 ? value : 'Unassigned';
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

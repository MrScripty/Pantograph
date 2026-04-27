import type {
  RunListProjectionRecord,
  RunListProjectionStatus,
} from '../../services/diagnostics/types';

export type SchedulerStatusFilter = 'all' | RunListProjectionStatus;

export type SchedulerSortKey =
  | 'last_updated_desc'
  | 'queued_desc'
  | 'duration_desc'
  | 'workflow_asc'
  | 'status_asc';

export interface SchedulerRunFilters {
  search: string;
  status: SchedulerStatusFilter;
  sort: SchedulerSortKey;
}

export const SCHEDULER_STATUS_FILTERS: SchedulerStatusFilter[] = [
  'all',
  'accepted',
  'queued',
  'running',
  'completed',
  'failed',
  'cancelled',
];

export const SCHEDULER_SORT_OPTIONS: { label: string; value: SchedulerSortKey }[] = [
  { label: 'Updated', value: 'last_updated_desc' },
  { label: 'Queued', value: 'queued_desc' },
  { label: 'Duration', value: 'duration_desc' },
  { label: 'Workflow', value: 'workflow_asc' },
  { label: 'Status', value: 'status_asc' },
];

export function formatSchedulerTimestamp(value: number | null | undefined): string {
  if (!value) {
    return 'Unscheduled';
  }
  return new Date(value).toLocaleString();
}

export function formatSchedulerDuration(
  value: number | null | undefined,
  status: RunListProjectionRecord['status'],
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

export function schedulerStatusClass(status: RunListProjectionRecord['status']): string {
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

export function filterAndSortSchedulerRuns(
  runs: RunListProjectionRecord[],
  filters: SchedulerRunFilters,
): RunListProjectionRecord[] {
  const search = filters.search.trim().toLowerCase();
  const filtered = runs
    .filter((run) => filters.status === 'all' || run.status === filters.status)
    .filter((run) => search.length === 0 || schedulerRunMatchesSearch(run, search));
  return [...filtered].sort((left, right) => compareSchedulerRuns(left, right, filters.sort));
}

function schedulerRunMatchesSearch(run: RunListProjectionRecord, search: string): boolean {
  return [
    run.workflow_run_id,
    run.workflow_id,
    run.workflow_version_id,
    run.workflow_semantic_version,
    run.scheduler_policy_id,
    run.retention_policy_id,
    run.status,
  ].some((value) => value?.toLowerCase().includes(search));
}

function compareSchedulerRuns(
  left: RunListProjectionRecord,
  right: RunListProjectionRecord,
  sort: SchedulerSortKey,
): number {
  switch (sort) {
    case 'last_updated_desc':
      return right.last_updated_at_ms - left.last_updated_at_ms;
    case 'queued_desc':
      return (right.enqueued_at_ms ?? right.accepted_at_ms ?? 0) - (left.enqueued_at_ms ?? left.accepted_at_ms ?? 0);
    case 'duration_desc':
      return (right.duration_ms ?? -1) - (left.duration_ms ?? -1);
    case 'workflow_asc':
      return compareStrings(left.workflow_id, right.workflow_id) || compareStrings(left.workflow_run_id, right.workflow_run_id);
    case 'status_asc':
      return compareStrings(left.status, right.status) || compareStrings(left.workflow_run_id, right.workflow_run_id);
  }
}

function compareStrings(left: string, right: string): number {
  return left.localeCompare(right);
}

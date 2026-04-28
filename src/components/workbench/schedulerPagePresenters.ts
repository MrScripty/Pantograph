import type {
  ProjectionStateRecord,
  RunListProjectionRecord,
  WorkflowRunListQueryRequest,
  SchedulerTimelineProjectionRecord,
} from '../../services/diagnostics/types';
import type { SchedulerRunFilters, SchedulerSortKey } from '../../stores/schedulerRunListStore';

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

export function formatSchedulerQueuePosition(value: number | null | undefined): string {
  if (value === null || value === undefined) {
    return 'Unassigned';
  }
  return String(value);
}

export function formatSchedulerPriority(value: number | null | undefined): string {
  if (value === null || value === undefined) {
    return 'Default';
  }
  return String(value);
}

export function formatSchedulerEstimateLabel(run: RunListProjectionRecord): string {
  const parts = [
    run.estimated_queue_wait_ms === null || run.estimated_queue_wait_ms === undefined
      ? null
      : `wait ${formatSchedulerDuration(run.estimated_queue_wait_ms, run.status)}`,
    run.estimated_duration_ms === null || run.estimated_duration_ms === undefined
      ? null
      : `run ${formatSchedulerDuration(run.estimated_duration_ms, run.status)}`,
  ].filter((part): part is string => part !== null);
  if (parts.length === 0) {
    return run.estimate_confidence ? `${run.estimate_confidence} confidence` : 'Unavailable';
  }
  const confidence = run.estimate_confidence ? ` (${run.estimate_confidence})` : '';
  return `${parts.join(' / ')}${confidence}`;
}

export function formatSchedulerReasonLabel(value: string | null | undefined): string {
  return value && value.trim().length > 0 ? value : 'Unavailable';
}

export function schedulerStatusClass(status: RunListProjectionRecord['status']): string {
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

export function formatSchedulerProjectionFreshness(state: ProjectionStateRecord | null): string {
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

export function formatSchedulerPolicyLabel(value: string | null | undefined): string {
  return value && value.trim().length > 0 ? value : 'Unassigned';
}

export function formatSchedulerRetentionLabel(value: string | null | undefined): string {
  return value && value.trim().length > 0 ? value : 'Unassigned';
}

export function formatSchedulerScopeLabel(value: string | null | undefined): string {
  return value && value.trim().length > 0 ? value : 'Unassigned';
}

export function schedulerRunSupportsQueueControls(run: RunListProjectionRecord | null | undefined): boolean {
  if (!run?.workflow_execution_session_id) {
    return false;
  }
  return run.status === 'queued' || run.status === 'delayed';
}

export function schedulerRunSupportsAdminQueueControls(run: RunListProjectionRecord | null | undefined): boolean {
  return run?.status === 'queued' || run?.status === 'delayed';
}

export function schedulerPolicyFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerPolicyLabel(run.scheduler_policy_id)));
}

export function schedulerRetentionFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerRetentionLabel(run.retention_policy_id)));
}

export function schedulerClientFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerScopeLabel(run.client_id)));
}

export function schedulerClientSessionFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerScopeLabel(run.client_session_id)));
}

export function schedulerBucketFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerScopeLabel(run.bucket_id)));
}

export function schedulerAcceptedDateFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerAcceptedDateLabel(run.accepted_at_ms)));
}

export function formatSchedulerAcceptedDateLabel(value: number | null | undefined): string {
  if (!value) {
    return 'Unassigned';
  }
  return new Date(value).toISOString().slice(0, 10);
}

export function formatSchedulerTimelineKind(
  event: Pick<SchedulerTimelineProjectionRecord, 'event_kind'>,
): string {
  return event.event_kind
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function formatSchedulerTimelineSource(
  event: Pick<SchedulerTimelineProjectionRecord, 'source_component'>,
): string {
  return event.source_component
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function schedulerTimelinePayloadLabel(
  event: Pick<SchedulerTimelineProjectionRecord, 'payload_json'>,
): string {
  const payload = event.payload_json.trim();
  if (payload.length > 0 && payload !== '{}' && payload !== 'null') {
    return 'Payload captured';
  }
  return 'Metadata only';
}

export function schedulerTimelineKindFilterOptions(
  events: Pick<SchedulerTimelineProjectionRecord, 'event_kind'>[],
): SchedulerTimelineProjectionRecord['event_kind'][] {
  return uniqueSortedOptions(events.map((event) => event.event_kind)) as SchedulerTimelineProjectionRecord['event_kind'][];
}

export function schedulerTimelineSourceFilterOptions(
  events: Pick<SchedulerTimelineProjectionRecord, 'source_component'>[],
): SchedulerTimelineProjectionRecord['source_component'][] {
  return uniqueSortedOptions(events.map((event) => event.source_component)) as SchedulerTimelineProjectionRecord['source_component'][];
}

export function filterSchedulerTimelineEvents(
  events: SchedulerTimelineProjectionRecord[],
  filters: { eventKind: string; sourceComponent: string },
): SchedulerTimelineProjectionRecord[] {
  return events.filter(
    (event) =>
      (filters.eventKind === 'all' || event.event_kind === filters.eventKind) &&
      (filters.sourceComponent === 'all' || event.source_component === filters.sourceComponent),
  );
}

export function filterAndSortSchedulerRuns(
  runs: RunListProjectionRecord[],
  filters: SchedulerRunFilters,
): RunListProjectionRecord[] {
  const search = filters.search.trim().toLowerCase();
  const filtered = runs
    .filter((run) => filters.status === 'all' || run.status === filters.status)
    .filter(
      (run) =>
        filters.schedulerPolicy === 'all' ||
        formatSchedulerPolicyLabel(run.scheduler_policy_id) === filters.schedulerPolicy,
    )
    .filter(
      (run) =>
        filters.retentionPolicy === 'all' ||
        formatSchedulerRetentionLabel(run.retention_policy_id) === filters.retentionPolicy,
    )
    .filter(
      (run) =>
        filters.client === 'all' ||
        formatSchedulerScopeLabel(run.client_id) === filters.client,
    )
    .filter(
      (run) =>
        filters.clientSession === 'all' ||
        formatSchedulerScopeLabel(run.client_session_id) === filters.clientSession,
    )
    .filter(
      (run) =>
        filters.bucket === 'all' ||
        formatSchedulerScopeLabel(run.bucket_id) === filters.bucket,
    )
    .filter(
      (run) =>
        filters.acceptedDate === 'all' ||
        formatSchedulerAcceptedDateLabel(run.accepted_at_ms) === filters.acceptedDate,
    )
    .filter((run) => search.length === 0 || schedulerRunMatchesSearch(run, search));
  return [...filtered].sort((left, right) => compareSchedulerRuns(left, right, filters.sort));
}

export function buildSchedulerRunListQuery(
  filters: SchedulerRunFilters,
  limit: number,
): WorkflowRunListQueryRequest {
  const request: WorkflowRunListQueryRequest = { limit };
  if (filters.status !== 'all') {
    request.status = filters.status;
  }
  if (isAssignedFilterValue(filters.schedulerPolicy)) {
    request.scheduler_policy_id = filters.schedulerPolicy;
  }
  if (isAssignedFilterValue(filters.retentionPolicy)) {
    request.retention_policy_id = filters.retentionPolicy;
  }
  if (isAssignedFilterValue(filters.client)) {
    request.client_id = filters.client;
  }
  if (isAssignedFilterValue(filters.clientSession)) {
    request.client_session_id = filters.clientSession;
  }
  if (isAssignedFilterValue(filters.bucket)) {
    request.bucket_id = filters.bucket;
  }
  const acceptedRange = schedulerAcceptedDateRange(filters.acceptedDate);
  if (acceptedRange) {
    request.accepted_at_from_ms = acceptedRange.fromMs;
    request.accepted_at_to_ms = acceptedRange.toMs;
  }
  return request;
}

function uniqueSortedOptions(values: string[]): string[] {
  return [...new Set(values)].sort(compareFilterOptions);
}

function compareFilterOptions(left: string, right: string): number {
  if (left === 'Unassigned' && right !== 'Unassigned') {
    return -1;
  }
  if (right === 'Unassigned' && left !== 'Unassigned') {
    return 1;
  }
  return compareStrings(left, right);
}

function isAssignedFilterValue(value: string): boolean {
  return value !== 'all' && value !== 'Unassigned';
}

function schedulerAcceptedDateRange(value: string): { fromMs: number; toMs: number } | null {
  if (!isAssignedFilterValue(value)) {
    return null;
  }
  const fromMs = Date.parse(`${value}T00:00:00.000Z`);
  if (!Number.isFinite(fromMs)) {
    return null;
  }
  return {
    fromMs,
    toMs: fromMs + 86_400_000 - 1,
  };
}

function schedulerRunMatchesSearch(run: RunListProjectionRecord, search: string): boolean {
  return [
    run.workflow_run_id,
    run.workflow_id,
    run.workflow_version_id,
    run.workflow_semantic_version,
    run.scheduler_policy_id,
    run.retention_policy_id,
    run.client_id,
    run.client_session_id,
    run.bucket_id,
    run.workflow_execution_session_id,
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

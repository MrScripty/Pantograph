import type {
  IoArtifactRetentionState,
  IoArtifactRetentionSummaryRecord,
  ProjectionStateRecord,
  RunListProjectionRecord,
  WorkflowSchedulerEstimateRecord,
  SchedulerTimelineProjectionRecord,
} from '../../services/diagnostics/types';
import {
  DEFAULT_SCHEDULER_RUN_FILTERS,
  SCHEDULER_SECONDARY_FILTER_KEYS,
  SCHEDULER_SORT_OPTIONS,
  SCHEDULER_STATUS_FILTERS,
  SCHEDULER_STATUS_SEGMENTS,
  type SchedulerRunFilters,
  type SchedulerSecondaryFilterKey,
  type SchedulerSortKey,
  type SchedulerStatusSegment,
} from '../../stores/schedulerRunListStore.ts';
import { formatProjectionFreshnessState } from './projectionFreshness.ts';
import {
  formatClockTime,
  formatRunDuration,
  groupRunsForContextPicker,
  startOfLocalDayMs,
  type RunContextGroupId,
  type RunContextRow,
} from './shell/runContextPicker.ts';
import type { SegmentOption } from './ui/types.ts';

export function formatSchedulerTimestamp(value: number | null | undefined): string {
  if (!value) {
    return 'Unscheduled';
  }
  return new Date(value).toLocaleString();
}

export function formatSchedulerModelCacheState(
  state: RunListProjectionRecord['model_cache_state'] | WorkflowSchedulerEstimateRecord['model_cache_state'],
): string {
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

export interface SchedulerRetentionSummaryRow {
  label: string;
  count: number;
}

export function buildSchedulerRetentionSummaryRows(
  summary: IoArtifactRetentionSummaryRecord[],
): SchedulerRetentionSummaryRow[] {
  return summary
    .map((item) => ({
      label: formatSchedulerRetentionStateLabel(item.retention_state),
      count: item.artifact_count,
    }))
    .sort((left, right) => right.count - left.count || left.label.localeCompare(right.label));
}

export function formatSchedulerRetentionStateLabel(retentionState: IoArtifactRetentionState): string {
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

export function formatSchedulerStatusLabel(status: RunListProjectionRecord['status']): string {
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

export function schedulerRunSupportsDependencyReadinessResume(
  run: RunListProjectionRecord | null | undefined,
): boolean {
  return Boolean(
    run?.workflow_execution_session_id
      && run.workflow_execution_session_resume_state === 'dependency_readiness_pending',
  );
}

export function formatSchedulerProjectionFreshness(state: ProjectionStateRecord | null): string {
  return formatProjectionFreshnessState(state);
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

export function formatSchedulerPlacementLabel(value: string | null | undefined): string {
  return value && value.trim().length > 0 ? value : 'Unassigned';
}

export function formatSchedulerRuntimePlacementLabel(
  run: Pick<RunListProjectionRecord, 'selected_runtime_id' | 'selected_runtime_variant_id'>,
): string {
  const runtimeId = formatSchedulerPlacementLabel(run.selected_runtime_id);
  const runtimeVariantId = formatSchedulerPlacementLabel(run.selected_runtime_variant_id);
  if (runtimeId === 'Unassigned') {
    return runtimeVariantId;
  }
  if (runtimeVariantId === 'Unassigned') {
    return runtimeId;
  }
  return `${runtimeId} / ${runtimeVariantId}`;
}

export function formatSchedulerDevicePlacementLabel(
  run: Pick<RunListProjectionRecord, 'selected_device_class' | 'selected_device_id'>,
): string {
  const deviceClass = formatSchedulerPlacementLabel(run.selected_device_class);
  const deviceId = formatSchedulerPlacementLabel(run.selected_device_id);
  if (deviceClass === 'Unassigned') {
    return deviceId;
  }
  if (deviceId === 'Unassigned') {
    return deviceClass;
  }
  return `${deviceClass} / ${deviceId}`;
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

export function schedulerSelectedRuntimeFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerPlacementLabel(run.selected_runtime_id)));
}

export function schedulerSelectedRuntimeVariantFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerPlacementLabel(run.selected_runtime_variant_id)));
}

export function schedulerSelectedBackendFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerPlacementLabel(run.selected_backend_key)));
}

export function schedulerSelectedDeviceClassFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerPlacementLabel(run.selected_device_class)));
}

export function schedulerSelectedDeviceFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerPlacementLabel(run.selected_device_id)));
}

export function schedulerSelectedNetworkNodeFilterOptions(runs: RunListProjectionRecord[]): string[] {
  return uniqueSortedOptions(runs.map((run) => formatSchedulerPlacementLabel(run.selected_network_node_id)));
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

export interface SchedulerTimelineAttemptFactRow {
  label: string;
  value: string;
}

export function buildSchedulerTimelineAttemptRows(
  event: Pick<
    SchedulerTimelineProjectionRecord,
    | 'scheduler_task_id'
    | 'scheduler_attempt_id'
    | 'scheduler_attempt_execution_class'
    | 'scheduler_attempt_transition'
    | 'scheduler_attempt_started_at_ms'
    | 'scheduler_attempt_ended_at_ms'
    | 'scheduler_attempt_duration_ms'
    | 'scheduler_attempt_runtime_id'
    | 'scheduler_attempt_runtime_variant_id'
    | 'scheduler_attempt_backend_key'
    | 'scheduler_attempt_device_class'
    | 'scheduler_attempt_device_id'
    | 'scheduler_attempt_network_node_id'
    | 'scheduler_attempt_reservation_id'
  >,
): SchedulerTimelineAttemptFactRow[] {
  const rows: SchedulerTimelineAttemptFactRow[] = [];
  addSchedulerAttemptRow(rows, 'Task', event.scheduler_task_id);
  addSchedulerAttemptRow(rows, 'Attempt', event.scheduler_attempt_id);
  addSchedulerAttemptRow(rows, 'Transition', formatSchedulerAttemptEnum(event.scheduler_attempt_transition));
  addSchedulerAttemptRow(rows, 'Class', formatSchedulerAttemptEnum(event.scheduler_attempt_execution_class));
  addSchedulerAttemptRow(rows, 'Started', formatSchedulerAttemptTimestamp(event.scheduler_attempt_started_at_ms));
  addSchedulerAttemptRow(rows, 'Ended', formatSchedulerAttemptTimestamp(event.scheduler_attempt_ended_at_ms));
  addSchedulerAttemptRow(rows, 'Duration', formatSchedulerAttemptDuration(event.scheduler_attempt_duration_ms));
  addSchedulerAttemptRow(rows, 'Runtime', event.scheduler_attempt_runtime_id);
  addSchedulerAttemptRow(rows, 'Variant', event.scheduler_attempt_runtime_variant_id);
  addSchedulerAttemptRow(rows, 'Backend', event.scheduler_attempt_backend_key);
  addSchedulerAttemptRow(rows, 'Device Class', event.scheduler_attempt_device_class);
  addSchedulerAttemptRow(rows, 'Device', event.scheduler_attempt_device_id);
  addSchedulerAttemptRow(rows, 'Network Node', event.scheduler_attempt_network_node_id);
  addSchedulerAttemptRow(rows, 'Reservation', event.scheduler_attempt_reservation_id);
  return rows;
}

function addSchedulerAttemptRow(
  rows: SchedulerTimelineAttemptFactRow[],
  label: string,
  value: string | null | undefined,
): void {
  if (value) {
    rows.push({ label, value });
  }
}

function formatSchedulerAttemptEnum(value: string | null | undefined): string | null {
  if (!value) {
    return null;
  }
  return value
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function formatSchedulerAttemptTimestamp(value: number | null | undefined): string | null {
  if (value === null || value === undefined) {
    return null;
  }
  return formatSchedulerTimestamp(value);
}

function formatSchedulerAttemptDuration(value: number | null | undefined): string | null {
  if (value === null || value === undefined) {
    return null;
  }
  return value < 1_000 ? `${Math.round(value)} ms` : `${(value / 1_000).toFixed(1)} s`;
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
        filters.selectedRuntime === 'all' ||
        formatSchedulerPlacementLabel(run.selected_runtime_id) === filters.selectedRuntime,
    )
    .filter(
      (run) =>
        filters.selectedRuntimeVariant === 'all' ||
        formatSchedulerPlacementLabel(run.selected_runtime_variant_id) === filters.selectedRuntimeVariant,
    )
    .filter(
      (run) =>
        filters.selectedBackend === 'all' ||
        formatSchedulerPlacementLabel(run.selected_backend_key) === filters.selectedBackend,
    )
    .filter(
      (run) =>
        filters.selectedDeviceClass === 'all' ||
        formatSchedulerPlacementLabel(run.selected_device_class) === filters.selectedDeviceClass,
    )
    .filter(
      (run) =>
        filters.selectedDevice === 'all' ||
        formatSchedulerPlacementLabel(run.selected_device_id) === filters.selectedDevice,
    )
    .filter(
      (run) =>
        filters.selectedNetworkNode === 'all' ||
        formatSchedulerPlacementLabel(run.selected_network_node_id) === filters.selectedNetworkNode,
    )
    .filter(
      (run) =>
        filters.acceptedDate === 'all' ||
        formatSchedulerAcceptedDateLabel(run.accepted_at_ms) === filters.acceptedDate,
    )
    .filter((run) => search.length === 0 || schedulerRunMatchesSearch(run, search));
  return [...filtered].sort((left, right) => compareSchedulerRuns(left, right, filters.sort));
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
    run.selected_runtime_id,
    run.selected_runtime_variant_id,
    run.selected_backend_key,
    run.selected_device_class,
    run.selected_device_id,
    run.selected_network_node_id,
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

// ---------------------------------------------------------------------------
// Redesigned Scheduler layout (docs/design/workbench-redesign-spec.md,
// "Scheduler"). Input is the shell's run-list projection; grouping and row
// decoration come from the run-context picker helpers so the list and the
// header picker always agree (issues WR-004, WR-005).
// ---------------------------------------------------------------------------

const SCHEDULER_STATUS_SEGMENT_LABELS: Record<SchedulerStatusSegment, string> = {
  all: 'All',
  active: 'Active',
  queued: 'Queued',
  done: 'Done',
  failed: 'Failed',
};

/**
 * Status segment of a run. There is no memory-wait status in the run-list
 * projection (WR-004), so Active holds running runs only and delayed runs
 * count as Queued, matching the picker's Queued group.
 */
export function schedulerStatusSegmentForRun(
  run: Pick<RunListProjectionRecord, 'status'>,
): Exclude<SchedulerStatusSegment, 'all'> {
  switch (run.status) {
    case 'running':
      return 'active';
    case 'accepted':
    case 'future':
    case 'scheduled':
    case 'queued':
    case 'delayed':
      return 'queued';
    case 'completed':
    case 'cancelled':
      return 'done';
    case 'failed':
      return 'failed';
  }
}

export function filterSchedulerRunsBySegment(
  runs: readonly RunListProjectionRecord[],
  segment: SchedulerStatusSegment,
): RunListProjectionRecord[] {
  return segment === 'all' ? [...runs] : runs.filter((run) => schedulerStatusSegmentForRun(run) === segment);
}

/** Segment buttons with counts over `pool` (the runs left after every other filter). */
export function buildSchedulerStatusSegments(
  pool: readonly RunListProjectionRecord[],
): SegmentOption<SchedulerStatusSegment>[] {
  const counts: Record<SchedulerStatusSegment, number> = { all: pool.length, active: 0, queued: 0, done: 0, failed: 0 };
  for (const run of pool) counts[schedulerStatusSegmentForRun(run)] += 1;
  return SCHEDULER_STATUS_SEGMENTS.map((segment) => ({
    value: segment,
    label: SCHEDULER_STATUS_SEGMENT_LABELS[segment],
    count: counts[segment],
  }));
}

export interface SchedulerRunRow extends RunContextRow {
  /** Right-hand time: HH:MM today, otherwise a short date. */
  time: string;
  timeTitle: string;
  /** Second sub-line: start time for running runs, placement otherwise. */
  detail: string | null;
}

export interface SchedulerRunGroup {
  id: RunContextGroupId;
  title: string;
  rows: SchedulerRunRow[];
}

const SHORT_MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

export function formatSchedulerRowTime(ms: number, startOfTodayMs: number): string {
  if (ms >= startOfTodayMs) return formatClockTime(ms);
  const date = new Date(ms);
  return `${SHORT_MONTHS[date.getMonth()]} ${date.getDate()}`;
}

/** Local wall-clock time as HH:MM:SS, for timeline rows. */
export function formatSchedulerClockTime(ms: number): string {
  const date = new Date(ms);
  return [date.getHours(), date.getMinutes(), date.getSeconds()]
    .map((part) => String(part).padStart(2, '0'))
    .join(':');
}

/** "llama.cpp on cuda:0", "llama.cpp, device not assigned yet", or null when nothing is placed. */
export function schedulerRunPlacementSummary(
  run: Pick<RunListProjectionRecord, 'selected_runtime_id' | 'selected_device_id'>,
): string | null {
  const runtime = run.selected_runtime_id?.trim();
  const device = run.selected_device_id?.trim();
  if (runtime && device) return `${runtime} on ${device}`;
  if (runtime) return `${runtime}, device not assigned yet`;
  if (device) return `on ${device}`;
  return null;
}

export function schedulerRunDetailLine(run: RunListProjectionRecord): string | null {
  if (run.status === 'running') {
    // The picker's context for running runs is already the placement.
    return typeof run.started_at_ms === 'number' ? `Started ${formatClockTime(run.started_at_ms)}` : null;
  }
  return schedulerRunPlacementSummary(run);
}

/**
 * Group already filtered and sorted runs the way the header picker does
 * (Running, Queued, Finished today, Finished earlier). Inside each group the
 * order of `sortedRuns` (the Sort filter) is kept.
 */
export function groupSchedulerRuns(
  sortedRuns: readonly RunListProjectionRecord[],
  options: { selectedRunId: string | null; nowMs: number },
): SchedulerRunGroup[] {
  const order = new Map(sortedRuns.map((run, index) => [run.workflow_run_id, index]));
  const position = (row: RunContextRow) => order.get(row.runId) ?? Number.MAX_SAFE_INTEGER;
  const startOfToday = startOfLocalDayMs(options.nowMs);
  return groupRunsForContextPicker(sortedRuns, {
    query: '',
    selectedRunId: options.selectedRunId,
    nowMs: options.nowMs,
  }).map((group) => ({
    id: group.id,
    title: group.title,
    rows: [...group.rows]
      .sort((left, right) => position(left) - position(right))
      .map((row) => ({
        ...row,
        time: formatSchedulerRowTime(row.record.last_updated_at_ms, startOfToday),
        timeTitle: `Updated ${formatSchedulerTimestamp(row.record.last_updated_at_ms)}`,
        detail: schedulerRunDetailLine(row.record),
      })),
  }));
}

function formatWorkMinutes(ms: number): string {
  if (ms < 60_000) return 'under a minute';
  return `about ${Math.round(ms / 60_000)} min`;
}

function plural(count: number, one: string, many: string): string {
  return `${count} ${count === 1 ? one : many}`;
}

/**
 * The dimension line: "N runs waiting, about M min of queued work on D
 * devices". Minutes come from the scheduler estimate facts carried on each
 * run-list record (`estimated_duration_ms`); without any estimate the line is
 * the count alone.
 */
export function buildSchedulerQueueSummary(
  runs: readonly RunListProjectionRecord[],
  list: { loaded: boolean; loading: boolean; unreachable: boolean },
): string {
  if (!list.loaded) {
    if (list.unreachable) return 'Run list unavailable, the engine did not answer';
    return list.loading ? 'Loading runs' : 'Run list unavailable';
  }
  const waiting = runs.filter((run) => schedulerStatusSegmentForRun(run) === 'queued');
  if (waiting.length === 0) {
    const running = runs.filter((run) => run.status === 'running').length;
    return running > 0 ? `No runs waiting, ${running} running` : 'No runs waiting';
  }
  const count = plural(waiting.length, 'run waiting', 'runs waiting');
  const estimated = waiting.filter(
    (run) => typeof run.estimated_duration_ms === 'number' && run.estimated_duration_ms >= 0,
  );
  if (estimated.length === 0) return count;
  const totalMs = estimated.reduce((sum, run) => sum + (run.estimated_duration_ms ?? 0), 0);
  const coverage = estimated.length < waiting.length ? ` (estimated for ${estimated.length} of them)` : '';
  const devices = new Set(
    runs
      .filter((run) => run.status === 'running' || schedulerStatusSegmentForRun(run) === 'queued')
      .map((run) => run.selected_device_id?.trim())
      .filter((device): device is string => Boolean(device)),
  );
  const onDevices = devices.size > 0 ? ` on ${plural(devices.size, 'device', 'devices')}` : '';
  return `${count}, ${formatWorkMinutes(totalMs)} of queued work${onDevices}${coverage}`;
}

// --- Filters disclosure -----------------------------------------------------

export interface SchedulerFilterOption {
  value: string;
  label: string;
}

export interface SchedulerFilterField {
  key: SchedulerSecondaryFilterKey;
  /** DOM id of the select, stable across renders. */
  id: string;
  label: string;
  value: string;
  options: SchedulerFilterOption[];
}

const SCHEDULER_FILTER_LABELS: Record<SchedulerSecondaryFilterKey, string> = {
  status: 'Status',
  sort: 'Sort',
  schedulerPolicy: 'Policy',
  retentionPolicy: 'Retention',
  client: 'Client',
  clientSession: 'Session',
  bucket: 'Bucket',
  selectedRuntime: 'Runtime',
  selectedRuntimeVariant: 'Runtime variant',
  selectedBackend: 'Backend',
  selectedDeviceClass: 'Device class',
  selectedDevice: 'Device',
  selectedNetworkNode: 'Network node',
  acceptedDate: 'Accepted',
};

type SchedulerOptionFilterKey = Exclude<SchedulerSecondaryFilterKey, 'status' | 'sort'>;

const SCHEDULER_FILTER_OPTION_SOURCES: Record<SchedulerOptionFilterKey, (runs: RunListProjectionRecord[]) => string[]> = {
  schedulerPolicy: schedulerPolicyFilterOptions,
  retentionPolicy: schedulerRetentionFilterOptions,
  client: schedulerClientFilterOptions,
  clientSession: schedulerClientSessionFilterOptions,
  bucket: schedulerBucketFilterOptions,
  selectedRuntime: schedulerSelectedRuntimeFilterOptions,
  selectedRuntimeVariant: schedulerSelectedRuntimeVariantFilterOptions,
  selectedBackend: schedulerSelectedBackendFilterOptions,
  selectedDeviceClass: schedulerSelectedDeviceClassFilterOptions,
  selectedDevice: schedulerSelectedDeviceFilterOptions,
  selectedNetworkNode: schedulerSelectedNetworkNodeFilterOptions,
  acceptedDate: schedulerAcceptedDateFilterOptions,
};

function schedulerFilterValueLabel(key: SchedulerSecondaryFilterKey, value: string): string {
  if (value === 'all') return 'All';
  if (key === 'status') return formatSchedulerStatusLabel(value as RunListProjectionRecord['status']);
  if (key === 'sort') return SCHEDULER_SORT_OPTIONS.find((option) => option.value === value)?.label ?? value;
  return value;
}

/**
 * The fourteen selects of the Filters disclosure, in display order. Options
 * come from the loaded runs; an applied value that no loaded run carries any
 * more stays listed so the select still shows it.
 */
export function buildSchedulerFilterFields(
  runs: RunListProjectionRecord[],
  filters: SchedulerRunFilters,
): SchedulerFilterField[] {
  return SCHEDULER_SECONDARY_FILTER_KEYS.map((key) => {
    const value = filters[key];
    let values: string[];
    if (key === 'status') values = [...SCHEDULER_STATUS_FILTERS];
    else if (key === 'sort') values = SCHEDULER_SORT_OPTIONS.map((option) => option.value);
    else {
      values = ['all', ...SCHEDULER_FILTER_OPTION_SOURCES[key](runs)];
      if (!values.includes(value)) values.push(value);
    }
    return {
      key,
      id: `scheduler-filter-${key}`,
      label: SCHEDULER_FILTER_LABELS[key],
      value,
      options: values.map((option) => ({ value: option, label: schedulerFilterValueLabel(key, option) })),
    };
  });
}

export interface SchedulerAppliedFilterChip {
  key: SchedulerSecondaryFilterKey;
  label: string;
}

/** One chip per disclosure filter that differs from its default. */
export function schedulerAppliedFilterChips(filters: SchedulerRunFilters): SchedulerAppliedFilterChip[] {
  return SCHEDULER_SECONDARY_FILTER_KEYS.filter((key) => filters[key] !== DEFAULT_SCHEDULER_RUN_FILTERS[key]).map(
    (key) => ({ key, label: `${SCHEDULER_FILTER_LABELS[key]}: ${schedulerFilterValueLabel(key, filters[key])}` }),
  );
}

// --- Detail panel -------------------------------------------------------------

/** First line of the Progress cell, from run-list facts only. */
export function schedulerRunProgressText(run: RunListProjectionRecord): string {
  const reason = run.scheduler_reason?.trim();
  const duration = formatRunDuration(run.duration_ms);
  switch (run.status) {
    case 'running':
      return typeof run.started_at_ms === 'number'
        ? `Running since ${formatSchedulerClockTime(run.started_at_ms)}`
        : 'Running';
    case 'accepted':
    case 'future':
    case 'scheduled':
    case 'queued': {
      const head = typeof run.scheduler_queue_position === 'number'
        ? `Queue position ${run.scheduler_queue_position}`
        : formatSchedulerStatusLabel(run.status);
      return reason ? `${head}, ${reason}` : head;
    }
    case 'delayed':
      return reason ? `Delayed: ${reason}` : 'Delayed by the scheduler';
    case 'completed':
      return duration ? `Finished in ${duration}` : 'Finished';
    case 'failed': {
      const error = run.latest_error_message?.trim() || run.latest_error_code?.trim();
      return error ? `Failed: ${error}` : 'Failed';
    }
    case 'cancelled':
      return duration ? `Cancelled after ${duration}` : 'Cancelled';
  }
}

/** Second line of the Progress cell, from the selected run's scheduler estimate query. */
export function schedulerEstimateSentence(
  estimate: WorkflowSchedulerEstimateRecord | null,
  state: { loading: boolean; error: string | null },
): string {
  if (state.error) return `Estimate unavailable: ${state.error}`;
  if (!estimate) return state.loading ? 'Loading the scheduler estimate' : 'No scheduler estimate for this run';
  const wait = formatRunDuration(estimate.estimated_queue_wait_ms);
  const run = formatRunDuration(estimate.estimated_duration_ms);
  const parts = [wait ? `about ${wait} queue wait` : null, run ? `about ${run} to run` : null].filter(
    (part): part is string => part !== null,
  );
  let sentence = parts.length > 0 ? parts.join(', ') : 'no duration estimate';
  if (estimate.estimate_confidence) sentence += `, ${estimate.estimate_confidence} confidence`;
  sentence = `${sentence.charAt(0).toUpperCase()}${sentence.slice(1)}.`;
  if (estimate.model_cache_state && estimate.model_cache_state !== 'unknown') {
    sentence += ` ${formatSchedulerModelCacheState(estimate.model_cache_state)}.`;
  }
  return sentence;
}

export function schedulerRunPlacementText(run: RunListProjectionRecord): string {
  const runtime = formatSchedulerRuntimePlacementLabel(run);
  const device = formatSchedulerDevicePlacementLabel(run);
  if (runtime === 'Unassigned' && device === 'Unassigned') return 'Not assigned to a device yet';
  if (device === 'Unassigned') return `${runtime}, device not assigned yet`;
  if (runtime === 'Unassigned') return `On ${device}`;
  return `${runtime} on ${device}`;
}

export function schedulerRunPolicyText(run: RunListProjectionRecord): string {
  const parts = [
    run.scheduler_policy_id ? `Policy ${run.scheduler_policy_id}` : null,
    run.bucket_id ? `bucket ${run.bucket_id}` : null,
    run.selected_backend_key ? `backend ${run.selected_backend_key}` : null,
    run.selected_network_node_id ? `network node ${run.selected_network_node_id}` : null,
  ].filter((part): part is string => part !== null);
  if (parts.length === 0) return 'No policy, bucket or backend recorded';
  const text = parts.join(', ');
  return `${text.charAt(0).toUpperCase()}${text.slice(1)}`;
}

export function schedulerRetentionText(
  summary: IoArtifactRetentionSummaryRecord[],
  state: { loading: boolean; error: string | null },
): string {
  if (state.error) return `Artifact summary unavailable: ${state.error}`;
  const rows = buildSchedulerRetentionSummaryRows(summary);
  if (rows.length === 0) return state.loading ? 'Loading the artifact summary' : 'No retained artifacts projected';
  return rows.map((row) => `${row.label}: ${row.count}`).join(', ');
}

export function schedulerRetentionPolicyText(run: Pick<RunListProjectionRecord, 'retention_policy_id'>): string {
  return run.retention_policy_id ? `Retention policy ${run.retention_policy_id}` : 'No retention policy recorded';
}

/** Chips after the status chip: placement, priority, queue position. */
export function schedulerRunDetailChips(run: RunListProjectionRecord): string[] {
  const chips: string[] = [];
  const placement = schedulerRunPlacementSummary(run);
  if (placement) chips.push(placement);
  if (typeof run.scheduler_priority === 'number' && run.scheduler_priority !== 0) {
    chips.push(`priority ${run.scheduler_priority}`);
  }
  if (typeof run.scheduler_queue_position === 'number') chips.push(`#${run.scheduler_queue_position} in queue`);
  return chips;
}

export function schedulerTimelineAttemptText(
  event: Parameters<typeof buildSchedulerTimelineAttemptRows>[0],
): string | null {
  const rows = buildSchedulerTimelineAttemptRows(event);
  return rows.length > 0 ? rows.map((row) => `${row.label} ${row.value}`).join(' · ') : null;
}

function lowerFirst(text: string): string {
  return `${text.charAt(0).toLowerCase()}${text.slice(1)}`;
}

/** One muted line naming how fresh each detail projection is (ADR-014 guardrail). */
export function schedulerDetailFreshnessText(states: {
  timeline: ProjectionStateRecord | null;
  estimate: ProjectionStateRecord | null;
  retention: ProjectionStateRecord | null;
}): string {
  return [
    `Timeline ${lowerFirst(formatProjectionFreshnessState(states.timeline))}`,
    `estimate ${lowerFirst(formatProjectionFreshnessState(states.estimate))}`,
    `artifacts ${lowerFirst(formatProjectionFreshnessState(states.retention))}`,
  ].join('; ');
}

// --- Footer -----------------------------------------------------------------

export type SchedulerFooterState =
  | { kind: 'none' }
  | { kind: 'missing' }
  | { kind: 'finished'; workflowId: string }
  | { kind: 'live'; run: RunListProjectionRecord };

/**
 * What the run panel footer offers. `missing` is an active run that is not in
 * the loaded run list (selected elsewhere, or older than the list window).
 */
export function schedulerFooterState(
  activeRunId: string | null,
  run: RunListProjectionRecord | null,
): SchedulerFooterState {
  if (!activeRunId) return { kind: 'none' };
  if (!run) return { kind: 'missing' };
  const segment = schedulerStatusSegmentForRun(run);
  if (segment === 'done' || segment === 'failed') return { kind: 'finished', workflowId: run.workflow_id };
  return { kind: 'live', run };
}

/** Why the session queue actions (Cancel, Front, Priority) are unavailable, or null when they apply. */
export function schedulerQueueControlsUnavailableReason(run: RunListProjectionRecord | null): string | null {
  if (schedulerRunSupportsQueueControls(run)) return null;
  if (!run || (run.status !== 'queued' && run.status !== 'delayed')) {
    return 'The scheduler accepts cancel, front and priority changes only while a run is queued or delayed.';
  }
  return 'This run has no workflow execution session. The GUI admin actions under More actions act on it by run id.';
}

export function schedulerAdminControlsUnavailableReason(run: RunListProjectionRecord | null): string | null {
  if (schedulerRunSupportsAdminQueueControls(run)) return null;
  return 'GUI admin actions apply only while a run is queued or delayed.';
}

export function schedulerResumeUnavailableReason(run: RunListProjectionRecord | null): string | null {
  if (schedulerRunSupportsDependencyReadinessResume(run)) return null;
  return 'Resume applies only to a session run waiting on runtime dependency readiness.';
}

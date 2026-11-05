/**
 * Pure presentation logic for the header run-context picker.
 *
 * Input is the scheduler run-list projection (`RunListProjectionRecord`, from
 * `workflowService.queryRunList`); output is what the chip and popover render.
 * Nothing here reads the diagnostic ledger, the DOM, timers or the clipboard.
 */
import type {
  RunListProjectionRecord,
  RunListProjectionStatus,
} from '../../../services/diagnostics/types';
import type { ActiveWorkflowRunContext } from '../../../stores/workbenchStore';
import { glyphStateForStatus, type GlyphState } from '../ui/statusGlyph.ts';

/** Rows the picker asks the run-list projection for. */
export const RUN_CONTEXT_RUN_LIMIT = 250;

export type RunContextGroupId = 'running' | 'queued' | 'finished_today' | 'finished_earlier';

export interface RunContextRow {
  runId: string;
  workflowId: string;
  versionLabel: string | null;
  statusLabel: string;
  statusIsFailure: boolean;
  /** Text after the status label, starting with ", " when present. */
  contextSuffix: string;
  glyph: GlyphState;
  selected: boolean;
  record: RunListProjectionRecord;
}

export interface RunContextGroup {
  id: RunContextGroupId;
  title: string;
  rows: RunContextRow[];
}

export type RunContextInputResolution =
  | { kind: 'empty' }
  | { kind: 'select'; run: RunListProjectionRecord }
  | { kind: 'unknown_run_id'; runId: string }
  | { kind: 'search'; query: string };

export type RunContextChip =
  | { kind: 'none' }
  | {
      kind: 'run';
      runId: string;
      workflowLabel: string;
      versionLabel: string | null;
      statusLabel: string;
      glyph: GlyphState;
    };

export type EngineAvailability =
  | { kind: 'unknown' }
  | { kind: 'online' }
  | { kind: 'unreachable'; sinceMs: number; hasProjection: boolean };

const GROUP_TITLES: Record<RunContextGroupId, string> = {
  running: 'Running',
  queued: 'Queued',
  finished_today: 'Finished today',
  finished_earlier: 'Finished earlier',
};

const GROUP_ORDER: RunContextGroupId[] = ['running', 'queued', 'finished_today', 'finished_earlier'];

const STATUS_LABELS: Record<RunListProjectionStatus, string> = {
  accepted: 'Accepted',
  future: 'Future',
  scheduled: 'Scheduled',
  queued: 'Queued',
  delayed: 'Delayed',
  running: 'Running',
  completed: 'Completed',
  failed: 'Failed',
  cancelled: 'Cancelled',
};

const RUN_ID_PATTERN = /^run_\S+$/i;

export function runContextStatusLabel(status: string | null | undefined): string {
  if (status && status in STATUS_LABELS) {
    return STATUS_LABELS[status as RunListProjectionStatus];
  }
  return 'Status unknown';
}

/**
 * Picker group for a run. The run-list projection has no "waiting for memory"
 * state today (see issues.md WR-004), so the spec's Waiting group is not
 * produced; delayed runs sit with the queued ones.
 */
export function runContextGroupForRun(
  run: Pick<RunListProjectionRecord, 'status' | 'completed_at_ms' | 'last_updated_at_ms'>,
  startOfTodayMs: number,
): RunContextGroupId {
  switch (run.status) {
    case 'running':
      return 'running';
    case 'accepted':
    case 'future':
    case 'scheduled':
    case 'queued':
    case 'delayed':
      return 'queued';
    case 'completed':
    case 'failed':
    case 'cancelled': {
      const finishedAt = run.completed_at_ms ?? run.last_updated_at_ms;
      return finishedAt >= startOfTodayMs ? 'finished_today' : 'finished_earlier';
    }
  }
}

/** Local midnight for the day containing `nowMs`. */
export function startOfLocalDayMs(nowMs: number): number {
  const day = new Date(nowMs);
  day.setHours(0, 0, 0, 0);
  return day.getTime();
}

export function looksLikeRunId(text: string): boolean {
  return RUN_ID_PATTERN.test(text.trim());
}

/**
 * Compact run id for narrow places: `run_` plus the last eight characters.
 * The full id stays available in the row title and the copied text.
 */
export function shortRunId(runId: string): string {
  if (runId.length <= 16) return runId;
  const prefix = runId.startsWith('run_') ? 'run_' : '';
  return `${prefix}…${runId.slice(-8)}`;
}

export function formatRunVersion(version: string | null | undefined): string | null {
  const trimmed = version?.trim();
  if (!trimmed) return null;
  return trimmed.startsWith('v') ? trimmed : `v${trimmed}`;
}

export function formatRunDuration(durationMs: number | null | undefined): string | null {
  if (durationMs === null || durationMs === undefined || !Number.isFinite(durationMs) || durationMs < 0) {
    return null;
  }
  const totalSeconds = Math.round(durationMs / 1000);
  if (totalSeconds < 60) return `${totalSeconds} s`;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes < 60) return `${minutes}m ${String(seconds).padStart(2, '0')}s`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ${String(minutes % 60).padStart(2, '0')}m`;
}

/** Local wall-clock time as HH:MM. */
export function formatClockTime(ms: number): string {
  const date = new Date(ms);
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;
}

function placementText(run: RunListProjectionRecord): string | null {
  const runtime = run.selected_runtime_id?.trim();
  const device = run.selected_device_id?.trim();
  if (runtime && device) return `${runtime} on ${device}`;
  if (runtime) return `${runtime}, device not assigned yet`;
  if (device) return `on ${device}`;
  return null;
}

function contextFor(run: RunListProjectionRecord): string | null {
  switch (run.status) {
    case 'running':
      return placementText(run);
    case 'accepted':
    case 'future':
    case 'scheduled':
    case 'queued':
      return typeof run.scheduler_queue_position === 'number'
        ? `#${run.scheduler_queue_position} in queue`
        : null;
    case 'delayed':
      return run.scheduler_reason?.trim() || null;
    case 'failed':
      return run.latest_error_message?.trim() || run.latest_error_code?.trim() || formatRunDuration(run.duration_ms);
    case 'completed':
    case 'cancelled':
      return formatRunDuration(run.duration_ms);
  }
}

export function decorateRunContextRow(
  run: RunListProjectionRecord,
  selectedRunId: string | null,
): RunContextRow {
  const context = contextFor(run);
  return {
    runId: run.workflow_run_id,
    workflowId: run.workflow_id,
    versionLabel: formatRunVersion(run.workflow_semantic_version),
    statusLabel: runContextStatusLabel(run.status),
    statusIsFailure: run.status === 'failed',
    contextSuffix: context ? `, ${context}` : '',
    glyph: glyphStateForStatus(run.status),
    selected: run.workflow_run_id === selectedRunId,
    record: run,
  };
}

export function runMatchesContextQuery(run: RunListProjectionRecord, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  const version = formatRunVersion(run.workflow_semantic_version);
  return [run.workflow_id, run.workflow_run_id, version, run.workflow_version_id]
    .some((value) => typeof value === 'string' && value.toLowerCase().includes(needle));
}

function compareRows(groupId: RunContextGroupId, left: RunListProjectionRecord, right: RunListProjectionRecord): number {
  if (groupId === 'queued') {
    const leftPosition = left.scheduler_queue_position ?? Number.MAX_SAFE_INTEGER;
    const rightPosition = right.scheduler_queue_position ?? Number.MAX_SAFE_INTEGER;
    if (leftPosition !== rightPosition) return leftPosition - rightPosition;
  }
  if (groupId === 'finished_today' || groupId === 'finished_earlier') {
    const leftDone = left.completed_at_ms ?? left.last_updated_at_ms;
    const rightDone = right.completed_at_ms ?? right.last_updated_at_ms;
    if (leftDone !== rightDone) return rightDone - leftDone;
  }
  if (left.last_updated_at_ms !== right.last_updated_at_ms) {
    return right.last_updated_at_ms - left.last_updated_at_ms;
  }
  return left.workflow_run_id.localeCompare(right.workflow_run_id);
}

/**
 * Filter and group runs for the popover. Empty groups are omitted; group order
 * is Running, Queued, Finished today, Finished earlier.
 */
export function groupRunsForContextPicker(
  runs: readonly RunListProjectionRecord[],
  options: { query: string; selectedRunId: string | null; nowMs: number },
): RunContextGroup[] {
  const startOfToday = startOfLocalDayMs(options.nowMs);
  const buckets = new Map<RunContextGroupId, RunListProjectionRecord[]>();
  for (const run of runs) {
    if (!runMatchesContextQuery(run, options.query)) continue;
    const groupId = runContextGroupForRun(run, startOfToday);
    const bucket = buckets.get(groupId) ?? [];
    bucket.push(run);
    buckets.set(groupId, bucket);
  }
  return GROUP_ORDER.flatMap((groupId) => {
    const bucket = buckets.get(groupId);
    if (!bucket || bucket.length === 0) return [];
    return [{
      id: groupId,
      title: GROUP_TITLES[groupId],
      rows: [...bucket]
        .sort((left, right) => compareRows(groupId, left, right))
        .map((run) => decorateRunContextRow(run, options.selectedRunId)),
    }];
  });
}

/**
 * Resolve typed or pasted text. An exact run id selects that run; text shaped
 * like a run id that is not in the list is reported as unknown; anything else
 * is a search query.
 */
export function resolveRunContextInput(
  text: string,
  runs: readonly RunListProjectionRecord[],
): RunContextInputResolution {
  const trimmed = text.trim();
  if (!trimmed) return { kind: 'empty' };
  const exact = runs.find((run) => run.workflow_run_id === trimmed);
  if (exact) return { kind: 'select', run: exact };
  if (looksLikeRunId(trimmed)) return { kind: 'unknown_run_id', runId: trimmed };
  return { kind: 'search', query: text };
}

export function runContextEmptyText(options: {
  query: string;
  runCount: number;
  availability: EngineAvailability;
  loading: boolean;
}): string {
  const query = options.query.trim();
  if (query && looksLikeRunId(query)) {
    return `No run with id ${query} in recent history.`;
  }
  if (query) return 'No runs match.';
  if (options.loading) return 'Loading runs…';
  if (options.availability.kind === 'unreachable') {
    return 'The run list is unavailable because the engine did not answer.';
  }
  return 'No runs yet. Runs appear here once a workflow is submitted.';
}

export function toActiveRunContext(
  run: RunListProjectionRecord,
): Omit<ActiveWorkflowRunContext, 'selected_at_ms'> {
  return {
    workflow_run_id: run.workflow_run_id,
    workflow_id: run.workflow_id,
    workflow_version_id: run.workflow_version_id ?? null,
    workflow_semantic_version: run.workflow_semantic_version ?? null,
    status: run.status,
  };
}

/**
 * Chip content for the active run. The run-list record wins over the status
 * captured at selection time because it is the fresher projection.
 */
export function runContextChip(
  active: Pick<ActiveWorkflowRunContext, 'workflow_run_id' | 'workflow_id' | 'workflow_semantic_version' | 'status'> | null,
  runs: readonly RunListProjectionRecord[],
): RunContextChip {
  if (!active) return { kind: 'none' };
  const record = runs.find((run) => run.workflow_run_id === active.workflow_run_id);
  const status = record?.status ?? active.status ?? null;
  return {
    kind: 'run',
    runId: active.workflow_run_id,
    workflowLabel: record?.workflow_id ?? active.workflow_id ?? active.workflow_run_id,
    versionLabel: formatRunVersion(record?.workflow_semantic_version ?? active.workflow_semantic_version),
    statusLabel: runContextStatusLabel(status),
    glyph: glyphStateForStatus(status),
  };
}

export interface EngineLineParts {
  /** Always visible, for example "Engine unreachable since 20:03". */
  headline: string;
  /** Trailing clause that may be truncated in a narrow header. */
  detail: string | null;
}

export function engineLineParts(availability: EngineAvailability): EngineLineParts {
  switch (availability.kind) {
    case 'unknown':
      return { headline: 'Local engine', detail: 'connecting' };
    case 'online':
      return { headline: 'Local engine', detail: null };
    case 'unreachable':
      return {
        headline: `Engine unreachable since ${formatClockTime(availability.sinceMs)}`,
        detail: availability.hasProjection ? 'showing last projection' : 'nothing loaded yet',
      };
  }
}

export function engineLineText(availability: EngineAvailability): string {
  const parts = engineLineParts(availability);
  return parts.detail ? `${parts.headline}, ${parts.detail}` : parts.headline;
}

/**
 * Pure projection for the Sessions tab.
 *
 * Sessions are a view over existing authority only (plan M8, spec "Sessions"):
 * - saved sessions are the saved workflows (`list_workflows`, `WorkflowMetadata`)
 *   and orchestrations (`list_orchestrations`, `OrchestrationGraphMetadata`);
 * - their runs are the rows of the scheduler run-list projection whose
 *   `workflow_id` equals the saved workflow's id (the file stem);
 * - live client sessions are not listed by any frontend service, so the Active
 *   group carries an explanation instead of rows (issue in the M8 report).
 *
 * Nothing here calls services, reads stores, the DOM or timers; the page passes
 * `nowMs` in so wording is deterministic. The one stateful helper,
 * `createLatestOnlyGate`, is the page's invocation authority, kept here so its
 * deterministic behaviour is tested.
 */
import type {
  RunListProjectionRecord,
  RunListProjectionStatus,
} from '../../services/diagnostics/types';
import type { WorkflowMetadata } from '../../services/workflow/types';
import type { OrchestrationGraphMetadata } from '../../stores/orchestrationStore';
import type { KeyValueRow, SegmentOption } from './ui/types';
import type { GlyphState } from './ui/statusGlyph.ts';
import { decorateRunContextRow, formatRunVersion, shortRunId, type RunContextRow } from './shell/runContextPicker.ts';

/** Rows the detail panel lists under "Recent runs". */
export const SESSION_RECENT_RUN_LIMIT = 5;

export type SessionKind = 'workflow' | 'orchestration';
export type SessionsFilter = 'all' | 'active' | 'saved';
export type RunOutcomeKind = 'running' | 'queued' | 'succeeded' | 'failed' | 'cancelled';

/** One saved source the page loaded, or why it could not be loaded. */
export type SessionSourceState<T> =
  | { status: 'loading' }
  | { status: 'ready'; items: T[] }
  | { status: 'failed'; message: string };

export interface OutcomeCounts {
  running: number;
  queued: number;
  succeeded: number;
  failed: number;
  cancelled: number;
  total: number;
}

export interface OutcomeStripSegment {
  kind: RunOutcomeKind;
  count: number;
}

export interface SessionEntry {
  /** Unique across kinds, for keyed lists and selection. */
  key: string;
  kind: SessionKind;
  /** Workflow file stem or orchestration id. */
  id: string;
  name: string;
  description: string | null;
  /** Short tag after the name, in place of the board's client tag. */
  tag: string;
  glyph: GlyphState;
  /** Sentence-case state for the detail chip. */
  stateLabel: string;
  sub: string;
  strip: OutcomeStripSegment[];
  /** The strip's content in words, for assistive technology and titles. */
  stripLabel: string;
  runCountLabel: string;
  whenLabel: string;
  createdMs: number | null;
  modifiedMs: number | null;
  lastActivityMs: number | null;
  lastActivityLabel: string;
  counts: OutcomeCounts;
  /** This session's runs from the run list, newest update first. */
  runs: RunListProjectionRecord[];
  nodeCount: number | null;
}

export interface SessionGroup {
  id: 'active' | 'saved';
  label: string;
  /** Omitted for Active, whose sessions are not listed by this build. */
  count?: number;
  entries: SessionEntry[];
  /** Muted sentence shown in place of rows; null when rows explain themselves. */
  note: string | null;
}

export interface SessionOpenItem {
  name: string;
  meta: string;
  glyph: GlyphState;
  sub: string;
  actionLabel: string;
  /** Null when the action works; otherwise why it is unavailable (button title). */
  disabledReason: string | null;
}

export interface SessionAction {
  id: 'rename' | 'save_copy' | 'resume' | 'close' | 'delete';
  label: string;
  /** Null when the action is backed by a service. */
  disabledReason: string | null;
}

export interface SessionDetail {
  key: string;
  kind: SessionKind;
  title: string;
  id: string;
  glyph: GlyphState;
  stateLabel: string;
  chips: string[];
  activity: KeyValueRow[];
  runSummary: KeyValueRow[];
  recentRuns: RunContextRow[];
  /** Runs beyond the recent list, for a "and N more in Scheduler" line. */
  moreRunCount: number;
  open: SessionOpenItem[];
  resumeSentence: string;
  actions: SessionAction[];
}

export const ACTIVE_SESSIONS_UNAVAILABLE_NOTE =
  'Live client sessions are not listed by this engine build. Runs still show their client session in the Scheduler.';

export const NO_SESSION_SERVICE_REASON =
  'Not available: this engine build has no session service. Sessions here are your saved workflows.';

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

const OUTCOME_ORDER: RunOutcomeKind[] = ['running', 'queued', 'succeeded', 'failed', 'cancelled'];

const OUTCOME_WORDS: Record<RunOutcomeKind, string> = {
  running: 'running',
  queued: 'queued',
  succeeded: 'succeeded',
  failed: 'failed',
  cancelled: 'cancelled',
};

const RUN_ACTIVITY_WORDS: Record<RunListProjectionStatus, string> = {
  accepted: 'run accepted',
  future: 'run scheduled',
  scheduled: 'run scheduled',
  queued: 'run queued',
  delayed: 'run delayed',
  running: 'run in progress',
  completed: 'run completed',
  failed: 'run failed',
  cancelled: 'run cancelled',
};

// ---------------------------------------------------------------------------
// Invocation authority
// ---------------------------------------------------------------------------

/** How a finished async call relates to its owner when it completes. */
export type InvocationOutcome = 'current' | 'superseded' | 'closed';

/**
 * Latest-call-wins authority for one operation of one page instance. The page
 * opens it on mount and closes it on destroy; each call takes a token with
 * `begin()` and may apply its result only when `classify(token)` is `current`.
 * A newer `begin()` supersedes older tokens; `close()` supersedes all of them.
 */
export interface LatestOnlyGate {
  open(): void;
  close(): void;
  begin(): number;
  classify(token: number): InvocationOutcome;
}

export function createLatestOnlyGate(): LatestOnlyGate {
  let generation = 0;
  let isOpen = false;
  return {
    open() {
      isOpen = true;
    },
    close() {
      isOpen = false;
      generation += 1;
    },
    begin() {
      generation += 1;
      return generation;
    },
    classify(token) {
      if (!isOpen) return 'closed';
      return token === generation ? 'current' : 'superseded';
    },
  };
}

// ---------------------------------------------------------------------------
// Source loading outcomes
// ---------------------------------------------------------------------------

function errorText(error: unknown): string {
  if (error instanceof Error && error.message.trim().length > 0) return error.message.trim();
  if (typeof error === 'string' && error.trim().length > 0) return error.trim();
  return 'the engine gave no reason';
}

/** Classify one settled listing call into the source state the page renders. */
export function sessionSourceFromResult<T>(result: PromiseSettledResult<T[]>): SessionSourceState<T> {
  if (result.status === 'fulfilled') {
    return Array.isArray(result.value)
      ? { status: 'ready', items: result.value }
      : { status: 'failed', message: 'the engine answered with something other than a list' };
  }
  return { status: 'failed', message: errorText(result.reason) };
}

// ---------------------------------------------------------------------------
// Runs and outcomes
// ---------------------------------------------------------------------------

export function runOutcomeKind(status: RunListProjectionStatus): RunOutcomeKind {
  switch (status) {
    case 'running':
      return 'running';
    case 'accepted':
    case 'future':
    case 'scheduled':
    case 'queued':
    case 'delayed':
      return 'queued';
    case 'completed':
      return 'succeeded';
    case 'failed':
      return 'failed';
    case 'cancelled':
      return 'cancelled';
  }
}

export function isLiveOutcome(kind: RunOutcomeKind): boolean {
  return kind === 'running' || kind === 'queued';
}

export function countRunOutcomes(runs: readonly Pick<RunListProjectionRecord, 'status'>[]): OutcomeCounts {
  const counts: OutcomeCounts = { running: 0, queued: 0, succeeded: 0, failed: 0, cancelled: 0, total: 0 };
  for (const run of runs) {
    counts[runOutcomeKind(run.status)] += 1;
    counts.total += 1;
  }
  return counts;
}

/** Strip segments in board order (running, queued, succeeded, failed, cancelled), empty kinds omitted. */
export function outcomeStrip(counts: OutcomeCounts): OutcomeStripSegment[] {
  return OUTCOME_ORDER.filter((kind) => counts[kind] > 0).map((kind) => ({ kind, count: counts[kind] }));
}

/** "2 running, 1 queued, 2 succeeded, 1 failed", or "no runs". */
export function outcomeSummaryText(counts: OutcomeCounts): string {
  const parts = outcomeStrip(counts).map((segment) => `${segment.count} ${OUTCOME_WORDS[segment.kind]}`);
  return parts.length > 0 ? parts.join(', ') : 'no runs';
}

export function liveRunCount(counts: OutcomeCounts): number {
  return counts.running + counts.queued;
}

export function pluralize(count: number, singular: string, plural = `${singular}s`): string {
  return `${count} ${count === 1 ? singular : plural}`;
}

/** Runs of one workflow id, newest update first (ties by run id for stable order). */
export function runsForWorkflow(
  runs: readonly RunListProjectionRecord[],
  workflowId: string,
): RunListProjectionRecord[] {
  return runs
    .filter((run) => run.workflow_id === workflowId)
    .sort(
      (left, right) =>
        right.last_updated_at_ms - left.last_updated_at_ms ||
        left.workflow_run_id.localeCompare(right.workflow_run_id),
    );
}

// ---------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------

/** Workflow metadata dates are RFC 3339 strings; anything unparsable is null. */
export function parseMetadataDate(value: string | null | undefined): number | null {
  if (typeof value !== 'string' || value.trim().length === 0) return null;
  const ms = Date.parse(value);
  return Number.isFinite(ms) ? ms : null;
}

function pad2(value: number): string {
  return String(value).padStart(2, '0');
}

function sameLocalDay(left: Date, right: Date): boolean {
  return (
    left.getFullYear() === right.getFullYear() &&
    left.getMonth() === right.getMonth() &&
    left.getDate() === right.getDate()
  );
}

/** "Oct 31" in the current year, "Jan 31, 2025" otherwise (local time). */
export function formatSessionDay(ms: number, nowMs: number): string {
  const date = new Date(ms);
  const now = new Date(nowMs);
  const day = `${MONTHS[date.getMonth()]} ${date.getDate()}`;
  return date.getFullYear() === now.getFullYear() ? day : `${day}, ${date.getFullYear()}`;
}

/** "Today 19:28", "Oct 31, 14:02", or "Jan 31, 2025, 14:02" (local time). */
export function formatSessionDateTime(ms: number, nowMs: number): string {
  const date = new Date(ms);
  const time = `${pad2(date.getHours())}:${pad2(date.getMinutes())}`;
  if (sameLocalDay(date, new Date(nowMs))) return `Today ${time}`;
  return `${formatSessionDay(ms, nowMs)}, ${time}`;
}

/**
 * Right-hand "when" of a row: "now" while a run is live, the time today,
 * otherwise the day. "never" when nothing has happened that the sources record.
 */
export function formatLastActivityShort(ms: number | null, nowMs: number, live: boolean): string {
  if (live) return 'now';
  if (ms === null) return 'never';
  const date = new Date(ms);
  if (sameLocalDay(date, new Date(nowMs))) return `${pad2(date.getHours())}:${pad2(date.getMinutes())}`;
  return formatSessionDay(ms, nowMs);
}

export interface LastActivity {
  ms: number | null;
  /** What happened at `ms`, for example "run completed" or "workflow saved". */
  what: string | null;
}

/** The newer of the latest run update and the saved file's modified time. */
export function lastActivityOf(runs: readonly RunListProjectionRecord[], modifiedMs: number | null): LastActivity {
  const latestRun = runs.reduce<RunListProjectionRecord | null>(
    (latest, run) => (latest === null || run.last_updated_at_ms > latest.last_updated_at_ms ? run : latest),
    null,
  );
  if (latestRun && (modifiedMs === null || latestRun.last_updated_at_ms >= modifiedMs)) {
    return { ms: latestRun.last_updated_at_ms, what: RUN_ACTIVITY_WORDS[latestRun.status] };
  }
  if (modifiedMs !== null) return { ms: modifiedMs, what: 'workflow saved' };
  return { ms: null, what: null };
}

export function formatLastActivityLong(activity: LastActivity, nowMs: number): string {
  if (activity.ms === null) return 'Nothing recorded yet';
  const when = formatSessionDateTime(activity.ms, nowMs);
  return activity.what ? `${when}, ${activity.what}` : when;
}

// ---------------------------------------------------------------------------
// Entries
// ---------------------------------------------------------------------------

function trimmedOrNull(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

function kindLabel(kind: SessionKind): string {
  return kind === 'workflow' ? 'Saved workflow' : 'Saved orchestration';
}

function entryGlyph(counts: OutcomeCounts): GlyphState {
  // Spec: running if any live run; saved sessions are otherwise idle.
  return liveRunCount(counts) > 0 ? 'running' : 'idle';
}

function entrySub(
  counts: OutcomeCounts,
  description: string | null,
  fallback: string,
): string {
  const live = liveRunCount(counts);
  if (live > 0) return `${pluralize(live, 'live run')}, ${outcomeSummaryText(counts)}`;
  return description ?? fallback;
}

function finishEntry(
  base: Pick<SessionEntry, 'key' | 'kind' | 'id' | 'name' | 'description' | 'tag' | 'createdMs' | 'modifiedMs' | 'nodeCount'>,
  runs: RunListProjectionRecord[],
  fallbackSub: string,
  nowMs: number,
): SessionEntry {
  const counts = countRunOutcomes(runs);
  const live = liveRunCount(counts) > 0;
  const activity = lastActivityOf(runs, base.modifiedMs);
  return {
    ...base,
    glyph: entryGlyph(counts),
    stateLabel: live ? `${kindLabel(base.kind)}, ${pluralize(liveRunCount(counts), 'live run')}` : kindLabel(base.kind),
    sub: entrySub(counts, base.description, fallbackSub),
    strip: outcomeStrip(counts),
    stripLabel: `Runs in recent history: ${outcomeSummaryText(counts)}`,
    runCountLabel: pluralize(counts.total, 'run'),
    whenLabel: formatLastActivityShort(activity.ms, nowMs, live),
    lastActivityMs: activity.ms,
    lastActivityLabel: formatLastActivityLong(activity, nowMs),
    counts,
    runs,
  };
}

/** The id `list_workflows` gives a saved workflow (file stem), else its name. */
export function savedWorkflowId(workflow: Pick<WorkflowMetadata, 'id' | 'name'>): string {
  return trimmedOrNull(workflow.id) ?? workflow.name;
}

export function workflowSessionEntry(
  workflow: WorkflowMetadata,
  runs: readonly RunListProjectionRecord[],
  nowMs: number,
): SessionEntry {
  const id = savedWorkflowId(workflow);
  const createdMs = parseMetadataDate(workflow.created);
  const fallback = createdMs === null ? 'Saved workflow' : `Created ${formatSessionDay(createdMs, nowMs)}`;
  return finishEntry(
    {
      key: `workflow:${id}`,
      kind: 'workflow',
      id,
      name: trimmedOrNull(workflow.name) ?? id,
      description: trimmedOrNull(workflow.description),
      tag: 'workflow',
      createdMs,
      modifiedMs: parseMetadataDate(workflow.modified),
      nodeCount: null,
    },
    runsForWorkflow(runs, id),
    fallback,
    nowMs,
  );
}

export function orchestrationSessionEntry(
  orchestration: OrchestrationGraphMetadata,
  runs: readonly RunListProjectionRecord[],
  nowMs: number,
): SessionEntry {
  const nodeCount = Number.isFinite(orchestration.nodeCount) ? orchestration.nodeCount : null;
  return finishEntry(
    {
      key: `orchestration:${orchestration.id}`,
      kind: 'orchestration',
      id: orchestration.id,
      name: trimmedOrNull(orchestration.name) ?? orchestration.id,
      description: trimmedOrNull(orchestration.description),
      tag: 'orchestration',
      createdMs: null,
      modifiedMs: null,
      nodeCount,
    },
    runsForWorkflow(runs, orchestration.id),
    nodeCount === null ? 'Orchestration' : `Orchestration, ${pluralize(nodeCount, 'node')}`,
    nowMs,
  );
}

/**
 * Saved sessions from both sources, most recent activity first, never-active
 * ones last by name. A source that is not ready contributes no entries; the
 * page explains it in place.
 */
export function buildSavedSessions(
  workflows: SessionSourceState<WorkflowMetadata>,
  orchestrations: SessionSourceState<OrchestrationGraphMetadata>,
  runs: readonly RunListProjectionRecord[],
  nowMs: number,
): SessionEntry[] {
  const entries = [
    ...(workflows.status === 'ready' ? workflows.items.map((item) => workflowSessionEntry(item, runs, nowMs)) : []),
    ...(orchestrations.status === 'ready'
      ? orchestrations.items.map((item) => orchestrationSessionEntry(item, runs, nowMs))
      : []),
  ];
  return entries.sort((left, right) => {
    const leftLive = liveRunCount(left.counts) > 0 ? 1 : 0;
    const rightLive = liveRunCount(right.counts) > 0 ? 1 : 0;
    if (leftLive !== rightLive) return rightLive - leftLive;
    const leftMs = left.lastActivityMs ?? Number.NEGATIVE_INFINITY;
    const rightMs = right.lastActivityMs ?? Number.NEGATIVE_INFINITY;
    if (leftMs !== rightMs) return rightMs - leftMs;
    return left.name.localeCompare(right.name);
  });
}

// ---------------------------------------------------------------------------
// Filtering, grouping, summary
// ---------------------------------------------------------------------------

export function sessionMatchesQuery(entry: SessionEntry, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return [entry.name, entry.id, entry.description, entry.tag].some(
    (value) => typeof value === 'string' && value.toLowerCase().includes(needle),
  );
}

export function sessionFilterOptions(saved: readonly SessionEntry[]): SegmentOption<SessionsFilter>[] {
  return [
    { value: 'all', label: 'All', count: saved.length },
    // No count: live sessions are not listed by this build, and 0 would claim there are none.
    { value: 'active', label: 'Active' },
    { value: 'saved', label: 'Saved', count: saved.length },
  ];
}

/**
 * List groups for the left panel. Active is always present for the `all` and
 * `active` filters and carries the explanation; Saved carries the entries that
 * match the query, or a sentence when there are none.
 */
export function groupSessions(
  saved: readonly SessionEntry[],
  filter: SessionsFilter,
  query: string,
): SessionGroup[] {
  const groups: SessionGroup[] = [];
  if (filter !== 'saved') {
    groups.push({ id: 'active', label: 'Active', entries: [], note: ACTIVE_SESSIONS_UNAVAILABLE_NOTE });
  }
  if (filter !== 'active') {
    const entries = saved.filter((entry) => sessionMatchesQuery(entry, query));
    let note: string | null = null;
    if (entries.length === 0) {
      note = saved.length === 0
        ? 'No saved workflows or orchestrations yet. Save a workflow from the Graph tab and it appears here.'
        : `No saved sessions match "${query.trim()}".`;
    }
    groups.push({ id: 'saved', label: 'Saved', count: entries.length, entries, note });
  }
  return groups;
}

/** Runs in the run list whose workflow is not one of the saved sessions. */
export function unsavedWorkflowRunCount(
  runs: readonly RunListProjectionRecord[],
  saved: readonly SessionEntry[],
): number {
  const ids = new Set(saved.map((entry) => entry.id));
  return runs.filter((run) => !ids.has(run.workflow_id)).length;
}

export interface SessionsSummaryInput {
  workflows: SessionSourceState<WorkflowMetadata>;
  orchestrations: SessionSourceState<OrchestrationGraphMetadata>;
  saved: readonly SessionEntry[];
  runs: readonly RunListProjectionRecord[];
  runsLoaded: boolean;
}

/** The dimension line under the header. */
export function sessionsDimensionLine(input: SessionsSummaryInput): string {
  const parts: string[] = [];
  if (input.workflows.status === 'loading' || input.orchestrations.status === 'loading') {
    parts.push('Loading saved workflows');
  } else {
    const workflows = input.workflows.status === 'ready' ? pluralize(input.workflows.items.length, 'saved workflow') : 'saved workflows unavailable';
    const orchestrations = input.orchestrations.status === 'ready'
      ? pluralize(input.orchestrations.items.length, 'orchestration')
      : 'orchestrations unavailable';
    parts.push(`${workflows}, ${orchestrations}`);
  }
  if (!input.runsLoaded) {
    parts.push('run history not loaded');
  } else {
    const counts = countRunOutcomes(input.runs);
    let runs = `${pluralize(counts.total, 'run')} in recent history, ${liveRunCount(counts)} live`;
    const elsewhere = input.workflows.status === 'ready' ? unsavedWorkflowRunCount(input.runs, input.saved) : 0;
    if (elsewhere > 0) runs += `, ${elsewhere} from workflows not saved here`;
    parts.push(runs);
  }
  return parts.map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join('. ');
}

export interface SavedSourceFailure {
  title: string;
  /** The listing command(s) that failed, shown as the muted code line. */
  code: string;
  explanation: string;
}

/**
 * One in-place explanation for failed listings. Both sources usually fail for
 * the same reason (engine unreachable), so a shared message gives one card.
 */
function endSentence(text: string): string {
  return /[.!?]$/.test(text) ? text : `${text}.`;
}

export function savedSourceFailure(
  workflows: SessionSourceState<WorkflowMetadata>,
  orchestrations: SessionSourceState<OrchestrationGraphMetadata>,
): SavedSourceFailure | null {
  const workflowFailure = workflows.status === 'failed' ? workflows.message : null;
  const orchestrationFailure = orchestrations.status === 'failed' ? orchestrations.message : null;
  if (workflowFailure !== null && orchestrationFailure !== null) {
    const same = workflowFailure === orchestrationFailure;
    return {
      title: 'Saved workflows and orchestrations could not be listed',
      code: 'list_workflows, list_orchestrations',
      explanation: same
        ? `The engine did not return them: ${endSentence(workflowFailure)}`
        : `Workflows: ${endSentence(workflowFailure)} Orchestrations: ${endSentence(orchestrationFailure)}`,
    };
  }
  if (workflowFailure !== null) {
    return {
      title: 'Saved workflows could not be listed',
      code: 'list_workflows',
      explanation: `The engine did not return them: ${endSentence(workflowFailure)}`,
    };
  }
  if (orchestrationFailure !== null) {
    return {
      title: 'Orchestrations could not be listed',
      code: 'list_orchestrations',
      explanation: `Saved workflows are shown; orchestrations are missing: ${endSentence(orchestrationFailure)}`,
    };
  }
  return null;
}

export interface DetailEmptyInput {
  loading: boolean;
  failed: boolean;
  savedCount: number;
  filter: SessionsFilter;
  query: string;
}

/** What the detail panel says when no session is selected. */
export function sessionDetailEmptyText(input: DetailEmptyInput): string {
  if (input.loading) return 'Loading saved sessions…';
  if (input.filter === 'active') return 'No live sessions to show here. Choose Saved to see your saved workflows.';
  if (input.failed && input.savedCount === 0) return 'Nothing to show while saved workflows cannot be listed.';
  if (input.savedCount === 0) return 'Nothing to show yet. Saved workflows appear here with the runs they produced.';
  if (input.query.trim()) return 'No session matches the search.';
  return 'Select a session to see what it ran and open its graph.';
}

// ---------------------------------------------------------------------------
// Detail
// ---------------------------------------------------------------------------

function distinctValues(values: readonly (string | null | undefined)[]): string[] {
  const seen: string[] = [];
  for (const value of values) {
    const trimmed = value?.trim();
    if (trimmed && !seen.includes(trimmed)) seen.push(trimmed);
  }
  return seen;
}

function listWithMore(values: string[], max: number): string {
  if (values.length <= max) return values.join(', ');
  return `${values.slice(0, max).join(', ')} and ${values.length - max} more`;
}

function liveNowText(runs: readonly RunListProjectionRecord[]): string {
  const live = runs.filter((run) => isLiveOutcome(runOutcomeKind(run.status)));
  if (live.length === 0) return 'none';
  return listWithMore(
    live.map((run) => {
      const word = run.status === 'running' ? 'running' : run.status === 'delayed' ? 'delayed' : 'queued';
      const position = typeof run.scheduler_queue_position === 'number' && word !== 'running'
        ? ` #${run.scheduler_queue_position}`
        : '';
      return `${shortRunId(run.workflow_run_id)} ${word}${position}`;
    }),
    3,
  );
}

const NOT_RECORDED_ORCHESTRATION = 'Not recorded for orchestrations';

export function buildSessionDetail(
  entry: SessionEntry,
  selectedRunId: string | null,
  nowMs: number,
): SessionDetail {
  const isWorkflow = entry.kind === 'workflow';
  const versions = distinctValues(entry.runs.map((run) => formatRunVersion(run.workflow_semantic_version)));
  const clients = distinctValues(entry.runs.map((run) => run.client_id));
  const clientSessions = distinctValues(entry.runs.map((run) => run.client_session_id));
  const devices = distinctValues(entry.runs.map((run) => run.selected_device_id));

  // The kind is already in the state chip (`stateLabel`).
  const chips: string[] = [];
  if (versions.length > 0) chips.push(`latest ${versions[0]}`);
  if (devices.length > 0) chips.push(listWithMore(devices, 2));
  if (!isWorkflow && entry.nodeCount !== null) chips.push(pluralize(entry.nodeCount, 'node'));

  const dateOrMissing = (ms: number | null): string => {
    if (ms !== null) return formatSessionDateTime(ms, nowMs);
    return isWorkflow ? 'Not recorded' : NOT_RECORDED_ORCHESTRATION;
  };

  const activity: KeyValueRow[] = [
    { key: 'Created', value: dateOrMissing(entry.createdMs) },
    { key: 'Modified', value: dateOrMissing(entry.modifiedMs) },
    { key: 'Last activity', value: entry.lastActivityLabel },
    {
      key: 'Clients',
      value: clients.length > 0
        ? `${listWithMore(clients, 2)}${clientSessions.length > 0 ? `, ${pluralize(clientSessions.length, 'client session')}` : ''}`
        : 'None recorded on its runs',
      mono: clients.length > 0,
    },
  ];

  const runSummary: KeyValueRow[] = [
    {
      key: 'Total',
      value: entry.counts.total > 0
        ? `${entry.counts.total} in recent history: ${outcomeSummaryText(entry.counts)}`
        : 'None in recent history',
    },
    { key: 'Live now', value: liveNowText(entry.runs) },
    { key: 'Versions run', value: versions.length > 0 ? listWithMore(versions, 4) : 'none' },
  ];

  const recentRuns = entry.runs
    .slice(0, SESSION_RECENT_RUN_LIMIT)
    .map((run) => decorateRunContextRow(run, selectedRunId));

  const open: SessionOpenItem[] = [
    isWorkflow
      ? {
          name: entry.name,
          meta: versions[0] ?? 'saved',
          glyph: entry.glyph,
          sub: `Graph, .pantograph/workflows/${entry.id}.json`,
          actionLabel: 'Open graph',
          disabledReason: null,
        }
      : {
          name: entry.name,
          meta: 'orchestration',
          glyph: entry.glyph,
          sub: entry.description ?? 'Orchestration graph',
          actionLabel: 'Open graph',
          disabledReason: 'Not available: the Graph tab does not open orchestrations in this build.',
        },
  ];

  const actions: SessionAction[] = [
    { id: 'resume', label: 'Resume', disabledReason: NO_SESSION_SERVICE_REASON },
    { id: 'rename', label: 'Rename', disabledReason: 'Not available: there is no rename command for saved workflows.' },
    { id: 'save_copy', label: 'Save a copy', disabledReason: 'Not available here: open the graph and save it under a new name.' },
    {
      id: 'delete',
      label: isWorkflow ? 'Delete workflow' : 'Delete',
      disabledReason: isWorkflow ? null : 'Not available: orchestrations are deleted from their own editor.',
    },
  ];

  return {
    key: entry.key,
    kind: entry.kind,
    title: entry.name,
    id: entry.id,
    glyph: entry.glyph,
    stateLabel: entry.stateLabel,
    chips,
    activity,
    runSummary,
    recentRuns,
    moreRunCount: Math.max(0, entry.runs.length - recentRuns.length),
    open,
    resumeSentence: isWorkflow
      ? 'This engine build keeps no session state, so nothing resumes on its own. Open graph loads the saved workflow into a new edit session; finished runs stay in the Scheduler history and are not re-run.'
      : 'This engine build keeps no session state for orchestrations; their runs, if any, stay in the Scheduler history.',
    actions,
  };
}

/**
 * Text for the Scheduler search when "Show in Scheduler" is used. The scheduler
 * filter store has no exact workflow filter; its search matches the workflow id
 * as a substring (and other run fields), so the result can include workflows
 * whose id contains this one.
 */
export function schedulerSearchForSession(entry: Pick<SessionEntry, 'id'>): string {
  return entry.id;
}

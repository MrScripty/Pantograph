import test from 'node:test';
import assert from 'node:assert/strict';

import type { RunListProjectionRecord, RunListProjectionStatus } from '../../services/diagnostics/types';
import type { WorkflowMetadata } from '../../services/workflow/types';
import type { OrchestrationGraphMetadata } from '../../stores/orchestrationStore';
import {
  ACTIVE_SESSIONS_UNAVAILABLE_NOTE,
  SESSION_RECENT_RUN_LIMIT,
  buildSavedSessions,
  createLatestOnlyGate,
  buildSessionDetail,
  countRunOutcomes,
  formatLastActivityShort,
  formatSessionDateTime,
  formatSessionDay,
  groupSessions,
  lastActivityOf,
  outcomeStrip,
  outcomeSummaryText,
  parseMetadataDate,
  runOutcomeKind,
  runsForWorkflow,
  savedWorkflowId,
  schedulerSearchForSession,
  sessionFilterOptions,
  sessionSourceFromResult,
  sessionsDimensionLine,
  sessionDetailEmptyText,
  savedSourceFailure,
  workflowSessionEntry,
  orchestrationSessionEntry,
  type SessionSourceState,
} from './sessionsPresenters.ts';

// Local-time constructors keep the date wording independent of the test machine's zone.
const NOW = new Date(2026, 10, 3, 20, 6, 0).getTime();
const TODAY_1928 = new Date(2026, 10, 3, 19, 28, 0).getTime();
const OCT_31_1402 = new Date(2026, 9, 31, 14, 2, 0).getTime();
const JAN_31_2025 = new Date(2025, 0, 31, 9, 5, 0).getTime();

function run(
  id: string,
  workflowId: string,
  status: RunListProjectionStatus,
  updatedMs: number,
  extra: Partial<RunListProjectionRecord> = {},
): RunListProjectionRecord {
  return {
    workflow_run_id: id,
    workflow_id: workflowId,
    status,
    last_event_seq: 1,
    last_updated_at_ms: updatedMs,
    ...extra,
  };
}

function workflow(id: string, extra: Partial<WorkflowMetadata> = {}): WorkflowMetadata {
  return {
    id,
    name: id,
    created: new Date(OCT_31_1402).toISOString(),
    modified: new Date(OCT_31_1402).toISOString(),
    ...extra,
  };
}

function ready<T>(items: T[]): SessionSourceState<T> {
  return { status: 'ready', items };
}

const RUNS: RunListProjectionRecord[] = [
  run('run_a1', 'ingest-docs-rag', 'running', NOW - 10_000, { client_id: 'client_gui', client_session_id: 'cs_1', workflow_semantic_version: '1.4.0', selected_device_id: 'cuda:0' }),
  run('run_a2', 'ingest-docs-rag', 'queued', NOW - 20_000, { scheduler_queue_position: 2, client_id: 'client_gui', client_session_id: 'cs_2', workflow_semantic_version: '1.4.0' }),
  run('run_a3', 'ingest-docs-rag', 'completed', NOW - 600_000, { client_id: 'client_cli', workflow_semantic_version: '1.3.9' }),
  run('run_a4', 'ingest-docs-rag', 'failed', NOW - 900_000, { workflow_semantic_version: '1.3.9' }),
  run('run_b1', 'summarize-daily', 'completed', TODAY_1928),
  run('run_b2', 'summarize-daily', 'cancelled', TODAY_1928 - 60_000),
  run('run_x1', 'not-saved-anywhere', 'delayed', NOW - 5_000),
];

test('runOutcomeKind folds projection statuses into the five strip kinds', () => {
  assert.equal(runOutcomeKind('running'), 'running');
  for (const status of ['accepted', 'future', 'scheduled', 'queued', 'delayed'] as const) {
    assert.equal(runOutcomeKind(status), 'queued', status);
  }
  assert.equal(runOutcomeKind('completed'), 'succeeded');
  assert.equal(runOutcomeKind('failed'), 'failed');
  assert.equal(runOutcomeKind('cancelled'), 'cancelled');
});

test('outcome strip keeps board order, omits empty kinds and sums to the total', () => {
  const counts = countRunOutcomes([
    { status: 'completed' },
    { status: 'failed' },
    { status: 'running' },
    { status: 'delayed' },
    { status: 'completed' },
    { status: 'running' },
  ]);
  assert.deepEqual(counts, { running: 2, queued: 1, succeeded: 2, failed: 1, cancelled: 0, total: 6 });
  assert.deepEqual(outcomeStrip(counts), [
    { kind: 'running', count: 2 },
    { kind: 'queued', count: 1 },
    { kind: 'succeeded', count: 2 },
    { kind: 'failed', count: 1 },
  ]);
  assert.equal(outcomeSummaryText(counts), '2 running, 1 queued, 2 succeeded, 1 failed');
  assert.deepEqual(outcomeStrip(countRunOutcomes([])), []);
  assert.equal(outcomeSummaryText(countRunOutcomes([])), 'no runs');
});

test('runsForWorkflow matches the exact workflow id and orders newest first', () => {
  const rows = runsForWorkflow([...RUNS, run('run_a0', 'ingest-docs-rag-v2', 'completed', NOW)], 'ingest-docs-rag');
  assert.deepEqual(rows.map((row) => row.workflow_run_id), ['run_a1', 'run_a2', 'run_a3', 'run_a4']);
});

test('metadata dates parse RFC 3339 and reject junk', () => {
  assert.equal(parseMetadataDate('2026-10-31T14:02:00Z'), Date.UTC(2026, 9, 31, 14, 2, 0));
  assert.equal(parseMetadataDate('not a date'), null);
  assert.equal(parseMetadataDate(''), null);
  assert.equal(parseMetadataDate(undefined), null);
});

test('date wording: today, this year, other years', () => {
  assert.equal(formatSessionDateTime(TODAY_1928, NOW), 'Today 19:28');
  assert.equal(formatSessionDateTime(OCT_31_1402, NOW), 'Oct 31, 14:02');
  assert.equal(formatSessionDateTime(JAN_31_2025, NOW), 'Jan 31, 2025, 09:05');
  assert.equal(formatSessionDay(OCT_31_1402, NOW), 'Oct 31');
  assert.equal(formatSessionDay(JAN_31_2025, NOW), 'Jan 31, 2025');
});

test('last-activity short wording: now while live, time today, day otherwise, never', () => {
  assert.equal(formatLastActivityShort(TODAY_1928, NOW, true), 'now');
  assert.equal(formatLastActivityShort(TODAY_1928, NOW, false), '19:28');
  assert.equal(formatLastActivityShort(OCT_31_1402, NOW, false), 'Oct 31');
  assert.equal(formatLastActivityShort(JAN_31_2025, NOW, false), 'Jan 31, 2025');
  assert.equal(formatLastActivityShort(null, NOW, false), 'never');
});

test('last activity is the newer of the latest run update and the saved modified time', () => {
  const runs = [run('r1', 'w', 'completed', TODAY_1928), run('r2', 'w', 'failed', TODAY_1928 - 1000)];
  assert.deepEqual(lastActivityOf(runs, OCT_31_1402), { ms: TODAY_1928, what: 'run completed' });
  assert.deepEqual(lastActivityOf(runs, NOW), { ms: NOW, what: 'workflow saved' });
  assert.deepEqual(lastActivityOf([], OCT_31_1402), { ms: OCT_31_1402, what: 'workflow saved' });
  assert.deepEqual(lastActivityOf([], null), { ms: null, what: null });
});

test('workflow entry projects glyph, strip, counts and when from its runs', () => {
  const live = workflowSessionEntry(workflow('ingest-docs-rag', { description: 'Index the handbook' }), RUNS, NOW);
  assert.equal(live.key, 'workflow:ingest-docs-rag');
  assert.equal(live.glyph, 'running');
  assert.equal(live.stateLabel, 'Saved workflow, 2 live runs');
  assert.equal(live.runCountLabel, '4 runs');
  assert.equal(live.whenLabel, 'now');
  assert.equal(live.sub, '2 live runs, 1 running, 1 queued, 1 succeeded, 1 failed');
  assert.equal(live.stripLabel, 'Runs in recent history: 1 running, 1 queued, 1 succeeded, 1 failed');
  assert.equal(live.lastActivityLabel, 'Today 20:05, run in progress');

  const idle = workflowSessionEntry(workflow('maid', { created: new Date(JAN_31_2025).toISOString() }), RUNS, NOW);
  assert.equal(idle.glyph, 'idle');
  assert.equal(idle.stateLabel, 'Saved workflow');
  assert.equal(idle.runCountLabel, '0 runs');
  assert.equal(idle.sub, 'Created Jan 31, 2025');
  assert.equal(idle.whenLabel, 'Oct 31');
  assert.deepEqual(idle.strip, []);
});

test('saved workflow id is the file stem from list_workflows, else the name', () => {
  assert.equal(savedWorkflowId({ id: 'coding-agent', name: 'Coding Agent' }), 'coding-agent');
  assert.equal(savedWorkflowId({ id: '  ', name: 'Coding Agent' }), 'Coding Agent');
  assert.equal(savedWorkflowId({ name: 'Coding Agent' }), 'Coding Agent');
});

test('orchestration entry has no dates and says so in the detail', () => {
  const orchestration: OrchestrationGraphMetadata = { id: 'coding-agent-orch', name: 'Coding Agent Orchestration', description: '', nodeCount: 4 };
  const entry = orchestrationSessionEntry(orchestration, RUNS, NOW);
  assert.equal(entry.tag, 'orchestration');
  assert.equal(entry.sub, 'Orchestration, 4 nodes');
  assert.equal(entry.whenLabel, 'never');
  assert.equal(entry.stateLabel, 'Saved orchestration');
  const detail = buildSessionDetail(entry, null, NOW);
  assert.equal(detail.activity.find((row) => row.key === 'Created')?.value, 'Not recorded for orchestrations');
  assert.ok(detail.open[0].disabledReason, 'orchestrations cannot be opened in Graph');
  assert.ok(detail.actions.find((action) => action.id === 'delete')?.disabledReason);
});

test('saved sessions sort live first, then by last activity, then by name', () => {
  const sessions = buildSavedSessions(
    ready([workflow('maid'), workflow('summarize-daily'), workflow('ingest-docs-rag'), workflow('alpha')]),
    ready<OrchestrationGraphMetadata>([]),
    RUNS,
    NOW,
  );
  assert.deepEqual(sessions.map((entry) => entry.id), ['ingest-docs-rag', 'summarize-daily', 'alpha', 'maid']);
});

test('a source that failed or is loading contributes no entries', () => {
  const sessions = buildSavedSessions(
    { status: 'failed', message: 'unreachable' },
    { status: 'loading' },
    RUNS,
    NOW,
  );
  assert.deepEqual(sessions, []);
});

test('grouping: Active carries the explanation, Saved filters by query', () => {
  const saved = buildSavedSessions(ready([workflow('ingest-docs-rag'), workflow('maid')]), ready<OrchestrationGraphMetadata>([]), RUNS, NOW);

  const all = groupSessions(saved, 'all', '');
  assert.deepEqual(all.map((group) => group.id), ['active', 'saved']);
  assert.equal(all[0].note, ACTIVE_SESSIONS_UNAVAILABLE_NOTE);
  assert.equal(all[0].count, undefined);
  assert.equal(all[1].count, 2);
  assert.equal(all[1].note, null);

  assert.deepEqual(groupSessions(saved, 'active', '').map((group) => group.id), ['active']);
  assert.deepEqual(groupSessions(saved, 'saved', '').map((group) => group.id), ['saved']);

  const searched = groupSessions(saved, 'saved', 'MAID');
  assert.deepEqual(searched[0].entries.map((entry) => entry.id), ['maid']);

  const none = groupSessions(saved, 'saved', 'zzz');
  assert.equal(none[0].entries.length, 0);
  assert.equal(none[0].note, 'No saved sessions match "zzz".');

  const empty = groupSessions([], 'saved', '');
  assert.match(empty[0].note ?? '', /No saved workflows or orchestrations yet/);
});

test('filter options never claim a count of active sessions', () => {
  const saved = buildSavedSessions(ready([workflow('a'), workflow('b')]), ready<OrchestrationGraphMetadata>([]), [], NOW);
  assert.deepEqual(sessionFilterOptions(saved), [
    { value: 'all', label: 'All', count: 2 },
    { value: 'active', label: 'Active' },
    { value: 'saved', label: 'Saved', count: 2 },
  ]);
});

test('dimension line states sources, run totals and runs of unsaved workflows', () => {
  const workflows = ready([workflow('ingest-docs-rag'), workflow('summarize-daily')]);
  const orchestrations = ready<OrchestrationGraphMetadata>([]);
  const saved = buildSavedSessions(workflows, orchestrations, RUNS, NOW);
  assert.equal(
    sessionsDimensionLine({ workflows, orchestrations, saved, runs: RUNS, runsLoaded: true }),
    '2 saved workflows, 0 orchestrations. 7 runs in recent history, 3 live, 1 from workflows not saved here',
  );
  assert.equal(
    sessionsDimensionLine({ workflows: { status: 'loading' }, orchestrations, saved: [], runs: [], runsLoaded: false }),
    'Loading saved workflows. Run history not loaded',
  );
  assert.equal(
    sessionsDimensionLine({ workflows: { status: 'failed', message: 'x' }, orchestrations: { status: 'failed', message: 'y' }, saved: [], runs: RUNS, runsLoaded: true }),
    'Saved workflows unavailable, orchestrations unavailable. 7 runs in recent history, 3 live',
  );
});

test('detail lists activity, run summary, recent runs and the open item', () => {
  const runs = [
    ...RUNS,
    ...Array.from({ length: 6 }, (_, index) => run(`run_old${index}`, 'ingest-docs-rag', 'completed', OCT_31_1402 - index)),
  ];
  const entry = workflowSessionEntry(workflow('ingest-docs-rag'), runs, NOW);
  const detail = buildSessionDetail(entry, 'run_a2', NOW);

  assert.equal(detail.title, 'ingest-docs-rag');
  assert.deepEqual(detail.chips, ['latest v1.4.0', 'cuda:0']);
  assert.deepEqual(detail.activity, [
    { key: 'Created', value: 'Oct 31, 14:02' },
    { key: 'Modified', value: 'Oct 31, 14:02' },
    { key: 'Last activity', value: 'Today 20:05, run in progress' },
    { key: 'Clients', value: 'client_gui, client_cli, 2 client sessions', mono: true },
  ]);
  assert.deepEqual(detail.runSummary, [
    { key: 'Total', value: '10 in recent history: 1 running, 1 queued, 7 succeeded, 1 failed' },
    { key: 'Live now', value: 'run_a1 running, run_a2 queued #2' },
    { key: 'Versions run', value: 'v1.4.0, v1.3.9' },
  ]);
  assert.equal(detail.recentRuns.length, SESSION_RECENT_RUN_LIMIT);
  assert.equal(detail.moreRunCount, 5);
  assert.equal(detail.recentRuns.find((row) => row.runId === 'run_a2')?.selected, true);
  assert.equal(detail.open[0].actionLabel, 'Open graph');
  assert.equal(detail.open[0].disabledReason, null);
  assert.equal(detail.open[0].sub, 'Graph, .pantograph/workflows/ingest-docs-rag.json');
});

test('detail actions: only Delete workflow is backed by a service', () => {
  const detail = buildSessionDetail(workflowSessionEntry(workflow('maid'), [], NOW), null, NOW);
  const byId = Object.fromEntries(detail.actions.map((action) => [action.id, action]));
  assert.equal(byId.delete.label, 'Delete workflow');
  assert.equal(byId.delete.disabledReason, null);
  for (const id of ['resume', 'rename', 'save_copy']) {
    assert.ok(byId[id].disabledReason, `${id} is disabled with a reason`);
  }
  assert.equal(byId.close, undefined, 'saved sessions have no Close action');
  assert.deepEqual(detail.runSummary[0], { key: 'Total', value: 'None in recent history' });
  assert.equal(detail.activity[3].value, 'None recorded on its runs');
});

test('listing outcomes classify into ready or failed with a reason', () => {
  assert.deepEqual(sessionSourceFromResult({ status: 'fulfilled', value: [1, 2] }), { status: 'ready', items: [1, 2] });
  assert.deepEqual(sessionSourceFromResult({ status: 'rejected', reason: new Error(' no host ') }), { status: 'failed', message: 'no host' });
  assert.deepEqual(sessionSourceFromResult({ status: 'rejected', reason: 'Failed to read workflows directory' }), {
    status: 'failed',
    message: 'Failed to read workflows directory',
  });
  assert.deepEqual(sessionSourceFromResult({ status: 'rejected', reason: {} }), { status: 'failed', message: 'the engine gave no reason' });
  assert.equal(sessionSourceFromResult({ status: 'fulfilled', value: null as unknown as number[] }).status, 'failed');
});

test('Show in Scheduler searches by the saved workflow id', () => {
  assert.equal(schedulerSearchForSession({ id: 'ingest-docs-rag' }), 'ingest-docs-rag');
});

test('failed listings collapse into one explanation when they share a reason', () => {
  const okWorkflows = ready([workflow('a')]);
  const okOrchestrations = ready<OrchestrationGraphMetadata>([]);
  assert.equal(savedSourceFailure(okWorkflows, okOrchestrations), null);
  assert.equal(savedSourceFailure({ status: 'loading' }, { status: 'loading' }), null);

  const both = savedSourceFailure({ status: 'failed', message: 'no host' }, { status: 'failed', message: 'no host' });
  assert.equal(both?.title, 'Saved workflows and orchestrations could not be listed');
  assert.equal(both?.explanation, 'The engine did not return them: no host.');

  const different = savedSourceFailure({ status: 'failed', message: 'a' }, { status: 'failed', message: 'b' });
  assert.equal(different?.explanation, 'Workflows: a. Orchestrations: b.');

  assert.equal(savedSourceFailure({ status: 'failed', message: 'a' }, okOrchestrations)?.code, 'list_workflows');
  assert.equal(
    savedSourceFailure({ status: 'failed', message: 'Failed to read workflows directory.' }, okOrchestrations)?.explanation,
    'The engine did not return them: Failed to read workflows directory.',
  );
  assert.equal(savedSourceFailure(okWorkflows, { status: 'failed', message: 'b' })?.code, 'list_orchestrations');
});

test('latest-only gate: newest call applies, older ones are superseded, close ends all', () => {
  const gate = createLatestOnlyGate();
  const beforeOpen = gate.begin();
  assert.equal(gate.classify(beforeOpen), 'closed', 'nothing applies before the page mounts');

  gate.open();
  const first = gate.begin();
  const second = gate.begin();
  assert.equal(gate.classify(first), 'superseded');
  assert.equal(gate.classify(second), 'current');

  gate.close();
  assert.equal(gate.classify(second), 'closed', 'a call finishing after destroy never applies');

  gate.open();
  assert.equal(gate.classify(second), 'superseded', 'reopening does not revive an old call');
  const third = gate.begin();
  assert.equal(gate.classify(third), 'current');
});

test('latest-only gates are independent per operation', () => {
  const load = createLatestOnlyGate();
  const open = createLatestOnlyGate();
  load.open();
  open.open();
  const loadToken = load.begin();
  open.begin();
  open.begin();
  assert.equal(load.classify(loadToken), 'current');
});

test('detail empty text follows loading, filter, failure, emptiness and search', () => {
  const base = { loading: false, failed: false, savedCount: 3, filter: 'all' as const, query: '' };
  assert.equal(sessionDetailEmptyText({ ...base, loading: true }), 'Loading saved sessions…');
  assert.match(sessionDetailEmptyText({ ...base, filter: 'active' }), /No live sessions/);
  assert.match(sessionDetailEmptyText({ ...base, failed: true, savedCount: 0 }), /cannot be listed/);
  assert.match(sessionDetailEmptyText({ ...base, savedCount: 0 }), /Nothing to show yet/);
  assert.equal(sessionDetailEmptyText({ ...base, query: 'zzz' }), 'No session matches the search.');
  assert.match(sessionDetailEmptyText(base), /Select a session/);
});

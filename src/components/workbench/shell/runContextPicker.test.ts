import test from 'node:test';
import assert from 'node:assert/strict';

import type { RunListProjectionRecord } from '../../../services/diagnostics/types';
import {
  decorateRunContextRow,
  engineLineParts,
  engineLineText,
  formatClockTime,
  formatRunDuration,
  formatRunVersion,
  groupRunsForContextPicker,
  looksLikeRunId,
  resolveRunContextInput,
  runContextChip,
  runContextEmptyText,
  runContextGroupForRun,
  runMatchesContextQuery,
  shortRunId,
  startOfLocalDayMs,
  toActiveRunContext,
} from './runContextPicker.ts';

// A fixed local noon keeps "today" independent of the machine's time zone.
const NOW = new Date(2026, 10, 3, 12, 0, 0).getTime();
const TODAY = startOfLocalDayMs(NOW);
const YESTERDAY = TODAY - 6 * 60 * 60 * 1000;

function run(
  id: string,
  status: RunListProjectionRecord['status'],
  extra: Partial<RunListProjectionRecord> = {},
): RunListProjectionRecord {
  return {
    workflow_run_id: id,
    workflow_id: 'ingest-docs-rag',
    workflow_semantic_version: '1.4.0',
    status,
    last_event_seq: 1,
    last_updated_at_ms: NOW - 1000,
    ...extra,
  };
}

const RUNS: RunListProjectionRecord[] = [
  run('run_running_a', 'running', { selected_runtime_id: 'llama.cpp', selected_device_id: 'cuda:0' }),
  run('run_queued_2', 'queued', { workflow_id: 'summarize-daily', scheduler_queue_position: 2 }),
  run('run_queued_1', 'scheduled', { workflow_id: 'embed-corpus', scheduler_queue_position: 1 }),
  run('run_delayed', 'delayed', { workflow_id: 'transcribe-meeting', scheduler_reason: 'needs 8 GB on cuda:0' }),
  run('run_done_today', 'completed', { completed_at_ms: NOW - 5000, duration_ms: 38_000 }),
  run('run_failed_today', 'failed', {
    workflow_id: 'image-gen-sdxl',
    workflow_semantic_version: '0.9.1',
    completed_at_ms: NOW - 2000,
    latest_error_message: 'out of memory, needed 14.2 GB',
  }),
  run('run_old', 'cancelled', { completed_at_ms: YESTERDAY, duration_ms: 70_000 }),
];

test('statuses map to Running, Queued and the finished groups by local day', () => {
  assert.equal(runContextGroupForRun(run('a', 'running'), TODAY), 'running');
  for (const status of ['accepted', 'future', 'scheduled', 'queued', 'delayed'] as const) {
    assert.equal(runContextGroupForRun(run('a', status), TODAY), 'queued', status);
  }
  assert.equal(runContextGroupForRun(run('a', 'completed', { completed_at_ms: TODAY }), TODAY), 'finished_today');
  assert.equal(runContextGroupForRun(run('a', 'failed', { completed_at_ms: YESTERDAY }), TODAY), 'finished_earlier');
  assert.equal(
    runContextGroupForRun(run('a', 'cancelled', { completed_at_ms: null, last_updated_at_ms: NOW }), TODAY),
    'finished_today',
  );
});

test('groups come out in spec order, omit empty groups and sort within a group', () => {
  const groups = groupRunsForContextPicker(RUNS, { query: '', selectedRunId: 'run_queued_1', nowMs: NOW });
  assert.deepEqual(groups.map((group) => group.title), ['Running', 'Queued', 'Finished today', 'Finished earlier']);
  assert.deepEqual(
    groups[1].rows.map((row) => row.runId),
    ['run_queued_1', 'run_queued_2', 'run_delayed'],
  );
  assert.deepEqual(groups[2].rows.map((row) => row.runId), ['run_failed_today', 'run_done_today']);
  assert.equal(groups[1].rows[0].selected, true);
  assert.equal(groups[1].rows[1].selected, false);

  const onlyRunning = groupRunsForContextPicker([RUNS[0]], { query: '', selectedRunId: null, nowMs: NOW });
  assert.deepEqual(onlyRunning.map((group) => group.id), ['running']);
});

test('search matches workflow id, run id and version', () => {
  assert.equal(runMatchesContextQuery(RUNS[0], 'INGEST'), true);
  assert.equal(runMatchesContextQuery(RUNS[0], 'running_a'), true);
  assert.equal(runMatchesContextQuery(RUNS[5], 'v0.9.1'), true);
  assert.equal(runMatchesContextQuery(RUNS[5], '0.9'), true);
  assert.equal(runMatchesContextQuery(RUNS[0], 'sdxl'), false);
  const groups = groupRunsForContextPicker(RUNS, { query: 'sdxl', selectedRunId: null, nowMs: NOW });
  assert.deepEqual(groups.flatMap((group) => group.rows.map((row) => row.runId)), ['run_failed_today']);
});

test('typed or pasted text resolves to select, unknown run id, search or empty', () => {
  const selected = resolveRunContextInput('  run_queued_2 ', RUNS);
  assert.equal(selected.kind, 'select');
  assert.equal(selected.kind === 'select' && selected.run.workflow_run_id, 'run_queued_2');

  assert.deepEqual(resolveRunContextInput('run_01JB2ZZZZZ', RUNS), {
    kind: 'unknown_run_id',
    runId: 'run_01JB2ZZZZZ',
  });
  assert.deepEqual(resolveRunContextInput('embed', RUNS), { kind: 'search', query: 'embed' });
  assert.deepEqual(resolveRunContextInput('   ', RUNS), { kind: 'empty' });
  assert.equal(looksLikeRunId('run_7c9e6679-7425-40de-944b-e07fc1f90ae7'), true);
  assert.equal(looksLikeRunId('run with spaces'), false);
});

test('empty text explains unknown ids, no matches, loading and an unreachable engine', () => {
  const online = { kind: 'online' } as const;
  assert.equal(
    runContextEmptyText({ query: 'run_abc', runCount: 3, availability: online, loading: false }),
    'No run with id run_abc in recent history.',
  );
  assert.equal(runContextEmptyText({ query: 'zzz', runCount: 3, availability: online, loading: false }), 'No runs match.');
  assert.equal(runContextEmptyText({ query: '', runCount: 0, availability: online, loading: true }), 'Loading runs…');
  assert.match(
    runContextEmptyText({
      query: '',
      runCount: 0,
      availability: { kind: 'unreachable', sinceMs: NOW, hasProjection: false },
      loading: false,
    }),
    /engine did not answer/,
  );
});

test('row decoration carries status label and one-line context', () => {
  const byId = new Map(RUNS.map((record) => [record.workflow_run_id, record]));
  const decorate = (id: string) => decorateRunContextRow(byId.get(id)!, null);
  assert.equal(decorate('run_running_a').contextSuffix, ', llama.cpp on cuda:0');
  assert.equal(decorate('run_queued_2').contextSuffix, ', #2 in queue');
  assert.equal(decorate('run_delayed').statusLabel, 'Delayed');
  assert.equal(decorate('run_delayed').contextSuffix, ', needs 8 GB on cuda:0');
  assert.equal(decorate('run_delayed').glyph, 'queued');
  assert.equal(decorate('run_failed_today').statusIsFailure, true);
  assert.equal(decorate('run_failed_today').contextSuffix, ', out of memory, needed 14.2 GB');
  assert.equal(decorate('run_done_today').contextSuffix, ', 38 s');
  assert.equal(decorate('run_done_today').versionLabel, 'v1.4.0');
});

test('formatters keep durations, versions, clock times and short ids compact', () => {
  assert.equal(formatRunDuration(38_000), '38 s');
  assert.equal(formatRunDuration(252_000), '4m 12s');
  assert.equal(formatRunDuration(3_780_000), '1h 03m');
  assert.equal(formatRunDuration(null), null);
  assert.equal(formatRunVersion('1.4.0'), 'v1.4.0');
  assert.equal(formatRunVersion('v2.0.0'), 'v2.0.0');
  assert.equal(formatRunVersion('  '), null);
  assert.equal(formatClockTime(new Date(2026, 10, 3, 9, 5).getTime()), '09:05');
  assert.equal(shortRunId('run_7c9e6679-7425-40de-944b-e07fc1f90ae7'), 'run_…c1f90ae7');
  assert.equal(shortRunId('run_01JB2K7Q3M'), 'run_01JB2K7Q3M');
});

test('chip prefers the fresher run-list status over the status captured at selection', () => {
  assert.deepEqual(runContextChip(null, RUNS), { kind: 'none' });
  const chip = runContextChip(
    { workflow_run_id: 'run_failed_today', workflow_id: 'image-gen-sdxl', workflow_semantic_version: '0.9.1', status: 'running' },
    RUNS,
  );
  assert.equal(chip.kind, 'run');
  assert.equal(chip.kind === 'run' && chip.glyph, 'failed');
  assert.equal(chip.kind === 'run' && chip.versionLabel, 'v0.9.1');

  const absent = runContextChip(
    { workflow_run_id: 'run_gone', workflow_id: null, workflow_semantic_version: null, status: 'completed' },
    RUNS,
  );
  assert.equal(absent.kind === 'run' && absent.workflowLabel, 'run_gone');
  assert.equal(absent.kind === 'run' && absent.glyph, 'succeeded');
});

test('selection context copies the projection identity fields', () => {
  assert.deepEqual(toActiveRunContext(RUNS[1]), {
    workflow_run_id: 'run_queued_2',
    workflow_id: 'summarize-daily',
    workflow_version_id: null,
    workflow_semantic_version: '1.4.0',
    status: 'queued',
  });
});

test('engine line names the unreachable time and whether a projection is shown', () => {
  const since = new Date(2026, 10, 3, 20, 3).getTime();
  assert.equal(engineLineText({ kind: 'online' }), 'Local engine');
  assert.equal(
    engineLineText({ kind: 'unreachable', sinceMs: since, hasProjection: true }),
    'Engine unreachable since 20:03, showing last projection',
  );
  assert.equal(
    engineLineText({ kind: 'unreachable', sinceMs: since, hasProjection: false }),
    'Engine unreachable since 20:03, nothing loaded yet',
  );
  assert.deepEqual(engineLineParts({ kind: 'unreachable', sinceMs: since, hasProjection: true }), {
    headline: 'Engine unreachable since 20:03',
    detail: 'showing last projection',
  });
  assert.equal(engineLineText({ kind: 'unknown' }), 'Local engine, connecting');
});

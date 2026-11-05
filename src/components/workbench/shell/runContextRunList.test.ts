import test from 'node:test';
import assert from 'node:assert/strict';
import { get } from 'svelte/store';

import type {
  RunListProjectionRecord,
  WorkflowRunListQueryResponse,
} from '../../../services/diagnostics/types';
import { createRunContextRunListSource, type RunListQueryFailure } from './runContextRunList.ts';

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (error: unknown) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function response(...ids: string[]): WorkflowRunListQueryResponse {
  const runs: RunListProjectionRecord[] = ids.map((id) => ({
    workflow_run_id: id,
    workflow_id: 'wf',
    status: 'running',
    last_event_seq: 1,
    last_updated_at_ms: 1,
  }));
  return {
    runs,
    facets: [],
    projection_state: {
      projection_name: 'run_list',
      projection_version: 1,
      last_applied_event_seq: 1,
      status: 'current',
      rebuilt_at_ms: null,
      updated_at_ms: 1,
    },
  } as WorkflowRunListQueryResponse;
}

function harness() {
  const pending: Deferred<WorkflowRunListQueryResponse>[] = [];
  const requests: unknown[] = [];
  let clock = 1_000;
  const source = createRunContextRunListSource({
    queryRunList: (request) => {
      requests.push(request);
      const next = deferred<WorkflowRunListQueryResponse>();
      pending.push(next);
      return next.promise;
    },
    classifyFailure: (error): RunListQueryFailure => ({
      transport: error === 'transport',
      message: String(error),
    }),
    now: () => clock,
    limit: 250,
  });
  return {
    source,
    pending,
    requests,
    tick(ms: number) {
      clock += ms;
    },
  };
}

test('refresh before attach does not query', async () => {
  const { source, requests } = harness();
  assert.equal(await source.refresh(), 'detached');
  assert.equal(requests.length, 0);
});

test('a second attach is rejected so the shell stays the single owner', () => {
  const { source } = harness();
  source.attach();
  assert.throws(() => source.attach(), /already attached/);
  source.detach();
  source.detach();
  assert.equal(source.isAttached(), false);
});

test('overlapping refreshes apply only the newest result', async () => {
  const { source, pending, requests } = harness();
  source.attach();
  const first = source.refresh();
  const second = source.refresh();
  assert.deepEqual(requests, [{ limit: 250 }, { limit: 250 }]);

  pending[1].resolve(response('run_new'));
  assert.equal(await second, 'applied');
  pending[0].resolve(response('run_stale'));
  assert.equal(await first, 'superseded');

  assert.deepEqual(get(source.state).runs.map((run) => run.workflow_run_id), ['run_new']);
  assert.equal(get(source.state).status, 'ready');
});

test('detach classifies an in-flight completion and leaves state untouched', async () => {
  const { source, pending } = harness();
  source.attach();
  const inFlight = source.refresh();
  source.detach();
  pending[0].resolve(response('run_late'));
  assert.equal(await inFlight, 'detached');
  assert.deepEqual(get(source.state).runs, []);
  assert.equal(get(source.state).status, 'idle');

  // Re-attaching does not resurrect the old invocation.
  source.attach();
  const fresh = source.refresh();
  pending[1].resolve(response('run_fresh'));
  assert.equal(await fresh, 'applied');
  assert.deepEqual(get(source.state).runs.map((run) => run.workflow_run_id), ['run_fresh']);
});

test('transport failure keeps the last projection and the first unreachable time', async () => {
  const { source, pending, tick } = harness();
  source.attach();
  const loaded = source.refresh();
  pending[0].resolve(response('run_kept'));
  await loaded;

  tick(5_000);
  const down = source.refresh();
  pending[1].reject('transport');
  assert.equal(await down, 'unreachable');
  let state = get(source.state);
  assert.deepEqual(state.availability, { kind: 'unreachable', sinceMs: 6_000, hasProjection: true });
  assert.deepEqual(state.runs.map((run) => run.workflow_run_id), ['run_kept']);

  tick(5_000);
  const stillDown = source.refresh();
  pending[2].reject('transport');
  await stillDown;
  state = get(source.state);
  assert.equal(state.availability.kind === 'unreachable' && state.availability.sinceMs, 6_000);

  const back = source.refresh();
  pending[3].resolve(response('run_back'));
  assert.equal(await back, 'applied');
  assert.deepEqual(get(source.state).availability, { kind: 'online' });
});

test('a backend error keeps the engine online and reports the message', async () => {
  const { source, pending } = harness();
  source.attach();
  const failed = source.refresh();
  pending[0].reject('scheduler_busy');
  assert.equal(await failed, 'failed');
  const state = get(source.state);
  assert.deepEqual(state.availability, { kind: 'online' });
  assert.equal(state.errorMessage, 'scheduler_busy');
  assert.equal(state.status, 'failed');
});

test('unreachable before any load says no projection is shown', async () => {
  const { source, pending } = harness();
  source.attach();
  const down = source.refresh();
  pending[0].reject('transport');
  await down;
  const availability = get(source.state).availability;
  assert.equal(availability.kind === 'unreachable' && availability.hasProjection, false);
});

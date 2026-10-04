import assert from 'node:assert/strict';
import test from 'node:test';
import { createUndoStore } from './createUndoStore.ts';
import { decodeUndoState, MAX_UNDO_HISTORY, UNDO_STORAGE_KEY } from './undoPersistence.ts';
import type { UndoEntry, UndoableAction } from './undoPersistence.ts';

const entry = (id = 'opaque-entry', hash = 'opaque-commit'): UndoEntry => ({
  id, action: { type: 'COMMIT_SOFT_DELETE', hash }, timestamp: 0,
});
const state = (history = [entry()], position = history.length - 1) => ({ history, position });

function fixture(initial: string | null) {
  let bytes = initial;
  const writes: string[] = [];
  const logs: { type: string; payload: Record<string, unknown>; severity?: string }[] = [];
  const callbacks: { operation: string; hash: string }[] = [];
  const storage = {
    getItem(key: string) { assert.equal(key, UNDO_STORAGE_KEY); return bytes; },
    setItem(key: string, value: string) {
      assert.equal(key, UNDO_STORAGE_KEY);
      writes.push(value);
      bytes = value;
    },
  };
  const create = () => {
    const store = createUndoStore({
      storage: () => storage,
      log: (type, payload, severity) => logs.push({ type, payload, severity }),
    });
    // These are recorders, never the timeline's destructive Git implementation.
    store.onAgedAction(async (action) => { callbacks.push({ operation: 'age', hash: action.hash }); });
    store.onUndo('COMMIT_SOFT_DELETE', (action) => { callbacks.push({ operation: 'undo', hash: action.hash }); });
    store.onRedo('COMMIT_SOFT_DELETE', (action) => { callbacks.push({ operation: 'redo', hash: action.hash }); });
    return store;
  };
  return { create, writes, logs, callbacks, bytes: () => bytes };
}

const malformed: [string, unknown][] = [
  ['null record', null],
  ['array record', []],
  ['missing history', {}],
  ['non-array history', { history: {} }],
  ['null entry', state([null as unknown as UndoEntry])],
  ['mixed valid and invalid entries', state([entry(), null as unknown as UndoEntry])],
  ['null action', state([{ ...entry(), action: null as unknown as UndoableAction }])],
  ['unknown action', { history: [{ ...entry(), action: { type: 'DELETE_ALL', hash: 'commit' } }], position: 0 }],
  ['missing identifier', { history: [{ action: entry().action, timestamp: 0 }], position: 0 }],
  ['blank identifier', state([entry('  ')])],
  ['non-string hash', { history: [{ ...entry(), action: { type: 'COMMIT_SOFT_DELETE', hash: 3 } }], position: 0 }],
  ['blank hash', state([entry('entry', ' \t ')])],
  ['missing timestamp', { history: [{ id: 'entry', action: entry().action }], position: 0 }],
  ['negative timestamp', state([{ ...entry(), timestamp: -1 }])],
  ['fractional timestamp', state([{ ...entry(), timestamp: 0.5 }])],
  ['unsafe timestamp', state([{ ...entry(), timestamp: Number.MAX_SAFE_INTEGER + 1 }])],
  ['negative cursor', state([entry()], -2)],
  ['fractional cursor', state([entry()], -0.5)],
  ['out-of-range cursor', state([entry()], 99)],
  ['null cursor', { history: [entry()], position: null }],
  ['string cursor', { history: [entry()], position: '0' }],
  ['empty history cursor', state([], 0)],
  ['oversize history', state(Array.from({ length: MAX_UNDO_HISTORY + 1 }, (_, i) => entry(`old-${i}`, `old-${i}`)))],
];

for (const [name, value] of malformed) {
  test(`rejects the whole persisted record without recovery effects: ${name}`, async () => {
    assert.equal(decodeUndoState(value).ok, false);
    const original = JSON.stringify(value);
    const f = fixture(original);
    const store = f.create();
    assert.deepEqual(store.getState(), state([], -1));
    assert.equal(store.canUndo(), false);
    assert.equal(store.canRedo(), false);
    assert.equal(store.undo(), false);
    assert.equal(store.redo(), false);
    assert.deepEqual(f.callbacks, []);
    assert.equal(f.logs[0].type, 'UNDO_STORE_LOAD_FAILED');
    assert.equal(f.logs[0].severity, 'warn');
    // No persisted entry can be aged into the destructive callback on a new push.
    await store.push({ type: 'COMMIT_SOFT_DELETE', hash: 'new-only' });
    assert.deepEqual(f.callbacks, []);
    store.clear();
    assert.deepEqual(f.writes, []);
    assert.equal(f.bytes(), original);
    assert.deepEqual(f.create().getState(), state([], -1));
    assert.equal(f.bytes(), original);
  });
}

test('rejects invalid JSON and empty bytes without overwriting either', async () => {
  for (const original of ['{broken', '']) {
    const f = fixture(original);
    const store = f.create();
    await store.push({ type: 'COMMIT_SOFT_DELETE', hash: 'fresh' });
    store.clear();
    assert.deepEqual(f.callbacks, []);
    assert.deepEqual(f.writes, []);
    assert.equal(f.bytes(), original);
    assert.equal(f.logs[0].type, 'UNDO_STORE_LOAD_FAILED');
  }
});

test('rejects non-finite numeric values at the unknown-value boundary', () => {
  for (const number of [NaN, Infinity, -Infinity]) {
    assert.equal(decodeUndoState(state([{ ...entry(), timestamp: number }])).ok, false);
    assert.equal(decodeUndoState(state([entry()], number)).ok, false);
  }
});

test('accepts valid legacy records and omitted-position fallback without restore writes', () => {
  const history = [entry('first', 'short'), entry('second', 'opaque/reference')];
  for (const record of [{ history }, ...[-1, 0, 1].map((position) => ({ history, position }))]) {
    const f = fixture(JSON.stringify(record));
    const store = f.create();
    assert.deepEqual(store.getState(), { history, position: 'position' in record ? record.position : 1 });
    assert.deepEqual(f.writes, []);
    assert.deepEqual(f.callbacks, []);
  }
  assert.deepEqual(decodeUndoState({ history: [] }), { ok: true, state: state([], -1) });
  assert.equal(decodeUndoState(state([{ ...entry(), timestamp: Number.MAX_SAFE_INTEGER }])).ok, true);
});

test('valid undo and redo invoke only their corresponding actions and survive reread', () => {
  const f = fixture(JSON.stringify(state([entry('first', 'a'), entry('second', 'b')])));
  const store = f.create();
  assert.equal(store.undo(), true);
  assert.equal(store.undo(), true);
  assert.equal(store.undo(), false);
  assert.equal(store.canUndo(), false);
  assert.equal(store.canRedo(), true);
  assert.equal(store.redo(), true);
  assert.equal(store.redo(), true);
  assert.equal(store.redo(), false);
  assert.deepEqual(f.callbacks, [
    { operation: 'undo', hash: 'b' }, { operation: 'undo', hash: 'a' },
    { operation: 'redo', hash: 'a' }, { operation: 'redo', hash: 'b' },
  ]);
  assert.deepEqual(f.create().getState(), store.getState());
});

test('a new action after undo truncates redo without aging discarded actions', async () => {
  const f = fixture(JSON.stringify(state([entry('first', 'a'), entry('second', 'b')], 0)));
  const store = f.create();
  await store.push({ type: 'COMMIT_SOFT_DELETE', hash: 'new' });
  assert.deepEqual(store.getState().history.map((item) => item.action.hash), ['a', 'new']);
  assert.equal(store.canRedo(), false);
  assert.deepEqual(f.callbacks, []);
});

test('valid full history ages exactly the oldest entry on a new action', async () => {
  const history = Array.from({ length: MAX_UNDO_HISTORY }, (_, i) => entry(`entry-${i}`, `commit-${i}`));
  const f = fixture(JSON.stringify(state(history)));
  const store = f.create();
  assert.deepEqual(f.callbacks, []);
  await store.push({ type: 'COMMIT_SOFT_DELETE', hash: 'new' });
  assert.deepEqual(f.callbacks, [{ operation: 'age', hash: 'commit-0' }]);
  assert.equal(store.getState().history.length, MAX_UNDO_HISTORY);
  assert.equal(store.getState().position, MAX_UNDO_HISTORY - 1);
  assert.equal(store.getState().history[0].action.hash, 'commit-1');
  assert.deepEqual(f.create().getState(), store.getState());
});

test('rejected history never reaches aging even after a full new in-memory history', async () => {
  const original = JSON.stringify(malformed.at(-1)![1]);
  const f = fixture(original);
  const store = f.create();
  for (let i = 0; i <= MAX_UNDO_HISTORY; i++) {
    await store.push({ type: 'COMMIT_SOFT_DELETE', hash: `new-${i}` });
  }
  assert.deepEqual(f.callbacks, [{ operation: 'age', hash: 'new-0' }]);
  assert.deepEqual(f.writes, []);
  assert.equal(f.bytes(), original);
});

test('missing storage supports ordinary persistence and explicit clear', async () => {
  const f = fixture(null);
  const store = f.create();
  await store.push({ type: 'COMMIT_SOFT_DELETE', hash: 'new' });
  assert.equal(f.writes.length, 1);
  assert.deepEqual(f.create().getState(), store.getState());
  store.clear();
  assert.deepEqual(JSON.parse(f.bytes()!), state([], -1));
  assert.deepEqual(f.callbacks, []);
});

test('unavailable storage reports failure and never attempts recovery writes', async () => {
  for (const failAcquisition of [true, false]) {
    const logs: string[] = [];
    let writes = 0;
    const store = createUndoStore({
      storage: () => {
        if (failAcquisition) throw new Error('storage denied');
        return { getItem: () => { throw new Error('read denied'); }, setItem: () => { writes++; } };
      },
      log: (type) => { logs.push(type); },
    });
    await store.push({ type: 'COMMIT_SOFT_DELETE', hash: 'fresh' });
    store.clear();
    assert.equal(logs[0], 'UNDO_STORE_LOAD_FAILED');
    assert.ok(logs.includes('UNDO_STORE_SAVE_FAILED'));
    assert.equal(writes, 0);
  }
});

test('store instances keep storage and action callbacks isolated', async () => {
  const a = fixture(JSON.stringify(state([entry('a', 'a')])));
  const b = fixture(JSON.stringify(state([entry('b', 'b')])));
  const first = a.create();
  const second = b.create();
  first.undo();
  assert.deepEqual(a.callbacks, [{ operation: 'undo', hash: 'a' }]);
  assert.deepEqual(b.callbacks, []);
  assert.equal(second.getState().position, 0);
  await second.push({ type: 'COMMIT_SOFT_DELETE', hash: 'b-next' });
  assert.equal(first.getState().history.length, 1);
  second.undo();
  assert.deepEqual(b.callbacks, [{ operation: 'undo', hash: 'b-next' }]);
});


test('ordinary save failure is reported while valid in-memory actions remain usable', async () => {
  const logs: string[] = [];
  const store = createUndoStore({
    storage: () => ({ getItem: () => null, setItem: () => { throw new Error('quota'); } }),
    log: (type) => { logs.push(type); },
  });
  await store.push({ type: 'COMMIT_SOFT_DELETE', hash: 'new' });
  assert.equal(store.canUndo(), true);
  assert.equal(store.undo(), true);
  assert.equal(store.redo(), true);
  assert.equal(logs.filter((type) => type === 'UNDO_STORE_SAVE_FAILED').length, 3);
});

test('valid-history aging callback failure retains existing error and state behavior', async () => {
  const f = fixture(JSON.stringify(state(Array.from({ length: MAX_UNDO_HISTORY }, (_, i) => entry(`id-${i}`)))));
  const store = f.create();
  let calls = 0;
  store.onAgedAction(async () => { calls++; throw new Error('controlled cleanup failure'); });
  await store.push({ type: 'COMMIT_SOFT_DELETE', hash: 'new' });
  assert.equal(calls, 1);
  assert.ok(f.logs.some(({ type, severity }) => type === 'UNDO_AGED_ACTION_FAILED' && severity === 'error'));
  assert.equal(store.getState().history.length, MAX_UNDO_HISTORY);
  assert.equal(store.getState().history.at(-1)?.action.hash, 'new');
});

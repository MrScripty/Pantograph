import assert from 'node:assert/strict';
import test, { type TestContext } from 'node:test';
import { get } from 'svelte/store';
import { createViewStores } from './createViewStores.ts';

function storageFixture(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial));
  const writes: { key: string; value: string }[] = [];
  let reads = 0;
  const storage = {
    getItem(key: string): string | null {
      reads++;
      return values.get(key) ?? null;
    },
    setItem(key: string, value: string): void {
      writes.push({ key, value });
      values.set(key, value);
    },
  };
  return { storage, values, writes, reads: () => reads };
}

function useStorage(t: TestContext, storage: Pick<Storage, 'getItem' | 'setItem'>) {
  const previous = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: storage });
  t.after(() => {
    if (previous) Object.defineProperty(globalThis, 'localStorage', previous);
    else Reflect.deleteProperty(globalThis, 'localStorage');
  });
  t.mock.timers.enable({ apis: ['setTimeout'] });
}

function snapshot(view: ReturnType<typeof createViewStores>) {
  return {
    viewLevel: get(view.viewLevel),
    orchestrationId: get(view.currentOrchestrationId),
    dataGraphId: get(view.currentDataGraphId),
    groupStack: get(view.groupStack),
  };
}

const malformed = '{"viewLevel":"group","orchestrationId":42,"dataGraphId":"graph","groupStack":{"length":1}}';

test('rejects malformed record without state application or broken breadcrumb iteration', (t) => {
  const f = storageFixture({ view: malformed });
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  view.currentOrchestrationId.set('existing');
  const before = snapshot(view);
  const breadcrumbs: unknown[] = [];
  const release = view.breadcrumb.subscribe(value => { breadcrumbs.push(value); });
  t.after(release);
  assert.equal(view.restoreViewState(), false);
  assert.deepEqual(snapshot(view), before);
  assert.equal(breadcrumbs.length, 1);
  assert.deepEqual(get(view.breadcrumb), [{ id: 'existing', name: 'existing', level: 'orchestration' }]);
  view.persistViewState();
  assert.equal(f.values.get('view'), malformed);
  assert.equal(f.writes.length, 0);
});

test('enable persistence validates before installing writeback', (t) => {
  const f = storageFixture({ view: malformed });
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  let subscriptions = 0;
  const subscribe = view.viewLevel.subscribe;
  view.viewLevel.subscribe = (...args) => { subscriptions++; return subscribe(...args); };
  const releaseA = view.enablePersistence();
  const releaseB = view.enablePersistence();
  assert.equal(subscriptions, 0);
  view.groupStack.set(['local']);
  t.mock.timers.tick(1000);
  assert.equal(f.writes.length, 0);
  assert.equal(f.values.get('view'), malformed);
  releaseA();
  releaseA();
  releaseB();
});

for (const [name, record] of [
  ['malformed JSON', '{'],
  ['empty bytes', ''],
  ['null root', 'null'],
  ['array root', '[]'],
  ['string root', '"view"'],
  ['numeric root', '42'],
  ['boolean root', 'false'],
  ['unknown level', '{"viewLevel":"other"}'],
  ['null level', '{"viewLevel":null}'],
  ['nonfinite identity', '{"dataGraphId":1e309}'],
  ['boolean identity', '{"orchestrationId":false}'],
  ['object identity', '{"dataGraphId":{"id":"graph"}}'],
  ['null stack', '{"groupStack":null}'],
  ['string stack', '{"groupStack":"group"}'],
  ['mixed stack', '{"groupStack":["group",42]}'],
  ['nested stack', '{"groupStack":[["group"]]}'],
] as const) {
  test(`invalid stored ${name} is retained across restore, explicit writes and enable`, (t) => {
    const f = storageFixture({ view: record });
    useStorage(t, f.storage);
    const view = createViewStores({ storageKey: 'view', storage: f.storage });
    const before = snapshot(view);
    assert.equal(view.restoreViewState(), false);
    assert.equal(view.restoreViewState(), false);
    assert.deepEqual(snapshot(view), before);
    view.persistViewState();
    const release = view.enablePersistence();
    t.mock.timers.tick(1000);
    release();
    assert.equal(f.values.get('view'), record);
    assert.equal(f.writes.length, 0);
  });
}

test('valid complete record roundtrips with iterable breadcrumbs and unchanged serialization', (t) => {
  const f = storageFixture();
  useStorage(t, f.storage);
  const first = createViewStores({ storageKey: 'view', storage: f.storage });
  first.viewLevel.set('group');
  first.currentOrchestrationId.set('orchestration');
  first.currentDataGraphId.set('graph');
  first.groupStack.set(['outer', 'inner']);
  first.persistViewState();
  const expected = snapshot(first);
  assert.equal(f.values.get('view'), JSON.stringify(expected));
  const second = createViewStores({ storageKey: 'view', storage: f.storage });
  assert.equal(second.restoreViewState(), true);
  assert.deepEqual(snapshot(second), expected);
  assert.deepEqual(get(second.breadcrumb), [
    { id: 'orchestration', name: 'orchestration', level: 'orchestration' },
    { id: 'graph', name: 'graph', level: 'data-graph' },
    { id: 'outer', name: 'outer', level: 'group' },
    { id: 'inner', name: 'inner', level: 'group' },
  ]);
});

test('partial and empty legacy records retain missing fields and tolerate ignored extensions', (t) => {
  const f = storageFixture({ view: '{"viewLevel":"group","legacyExtension":{"enabled":true}}' });
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  view.currentDataGraphId.set('existing');
  view.groupStack.set(['existing-group']);
  assert.equal(view.restoreViewState(), true);
  assert.deepEqual(snapshot(view), {
    viewLevel: 'group', orchestrationId: null, dataGraphId: 'existing', groupStack: ['existing-group'],
  });
  f.values.set('view', '{}');
  const before = snapshot(view);
  assert.equal(view.restoreViewState(), true);
  assert.deepEqual(snapshot(view), before);
  assert.equal(f.writes.length, 0);
});

test('repeated complete restoration clears explicit null identities without writing', (t) => {
  const f = storageFixture({ view: '{"viewLevel":"group","orchestrationId":"old","dataGraphId":"old-graph","groupStack":["old-group"]}' });
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  assert.equal(view.restoreViewState(), true);
  const cleared = '{"viewLevel":"data-graph","orchestrationId":null,"dataGraphId":null,"groupStack":[]}';
  f.values.set('view', cleared);
  assert.equal(view.restoreViewState(), true);
  assert.equal(view.restoreViewState(), true);
  assert.deepEqual(snapshot(view), { viewLevel: 'data-graph', orchestrationId: null, dataGraphId: null, groupStack: [] });
  assert.deepEqual(get(view.breadcrumb), []);
  assert.equal(f.values.get('view'), cleared);
  assert.equal(f.writes.length, 0);
});

test('enable restores valid data before initial subscription writeback can replace it', (t) => {
  const record = { viewLevel: 'group', orchestrationId: 'o', dataGraphId: 'g', groupStack: ['nested'] };
  const f = storageFixture({ view: JSON.stringify(record) });
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  const releaseA = view.enablePersistence();
  assert.deepEqual(snapshot(view), record);
  view.groupStack.set(['local']);
  const releaseB = view.enablePersistence();
  assert.deepEqual(get(view.groupStack), ['local'], 'duplicate enable cannot restore over newer state');
  t.mock.timers.tick(499);
  assert.equal(f.writes.length, 0);
  t.mock.timers.tick(1);
  assert.equal(f.writes.length, 1);
  assert.deepEqual(JSON.parse(f.writes[0].value), { ...record, groupStack: ['local'] });
  releaseA();
  releaseB();
});

test('restore suppresses synchronous subscriber writes before the pending snapshot', (t) => {
  const f = storageFixture();
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  const releasePersistence = view.enablePersistence();
  let observingRestore = false;
  const releaseObserver = view.groupStack.subscribe(() => {
    if (observingRestore) view.persistViewState();
  });
  const record = { viewLevel: 'group', orchestrationId: 'o', dataGraphId: 'g', groupStack: ['restored'] };
  const bytes = JSON.stringify(record);
  f.values.set('view', bytes);
  observingRestore = true;
  assert.equal(view.restoreViewState(), true);
  assert.deepEqual(snapshot(view), record);
  assert.equal(f.writes.length, 0, 'no partial record can be written during restore');
  t.mock.timers.tick(500);
  assert.equal(f.values.get('view'), bytes);
  assert.equal(f.writes.length, 1);
  assert.deepEqual(JSON.parse(f.writes[0].value), record);
  observingRestore = false;
  view.groupStack.set(['later']);
  t.mock.timers.tick(500);
  assert.deepEqual(JSON.parse(f.writes[1].value), { ...record, groupStack: ['later'] });
  releaseObserver();
  releasePersistence();
});

test('malformed storage appearing before a pending write is rechecked and retained', (t) => {
  const f = storageFixture();
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  const release = view.enablePersistence();
  f.values.set('view', malformed);
  t.mock.timers.tick(500);
  view.groupStack.set(['later']);
  view.persistViewState();
  t.mock.timers.tick(1000);
  assert.equal(f.values.get('view'), malformed);
  assert.equal(f.writes.length, 0);
  release();
  release();
});

test('invalid repeated restore cancels an active scope and cannot restart implicit writes', (t) => {
  const f = storageFixture();
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  const releaseA = view.enablePersistence();
  const releaseB = view.enablePersistence();
  view.groupStack.set(['local']);
  f.values.set('view', malformed);
  const before = snapshot(view);
  assert.equal(view.restoreViewState(), false);
  assert.deepEqual(snapshot(view), before);
  t.mock.timers.tick(1000);
  const releaseC = view.enablePersistence();
  releaseA();
  releaseB();
  releaseC();
  assert.equal(f.writes.length, 0);
  assert.equal(f.values.get('view'), malformed);
});

test('valid explicit restore after rejection can update memory but cannot recover writeback', (t) => {
  const f = storageFixture({ view: malformed });
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  assert.equal(view.restoreViewState(), false);
  const repairedBytes = '{"viewLevel":"group","groupStack":["external-repair"]}';
  f.values.set('view', repairedBytes);
  assert.equal(view.restoreViewState(), true);
  assert.deepEqual(get(view.groupStack), ['external-repair']);
  view.persistViewState();
  const release = view.enablePersistence();
  t.mock.timers.tick(500);
  release();
  assert.equal(f.values.get('view'), repairedBytes);
  assert.equal(f.writes.length, 0);
});

test('unavailable storage leaves state intact and no later call starts recovery writes', (t) => {
  const f = storageFixture();
  let unavailable = true;
  const storage = {
    getItem: (key: string) => {
      if (unavailable) throw new Error('storage denied');
      return f.storage.getItem(key);
    },
    setItem: f.storage.setItem,
  };
  useStorage(t, storage);
  const view = createViewStores({ storageKey: 'view', storage });
  view.currentDataGraphId.set('local');
  const before = snapshot(view);
  assert.equal(view.restoreViewState(), false);
  assert.deepEqual(snapshot(view), before);
  unavailable = false;
  view.persistViewState();
  const release = view.enablePersistence();
  t.mock.timers.tick(500);
  release();
  assert.equal(f.writes.length, 0);
});

test('ambient storage accessor failure is observed without attempting writeback', (t) => {
  const f = storageFixture();
  useStorage(t, f.storage);
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    get: () => { throw new Error('storage accessor denied'); },
  });
  const view = createViewStores({ storageKey: 'view' });
  assert.equal(view.restoreViewState(), false);
  const release = view.enablePersistence();
  view.persistViewState();
  t.mock.timers.tick(500);
  release();
  assert.equal(f.writes.length, 0);
});

test('setItem failure closes the scope and prevents automatic retry writes', (t) => {
  const f = storageFixture();
  let attempts = 0;
  const storage = {
    getItem: f.storage.getItem,
    setItem: () => { attempts++; throw new Error('storage full'); },
  };
  useStorage(t, storage);
  const view = createViewStores({ storageKey: 'view', storage });
  const release = view.enablePersistence();
  t.mock.timers.tick(500);
  assert.equal(attempts, 1);
  view.groupStack.set(['later']);
  view.persistViewState();
  t.mock.timers.tick(1000);
  assert.equal(attempts, 1);
  assert.equal(f.writes.length, 0);
  release();
});

test('isolated views with the same key keep injected storage and rejection authority separate', (t) => {
  const bad = storageFixture({ view: malformed });
  const good = storageFixture({ view: '{"viewLevel":"group","groupStack":["good"]}' });
  useStorage(t, bad.storage);
  const first = createViewStores({ storageKey: 'view', storage: bad.storage });
  const second = createViewStores({ storageKey: 'view', storage: good.storage });
  const releaseFirst = first.enablePersistence();
  const releaseSecond = second.enablePersistence();
  assert.deepEqual(get(second.groupStack), ['good']);
  t.mock.timers.tick(500);
  assert.deepEqual(bad.writes, []);
  assert.equal(bad.values.get('view'), malformed);
  assert.equal(good.writes.length, 1);
  releaseFirst();
  releaseSecond();
});

test('public writable values are validated before they can corrupt a valid record', (t) => {
  const bytes = '{"viewLevel":"data-graph","orchestrationId":null,"dataGraphId":null,"groupStack":[]}';
  const f = storageFixture({ view: bytes });
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  // JavaScript consumers can bypass TypeScript at the public writable boundary.
  Reflect.apply(view.currentDataGraphId.set, undefined, [Infinity]);
  view.persistViewState();
  assert.equal(f.values.get('view'), bytes);
  assert.equal(f.writes.length, 0);
});

test('missing record and disabled key preserve normal in-memory behavior', (t) => {
  const f = storageFixture();
  useStorage(t, f.storage);
  const missing = createViewStores({ storageKey: 'view', storage: f.storage });
  assert.equal(missing.restoreViewState(), false);
  missing.persistViewState();
  assert.equal(f.writes.length, 1);
  const reads = f.reads();
  const disabled = createViewStores({ storage: f.storage });
  disabled.groupStack.set(['local']);
  assert.equal(disabled.restoreViewState(), false);
  disabled.persistViewState();
  const release = disabled.enablePersistence();
  t.mock.timers.tick(500);
  release();
  assert.equal(f.reads(), reads);
  assert.equal(f.writes.length, 1);
});

test('storage becoming unavailable before a pending write closes only its active scope', (t) => {
  const f = storageFixture();
  let unavailable = false;
  const storage = {
    getItem: (key: string) => {
      if (unavailable) throw new Error('storage denied');
      return f.storage.getItem(key);
    },
    setItem: f.storage.setItem,
  };
  useStorage(t, storage);
  const view = createViewStores({ storageKey: 'view', storage });
  const release = view.enablePersistence();
  unavailable = true;
  t.mock.timers.tick(500);
  unavailable = false;
  view.groupStack.set(['later']);
  view.persistViewState();
  t.mock.timers.tick(1000);
  assert.equal(f.writes.length, 0);
  release();
});

test('re-enabling validates storage without replacing edits made while writeback was off', (t) => {
  const f = storageFixture({ view: '{"viewLevel":"group","groupStack":["stored"]}' });
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  const release = view.enablePersistence();
  release();
  view.groupStack.set(['local']);
  const releaseAgain = view.enablePersistence();
  assert.deepEqual(get(view.groupStack), ['local']);
  t.mock.timers.tick(500);
  assert.deepEqual(JSON.parse(f.writes[0].value).groupStack, ['local']);
  releaseAgain();
});

test('explicit restore before first enable preserves later local edits', (t) => {
  const f = storageFixture({ view: '{"viewLevel":"group","groupStack":["stored"]}' });
  useStorage(t, f.storage);
  const view = createViewStores({ storageKey: 'view', storage: f.storage });
  assert.equal(view.restoreViewState(), true);
  view.groupStack.set(['local']);
  const release = view.enablePersistence();
  assert.deepEqual(get(view.groupStack), ['local']);
  t.mock.timers.tick(500);
  assert.deepEqual(JSON.parse(f.writes[0].value).groupStack, ['local']);
  release();
});

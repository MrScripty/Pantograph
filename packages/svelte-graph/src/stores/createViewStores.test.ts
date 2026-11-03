import assert from 'node:assert/strict';
import test, { type TestContext } from 'node:test';
import { get } from 'svelte/store';
import { createViewStores } from './createViewStores.ts';

function useClock(t: TestContext) {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const view = createViewStores();
  view.setAnimationConfig({ duration: 100 });
  return view;
}

test('single group navigation keeps target until its configured completion', async (t) => {
  const view = useClock(t);
  const completion = view.tabIntoGroup('first');
  assert.equal(get(view.isAnimating), true);
  assert.equal(get(view.zoomTarget)?.nodeId, 'first');
  assert.deepEqual(get(view.groupStack), ['first']);
  t.mock.timers.tick(99);
  assert.equal(get(view.isAnimating), true);
  t.mock.timers.tick(1);
  await completion;
  assert.equal(get(view.isAnimating), false);
  assert.equal(get(view.zoomTarget), null);
});

test('superseded navigation settles without clearing the newest animation', async (t) => {
  const view = useClock(t);
  let firstSettled = false;
  const first = view.tabIntoGroup('first').then(() => { firstSettled = true; });
  t.mock.timers.tick(20);
  view.setAnimationConfig({ duration: 200 });
  const second = view.tabIntoGroup('second');
  await Promise.resolve();
  await Promise.resolve();
  assert.equal(firstSettled, true, 'supersession settles the previous caller');
  await first;
  t.mock.timers.tick(80);
  assert.equal(get(view.isAnimating), true);
  assert.equal(get(view.zoomTarget)?.nodeId, 'second');
  assert.deepEqual(get(view.groupStack), ['first', 'second']);
  t.mock.timers.tick(120);
  await second;
  assert.equal(get(view.isAnimating), false);
  assert.equal(get(view.zoomTarget), null);
});

test('reset cancels owned timer and settles the pending navigation', async (t) => {
  const view = useClock(t);
  const first = view.tabIntoGroup('first');
  view.resetViewState();
  await first;
  assert.deepEqual(get(view.groupStack), []);
  assert.equal(get(view.viewLevel), 'data-graph');
  assert.equal(get(view.isAnimating), false);
  assert.equal(get(view.zoomTarget), null);
  view.setAnimationConfig({ duration: 200 });
  const second = view.tabIntoGroup('after-reset');
  t.mock.timers.tick(100);
  assert.equal(get(view.isAnimating), true);
  assert.equal(get(view.zoomTarget)?.nodeId, 'after-reset');
  t.mock.timers.tick(100);
  await second;
});

test('orchestration navigation supersedes group animation without retaining its target', async (t) => {
  const view = useClock(t);
  const group = view.tabIntoGroup('group');
  const orchestration = view.zoomToOrchestration();
  await group;
  assert.equal(get(view.zoomTarget), null);
  assert.equal(get(view.isAnimating), true);
  assert.equal(get(view.viewLevel), 'orchestration');
  assert.deepEqual(get(view.groupStack), []);
  t.mock.timers.tick(100);
  await orchestration;
});

test('data-graph navigation supersedes orchestration and retains its own target', async (t) => {
  const view = useClock(t);
  const orchestration = view.zoomToOrchestration('old');
  const graph = view.zoomToDataGraph('new', 'graph-id');
  await orchestration;
  assert.equal(get(view.zoomTarget)?.nodeId, 'new');
  assert.equal(get(view.currentDataGraphId), 'graph-id');
  assert.equal(get(view.viewLevel), 'data-graph');
  t.mock.timers.tick(100);
  await graph;
});

test('tab out supersedes tab in and preserves stack and target semantics', async (t) => {
  const view = useClock(t);
  const enter = view.tabIntoGroup('group');
  const leave = view.tabOutOfGroup();
  await enter;
  assert.equal(get(view.viewLevel), 'data-graph');
  assert.deepEqual(get(view.groupStack), []);
  assert.equal(get(view.zoomTarget)?.nodeId, 'group');
  assert.equal(get(view.isAnimating), true);
  t.mock.timers.tick(100);
  await leave;
});

for (const level of ['data-graph', 'group'] as const) {
  test(`breadcrumb navigation cancels obsolete animation: ${level}`, async (t) => {
    const view = useClock(t);
    const first = view.tabIntoGroup('first');
    const second = view.tabIntoGroup('second');
    await first;
    await view.navigateToBreadcrumb({ id: 'first', name: 'First', level });
    await second;
    assert.equal(get(view.isAnimating), false);
    assert.equal(get(view.zoomTarget), null);
    assert.deepEqual(get(view.groupStack), level === 'group' ? ['first'] : []);
    t.mock.timers.tick(100);
    assert.equal(get(view.isAnimating), false);
  });
}

test('no-op navigation leaves the current animation and completion intact', async (t) => {
  const view = useClock(t);
  const current = view.zoomToOrchestration('current');
  await view.zoomToOrchestration('ignored');
  await view.tabOutOfGroup();
  assert.equal(get(view.zoomTarget)?.nodeId, 'current');
  assert.equal(get(view.isAnimating), true);
  t.mock.timers.tick(100);
  await current;
});

test('independent view stores do not supersede each other', async (t) => {
  const first = useClock(t);
  const second = createViewStores();
  second.setAnimationConfig({ duration: 200 });
  const a = first.tabIntoGroup('a');
  const b = second.tabIntoGroup('b');
  first.resetViewState();
  await a;
  t.mock.timers.tick(100);
  assert.equal(get(second.isAnimating), true);
  assert.equal(get(second.zoomTarget)?.nodeId, 'b');
  t.mock.timers.tick(100);
  await b;
});


function useStorage(t: TestContext) {
  const previous = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  const writes: { key: string; value: string }[] = [];
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: () => null,
      setItem: (key: string, value: string) => { writes.push({ key, value }); },
    },
  });
  t.after(() => {
    if (previous) Object.defineProperty(globalThis, 'localStorage', previous);
    else Reflect.deleteProperty(globalThis, 'localStorage');
  });
  t.mock.timers.enable({ apis: ['setTimeout'] });
  return writes;
}

test('duplicate persistence starts share one debounced write', (t) => {
  const writes = useStorage(t);
  const view = createViewStores({ storageKey: 'view' });
  const releaseA = view.enablePersistence();
  const releaseB = view.enablePersistence();
  view.groupStack.set(['latest']);
  t.mock.timers.tick(499);
  assert.equal(writes.length, 0);
  t.mock.timers.tick(1);
  assert.equal(writes.length, 1);
  assert.deepEqual(JSON.parse(writes[0].value).groupStack, ['latest']);
  releaseA();
  releaseB();
});

test('releasing one persistence owner preserves the remaining owner', (t) => {
  const writes = useStorage(t);
  const view = createViewStores({ storageKey: 'view' });
  const releaseA = view.enablePersistence();
  const releaseB = view.enablePersistence();
  releaseA();
  releaseA(); // Repeated release cannot consume another caller's ownership.
  view.viewLevel.set('orchestration');
  t.mock.timers.tick(500);
  assert.equal(writes.length, 1);
  releaseB();
  view.viewLevel.set('group');
  t.mock.timers.tick(500);
  assert.equal(writes.length, 1);
});

test('final persistence release cancels a pending write and supports a fresh scope', (t) => {
  const writes = useStorage(t);
  const view = createViewStores({ storageKey: 'view' });
  const release = view.enablePersistence();
  t.mock.timers.tick(499);
  release();
  t.mock.timers.tick(1);
  assert.equal(writes.length, 0);
  view.groupStack.set(['new-state']);
  t.mock.timers.tick(500);
  assert.equal(writes.length, 0);
  const releaseAgain = view.enablePersistence();
  t.mock.timers.tick(500);
  assert.equal(writes.length, 1);
  assert.deepEqual(JSON.parse(writes[0].value).groupStack, ['new-state']);
  releaseAgain();
});

test('independent persistence scopes retain their own storage keys and cleanup', (t) => {
  const writes = useStorage(t);
  const first = createViewStores({ storageKey: 'first' });
  const second = createViewStores({ storageKey: 'second' });
  const releaseFirst = first.enablePersistence();
  const releaseSecond = second.enablePersistence();
  releaseFirst();
  t.mock.timers.tick(500);
  assert.deepEqual(writes.map(({ key }) => key), ['second']);
  releaseSecond();
});

test('disabled persistence schedules no writes', (t) => {
  const writes = useStorage(t);
  const view = createViewStores();
  const release = view.enablePersistence();
  view.groupStack.set(['group']);
  t.mock.timers.tick(500);
  release();
  release();
  assert.deepEqual(writes, []);
});

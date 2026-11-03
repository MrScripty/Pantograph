import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { parse } from 'svelte/compiler';
import {
  createWorkflowGraphMount,
  registerWorkflowGraphWindowListeners,
  type WorkflowGraphWindowListenerTarget,
} from '../../packages/svelte-graph/src/workflowGraphWindowListeners.ts';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((accept, fail) => {
    resolve = accept;
    reject = fail;
  });
  return { promise, resolve, reject };
}

interface Listener {
  type: string;
  callback: EventListenerOrEventListenerObject;
  options?: boolean | AddEventListenerOptions | EventListenerOptions;
}

function mountFixture(path: URL) {
  const source = readFileSync(path, 'utf8');
  const script = parse(source, { modern: true }).instance;
  assert.ok(script);
  const statement = script.content.body.find(node =>
    node.type === 'ExpressionStatement' &&
    node.expression.type === 'CallExpression' &&
    node.expression.callee.type === 'Identifier' &&
    node.expression.callee.name === 'onMount',
  );
  assert.ok(statement && statement.type === 'ExpressionStatement');
  assert.equal(statement.expression.type, 'CallExpression');
  const expression = statement.expression;
  assert.ok(expression.type === 'CallExpression');
  const callback = expression.arguments[0];
  assert.ok(callback && callback.type === 'ArrowFunctionExpression');
  assert.ok('start' in callback && typeof callback.start === 'number');
  assert.ok('end' in callback && typeof callback.end === 'number');
  const added: Listener[] = [];
  const removed: Listener[] = [];
  const active = new Set<Listener>();
  const target: WorkflowGraphWindowListenerTarget = {
    addEventListener(type, listener, options) {
      const record = { type, callback: listener, options };
      added.push(record);
      active.add(record);
    },
    removeEventListener(type, listener, options) {
      for (const record of active) {
        if (record.type === type && record.callback === listener && record.options === options) {
          active.delete(record);
          removed.push(record);
          break;
        }
      }
    },
  };
  const requests: ReturnType<typeof deferred<string[]>>[] = [];
  const scopes: { ready: Promise<void> }[] = [];
  const applied: unknown[] = [];
  const failures: unknown[][] = [];
  let keyboardActions = 0;
  const service = {
    getNodeDefinitions: () => {
      const request = deferred<string[]>();
      requests.push(request);
      return request.promise;
    },
  };
  const store = { set: (value: unknown) => { applied.push(value); } };
  const mount = vm.runInNewContext(`(${source.slice(callback.start, callback.end)})`, {
    window: target,
    registerWorkflowGraphWindowListeners,
    createWorkflowGraphMount: (...args: Parameters<typeof createWorkflowGraphMount>) => {
      const scope = createWorkflowGraphMount(...args);
      scopes.push(scope);
      return scope;
    },
    workflowService: service,
    backend: service,
    nodeDefinitions: store,
    nodeDefsStore: store,
    handleWindowKeyDown: () => { keyboardActions++; },
    handleWorkflowPaletteDragEnd: () => {},
    handleWorkflowPaletteDragStart: () => {},
    console: { error: (...args: unknown[]) => { failures.push(args); } },
  }) as () => unknown;

  return {
    mount, active, added, removed, requests, scopes, applied, failures,
    keyboardActions: () => keyboardActions,
    keyDown: () => {
      for (const record of active) {
        if (record.type === 'keydown' && typeof record.callback === 'function') {
          record.callback(new Event('keydown'));
        }
      }
    },
  };
}

for (const [name, path] of [
  ['application', new URL('./WorkflowGraph.svelte', import.meta.url)],
  ['package', new URL('../../packages/svelte-graph/src/components/WorkflowGraph.svelte', import.meta.url)],
] as const) {
  test(`${name} graph mount returns synchronous cleanup before definitions resolve`, async () => {
    const f = mountFixture(path);
    const result = f.mount();
    try {
      assert.equal(typeof result, 'function');
      assert.equal(f.active.size, 6);
      await Promise.resolve();
      assert.equal(f.requests.length, 1);
      (result as () => void)();
      f.requests[0].resolve(['late']);
      await f.scopes[0].ready;
      assert.equal(f.active.size, 0);
      assert.deepEqual(f.removed, f.added);
      assert.deepEqual(f.applied, []);
    } finally {
      // Also settle the old asynchronous mount in the baseline regression run.
      f.requests[0]?.resolve(['late']);
      if (typeof result === 'function') result();
      else {
        const cleanup = await result;
        if (typeof cleanup === 'function') cleanup();
      }
    }
  });

  test(`${name} graph applies live definitions and removes each handler once`, async () => {
    const f = mountFixture(path);
    const stop = f.mount() as () => void;
    await Promise.resolve();
    f.requests[0].resolve(['live']);
    await f.scopes[0].ready;
    assert.deepEqual(f.applied, [['live']]);
    f.keyDown();
    assert.equal(f.keyboardActions(), 1);
    stop();
    stop();
    f.keyDown();
    assert.equal(f.keyboardActions(), 1);
    assert.equal(f.active.size, 0);
    assert.equal(f.removed.length, 6);
  });

  test(`${name} graph observes live definition failure and retains cleanup`, async () => {
    const f = mountFixture(path);
    const stop = f.mount() as () => void;
    await Promise.resolve();
    const failure = new Error('definitions unavailable');
    f.requests[0].reject(failure);
    await f.scopes[0].ready;
    assert.equal(f.failures.length, 1);
    assert.equal(f.failures[0][1], failure);
    assert.deepEqual(f.applied, []);
    stop();
    assert.equal(f.active.size, 0);
  });

  test(`${name} graph observes late rejection without an unmounted error callback`, async () => {
    const f = mountFixture(path);
    const stop = f.mount() as () => void;
    await Promise.resolve();
    stop();
    f.requests[0].reject(new Error('late failure'));
    await f.scopes[0].ready;
    assert.deepEqual(f.failures, []);
    assert.equal(f.active.size, 0);
  });

  test(`${name} graph destroyed immediately starts no definitions request`, async () => {
    const f = mountFixture(path);
    const stop = f.mount() as () => void;
    stop();
    await f.scopes[0].ready;
    assert.equal(f.requests.length, 0);
    assert.equal(f.active.size, 0);
  });

  test(`${name} graph remount owns six listeners and rejects the prior mount result`, async () => {
    const f = mountFixture(path);
    const stopFirst = f.mount() as () => void;
    await Promise.resolve();
    stopFirst();
    const stopSecond = f.mount() as () => void;
    await Promise.resolve();
    assert.equal(f.active.size, 6);
    f.requests[1].resolve(['current']);
    await f.scopes[1].ready;
    f.requests[0].resolve(['old']);
    await f.scopes[0].ready;
    assert.deepEqual(f.applied, [['current']]);
    f.keyDown();
    assert.equal(f.keyboardActions(), 1);
    stopSecond();
    assert.equal(f.active.size, 0);
    assert.equal(f.removed.length, 12);
  });
}

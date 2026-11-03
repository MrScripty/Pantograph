import test from 'node:test';
import assert from 'node:assert/strict';
import { createDeviceConfigLifecycle } from './deviceConfigLifecycle.ts';
import type { EmbeddingMemoryMode } from '../services/ConfigService';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((accept, fail) => {
    resolve = accept;
    reject = fail;
  });
  return { promise, resolve, reject };
}

function fixture() {
  const devices = deferred<void>();
  const mode = deferred<EmbeddingMemoryMode>();
  const applied: EmbeddingMemoryMode[] = [];
  const failures: unknown[] = [];
  let deviceReads = 0;
  let modeReads = 0;
  let refreshStarts = 0;
  let refreshStops = 0;
  const lifecycle = createDeviceConfigLifecycle({
    loadDevices: () => { deviceReads++; return devices.promise; },
    startRefresh: () => {
      refreshStarts++;
      return () => { refreshStops++; };
    },
    loadEmbeddingMemoryMode: () => { modeReads++; return mode.promise; },
    applyEmbeddingMemoryMode: value => { applied.push(value); },
    onFailure: error => { failures.push(error); },
  });
  return {
    lifecycle, devices, mode, applied, failures,
    counts: () => ({ deviceReads, modeReads, refreshStarts, refreshStops }),
  };
}

async function reachMode(f: ReturnType<typeof fixture>) {
  f.devices.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

test('device initialization is shared by duplicate starts and owns one refresh cleanup', async () => {
  const f = fixture();
  const first = f.lifecycle.start();
  assert.equal(f.lifecycle.start(), first);
  await reachMode(f);
  f.mode.resolve('gpu_parallel');
  await first;
  assert.deepEqual(f.applied, ['gpu_parallel']);
  assert.deepEqual(f.counts(), { deviceReads: 1, modeReads: 1, refreshStarts: 1, refreshStops: 0 });
  f.lifecycle.stop();
  f.lifecycle.stop();
  assert.equal(f.lifecycle.isActive(), false);
  assert.equal(f.counts().refreshStops, 1);
});

test('destroying before initialization begins starts no backend read', async () => {
  const f = fixture();
  const pending = f.lifecycle.start();
  f.lifecycle.stop();
  await pending;
  await f.lifecycle.start();
  assert.deepEqual(f.counts(), { deviceReads: 0, modeReads: 0, refreshStarts: 0, refreshStops: 0 });
});

test('destroying during the initial device read cannot create a late refresh', async () => {
  const f = fixture();
  const pending = f.lifecycle.start();
  await Promise.resolve();
  assert.equal(f.counts().deviceReads, 1);
  f.lifecycle.stop();
  f.devices.resolve();
  await pending;
  assert.deepEqual(f.counts(), { deviceReads: 1, modeReads: 0, refreshStarts: 0, refreshStops: 0 });
  assert.deepEqual(f.applied, []);
});

test('destroying during mode loading stops refresh and ignores the late mode', async () => {
  const f = fixture();
  const pending = f.lifecycle.start();
  await reachMode(f);
  assert.equal(f.counts().modeReads, 1);
  f.lifecycle.stop();
  f.mode.resolve('sequential');
  await pending;
  assert.deepEqual(f.applied, []);
  assert.equal(f.counts().refreshStops, 1);
});

test('live device-read failure is observed and skips later initialization', async () => {
  const f = fixture();
  const failure = new Error('devices unavailable');
  const pending = f.lifecycle.start();
  f.devices.reject(failure);
  await pending;
  assert.deepEqual(f.failures, [failure]);
  assert.equal(f.counts().modeReads, 0);
  assert.equal(f.counts().refreshStarts, 0);
});

test('live mode-read failure is observed and refresh remains owned until cleanup', async () => {
  const f = fixture();
  const pending = f.lifecycle.start();
  await reachMode(f);
  const failure = new Error('mode unavailable');
  f.mode.reject(failure);
  await pending;
  assert.deepEqual(f.failures, [failure]);
  assert.deepEqual(f.applied, []);
  f.lifecycle.stop();
  assert.equal(f.counts().refreshStops, 1);
});

test('late device rejection settles without an unmounted error callback', async () => {
  const f = fixture();
  const pending = f.lifecycle.start();
  await Promise.resolve();
  f.lifecycle.stop();
  f.devices.reject(new Error('late device failure'));
  await pending;
  assert.deepEqual(f.failures, []);
});

test('late mode rejection settles without an unmounted error callback', async () => {
  const f = fixture();
  const pending = f.lifecycle.start();
  await reachMode(f);
  f.lifecycle.stop();
  f.mode.reject(new Error('late mode failure'));
  await pending;
  assert.deepEqual(f.failures, []);
});

test('a stopped initialization cannot restart while a new mount is independent', async () => {
  const oldMount = fixture();
  oldMount.lifecycle.stop();
  await oldMount.lifecycle.start();
  const newMount = fixture();
  const pending = newMount.lifecycle.start();
  await reachMode(newMount);
  newMount.mode.resolve('cpu_parallel');
  await pending;
  assert.deepEqual(oldMount.applied, []);
  assert.deepEqual(newMount.applied, ['cpu_parallel']);
  assert.equal(newMount.counts().refreshStarts, 1);
  newMount.lifecycle.stop();
});

test('cleanup returned by refresh creation is released if creation destroys the mount', async () => {
  let stops = 0;
  let modeReads = 0;
  const lifecycle = createDeviceConfigLifecycle({
    loadDevices: async () => {},
    startRefresh: () => {
      lifecycle.stop();
      return () => { stops++; };
    },
    loadEmbeddingMemoryMode: async () => { modeReads++; return 'cpu_parallel'; },
    applyEmbeddingMemoryMode: () => { assert.fail('destroyed mount applied mode'); },
    onFailure: error => { assert.fail(String(error)); },
  });
  await lifecycle.start();
  assert.equal(stops, 1);
  assert.equal(modeReads, 0);
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { createScopedDeviceRefresh, type DeviceRefreshTimerApi } from './deviceConfigRefreshScope.ts';

function createTimerApi() {
  let nextHandle = 1;
  const active = new Set<number>();
  const callbacks = new Map<number, () => void>();

  const timerApi: DeviceRefreshTimerApi = {
    setInterval(callback) {
      const handle = nextHandle;
      nextHandle += 1;
      active.add(handle);
      callbacks.set(handle, callback);
      return handle;
    },
    clearInterval(handle) {
      active.delete(handle as number);
      callbacks.delete(handle as number);
    },
  };

  return {
    timerApi,
    active,
    tick(handle: number) {
      callbacks.get(handle)?.();
    },
  };
}

test('device refresh scope starts once and refreshes immediately', () => {
  const timers = createTimerApi();
  let refreshCount = 0;
  const scope = createScopedDeviceRefresh(() => {
    refreshCount += 1;
  }, timers.timerApi);

  scope.update(true);
  scope.update(true);

  assert.equal(refreshCount, 1);
  assert.equal(scope.isRunning(), true);
  assert.deepEqual([...timers.active], [1]);

  timers.tick(1);
  assert.equal(refreshCount, 2);
});

test('device refresh scope stops deterministically', () => {
  const timers = createTimerApi();
  let refreshCount = 0;
  const scope = createScopedDeviceRefresh(() => {
    refreshCount += 1;
  }, timers.timerApi);

  scope.update(true);
  scope.update(false);
  scope.stop();

  assert.equal(scope.isRunning(), false);
  assert.equal(timers.active.size, 0);

  timers.tick(1);
  assert.equal(refreshCount, 1);
});

test('device refresh scope can restart after inactive state', () => {
  const timers = createTimerApi();
  let refreshCount = 0;
  const scope = createScopedDeviceRefresh(() => {
    refreshCount += 1;
  }, timers.timerApi);

  scope.update(true);
  scope.update(false);
  scope.update(true);

  assert.equal(refreshCount, 2);
  assert.deepEqual([...timers.active], [2]);
});

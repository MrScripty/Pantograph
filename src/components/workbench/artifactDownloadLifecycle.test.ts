import assert from 'node:assert/strict';
import test from 'node:test';
import { createArtifactDownloadLifecycle, type PendingArtifactDownload } from './artifactDownloadLifecycle.ts';

function setup() {
  let pending: PendingArtifactDownload[] = [];
  const revoked: string[] = [];
  const timers = new Map<number, () => void>();
  const delays: number[] = [];
  let timerId = 0;
  const lifecycle = createArtifactDownloadLifecycle({
    publish: (value) => { pending = value; },
    revoke: (url) => { revoked.push(url); },
    schedule: (callback, delay) => { delays.push(delay); timers.set(++timerId, callback); return timerId; },
    cancel: (id) => { timers.delete(id); },
  });
  return { lifecycle, revoked, timers, delays, pending: () => pending };
}

test('concurrent downloads keep distinct filenames and URLs until their own delayed release', async () => {
  const h = setup();
  const first = h.lifecycle.enqueue('blob:first', 'first.png');
  const second = h.lifecycle.enqueue('blob:second', 'second.wav');
  const [a, b] = h.pending();
  assert.deepEqual([a.filename, b.filename], ['first.png', 'second.wav']);
  let clicks = 0;
  h.lifecycle.activate(b.id, () => { clicks += 1; });
  await second;
  assert.deepEqual(h.pending(), [a]);
  h.lifecycle.activate(a.id, () => { clicks += 1; });
  await first;
  h.lifecycle.activate(a.id, () => { clicks += 1; });
  assert.equal(clicks, 2);
  assert.deepEqual(h.pending(), []);
  assert.deepEqual(h.revoked, []);
  assert.deepEqual(h.delays, [30_000, 30_000]);
  h.timers.get(1)!();
  assert.deepEqual(h.revoked, ['blob:second']);
  h.timers.get(2)!();
  assert.deepEqual(h.revoked, ['blob:second', 'blob:first']);
  h.lifecycle.dispose();
  assert.equal(h.revoked.length, 2);
});

test('click failure preserves the original error and immediately releases its URL', async () => {
  const h = setup();
  const result = h.lifecycle.enqueue('blob:failed', 'failed.bin');
  const original = new Error('activation rejected');
  const rejection = assert.rejects(result, (error) => error === original);
  h.lifecycle.activate(h.pending()[0].id, () => { throw original; });
  await rejection;
  assert.deepEqual(h.revoked, ['blob:failed']);
  assert.equal(h.timers.size, 0);
  assert.deepEqual(h.pending(), []);
});

test('unmount settles pending work but preserves the clicked URL grace period', async () => {
  const h = setup();
  const active = h.lifecycle.enqueue('blob:active', 'active.bin');
  h.lifecycle.activate(h.pending()[0].id, () => {});
  await active;
  const pending = h.lifecycle.enqueue('blob:pending', 'pending.bin');
  const id = h.pending()[0].id;
  const rejection = assert.rejects(pending, /disposed before activation/);
  h.lifecycle.dispose();
  await rejection;
  h.lifecycle.activate(id, () => assert.fail('disposed download clicked'));
  h.lifecycle.dispose();
  assert.deepEqual(h.revoked, ['blob:pending']);
  assert.equal(h.timers.size, 1);
  h.timers.get(1)!();
  assert.deepEqual(h.revoked, ['blob:pending', 'blob:active']);
  assert.equal(h.timers.size, 0);
  assert.deepEqual(h.pending(), []);
});

test('a late URL handed to an unmounted owner is reclaimed and cannot hang', async () => {
  const h = setup();
  h.lifecycle.dispose();
  await assert.rejects(h.lifecycle.enqueue('blob:late', 'late.bin'), /disposed/);
  assert.deepEqual(h.revoked, ['blob:late']);
  assert.deepEqual(h.pending(), []);
});


test('successful click that unmounts still retains its URL for the grace period', async () => {
  const h = setup();
  const result = h.lifecycle.enqueue('blob:unmount', 'unmount.bin');
  h.lifecycle.activate(h.pending()[0].id, () => h.lifecycle.dispose());
  await result;
  assert.deepEqual(h.revoked, []);
  assert.deepEqual(h.delays, [30_000]);
  h.timers.get(1)!();
  assert.deepEqual(h.revoked, ['blob:unmount']);
  assert.equal(h.timers.size, 0);
});

test('failed click that unmounts preserves the error and reclaims immediately', async () => {
  const h = setup();
  const result = h.lifecycle.enqueue('blob:failed-unmount', 'failed.bin');
  const original = new Error('click failed after navigation');
  const rejection = assert.rejects(result, (error) => error === original);
  h.lifecycle.activate(h.pending()[0].id, () => { h.lifecycle.dispose(); throw original; });
  await rejection;
  assert.deepEqual(h.revoked, ['blob:failed-unmount']);
  assert.equal(h.timers.size, 0);
});

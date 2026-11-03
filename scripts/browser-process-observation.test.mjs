import assert from 'node:assert/strict';
import { execFileSync, spawn } from 'node:child_process';
import { once } from 'node:events';
import test from 'node:test';
import { MAX_BROWSER_STDERR_BYTES, observeBrowserProcess, waitForBrowserParentExit } from './browser-process-observation.mjs';

const facts = { checkpoint: 'launch_deadline', spawned: true, portError: 'ENOENT', stderrBytes: 0, stderrSaturated: false };
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));

function groupHasLiveMembers(pid) {
  return execFileSync('ps', ['-eo', 'pgid=,stat='], { encoding: 'utf8' }).split('\n').some(line => {
    const [group, state] = line.trim().split(/\s+/);
    return Number(group) === pid && state && !state.startsWith('Z') && !state.startsWith('X');
  });
}

function signalGroup(pid, signal) {
  try { process.kill(-pid, signal); }
  catch (error) { if (error.code !== 'ESRCH') throw error; }
}

async function reap(child, exited) {
  if (child.pid) signalGroup(child.pid, 'SIGKILL');
  await waitForBrowserParentExit(exited, Date.now() + 2000);
  assert.ok(child.exitCode !== null || child.signalCode !== null, 'test retains direct-child exit ownership');
}

test('observes the direct child exit event after synchronous ps sees its terminated group', async () => {
  const child = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], { detached: true, stdio: 'ignore' });
  const exited = once(child, 'exit');
  try {
    await once(child, 'spawn');
    signalGroup(child.pid, 'SIGTERM');
    const pollDeadline = Date.now() + 200;
    while (groupHasLiveMembers(child.pid) && Date.now() < pollDeadline) { /* Deliberately reproduce the event-loop observation gap. */ }
    assert.equal(groupHasLiveMembers(child.pid), false);
    assert.equal(child.exitCode, null);
    assert.equal(child.signalCode, null);
    await waitForBrowserParentExit(exited, Date.now() + 2000);
    assert.equal(child.signalCode, 'SIGTERM');
    const observation = await observeBrowserProcess(child, { ...facts, checkpoint: 'cleanup_observation' });
    assert.equal(observation.parent_exit_observed, true);
    assert.equal(observation.signal, 'SIGTERM');
    assert.equal(observation.proc_state, 'unavailable');
    assert.equal(observation.group_matches, null);
    assert.equal(observation.executable_kind, 'unavailable');
  } finally {
    await reap(child, exited);
  }
});

test('parent observation consumes only the supplied remaining deadline', async () => {
  const unresolved = new Promise(() => {});
  const started = Date.now();
  const deadline = started + 40;
  await delay(25);
  const originalSetTimeout = globalThis.setTimeout;
  const scheduledDelays = [];
  globalThis.setTimeout = (callback, duration) => {
    scheduledDelays.push(duration);
    return originalSetTimeout(callback, duration);
  };
  try {
    const beforeWait = Date.now();
    const waiting = waitForBrowserParentExit(unresolved, deadline);
    if (beforeWait < deadline) {
      assert.equal(scheduledDelays.length, 1);
      assert.ok(scheduledDelays[0] <= deadline - beforeWait);
      assert.ok(scheduledDelays[0] >= Math.max(0, deadline - Date.now()));
    } else {
      assert.equal(scheduledDelays.length, 0);
    }
    await waiting;
    const scheduledBeforeExpiry = scheduledDelays.length;
    await waitForBrowserParentExit(unresolved, Date.now() - 1);
    await waitForBrowserParentExit(unresolved, NaN);
    assert.equal(scheduledDelays.length, scheduledBeforeExpiry);
  } finally {
    globalThis.setTimeout = originalSetTimeout;
  }
  assert.ok(Date.now() >= deadline);
  // Event-loop tolerance permits loaded runners without replacing the caller's
  // absolute budget with a second multi-second cleanup deadline.
  assert.ok(Date.now() - started < 1000);
});

test('resolved or rejected parent observations clear their deadline timers', async () => {
  const originalSetTimeout = globalThis.setTimeout;
  const originalClearTimeout = globalThis.clearTimeout;
  const created = [];
  const cleared = [];
  globalThis.setTimeout = (...args) => {
    const timer = originalSetTimeout(...args);
    created.push(timer);
    return timer;
  };
  globalThis.clearTimeout = timer => { cleared.push(timer); originalClearTimeout(timer); };
  try {
    await waitForBrowserParentExit(Promise.resolve(), Date.now() + 2000);
    await assert.rejects(waitForBrowserParentExit(Promise.reject(new Error('owned failure')), Date.now() + 2000), /owned failure/);
    assert.equal(created.length, 2);
    assert.deepEqual(cleared, created);
  } finally {
    globalThis.setTimeout = originalSetTimeout;
    globalThis.clearTimeout = originalClearTimeout;
  }
});

test('own-child proc parsing discards a comm containing spaces and a closing parenthesis', async () => {
  const privateTitle = 'private) title';
  const child = spawn(process.execPath, ['-e', `process.title = ${JSON.stringify(privateTitle)}; process.send('ready'); setInterval(() => {}, 1000)`], {
    detached: true, stdio: ['ignore', 'ignore', 'ignore', 'ipc'],
  });
  const exited = once(child, 'exit');
  try {
    await once(child, 'message');
    const observation = await observeBrowserProcess(child, facts);
    assert.equal(observation.parent_exit_observed, false);
    assert.equal(observation.exit_code, null);
    assert.equal(observation.signal, null);
    assert.ok(['R', 'S', 'D'].includes(observation.proc_state));
    assert.equal(observation.group_matches, true);
    assert.equal(observation.executable_kind, 'other');
    assert.equal(observation.port_read_error, 'ENOENT');
    assert.equal(observation.stderr_bytes, 0);
    const serialized = JSON.stringify(observation);
    for (const secret of [privateTitle, '/proc/', process.execPath, 'private']) assert.ok(!serialized.includes(secret));
  } finally {
    await reap(child, exited);
  }
});

test('own shell child is classified without exposing its executable path', async () => {
  const child = spawn('/bin/sh', ['-c', 'read line'], { detached: true, stdio: ['pipe', 'ignore', 'ignore'] });
  const exited = once(child, 'exit');
  try {
    await once(child, 'spawn');
    const observation = await observeBrowserProcess(child, facts);
    assert.equal(observation.executable_kind, 'shell');
    assert.equal(observation.group_matches, true);
    assert.ok(!JSON.stringify(observation).includes('/bin/'));
  } finally {
    child.stdin.destroy();
    await reap(child, exited);
  }
});

test('invalid observations contain only fixed vocabulary and bounded scalar fields', async () => {
  const privateValue = '/private/profile\n::error::payload https://private.invalid';
  const observation = await observeBrowserProcess({ pid: privateValue, exitCode: privateValue, signalCode: privateValue }, {
    checkpoint: privateValue, spawned: privateValue, portError: privateValue, stderrBytes: privateValue, stderrSaturated: privateValue,
  });
  assert.deepEqual(observation, {
    event: 'image_browser_process', checkpoint: 'unavailable', spawned: false,
    parent_exit_observed: false, exit_code: null, signal: null,
    proc_state: 'unavailable', group_matches: null, executable_kind: 'unavailable',
    port_read_error: 'unavailable', stderr_bytes: 'unavailable', stderr_saturated: false,
  });
  assert.ok(!JSON.stringify(observation).includes('private'));
  for (const invalid of [-1, NaN, Infinity, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
    const value = await observeBrowserProcess(null, { ...facts, stderrBytes: invalid });
    assert.equal(value.stderr_bytes, 'unavailable');
    assert.equal(value.parent_exit_observed, false);
  }
});

test('stderr accounting saturates at one MiB and preserves an existing saturation flag', async () => {
  assert.equal(MAX_BROWSER_STDERR_BYTES, 1_048_576);
  for (const [count, previouslySaturated, expectedCount, expectedSaturated] of [
    [0, false, 0, false],
    [MAX_BROWSER_STDERR_BYTES, false, MAX_BROWSER_STDERR_BYTES, false],
    [MAX_BROWSER_STDERR_BYTES + 1, false, MAX_BROWSER_STDERR_BYTES, true],
    [MAX_BROWSER_STDERR_BYTES, true, MAX_BROWSER_STDERR_BYTES, true],
  ]) {
    const observation = await observeBrowserProcess(null, { ...facts, stderrBytes: count, stderrSaturated: previouslySaturated });
    assert.equal(observation.stderr_bytes, expectedCount);
    assert.equal(observation.stderr_saturated, expectedSaturated);
  }
});

test('exit status and port errors publish only validated outcomes', async () => {
  for (const code of [0, 1, 255]) {
    const value = await observeBrowserProcess({ exitCode: code, signalCode: null }, { ...facts, portError: null });
    assert.equal(value.exit_code, code);
    assert.equal(value.parent_exit_observed, true);
    assert.equal(value.port_read_error, 'none');
  }
  for (const code of [-1, 256, NaN, 1.5]) {
    const value = await observeBrowserProcess({ exitCode: code, signalCode: 'private' }, facts);
    assert.equal(value.exit_code, null);
    assert.equal(value.signal, null);
    assert.equal(value.parent_exit_observed, false);
  }
  for (const portError of ['ENOENT', 'EACCES', 'EPERM', 'EIO']) {
    assert.equal((await observeBrowserProcess(null, { ...facts, portError })).port_read_error, portError);
  }
});

test('throwing observation getters cannot expose their exception payload', async () => {
  const throwing = Object.fromEntries(['pid', 'exitCode', 'signalCode', 'checkpoint', 'spawned', 'portError', 'stderrBytes', 'stderrSaturated']
    .map(key => [key, { get() { throw new Error('private payload'); } }]));
  const source = Object.defineProperties({}, throwing);
  const observation = await observeBrowserProcess(source, source);
  assert.equal(observation.proc_state, 'unavailable');
  assert.equal(observation.port_read_error, 'unavailable');
  assert.ok(!JSON.stringify(observation).includes('private'));
});

import assert from 'node:assert/strict';
import process from 'node:process';
import { execFileSync } from 'node:child_process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { checkDependencies, directory, inventory, json, lock, validateNative, verifyInventory } from './contracts.mjs';
import { inspectBundle, runComposition } from './composition.mjs';
import { command, loggedJson } from './process.mjs';

// Required real dependencies: absence fails setup instead of silently skipping qualification.
const prepared = process.env.DEMO_PREPARED;
const chrema = process.env.CHREMA_CHECKOUT;
assert.ok(prepared && chrema, 'set DEMO_PREPARED and CHREMA_CHECKOUT to exact prepared dependencies');
const root = mkdtempSync(join(tmpdir(), 'pantograph-composition-contracts-'));
const out = join(root, 'completed');
let completed;
test.after(() => rmSync(root, { recursive: true }));

test('exact public dependency contracts accept the selected clean checkout', () => {
  checkDependencies(chrema);
});

test('different Chrema commit is rejected before building or inference', () => {
  const clone = join(root, 'wrong-revision');
  execFileSync('git', ['clone', '--quiet', '--no-hardlinks', chrema, clone]);
  writeFileSync(join(clone, 'README.md'), 'Independent wrong-revision fixture.\n');
  execFileSync('git', ['-C', clone, 'add', 'README.md']);
  execFileSync('git', ['-C', clone, '-c', 'user.name=Contract Test', '-c', 'user.email=test@example.invalid', 'commit', '--quiet', '-m', 'test: distinct dependency revision']);
  assert.throws(() => checkDependencies(clone), /Chrema revision mismatch/);
});

test('dirty exact Chrema checkout is rejected at its cleanliness gate', () => {
  const clone = join(root, 'dirty-revision');
  execFileSync('git', ['clone', '--quiet', '--no-hardlinks', chrema, clone]);
  writeFileSync(join(clone, 'untracked-evidence.txt'), 'Unqualified source.\n');
  assert.throws(() => checkDependencies(clone), /Chrema checkout must be clean/);
});

test('actual CPU inference and separate retained-store inspection complete both Chrema components', async () => {
  completed = await runComposition({ prepared, out });
  assert.equal(completed.status, 'completed');
  assert.equal(completed.modelQualification, 'untrained-fixture');
  const journal = json(join(out, 'chrema-journal.json'));
  assert.equal(journal.run.metrics.completedComponents, 2);
  assert.equal(journal.run.events.filter(event => event.eventType.endsWith('completed')).length, 2);
  assert.equal(json(join(out, 'native/result.json')).model_acquisition_bytes, 0);
});

test('two fresh CLI inspectors preserve exact owner-store bytes and run/artifact links', async () => {
  assert.ok(completed, 'real composition precondition');
  const before = inventory(out);
  for (const index of [1, 2]) {
    const log = join(root, `fresh-inspect-${index}`);
    await command(process.execPath, [join(directory, 'inspect.mjs'), '--prepared', prepared, '--out', out], { log });
    const inspected = loggedJson(log);
    assert.equal(inspected.status, 'verified');
    assert.equal(inspected.pantographRunId, completed.pantographRunId);
    assert.equal(inspected.artifactId, completed.artifactId);
    assert.deepEqual(inspected.vector, completed.vector);
  }
  assert.deepEqual(inventory(out), before);
});

test('reusing an existing evidence root refuses to overwrite any original result', async () => {
  assert.ok(completed, 'real composition precondition');
  const before = inventory(out);
  await assert.rejects(runComposition({ prepared, out }), { code: 'EEXIST' });
  assert.deepEqual(inventory(out), before);
});

test('native deadline records failed Chrema assignment and retained logs; no false completion', async () => {
  const failed = join(root, 'deadline');
  await assert.rejects(runComposition({ prepared, out: failed, timeoutMs: 1 }), /child_deadline_exceeded/);
  const journal = json(join(failed, 'chrema-journal.json'));
  assert.equal(journal.run.status, 'failed');
  assert.equal(journal.run.failure.code, 'child_deadline_exceeded');
  assert.equal(journal.run.failure.details.timedOut, true);
  assert.equal(journal.run.assignments.length, 1);
  assert.equal(journal.run.assignments[0].stepKey, 'native-inference');
  assert.equal(journal.run.assignments[0].status, 'failed');
  assert.equal(json(join(failed, 'bundle.json')).status, 'failed');
  assert.ok(readFileSync(join(failed, 'native-inference.stderr')));
  await assert.rejects(inspectBundle({ prepared, out: failed }), /composition did not complete/);
});

test('child process nonzero exit retains structured failure and original diagnostic bytes', async () => {
  const log = join(root, 'exit-failure');
  await assert.rejects(command(process.execPath, ['-e', 'process.stderr.write("controlled-failure\\n"); process.exit(7)'], { log }), error => {
    assert.equal(error.message, 'child_execution_failed');
    assert.equal(error.details.code, 7);
    assert.equal(error.details.timedOut, false);
    return true;
  });
  assert.equal(readFileSync(`${log}.stderr`, 'utf8'), 'controlled-failure\n');
});

test('deadline escalates to kill and waits for a child that ignores termination', async () => {
  const log = join(root, 'ignores-termination');
  await assert.rejects(command(process.execPath, ['-e', 'process.on("SIGTERM",()=>{}); console.log(process.pid); setInterval(()=>{},1000)'], { log, timeoutMs: 1_000 }), error => {
    assert.equal(error.message, 'child_deadline_exceeded');
    assert.equal(error.details.signal, 'SIGKILL');
    return true;
  });
  const pid = Number(readFileSync(`${log}.stdout`, 'utf8').trim());
  assert.ok(pid > 0, 'child reached its termination-ignore handler');
  assert.throws(() => process.kill(pid, 0), { code: 'ESRCH' });
});

test('corrupted retained output is rejected by bundle checksums before native inspection', async () => {
  assert.ok(completed, 'real composition precondition');
  const corrupted = join(root, 'corrupted');
  cpSync(out, corrupted, { recursive: true });
  writeFileSync(join(corrupted, 'native/result.json'), '{}\n');
  await assert.rejects(inspectBundle({ prepared, out: corrupted }), /bundle checksum mismatch: native\/result.json/);
});

test('well-shaped crossed model or run provenance refuses completion at the expected contract', () => {
  assert.ok(completed, 'real composition precondition');
  const result = json(join(out, 'native/result.json'));
  const observation = json(join(out, 'native/pumas-model.json'));
  const inspection = json(join(out, 'retained-inspection.json'));
  validateNative(result, observation, inspection); // Establish every unrelated precondition.
  const wrongModel = JSON.parse(JSON.stringify(observation));
  wrongModel.load_target.target.model_ref.model_id = 'embedding/qualification/other';
  assert.throws(() => validateNative(result, wrongModel, inspection), error => error.code === 'ERR_ASSERTION' && error.actual === 'embedding/qualification/other' && error.expected === lock.model_id);
  const wrongRun = JSON.parse(JSON.stringify(inspection));
  wrongRun.run_id = 'run_another';
  assert.throws(() => validateNative(result, observation, wrongRun), error => error.code === 'ERR_ASSERTION' && error.actual === 'run_another' && error.expected === completed.pantographRunId);
});

test('missing retained stores fail before owner APIs can create replacement state', async () => {
  const missing = join(root, 'missing');
  mkdirSync(missing);
  const log = join(root, 'missing-inspection');
  await assert.rejects(command(join(prepared, 'bin', lock.native_example), [missing, '--inspect-retained'], { log }), /child_execution_failed/);
  assert.match(readFileSync(`${log}.stderr`, 'utf8'), /missing retained evidence: result.json/);
  assert.deepEqual(inventory(missing), {});
});

test('bundle path traversal is rejected independently of checksum validity', () => {
  assert.throws(() => verifyInventory(root, { '../escape': '0'.repeat(64) }), /invalid bundle path/);
});

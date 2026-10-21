import assert from 'node:assert/strict';
import test from 'node:test';
import { browserVersion, createBrowserDiagnostics, failureDiagnostic } from './browser-qualification-diagnostics.mjs';

test('hostile exception content never reaches diagnostics or workflow commands', () => {
  const privatePayload = 'secret-token\n::error::injected /private/profile https://private.invalid?token=secret';
  const error = new assert.AssertionError({ actual: privatePayload, expected: privatePayload, operator: 'strictEqual', message: privatePayload });
  error.stack = privatePayload;
  error.cause = { exceptionDetails: privatePayload };
  const lines = [];
  const diagnostics = createBrowserDiagnostics(line => lines.push(line));
  diagnostics.at('assertion', 'keyboard_modal');
  diagnostics.failure(error);
  assert.deepEqual(JSON.parse(lines[1]), {
    event: 'image_browser_failure', stage: 'assertion', checkpoint: 'keyboard_modal',
    name: 'AssertionError', code: 'ERR_ASSERTION', operator: 'strictEqual', message: 'Qualification assertion failed.', browser: 'unavailable',
  });
  assert.equal(lines.length, 3);
  assert.match(lines[2], /^::error title=Image browser qualification::assertion\/keyboard_modal:/);
  assert.ok(!lines.join('\n').includes(privatePayload));
  for (const fragment of ['secret', '/private', 'https:', 'exceptionDetails', 'injected']) assert.ok(!lines.join('\n').includes(fragment));
});

test('unknown fields and injected stage identifiers collapse to fixed vocabulary', () => {
  const hostile = '::error::payload\r\n%0A';
  assert.deepEqual(failureDiagnostic(hostile, hostile, { name: hostile, code: hostile, operator: hostile }), {
    event: 'image_browser_failure', stage: 'build', checkpoint: 'setup', name: 'Error', code: 'unavailable',
    operator: 'unavailable', message: 'Qualification operation failed.',
  });
  const diagnostics = createBrowserDiagnostics(() => {});
  assert.throws(() => diagnostics.at('start', hostile), /Unknown qualification checkpoint/);
});

test('primary and cleanup failures preserve independent stages and order', () => {
  const lines = [];
  const diagnostics = createBrowserDiagnostics(line => lines.push(line));
  diagnostics.at('start', 'chrome_launch');
  diagnostics.failure(Object.assign(new Error('private launch stderr'), { code: 'ENOENT' }));
  diagnostics.at('cleanup', 'process_group');
  diagnostics.failure(Object.assign(new Error('private process state'), { code: 'EPERM' }));
  const failures = lines.filter(line => line.startsWith('{')).map(line => JSON.parse(line)).filter(record => record.event === 'image_browser_failure');
  assert.deepEqual(failures.map(({ stage, checkpoint, code }) => [stage, checkpoint, code]), [
    ['start', 'chrome_launch', 'ENOENT'], ['cleanup', 'process_group', 'EPERM'],
  ]);
});

test('Chrome versions are bounded validated scalar data before publication', () => {
  for (const version of ['Chrome/151.0.1.2', 'HeadlessChrome/151.0.1.2', 'Google Chrome 151.0.1.2', 'Chromium 151.0.1.2 built on Debian 13.1']) {
    assert.equal(browserVersion(version + '\n'), version);
  }
  for (const value of [null, {}, 'Chrome/151.0.1.2\n::error::secret', 'secret'.repeat(100), 'Chrome/151.0.1.2 https://private.invalid']) {
    assert.equal(browserVersion(value), 'unavailable');
  }
  const lines = [];
  createBrowserDiagnostics(line => lines.push(line)).version('Chrome/151.0.1.2\nsecret');
  assert.deepEqual(JSON.parse(lines[0]), { event: 'image_browser_version', browser: 'unavailable' });
});

test('failure annotation carries the last validated browser version without raw probe output', () => {
  const lines = [];
  const diagnostics = createBrowserDiagnostics(line => lines.push(line));
  diagnostics.version('Chrome/151.0.1.2');
  diagnostics.version('private invalid version');
  diagnostics.at('assertion', 'png_jpeg');
  const failure = diagnostics.failure(new Error('private browser payload'));
  assert.equal(failure.browser, 'Chrome/151.0.1.2');
  assert.match(lines.at(-1), /browser=Chrome\/151\.0\.1\.2$/);
  assert.ok(!lines.join('\n').includes('private'));
});

test('exception getters are read once and throwing getters fail closed', () => {
  for (const [field, first] of [['name', 'AssertionError'], ['code', 'ENOENT'], ['operator', 'strictEqual']]) {
    let reads = 0;
    const error = { get [field]() { return ++reads === 1 ? first : 'private-payload'; } };
    assert.equal(failureDiagnostic('build', 'setup', error)[field], first);
    assert.equal(reads, 1);
    const throwing = { get [field]() { throw new Error('private-payload'); } };
    const record = failureDiagnostic('build', 'setup', throwing);
    assert.equal(record[field], field === 'name' ? 'Error' : 'unavailable');
    assert.ok(!JSON.stringify(record).includes('private-payload'));
  }
});

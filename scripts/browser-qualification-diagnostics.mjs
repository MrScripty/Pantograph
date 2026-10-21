// Publish only authored vocabulary: exception contents and browser payloads are private.
const checkpoints = new Map([
  ['build', ['setup', 'dependencies', 'fixture_build', 'assets']],
  ['start', ['server', 'chrome_launch', 'cdp_connect', 'browser_version', 'fixture_start']],
  ['assertion', ['png_jpeg', 'keyboard_modal', 'reference_lifecycle', 'malformed_artifacts', 'superseded_read', 'unmount']],
  ['cleanup', ['process_group', 'server', 'temporary_root']],
]);
const names = new Set(['Error', 'AssertionError', 'TypeError', 'RangeError', 'SyntaxError', 'AggregateError']);
const codes = new Set(['ERR_ASSERTION', 'ENOENT', 'EACCES', 'EPERM', 'ECONNREFUSED', 'ECONNRESET', 'ETIMEDOUT']);
const canonicalField = (error, field, allowed, fallback) => {
  try {
    const value = error?.[field];
    return allowed.has(value) ? value : fallback;
  } catch { return fallback; }
};
const operators = new Set(['==', '===', '!=', '!==', 'deepEqual', 'deepStrictEqual', 'strictEqual', 'notStrictEqual', 'ok', 'fail']);
const escapeCommand = value => value.replaceAll('%', '%25').replaceAll('\r', '%0D').replaceAll('\n', '%0A');

export function browserVersion(value) {
  if (typeof value !== 'string' || value.length > 128) return 'unavailable';
  const version = value.trim();
  return /^(?:Google Chrome|Chromium|Chrome|HeadlessChrome)[ /][0-9]{1,4}(?:\.[0-9]{1,6}){1,3}(?: built on Debian [0-9]+\.[0-9]+)?$/.test(version)
    ? version : 'unavailable';
}

export function failureDiagnostic(stage, checkpoint, error) {
  const known = checkpoints.get(stage)?.includes(checkpoint);
  const name = canonicalField(error, 'name', names, 'Error');
  const code = canonicalField(error, 'code', codes, 'unavailable');
  const operator = canonicalField(error, 'operator', operators, 'unavailable');
  return {
    event: 'image_browser_failure', stage: known ? stage : 'build',
    checkpoint: known ? checkpoint : 'setup', name, code, operator,
    message: name === 'AssertionError' ? 'Qualification assertion failed.' : 'Qualification operation failed.',
  };
}

export function createBrowserDiagnostics(write = line => console.log(line)) {
  let version = 'unavailable';
  let stage = 'build';
  let checkpoint = 'setup';
  return {
    at(nextStage, nextCheckpoint) {
      if (!checkpoints.get(nextStage)?.includes(nextCheckpoint)) throw new Error('Unknown qualification checkpoint');
      stage = nextStage;
      checkpoint = nextCheckpoint;
      write(JSON.stringify({ event: 'image_browser_stage', stage, checkpoint }));
    },
    version(value) {
      const observed = browserVersion(value);
      if (observed !== 'unavailable') version = observed;
      write(JSON.stringify({ event: 'image_browser_version', browser: observed }));
    },
    failure(error) {
      const record = { ...failureDiagnostic(stage, checkpoint, error), browser: version };
      write(JSON.stringify(record));
      write('::error title=Image browser qualification::' + escapeCommand(
        `${record.stage}/${record.checkpoint}: ${record.message} name=${record.name} code=${record.code} operator=${record.operator} browser=${record.browser}`,
      ));
      return record;
    },
  };
}

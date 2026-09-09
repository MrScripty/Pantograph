import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const wrapperSource = join(
  repositoryRoot,
  'scripts',
  'check-workflow-editor-image-generation-gui-smoke.sh',
);
const workflowId = 'fixture-workflow';
const workflowContent = 'fixture-workflow-content';

function writeExecutable(filePath, content) {
  writeFileSync(filePath, content);
  chmodSync(filePath, 0o755);
}

function createFixture(childStatus, { defaultSignal = false } = {}) {
  const fixtureRoot = mkdtempSync(join(tmpdir(), 'pantograph-gui-smoke-test-'));
  const repoRoot = join(fixtureRoot, 'repo');
  const commandBin = join(fixtureRoot, 'commands');
  const wrapperPath = join(
    repoRoot,
    'scripts',
    'check-workflow-editor-image-generation-gui-smoke.sh',
  );
  const workflowPath = join(
    repoRoot,
    '.pantograph',
    'workflows',
    `${workflowId}.json`,
  );
  const observationPath = join(fixtureRoot, 'observation.txt');
  const pythonExecutable = join(fixtureRoot, 'python');
  const readyPath = join(fixtureRoot, 'ready.txt');

  mkdirSync(join(repoRoot, 'scripts'), { recursive: true });
  mkdirSync(join(repoRoot, 'src-tauri'), { recursive: true });
  mkdirSync(dirname(workflowPath), { recursive: true });
  mkdirSync(join(repoRoot, 'node_modules', '.bin'), { recursive: true });
  mkdirSync(commandBin, { recursive: true });
  writeFileSync(join(repoRoot, 'Cargo.toml'), '[workspace]\n');
  writeFileSync(join(repoRoot, 'src-tauri', 'Cargo.toml'), '[package]\n');
  writeFileSync(workflowPath, workflowContent);
  copyFileSync(wrapperSource, wrapperPath);
  chmodSync(wrapperPath, 0o755);

  const fakeWdio = defaultSignal
    ? `#!/usr/bin/env bash
set -euo pipefail

smoke_root="$PANTOGRAPH_GUI_SMOKE_PROJECT_ROOT"
workflow="$smoke_root/.pantograph/workflows/${workflowId}.json"
if [[ ! -d "$smoke_root" || ! -f "$workflow" ]]; then
  exit 91
fi
if [[ "$(<"$workflow")" != "$PANTOGRAPH_EXPECTED_WORKFLOW_CONTENT" ]]; then
  exit 92
fi
printf '{"childPid":%s,"root":"%s","workflow":"%s","rootExistsWhenSignalReceived":true}' \\
  "$$" "$smoke_root" "$workflow" > "$PANTOGRAPH_SMOKE_OBSERVATION_FILE"
printf '%s' "$$" > "$PANTOGRAPH_SMOKE_READY_FILE"
exec /bin/sleep 60
`
    : `#!/usr/bin/env node
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const smokeRoot = process.env.PANTOGRAPH_GUI_SMOKE_PROJECT_ROOT;
const workflow = join(
  smokeRoot,
  '.pantograph',
  'workflows',
  process.env.PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID + '.json',
);
const observationPath = process.env.PANTOGRAPH_SMOKE_OBSERVATION_FILE;
const record = { childPid: process.pid, root: smokeRoot, workflow };

if (!existsSync(smokeRoot) || !existsSync(workflow)) {
  process.exit(91);
}
if (readFileSync(workflow, 'utf8') !== process.env.PANTOGRAPH_EXPECTED_WORKFLOW_CONTENT) {
  process.exit(92);
}

function finish(signal, status) {
  record.rootExistsWhenSignalReceived = existsSync(smokeRoot);
  record.signal = signal;
  writeFileSync(observationPath, JSON.stringify(record));
  process.exit(status);
}

if (process.env.PANTOGRAPH_FAKE_WDIO_WAIT_FOR_SIGNAL === '1') {
  writeFileSync(process.env.PANTOGRAPH_SMOKE_READY_FILE, String(process.pid));
  process.on('SIGINT', () => finish('SIGINT', 130));
  process.on('SIGTERM', () => finish('SIGTERM', 143));
  setInterval(() => {}, 1000);
} else {
  record.rootExistsWhenSignalReceived = existsSync(smokeRoot);
  writeFileSync(observationPath, JSON.stringify(record));
  process.exit(Number(process.env.PANTOGRAPH_FAKE_WDIO_STATUS));
}
`;
  writeExecutable(join(repoRoot, 'node_modules', '.bin', 'wdio'), fakeWdio);

  for (const commandName of ['tauri-driver', 'WebKitWebDriver']) {
    writeExecutable(join(commandBin, commandName), '#!/usr/bin/env bash\nexit 0\n');
  }
  writeExecutable(pythonExecutable, '#!/usr/bin/env bash\nexit 0\n');

  return {
    childStatus,
    fixtureRoot,
    observationPath,
    repoRoot,
    readyPath,
    wrapperPath,
    pythonExecutable,
  };
}

function runSmoke(childStatus) {
  const fixture = createFixture(childStatus);
  const environment = {
    ...process.env,
    DISPLAY: ':99',
    PATH: `${join(fixture.fixtureRoot, 'commands')}:${process.env.PATH ?? ''}`,
    PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID: 'fixture-artifact',
    PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID: 'fixture-model',
    PANTOGRAPH_EXPECTED_WORKFLOW_CONTENT: workflowContent,
    PANTOGRAPH_FAKE_WDIO_STATUS: String(childStatus),
    PANTOGRAPH_FAKE_WDIO_WAIT_FOR_SIGNAL: '0',
    PANTOGRAPH_GUI_SMOKE_PROJECT_ROOT: '',
    PANTOGRAPH_PYTHON_EXECUTABLE: fixture.pythonExecutable,
    PANTOGRAPH_SMOKE_OBSERVATION_FILE: fixture.observationPath,
    PANTOGRAPH_SMOKE_READY_FILE: fixture.readyPath,
    PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID: workflowId,
    TMPDIR: fixture.fixtureRoot,
  };
  delete environment.PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH;
  delete environment.PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH;

  try {
    const result = spawnSync('/bin/bash', [fixture.wrapperPath], {
      cwd: fixture.repoRoot,
      encoding: 'utf8',
      env: environment,
    });

    assert.equal(result.error, undefined, result.error?.message);
    assert.equal(result.signal, null, result.stderr);
    assert.equal(result.status, childStatus, result.stderr);

    const observation = JSON.parse(
      readFileSync(fixture.observationPath, 'utf8'),
    );
    assert.equal(dirname(observation.root), fixture.fixtureRoot);
    assert.match(basename(observation.root), /^pantograph-workflow-editor-image-smoke\./);
    assert.equal(
      observation.workflow,
      join(observation.root, '.pantograph', 'workflows', `${workflowId}.json`),
    );
    assert.equal(observation.rootExistsWhenSignalReceived, true);
    assert.equal(existsSync(observation.root), false);
    assert.equal(existsSync(fixture.fixtureRoot), true);
    assert.equal(existsSync(fixture.repoRoot), true);
  } finally {
    rmSync(fixture.fixtureRoot, { force: true, recursive: true });
  }
}

function processIsAlive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    if (error.code === 'ESRCH') {
      return false;
    }
    throw error;
  }
}

async function runSmokeWithSignal(signal, { defaultSignal = false } = {}) {
  const fixture = createFixture(0, { defaultSignal });
  const environment = {
    ...process.env,
    DISPLAY: ':99',
    PATH: `${join(fixture.fixtureRoot, 'commands')}:${process.env.PATH ?? ''}`,
    PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID: 'fixture-artifact',
    PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID: 'fixture-model',
    PANTOGRAPH_EXPECTED_WORKFLOW_CONTENT: workflowContent,
    PANTOGRAPH_FAKE_WDIO_STATUS: '0',
    PANTOGRAPH_FAKE_WDIO_WAIT_FOR_SIGNAL: '1',
    PANTOGRAPH_GUI_SMOKE_PROJECT_ROOT: '',
    PANTOGRAPH_PYTHON_EXECUTABLE: fixture.pythonExecutable,
    PANTOGRAPH_SMOKE_OBSERVATION_FILE: fixture.observationPath,
    PANTOGRAPH_SMOKE_READY_FILE: fixture.readyPath,
    PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID: workflowId,
    TMPDIR: fixture.fixtureRoot,
  };
  delete environment.PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH;
  delete environment.PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH;

  try {
    const result = await new Promise((resolveResult, rejectResult) => {
      const wrapper = spawn('/bin/bash', [fixture.wrapperPath], {
        cwd: fixture.repoRoot,
        env: environment,
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      let stderr = '';
      let fakeChildPid;
      let sentSignal = false;
      let timedOut = false;
      const readyPoll = setInterval(() => {
        if (sentSignal || !existsSync(fixture.readyPath)) {
          return;
        }
        fakeChildPid = Number(readFileSync(fixture.readyPath, 'utf8'));
        sentSignal = true;
        process.kill(wrapper.pid, signal);
      }, 5);
      const timeout = setTimeout(() => {
        timedOut = true;
        if (fakeChildPid && processIsAlive(fakeChildPid)) {
          process.kill(fakeChildPid, 'SIGKILL');
        }
        wrapper.kill('SIGKILL');
      }, 5000);
      wrapper.stderr.on('data', (chunk) => {
        stderr += chunk;
      });
      wrapper.once('error', rejectResult);
      wrapper.once('close', (status, terminatingSignal) => {
        clearInterval(readyPoll);
        clearTimeout(timeout);
        resolveResult({ fakeChildPid, status, stderr, terminatingSignal, timedOut });
      });
    });

    assert.equal(result.timedOut, false, result.stderr);
    assert.equal(result.terminatingSignal, null, result.stderr);
    assert.equal(result.status, signal === 'SIGINT' ? 130 : 143, result.stderr);
    assert.equal(result.fakeChildPid > 0, true);
    const observation = JSON.parse(readFileSync(fixture.observationPath, 'utf8'));
    assert.equal(observation.signal, defaultSignal ? undefined : signal);
    assert.equal(observation.rootExistsWhenSignalReceived, true);
    assert.equal(observation.childPid, result.fakeChildPid);
    assert.equal(existsSync(observation.root), false);
    assert.equal(processIsAlive(result.fakeChildPid), false);
  } finally {
    rmSync(fixture.fixtureRoot, { force: true, recursive: true });
  }
}

test('GUI smoke wrapper cleans its isolated project after success', () => {
  runSmoke(0);
});

test('GUI smoke wrapper cleans its isolated project after child failure', () => {
  runSmoke(17);
});

test('GUI smoke wrapper forwards INT, reaps the child, and cleans up', async () => {
  await runSmokeWithSignal('SIGINT');
});

test('GUI smoke wrapper forwards TERM, reaps the child, and cleans up', async () => {
  await runSmokeWithSignal('SIGTERM');
});

test('GUI smoke wrapper preserves default INT handling during startup', async () => {
  await runSmokeWithSignal('SIGINT', { defaultSignal: true });
});

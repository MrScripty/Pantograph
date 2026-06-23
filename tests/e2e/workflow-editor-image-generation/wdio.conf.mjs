import { spawn, spawnSync } from 'node:child_process';
import { chmodSync, existsSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const configDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(configDir, '../../..');
const appBinary = path.join(repoRoot, 'target', 'debug', 'pantograph');
const smokeProjectRoot = process.env.PANTOGRAPH_GUI_SMOKE_PROJECT_ROOT;
const appLauncher = smokeProjectRoot
  ? path.join(smokeProjectRoot, '.pantograph', 'workflow-editor-image-smoke-launcher.sh')
  : path.join(repoRoot, '.missing-workflow-editor-image-smoke-launcher');

let tauriDriverProcess;
let closing = false;

function shellQuote(value) {
  return `'${value.replaceAll("'", "'\\''")}'`;
}

function closeTauriDriver() {
  closing = true;

  if (tauriDriverProcess && !tauriDriverProcess.killed) {
    tauriDriverProcess.kill();
  }

  tauriDriverProcess = undefined;
}

export const config = {
  host: '127.0.0.1',
  port: 4444,
  path: '/',
  specs: [path.join(configDir, 'workflow-editor-image-generation.e2e.mjs')],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      'tauri:options': {
        application: appLauncher,
      },
    },
  ],
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 240000,
  },
  onPrepare: () => {
    const build = spawnSync(
      'npm',
      ['run', 'build:desktop', '--', '--debug', '--no-bundle', '--features', 'backend-pytorch'],
      {
        cwd: repoRoot,
        stdio: 'inherit',
        shell: false,
      },
    );

    if (build.status !== 0) {
      throw new Error(`Tauri debug build failed with status ${build.status ?? 'unknown'}`);
    }

    if (!existsSync(appBinary)) {
      throw new Error(`Tauri debug build did not produce expected application binary: ${appBinary}`);
    }

    if (!smokeProjectRoot) {
      throw new Error('PANTOGRAPH_GUI_SMOKE_PROJECT_ROOT is required for isolated GUI smoke runs');
    }

    if (!existsSync(smokeProjectRoot)) {
      throw new Error(`PANTOGRAPH_GUI_SMOKE_PROJECT_ROOT does not exist: ${smokeProjectRoot}`);
    }

    const launcher = `#!/usr/bin/env bash
set -euo pipefail
export PANTOGRAPH_PROJECT_ROOT=${shellQuote(smokeProjectRoot)}
exec ${shellQuote(appBinary)} "$@"
`;
    writeFileSync(appLauncher, launcher, { mode: 0o700 });
    chmodSync(appLauncher, 0o700);
  },
  beforeSession: () => {
    tauriDriverProcess = spawn('tauri-driver', [], {
      cwd: repoRoot,
      stdio: ['ignore', process.stdout, process.stderr],
    });

    tauriDriverProcess.on('error', (error) => {
      console.error('tauri-driver failed to start:', error);
      process.exit(1);
    });

    tauriDriverProcess.on('exit', (code) => {
      if (!closing) {
        console.error('tauri-driver exited before the WebDriver session closed:', code);
        process.exit(1);
      }
    });
  },
  afterSession: closeTauriDriver,
  onComplete: closeTauriDriver,
};

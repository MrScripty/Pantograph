import { spawn, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const configDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(configDir, '../../..');
const appBinary = path.join(repoRoot, 'target', 'debug', 'pantograph');

let tauriDriverProcess;
let closing = false;

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
        application: appBinary,
      },
    },
  ],
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 120000,
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

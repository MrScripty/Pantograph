import { spawn, spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const directory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(directory, '../../..');
const project = process.env.PANTOGRAPH_GUI_SMOKE_PROJECT_ROOT;
const evidence = process.env.PANTOGRAPH_NATIVE_CPU_EVIDENCE_DIR;
const binary = path.resolve(process.env.CARGO_TARGET_DIR || path.join(root, 'target'), 'debug/pantograph');
const launcher = project && path.join(project, '.pantograph/native-cpu-launcher.sh');
let driver;
let closing = false;
const quote = (value) => `'${value.replaceAll("'", "'\\''")}'`;
function closeDriver() {
  closing = true;
  driver?.kill();
  driver = undefined;
}

export const config = {
  host: '127.0.0.1', port: 4444, path: '/',
  specs: [path.join(directory, 'native-desktop-cpu.e2e.mjs')], maxInstances: 1,
  capabilities: [{ maxInstances: 1, 'tauri:options': { application: launcher } }],
  reporters: ['spec'], framework: 'mocha', mochaOpts: { ui: 'bdd', timeout: 240000 },
  onPrepare: () => {
    if (!project || !evidence || !existsSync(project)) throw new Error('Isolated fixture/evidence roots are required');
    mkdirSync(evidence, { recursive: true });
    if (process.env.ORT_SKIP_DOWNLOAD !== '1') throw new Error('ORT_SKIP_DOWNLOAD=1 is required');
    // Packaged Tauri builds add custom-protocol; include it in the effective audit.
    const scope = ['-p', 'pantograph', '--no-default-features', '--features', 'backend-candle,tauri/custom-protocol'];
    const audit = spawnSync('cargo', ['tree', '--locked', '--target', 'all', '--edges', 'normal,build,dev', '--prefix', 'none', '--format', '{p}|{f}', ...scope], { cwd: root, encoding: 'utf8' });
    writeFileSync(path.join(evidence, 'desktop-features.txt'), audit.stdout || '');
    if (audit.status !== 0) throw new Error(`Complete feature audit failed: ${audit.stderr}`);
    const dynamicOwners = new Set();
    for (const line of audit.stdout.split('\n')) {
      if (!/^(ort|ort-sys) /.test(line)) continue;
      const [identity, features] = line.split('|');
      const enabled = new Set(features.replace(/ \(\*\)$/, '').split(','));
      if (['download-binaries', 'fetch-models', 'copy-dylibs', 'tls-native'].some((feature) => enabled.has(feature))) throw new Error(`Forbidden ONNX feature: ${identity}`);
      if (!enabled.has(identity.startsWith('ort-sys ') ? 'disable-linking' : 'load-dynamic')) throw new Error(`Dynamic ONNX configuration missing: ${identity}`);
      dynamicOwners.add(identity.split(' ')[0]);
    }
    if (!dynamicOwners.has('ort') || !dynamicOwners.has('ort-sys')) throw new Error('Complete dynamic ONNX owner graph was not observed');
    if (!audit.stdout.includes('26a84e323cae566a46a8f76bef48fa1010aed48b') || audit.stdout.includes('2243a2b')) throw new Error('Unexpected Pumas source');
    const build = spawnSync('npm', ['run', 'build:desktop', '--', '--ci', '--debug', '--no-bundle', '--features', 'backend-candle', '--', '--no-default-features', '--locked'], { cwd: root, stdio: 'inherit', shell: false });
    if (build.status !== 0 || !existsSync(binary)) throw new Error(`Actual desktop build failed: ${build.status}`);
    writeFileSync(launcher, `#!/usr/bin/env bash\nset -euo pipefail\nexport PANTOGRAPH_PROJECT_ROOT=${quote(project)}\nexec ${quote(binary)} "$@"\n`, { mode: 0o700 });
    chmodSync(launcher, 0o700);
  },
  beforeSession: () => {
    driver = spawn('tauri-driver', [], { cwd: root, stdio: ['ignore', process.stdout, process.stderr] });
    driver.on('error', (error) => { throw error; });
    driver.on('exit', (code) => { if (!closing) throw new Error(`Native driver exited early: ${code}`); });
  },
  afterTest: async function (_test, _context, { passed }) {
    if (evidence && globalThis.browser) {
      const events = await browser.execute(() => window.__nativeCpuValidationEvents || []);
      writeFileSync(path.join(evidence, 'native-validation-events.json'), JSON.stringify(events, null, 2));
      const active = events.at(-1);
      if (active) {
        const projection = await browser.executeAsync((request, done) => {
          window.__TAURI_INTERNALS__.invoke('current_graph_validation_projection', { request })
            .then((response) => done({ response })).catch((error) => done({ error: String(error) }));
        }, { graph_session_id: active.graph_session_id, graph_revision: active.graph_revision });
        writeFileSync(path.join(evidence, 'native-owner-validation.json'), JSON.stringify(projection, null, 2));
      }
    }
    if (!passed && evidence && globalThis.browser) {
      await browser.saveScreenshot(path.join(evidence, 'native-failure.png'));
      writeFileSync(path.join(evidence, 'native-failure.html'), await browser.getPageSource());
    }
  },
  afterSession: closeDriver, onComplete: closeDriver,
};

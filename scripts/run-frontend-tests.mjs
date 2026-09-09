import { spawnSync } from 'node:child_process';
import { globSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const cwd = fileURLToPath(new URL('../', import.meta.url));
// Build output lives outside these roots; runtime-authored UI lives in src/generated.
// Node's test CLI excludes node_modules, but cannot exclude that generated tree.
const files = globSync('{src,packages}/**/*.test.ts', {
  cwd,
  exclude: ['src/generated/**', '**/node_modules/**'],
}).sort();

if (files.length === 0) {
  throw new Error('No frontend .test.ts files found under src/ or packages/');
}

const result = spawnSync(process.execPath, [
  '--experimental-strip-types', '--test', ...process.argv.slice(2), ...files,
], { cwd, stdio: 'inherit' });

if (result.error) throw result.error;
if (result.signal) process.kill(process.pid, result.signal);
else process.exitCode = result.status;

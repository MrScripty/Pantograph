import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import test from 'node:test';

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'pantograph frontend discovery '));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'scripts'));
  copyFileSync(new URL('./run-frontend-tests.mjs', import.meta.url), join(root, 'scripts/run-frontend-tests.mjs'));
  const { scripts } = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
  writeFileSync(join(root, 'package.json'), JSON.stringify({ type: 'module', scripts: { 'test:frontend': scripts['test:frontend'] } }));
  return {
    write(path, content) {
      const file = join(root, path);
      mkdirSync(dirname(file), { recursive: true });
      writeFileSync(file, content, { flag: 'wx' });
    },
    run() {
      const env = { ...process.env };
      // The fixture starts an independent test runner, not a child test file.
      delete env.NODE_TEST_CONTEXT;
      return spawnSync('npm', ['run', 'test:frontend', '--', '--test-reporter=tap'], {
        cwd: root, env, encoding: 'utf8', timeout: 30_000,
      });
    },
  };
}

test('canonical command discovers nested tests in both roots and excludes output and unrelated roots', (t) => {
  const f = fixture(t);
  for (const path of ['src/direct.test.ts', 'src/new/deep/regression.test.ts', 'packages/new/src/deep/regression.test.ts', 'src/build/domain.test.ts', 'packages/new/src/target/domain.test.ts']) {
    f.write(path, `import test from 'node:test'; test(${JSON.stringify(path)}, () => {});`);
  }
  for (const root of ['src', 'packages/new/src']) {
    f.write(`${root}/nested/node_modules/deep/excluded.test.ts`, "throw new Error('DEPENDENCY_TEST_EXECUTED');");
  }
  f.write('src/generated/deep/excluded.test.ts', "throw new Error('GENERATED_TEST_EXECUTED');");
  f.write('dist/output.test.ts', "throw new Error('BUILD_OUTPUT_EXECUTED');");
  f.write('target/output.test.ts', "throw new Error('BUILD_OUTPUT_EXECUTED');");
  f.write('tests/outside.test.ts', "throw new Error('OUTSIDE_ROOT_EXECUTED');");
  const result = f.run();
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stdout, /src\/new\/deep\/regression\.test\.ts/);
  assert.match(result.stdout, /packages\/new\/src\/deep\/regression\.test\.ts/);
  assert.match(result.stdout, /src\/build\/domain\.test\.ts/);
  assert.match(result.stdout, /packages\/new\/src\/target\/domain\.test\.ts/);
  assert.match(result.stdout, /# tests 5\b/);
});

test('canonical command preserves an intentional nested assertion failure', (t) => {
  const f = fixture(t);
  f.write('src/new/deep/failing.test.ts', "import test from 'node:test'; import assert from 'node:assert/strict'; test('intentional discovery failure', () => assert.fail('P02_ASSERTION_PROBE'));");
  const result = f.run();
  assert.equal(result.error, undefined);
  assert.equal(result.status, 1);
  assert.match(result.stdout, /P02_ASSERTION_PROBE/);
  assert.match(result.stdout, /# fail 1\b/);
});

test('empty discovery fails instead of reporting successful absence', (t) => {
  const result = fixture(t).run();
  assert.equal(result.error, undefined);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /No frontend \.test\.ts files found/);
});

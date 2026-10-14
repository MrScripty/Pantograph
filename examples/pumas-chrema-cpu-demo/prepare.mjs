import assert from 'node:assert/strict';
import console from 'node:console';
import process from 'node:process';
import { chmodSync, copyFileSync, cpSync, mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { absoluteDirectory, checkDependencies, git, inventory, lock, options, repository, save, sha256, sourceIdentity } from './contracts.mjs';
import { command, loggedJson } from './process.mjs';

const args = options(['chrema', 'out']);
const chrema = absoluteDirectory(args.chrema);
const out = absoluteDirectory(args.out);
checkDependencies(chrema);
mkdirSync(out); // A previous candidate is never overwritten.
const source = sourceIdentity();
try {
  await command(process.execPath, [join(chrema, 'scripts/build.mjs'), '--out', join(out, 'packages')], { log: join(out, 'chrema-build') });
  await command(process.execPath, [join(chrema, 'scripts/audit.mjs'), '--packages', join(out, 'packages')], { log: join(out, 'chrema-audit') });
  await command('cargo', ['build', '--locked', '-p', 'pantograph-embedded-runtime', '--no-default-features', '--features', lock.native_features, '--example', lock.native_example], {
    cwd: repository, log: join(out, 'native-build'), timeoutMs: 15 * 60_000,
  });
  await command('cargo', ['metadata', '--locked', '--no-deps', '--format-version', '1'], { cwd: repository, log: join(out, 'cargo-metadata') });
  const target = loggedJson(join(out, 'cargo-metadata')).target_directory;
  mkdirSync(join(out, 'bin'));
  copyFileSync(join(target, 'debug/examples', lock.native_example), join(out, 'bin', lock.native_example));
  chmodSync(join(out, 'bin', lock.native_example), 0o755);
  mkdirSync(join(out, 'fixtures'));
  cpSync(join(repository, 'crates/inference/tests/fixtures/candle_bert'), join(out, 'fixtures/candle_bert'), { recursive: true, dereference: true });
  assert.deepEqual(sourceIdentity(), source, 'native source changed during build');
  checkDependencies(chrema);
  save(join(out, 'prepared.json'), {
    contract_version: 1, compatibility: lock, pantograph_revision: git(repository, 'rev-parse', 'HEAD'),
    native_source_sha256: source, node_version: process.version,
    cargo_lock_sha256: sha256(readFileSync(join(repository, 'Cargo.lock'))),
    files: inventory(out),
  });
  console.log(JSON.stringify({ status: 'prepared', directory: out, compatibility: lock }));
} catch (error) {
  save(join(out, 'prepare-failure.json'), { status: 'failed', message: error.message, details: error.details ?? null });
  throw error;
}

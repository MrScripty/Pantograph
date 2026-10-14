import assert from 'node:assert/strict';
import process from 'node:process';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { lstatSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const directory = dirname(fileURLToPath(import.meta.url));
export const repository = resolve(directory, '../..');
export const lock = JSON.parse(readFileSync(join(directory, 'compatibility-lock.json')));
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
export const json = path => JSON.parse(readFileSync(path, 'utf8'));
export const save = (path, value) => writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' });
export const git = (root, ...args) => execFileSync('git', ['-C', root, ...args], { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }).trim();

export function absoluteDirectory(path) {
  if (!path || !isAbsolute(path)) throw new Error('absolute_directory_required');
  return resolve(path);
}

export function options(names) {
  const args = process.argv.slice(2);
  const result = {};
  for (let index = 0; index < args.length; index += 2) {
    const name = args[index]?.replace(/^--/, '');
    if (!args[index]?.startsWith('--') || !names.includes(name) || result[name] !== undefined || !args[index + 1]) {
      throw new Error('invalid_command_arguments');
    }
    result[name] = args[index + 1];
  }
  return result;
}

export function checkDependencies(chrema) {
  assert.equal(process.platform, 'linux', 'this process-group host supports Linux');
  assert.equal(lock.contract_version, 1);
  assert.equal(git(chrema, 'rev-parse', 'HEAD'), lock.chrema_revision, 'Chrema revision mismatch');
  assert.equal(git(chrema, 'status', '--porcelain', '--untracked-files=all'), '', 'Chrema checkout must be clean');
  const manifest = readFileSync(join(repository, 'Cargo.toml'), 'utf8');
  const cargoLock = readFileSync(join(repository, 'Cargo.lock'), 'utf8');
  assert.ok(manifest.includes(`rev = "${lock.pumas_revision}"`), 'Pumas manifest pin mismatch');
  assert.ok(cargoLock.includes(`?rev=${lock.pumas_revision}#${lock.pumas_revision}`), 'Pumas resolved pin mismatch');
  const candleSources = cargoLock.match(/source = "git\+https:\/\/github.com\/huggingface\/candle[^\n]+/g);
  assert.equal(candleSources?.length, 3);
  assert.ok(candleSources.every(source => source.endsWith(`#${lock.candle_revision}"`)), 'Candle resolved pin mismatch');
  assert.equal(git(repository, 'merge-base', lock.pantograph_base, 'HEAD'), lock.pantograph_base, 'Pantograph base is not an ancestor');
}

export function sourceIdentity() {
  const paths = git(repository, 'ls-files', '--cached', '--others', '--exclude-standard', '-z')
    .split('\0').filter(path => path && (path.startsWith('crates/') || ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'].includes(path)));
  return Object.fromEntries([...new Set(paths)].sort().map(path => [path, sha256(readFileSync(join(repository, path)))]));
}

export function inventory(root) {
  for (let parent = resolve(root); ; parent = dirname(parent)) {
    const stat = lstatSync(parent);
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('bundle_requires_regular_directories');
    if (parent === dirname(parent)) break;
  }
  const files = {};
  function visit(path) {
    for (const entry of readdirSync(path, { withFileTypes: true })) {
      const full = join(path, entry.name);
      if (entry.isDirectory()) visit(full);
      else if (entry.isFile() && !lstatSync(full).isSymbolicLink()) files[relative(root, full)] = sha256(readFileSync(full));
      else throw new Error('bundle_requires_regular_files');
    }
  }
  visit(root);
  return files;
}

export function verifyInventory(root, files) {
  assert.ok(files && typeof files === 'object' && !Array.isArray(files), 'invalid bundle inventory');
  for (const [path, digest] of Object.entries(files)) {
    assert.ok(path && !isAbsolute(path) && !path.split(/[\\/]/).includes('..'), 'invalid bundle path');
    assert.match(digest, /^[0-9a-f]{64}$/);
  }
  // Walk the complete selected tree. A regular final file beneath a symlinked
  // ancestor is not a private regular-file bundle, even when its hash matches.
  const actual = inventory(root);
  for (const [path, digest] of Object.entries(files)) {
    assert.equal(actual[path], digest, `bundle checksum mismatch: ${path}`);
  }
}

export function validateNative(result, observation, inspection) {
  assert.equal(result.qualification.status, 'passed', 'native qualification failed');
  assert.equal(result.shutdown_ok, true);
  assert.equal(result.model_acquisition_bytes, 0);
  assert.equal(result.model_weights_blake3, lock.weights_blake3);
  const output = result.qualification.output;
  assert.equal(output.model_id, lock.model_id);
  assert.equal(output.model_training, 'untrained');
  assert.equal(output.backend, 'candle');
  assert.equal(output.device, 'cpu_f32');
  const vectors = output.response.outputs.filter(port => port.node_id === 'vectors' && port.port_id === 'vector');
  assert.equal(vectors.length, 1);
  assert.equal(vectors[0].value.length, 8);
  assert.ok(vectors[0].value.every(Number.isFinite));
  assert.equal(observation.indexed_model.id, lock.model_id);
  assert.equal(observation.execution_descriptor.model_id, lock.model_id);
  assert.equal(observation.execution_descriptor.validation_state, 'valid');
  assert.equal(observation.package_facts.model_ref.model_id, lock.model_id);
  assert.equal(observation.package_facts.package_facts_contract_version, 3);
  assert.ok(observation.load_target.target, 'Pumas did not resolve a load target');
  assert.equal(observation.load_target.target.model_ref.model_id, lock.model_id);
  assert.equal(observation.load_target.target.model_ref.selected_artifact_id, 'main');
  assert.equal(observation.load_target.target.validation_state, 'valid');
  assert.equal(observation.load_target.target.load_path_kind, 'directory');
  assert.equal(inspection.contract_version, 1);
  assert.equal(inspection.run_id, output.response.workflow_run_id);
  assert.equal(inspection.run_list.runs.length, 1);
  assert.equal(inspection.run_list.runs[0].status, 'completed');
  assert.equal(inspection.run_list.runs[0].selected_model_id, lock.model_id);
  assert.equal(inspection.run_list.runs[0].selected_runtime_id, 'candle');
  assert.equal(inspection.inspection.run.workflow_run_id, inspection.run_id);
  assert.ok(inspection.inspection.run_graph, 'run graph was not retained');
  assert.equal(inspection.inspection.run_graph.workflow_run_id, inspection.run_id);
  assert.ok(inspection.inspection.node_statuses.some(node => node.node_id === 'infer' && node.status === 'completed' && node.model_id === lock.model_id && node.runtime_id === 'candle'));
  assert.ok(inspection.library_usage.assets.some(asset => asset.asset_id === `pumas://models/${lock.model_id}` && asset.last_workflow_run_id === inspection.run_id && asset.run_access_count === 1));
  const bodies = inspection.artifact_bodies.filter(body => body.descriptor.attribution.node_id === 'vectors' && body.descriptor.attribution.port_id === 'vector');
  assert.equal(bodies.length, 1, 'retained vector artifact missing');
  assert.deepEqual(bodies[0].value, vectors[0].value, 'retained body differs from scheduler response');
  assert.equal(bodies[0].read.complete, true);
  assert.equal(bodies[0].descriptor.content_hash, `blake3:${bodies[0].body_blake3}`);
  assert.equal(bodies[0].descriptor.attribution.workflow_run_id, inspection.run_id);
  return { pantographRunId: inspection.run_id, modelId: lock.model_id, vector: vectors[0].value, artifactId: bodies[0].descriptor.artifact_id };
}

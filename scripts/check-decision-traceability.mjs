import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { Parser } from 'commonmark';

const markdown = new Parser();

// Audited transition from the retired directory-template gate. This is not a
// caller override: every other prior revision must contain the selected map.
const LEGACY_BASE = '4938e405c7f656365eefdca492774ccae110c90d';
const PROJECT_MAP = 'scripts/decision-traceability-map.json';
const ADR_PATH = /^docs\/adr\/ADR-\d+[^/]*\.md$/;
const PROFILES = new Set(['boundary-readme', 'contract-readme', 'adr', 'runbook']);

function fail(kind, message) {
  throw new Error(`${kind}: ${message}`);
}

function git(...args) {
  const result = spawnSync('git', args, { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
  if (result.error || result.status !== 0) {
    fail('unsupported', `cannot read Git input (${args[0]}): ${result.error?.message || result.stderr.trim() || result.signal || result.status}`);
  }
  return result.stdout;
}

function repositoryPath(value, label) {
  if (typeof value !== 'string' || !value || value.includes('\\') || value.includes('\0') ||
      path.posix.isAbsolute(value) || path.posix.normalize(value) !== value ||
      value === '..' || value.startsWith('../')) {
    fail('invalid', `${label} must be an exact repository-relative file path`);
  }
  return value;
}

function revision(ref, label) {
  if (!ref) fail('unavailable', `${label} is required in range mode`);
  return git('rev-parse', '--verify', '--end-of-options', `${ref}^{commit}`).trim();
}

function snapshot(rev) {
  const files = new Map();
  const output = git('ls-tree', '-rtz', '--full-tree', rev);
  if (output && !output.endsWith('\0')) fail('invalid', 'malformed Git tree input');
  for (const record of output.split('\0').filter(Boolean)) {
    const match = /^(\d{6}) (\w+) ([0-9a-f]+)\t([\s\S]+)$/.exec(record);
    if (!match) fail('invalid', 'malformed Git tree entry');
    files.set(match[4], { mode: match[1], type: match[2], oid: match[3] });
  }
  files.set('.', { mode: '040000', type: 'tree', oid: rev });
  const cache = new Map();
  return {
    rev,
    files,
    read(file) {
      const entry = files.get(file);
      if (!entry) fail('unavailable', `${file} is missing at ${rev}`);
      if (entry.type !== 'blob' || !['100644', '100755'].includes(entry.mode)) {
        fail('unsupported', `${file} must be a regular tracked file at ${rev}`);
      }
      if (!cache.has(file)) cache.set(file, git('cat-file', 'blob', entry.oid));
      return cache.get(file);
    },
  };
}

function changedPaths(base, head) {
  // No rename inference: both the removed and added paths remain obligations.
  const output = git('diff', '--no-ext-diff', '--no-textconv', '--name-status', '-z', '--no-renames', base, head, '--');
  if (output && !output.endsWith('\0')) fail('invalid', 'malformed Git diff: missing NUL terminator');
  const fields = output ? output.slice(0, -1).split('\0') : [];
  if (fields.length % 2) fail('invalid', 'malformed Git diff: incomplete status/path pair');
  const changed = new Set();
  for (let i = 0; i < fields.length; i += 2) {
    if (!/^[AMDT]$/.test(fields[i]) || !fields[i + 1]) {
      fail('invalid', `malformed Git diff: unsupported status/path pair ${JSON.stringify(fields[i])}`);
    }
    if (changed.has(fields[i + 1])) fail('invalid', 'malformed Git diff: duplicate path');
    changed.add(fields[i + 1]);
  }
  return changed;
}

function readMap(state, mapPath) {
  let value;
  try { value = JSON.parse(state.read(mapPath)); }
  catch (error) {
    if (/^(unavailable|unsupported):/.test(error.message)) throw error;
    fail('invalid', `cannot parse ${mapPath} at ${state.rev}: ${error.message}`);
  }
  if (value?.version !== 1 || !Array.isArray(value.boundaries) || !value.boundaries.length ||
      Object.keys(value).some(key => !['version', 'boundaries'].includes(key))) {
    fail('invalid', `${mapPath} must declare version 1 and a nonempty boundaries array`);
  }
  const rows = new Map();
  const owners = new Map();
  for (const row of value.boundaries) {
    if (!row || typeof row.id !== 'string' || !row.id.trim() || rows.has(row.id) ||
        !PROFILES.has(row.profile) || typeof row.knowledge !== 'string' || !row.knowledge.trim() ||
        !Array.isArray(row.triggers) || !row.triggers.length ||
        Object.keys(row).some(key => !['id', 'profile', 'knowledge', 'artifact', 'triggers'].includes(key))) {
      fail('invalid', `${mapPath} contains an invalid or duplicate boundary`);
    }
    repositoryPath(row.artifact, `artifact for ${row.id}`);
    if (!row.artifact.endsWith('.md')) fail('invalid', `artifact for ${row.id} must be Markdown`);
    if (!state.read(row.artifact).trim()) fail('invalid', `empty canonical artifact ${row.artifact}`);
    for (const trigger of row.triggers) {
      repositoryPath(trigger, `trigger for ${row.id}`);
      if (trigger === row.artifact || owners.has(trigger)) fail('invalid', `contradictory trigger ownership: ${trigger}`);
      // Decision documents and the map are the entire admitted trigger surface.
      // Mixed implementation files would require semantic review, not path rules.
      if (!ADR_PATH.test(trigger) && trigger !== mapPath) fail('unsupported', `undeclared decision-source trigger: ${trigger}`);
      state.read(trigger);
      owners.set(trigger, row.id);
    }
    rows.set(row.id, { id: row.id, profile: row.profile, knowledge: row.knowledge, artifact: row.artifact, triggers: [...row.triggers].sort() });
  }
  if (owners.get(mapPath) === undefined) fail('invalid', `${mapPath} must map its own coverage changes to an owning guide`);
  return rows;
}

function localTarget(owner, reference, rootRelative = false) {
  if (/^[a-z][a-z\d+.-]*:/i.test(reference) || reference.startsWith('//')) return null;
  const file = reference.split(/[?#]/, 1)[0];
  if (!file) return null;
  let decoded;
  try { decoded = decodeURIComponent(file); }
  catch { fail('invalid', `malformed local reference in ${owner}: ${reference}`); }
  const target = decoded.startsWith('/') ? decoded.slice(1) :
    rootRelative && decoded.startsWith('docs/') ? decoded : path.posix.join(path.posix.dirname(owner), decoded);
  return repositoryPath(path.posix.normalize(target).replace(/\/$/, ''), `local reference in ${owner}`);
}

function documentReferences(state, file, allLocalLinks, changed, mappedArtifacts) {
  const text = state.read(file);
  const refs = [];
  // Use CommonMark's syntax owner: examples in fenced/indented blocks are not
  // live links, and balanced/escaped destinations and references stay intact.
  const walker = markdown.parse(text).walker();
  let event;
  while ((event = walker.next())) {
    if (!event.entering) continue;
    const node = event.node;
    if (node.type === 'link' || node.type === 'image') refs.push([node.destination, false]);
    if (node.type === 'code' && /ADR-\d+[^\n]*\.md(?:#[^\n]*)?$/.test(node.literal)) {
      refs.push([node.literal, true]);
    }
  }
  for (const [reference, rootRelative] of refs) {
    const target = localTarget(file, reference, rootRelative);
    if (!target || (!allLocalLinks && (!changed.has(target) || (!ADR_PATH.test(target) && !mappedArtifacts.has(target))))) continue;
    if (!state.files.has(target)) fail('unavailable', `broken local reference in ${file}: ${reference} -> ${target}`);
    if (ADR_PATH.test(target) && !state.read(target).trim()) fail('invalid', `empty ADR target in ${file}: ${target}`);
  }
}

function main() {
  if (process.argv.length !== 4 || process.argv[2] !== '--map') fail('unavailable', 'invoke with --map <repository-relative map>');
  const mapPath = repositoryPath(process.argv[3], 'map');
  const env = process.env;
  for (const retired of ['TRACEABILITY_SOURCE_ROOTS', 'TRACEABILITY_HOST_FACING_DIRS', 'TRACEABILITY_STRUCTURED_PRODUCER_DIRS', 'ADR_DIR']) {
    if (env[retired] !== undefined) fail('invalid', `${retired} is retired; use the reviewed decision-source map`);
  }
  if (env.TRACEABILITY_STAGED_ONLY !== undefined && env.TRACEABILITY_STAGED_ONLY !== '1') {
    fail('invalid', 'TRACEABILITY_STAGED_ONLY must be 1 when supplied');
  }
  const mode = env.TRACEABILITY_STAGED_ONLY === '1' ? 'staged' : env.TRACEABILITY_MODE;
  if (!['staged', 'range'].includes(mode)) fail('unavailable', 'select TRACEABILITY_STAGED_ONLY=1 or TRACEABILITY_MODE=range with explicit base/head refs');
  if (env.TRACEABILITY_STAGED_ONLY === '1' && env.TRACEABILITY_MODE && env.TRACEABILITY_MODE !== 'staged') {
    fail('invalid', 'contradictory traceability modes');
  }
  if (mode === 'staged' && (env.TRACEABILITY_BASE_REF || env.TRACEABILITY_HEAD_REF)) fail('invalid', 'staged mode cannot use range refs');
  const root = git('rev-parse', '--show-toplevel').trim();
  process.chdir(root);
  const baseRev = mode === 'staged' ? revision('HEAD', 'HEAD') : revision(env.TRACEABILITY_BASE_REF, 'TRACEABILITY_BASE_REF');
  const headRev = mode === 'staged' ? git('write-tree').trim() : revision(env.TRACEABILITY_HEAD_REF, 'TRACEABILITY_HEAD_REF');
  console.log(`Decision traceability input: ${mode} ${baseRev} -> ${headRev}; map ${mapPath}`);
  const base = snapshot(baseRev);
  const head = snapshot(headRev);
  const changed = changedPaths(baseRev, headRev);
  const current = readMap(head, mapPath);
  let prior;
  if (!base.files.has(mapPath) && baseRev === LEGACY_BASE && mapPath === PROJECT_MAP) {
    // The audited legacy tree had no impact map. Validate current decisions at
    // the prior snapshot too; only the map's own addition is new at adoption.
    prior = new Map();
    for (const [id, row] of current) {
      if (row.triggers.includes(mapPath)) continue;
      base.read(row.artifact);
      for (const trigger of row.triggers) base.read(trigger);
      prior.set(id, row);
    }
    console.log(`Explicit legacy-map adoption from ${LEGACY_BASE}`);
  } else {
    prior = readMap(base, mapPath);
  }
  const canonicalMap = rows => JSON.stringify([...rows].sort(([a], [b]) => a.localeCompare(b)));
  const mapChanged = canonicalMap(prior) !== canonicalMap(current);
  const affected = new Set();
  for (const [id, row] of [...prior, ...current]) {
    if (row.triggers.some(trigger => trigger === mapPath ? mapChanged : changed.has(trigger)) ||
        JSON.stringify(prior.get(id)) !== JSON.stringify(current.get(id))) affected.add(id);
  }
  for (const file of changed) {
    if (ADR_PATH.test(file) && ![...prior.values(), ...current.values()].some(row => row.triggers.includes(file))) {
      fail('unavailable', `changed ADR has no declared impact mapping: ${file}`);
    }
  }
  for (const id of affected) {
    const old = prior.get(id);
    const next = current.get(id);
    for (const row of [old, next].filter(Boolean)) {
      // A moved owner may be removed only when its replacement exists in the
      // current map. A retained old owner must explain retirement/migration.
      const movedAndRemoved = row === old && next && old.artifact !== next.artifact && !head.files.has(old.artifact);
      if (!movedAndRemoved) head.read(row.artifact);
      if (!changed.has(row.artifact)) fail('unavailable', `boundary ${id} requires an update to ${row.artifact} (${row.knowledge}); unrelated documents cannot satisfy it`);
    }
  }
  const owners = new Set([...current.values()].map(row => row.artifact));
  const mappedArtifacts = new Set([...prior.values(), ...current.values()].map(row => row.artifact));
  for (const [file, entry] of head.files) {
    if (!file.endsWith('.md') || entry.type !== 'blob') continue;
    // Preserve readers of changed decisions and prior/current canonical guides.
    // Other references are checked in owners and changed documents; untouched
    // historical targets are not promoted into current documentation authority.
    documentReferences(head, file, owners.has(file) || changed.has(file), changed, mappedArtifacts);
  }
  console.log(`Decision traceability passed: ${changed.size} changed path(s), ${affected.size} mapped impact(s). Semantic contract review remains required.`);
}

try { main(); }
catch (error) {
  console.error(`Decision traceability failed: ${error.message}`);
  process.exitCode = 1;
}

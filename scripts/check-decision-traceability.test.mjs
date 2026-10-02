import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const checker = fileURLToPath(new URL('./check-decision-traceability.sh', import.meta.url));
const mapPath = 'scripts/decision-traceability-map.json';
const decision = 'docs/adr/ADR-001-example.md';
const guide = 'docs/consumer.md';
const realGit = spawnSync('which', ['git'], { encoding: 'utf8' }).stdout.trim();

function fixture(t, options = {}) {
  const root = mkdtempSync(path.join(tmpdir(), 'pantograph-traceability-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  function git(...args) {
    const result = spawnSync(realGit, args, {
      cwd: root, encoding: 'utf8',
      // Synthetic identities belong only to disposable test commits.
      env: { ...process.env, GIT_AUTHOR_NAME: 'Test Fixture', GIT_AUTHOR_EMAIL: 'fixture@example.invalid', GIT_COMMITTER_NAME: 'Test Fixture', GIT_COMMITTER_EMAIL: 'fixture@example.invalid' },
    });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout.trim();
  }
  function write(file, text) {
    mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
    writeFileSync(path.join(root, file), text);
  }
  function map(rows = []) {
    return {
      version: 1,
      boundaries: [
        { id: 'consumer', triggers: [decision], artifact: guide, profile: 'contract-readme', knowledge: 'Consumer lifecycle' },
        { id: 'gate', triggers: [mapPath], artifact: 'scripts/README.md', profile: 'boundary-readme', knowledge: 'Coverage of the gate' },
        ...rows,
      ],
    };
  }
  function writeMap(value = map()) { write(mapPath, `${JSON.stringify(value, null, 2)}\n`); }
  function commit() {
    git('add', '-A');
    const tree = git('write-tree');
    const previous = spawnSync(realGit, ['rev-parse', '--verify', 'HEAD'], { cwd: root, encoding: 'utf8' });
    const sha = git('commit-tree', tree, ...(previous.status === 0 ? ['-p', previous.stdout.trim()] : []), '-m', 'test: fixture snapshot');
    git('update-ref', 'HEAD', sha);
    return sha;
  }
  function run(env = {}) {
    const cleanEnv = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith('TRACEABILITY_') && key !== 'ADR_DIR'));
    const result = spawnSync('bash', [checker], { cwd: root, encoding: 'utf8', env: { ...cleanEnv, TRACEABILITY_STAGED_ONLY: '1', ...env } });
    return { status: result.status, output: result.stdout + result.stderr };
  }
  git('init', '-q');
  write('src/internal.rs', 'fn internal() {}\n');
  write(decision, '# Decision\n\nThe service owns the consumer lifecycle.\n');
  write(guide, '# Consumer\n\n[Decision](adr/ADR-001-example.md)\n');
  write('scripts/README.md', '# Gate\n\nCurrent impact coverage.\n');
  if (!options.withoutMap) writeMap();
  const base = commit();
  return { root, git, write, map, writeMap, commit, run, base };
}

function passed(result) { assert.equal(result.status, 0, result.output); assert.match(result.output, /Decision traceability passed:/); }
function failed(result, diagnostic) { assert.equal(result.status, 1, result.output); assert.match(result.output, diagnostic); assert.doesNotMatch(result.output, /Decision traceability passed:/); }

test('unchanged contracts allow code-only changes, nested paths, spaces and newlines without README churn', t => {
  const f = fixture(t);
  f.write('src/new owner/nested\nmodule.rs', 'fn implementation() {}\n');
  f.git('add', '-A');
  passed(f.run());
});

test('explicit valid input with no changes passes only after validating the map', t => {
  const f = fixture(t);
  passed(f.run());
  f.writeMap({ version: 1, boundaries: [] });
  f.git('add', mapPath);
  failed(f.run(), /invalid: .*nonempty boundaries/);
});

test('a changed decision requires its canonical guide; an unrelated README or ADR does not count', t => {
  const f = fixture(t);
  f.write(decision, '# Decision\n\nConsumer shutdown must be awaited.\n');
  f.write('README.md', '# Unrelated documentation\n');
  f.git('add', '-A');
  failed(f.run(), /unavailable: boundary consumer requires an update to docs\/consumer.md/);
  f.write(guide, '# Consumer\n\nAwait shutdown. [Decision](adr/ADR-001-example.md)\n');
  f.git('add', guide);
  passed(f.run());
});

test('staged mode reads guide and map from index, never unstaged repairs', t => {
  const f = fixture(t);
  f.write(decision, '# Changed decision\n');
  f.git('add', decision);
  f.write(guide, '# Unstaged guide repair\n');
  failed(f.run(), /boundary consumer requires an update/);
  f.git('add', guide);
  f.write(guide, '[Unstaged broken link](missing.md)\n');
  f.writeMap({ invalid: true });
  passed(f.run());
});

test('explicit range reads exact commits even when worktree and index differ', t => {
  const f = fixture(t);
  f.write('src/internal.rs', 'fn repaired() {}\n');
  const head = f.commit();
  f.writeMap({ invalid: true });
  f.git('add', mapPath);
  passed(f.run({ TRACEABILITY_STAGED_ONLY: undefined, TRACEABILITY_MODE: 'range', TRACEABILITY_BASE_REF: f.base, TRACEABILITY_HEAD_REF: head }));
});

test('missing modes, revisions, invalid refs and contradictory modes fail clearly', t => {
  const f = fixture(t);
  failed(f.run({ TRACEABILITY_STAGED_ONLY: undefined }), /unavailable: select TRACEABILITY/);
  failed(f.run({ TRACEABILITY_STAGED_ONLY: undefined, TRACEABILITY_MODE: 'range', TRACEABILITY_BASE_REF: f.base }), /unavailable: TRACEABILITY_HEAD_REF/);
  failed(f.run({ TRACEABILITY_STAGED_ONLY: undefined, TRACEABILITY_MODE: 'range', TRACEABILITY_BASE_REF: 'missing-base', TRACEABILITY_HEAD_REF: 'HEAD' }), /unsupported: cannot read Git input \(rev-parse\)/);
  failed(f.run({ TRACEABILITY_MODE: 'range' }), /invalid: contradictory traceability modes/);
  failed(f.run({ TRACEABILITY_BASE_REF: f.base }), /invalid: staged mode cannot use range refs/);
  failed(f.run({ TRACEABILITY_SOURCE_ROOTS: 'unrelated' }), /invalid: TRACEABILITY_SOURCE_ROOTS is retired/);
});

test('missing current or arbitrary prior map never silently succeeds', t => {
  const f = fixture(t, { withoutMap: true });
  failed(f.run(), /unavailable: scripts\/decision-traceability-map.json is missing/);
  f.writeMap();
  f.write('scripts/README.md', '# Gate adoption\n');
  f.git('add', '-A');
  failed(f.run(), new RegExp(`unavailable: scripts/decision-traceability-map.json is missing at ${f.base}`));
});

test('missing canonical artifact and broken local ADR or ordinary local links fail', t => {
  const f = fixture(t);
  f.git('rm', guide);
  failed(f.run(), /unavailable: docs\/consumer.md is missing/);
  f.write(guide, '# Consumer\n\n[Decision](adr/ADR-099-missing.md)\n');
  f.git('add', guide);
  failed(f.run(), /broken local reference in docs\/consumer.md: adr\/ADR-099-missing.md/);
  f.write(guide, '# Consumer\n\n[Instructions](missing.md)\n');
  f.git('add', guide);
  failed(f.run(), /broken local reference in docs\/consumer.md: missing.md/);
});

test('root-relative code-span ADR references are checked; external URLs are not fetched', t => {
  const f = fixture(t);
  f.write('docs/review.md', '# Review\n\n`docs/adr/ADR-099-missing.md`\n');
  f.git('add', '-A');
  failed(f.run(), /broken local reference in docs\/review.md: docs\/adr\/ADR-099-missing.md/);
  f.write('docs/review.md', '# Review\n\n[External](https://example.invalid/ADR-099-remote.md)\n');
  f.git('add', '-A');
  passed(f.run());
});

test('an unmapped added or changed ADR is unresolved, even with unrelated guide edits', t => {
  const f = fixture(t);
  f.write('docs/adr/ADR-002-new.md', '# New durable decision\n');
  f.write(guide, '# Updated consumer\n');
  f.git('add', '-A');
  failed(f.run(), /unavailable: changed ADR has no declared impact mapping: docs\/adr\/ADR-002-new.md/);
});

test('deleting a map row cannot hide the prior obligation', t => {
  const f = fixture(t);
  const next = f.map();
  next.boundaries = next.boundaries.filter(row => row.id !== 'consumer');
  f.writeMap(next);
  f.write('scripts/README.md', '# Removed consumer map row\n');
  f.git('add', '-A');
  failed(f.run(), /boundary consumer requires an update to docs\/consumer.md/);
});

test('renaming a mapped ADR requires its guide and cannot leave an unchanged reader dangling', t => {
  const f = fixture(t);
  f.write('docs/reader.md', '# Reader\n\n[Decision](adr/ADR-001-example.md)\n');
  f.commit();
  const moved = 'docs/adr/ADR-003-renamed.md';
  f.git('mv', decision, moved);
  const next = f.map();
  next.boundaries[0].triggers = [moved];
  f.writeMap(next);
  f.write('scripts/README.md', '# Gate\n\nDecision path migrated.\n');
  f.git('add', '-A');
  failed(f.run(), /boundary consumer requires an update/);
  f.write(guide, '# Consumer\n\n[Decision](adr/ADR-003-renamed.md)\n');
  f.git('add', guide);
  failed(f.run(), /broken local reference in docs\/reader.md/);
  f.write('docs/reader.md', '# Reader\n\n[Decision](adr/ADR-003-renamed.md)\n');
  f.git('add', '-A');
  passed(f.run());
});

test('moving the canonical guide cannot leave unchanged readers dangling', t => {
  const f = fixture(t);
  f.write('docs/reader.md', '# Reader\n\n[Consumer](consumer.md)\n');
  f.commit();
  f.git('mv', guide, 'docs/new consumer.md');
  const next = f.map();
  next.boundaries[0].artifact = 'docs/new consumer.md';
  f.writeMap(next);
  f.write('scripts/README.md', '# Gate\n\nConsumer guide moved.\n');
  f.git('add', '-A');
  failed(f.run(), /broken local reference in docs\/reader.md: consumer.md/);
  f.write('docs/reader.md', '# Reader\n\n[Consumer](<new consumer.md>)\n');
  f.git('add', 'docs/reader.md');
  passed(f.run());
});

test('contradictory trigger ownership, malformed maps and nonregular owners fail', t => {
  const f = fixture(t);
  const next = f.map();
  next.boundaries.push({ ...next.boundaries[0], id: 'duplicate' });
  f.writeMap(next);
  f.git('add', '-A');
  failed(f.run(), /invalid: contradictory trigger ownership/);
  f.write(mapPath, '{ invalid JSON');
  f.git('add', mapPath);
  failed(f.run(), /invalid: cannot parse/);
  f.writeMap();
  f.git('add', mapPath);
  f.git('update-index', '--cacheinfo', `120000,${f.git('rev-parse', `HEAD:${guide}`)},${guide}`);
  failed(f.run(), /unsupported: docs\/consumer.md must be a regular tracked file/);
});

for (const [name, body, expected] of [
  ['unreadable', 'echo "diff read denied" >&2; exit 42', /unsupported: cannot read Git input \(diff\): diff read denied/],
  ['unterminated', "printf 'M\\0src/internal.rs'", /invalid: malformed Git diff: missing NUL terminator/],
  ['malformed', "printf 'M\\0'", /invalid: malformed Git diff: incomplete status\/path pair/],
  ['unsupported status', "printf 'U\\0src/internal.rs\\0'", /invalid: malformed Git diff: unsupported status\/path pair/],
]) {
  test(`${name} Git diff fails instead of reporting no changed files`, t => {
    const f = fixture(t);
    const bin = path.join(f.root, 'test-bin');
    mkdirSync(bin);
    writeFileSync(path.join(bin, 'git'), `#!/usr/bin/env bash\nif [ "$1" = diff ]; then ${body}; fi\nexec "${realGit}" "$@"\n`, { mode: 0o755 });
    failed(f.run({ PATH: `${bin}:${process.env.PATH}` }), expected);
  });
}

test('untouched historical missing links do not expand the gate beyond affected authority', t => {
  const f = fixture(t);
  f.write('docs/old-plan.md', '# Historical proposal\n\n`docs/adr/ADR-099-not-adopted.md`\n');
  f.commit();
  f.write('src/internal.rs', 'fn repaired() {}\n');
  f.git('add', '-A');
  passed(f.run());
});

test('a deleted mapped decision still requires its guide after removing the trigger row', t => {
  const f = fixture(t);
  f.git('rm', decision);
  const next = f.map();
  next.boundaries = next.boundaries.filter(row => row.id !== 'consumer');
  f.writeMap(next);
  f.write('scripts/README.md', '# Gate\n\nDecision retired.\n');
  f.git('add', '-A');
  failed(f.run(), /boundary consumer requires an update/);
  f.write(guide, '# Consumer\n\nDecision retired; no remaining lifecycle promise.\n');
  f.git('add', guide);
  passed(f.run());
});

test('Markdown paths are document-relative and support encoded or angle-bracket spaces', t => {
  const f = fixture(t);
  f.write('docs/extra guide.md', '# Extra guide\n');
  f.write(guide, '# Consumer\n\n[Extra](<extra guide.md>) and [Encoded](extra%20guide.md)\n');
  f.git('add', '-A');
  passed(f.run());
  f.write(guide, '# Consumer\n\n[Broken](<missing guide.md>)\n');
  f.git('add', guide);
  failed(f.run(), /broken local reference in docs\/consumer.md: missing(?:%20| )guide.md/);
  f.write(guide, '# Consumer\n\n[Relative](docs/adr/ADR-001-example.md)\n');
  f.git('add', guide);
  failed(f.run(), /docs\/docs\/adr\/ADR-001-example.md/);
});

test('CommonMark destinations, references and directory links resolve without parsing examples', t => {
  const f = fixture(t);
  f.write('docs/consumer(v2).md', '# Second consumer\n');
  f.write('docs/examples/README.md', '# Examples\n');
  f.write(guide, '# Consumer\n\n[Balanced](consumer(v2).md)\n[Directory](examples/)\n[Reference][next]\n\n[next]: consumer(v2).md\n\n```markdown\n[Example](missing.md)\n`docs/adr/ADR-999-example.md`\n```\n\n    [Indented](missing.md)\n');
  f.git('add', '-A');
  passed(f.run());
  f.write(guide, '# Consumer\n\n[Missing directory](missing/)\n');
  f.git('add', guide);
  failed(f.run(), /broken local reference.*missing\//);
});

test('map object and boundary ordering do not change declared knowledge', t => {
  const f = fixture(t);
  const next = f.map();
  next.boundaries = next.boundaries.reverse().map(row => Object.fromEntries(Object.entries(row).reverse()));
  f.writeMap(next);
  f.git('add', mapPath);
  passed(f.run());
});

test('changing the canonical owner requires disposition of the retained prior guide', t => {
  const f = fixture(t);
  const next = f.map();
  next.boundaries[0].artifact = 'docs/new-consumer.md';
  f.writeMap(next);
  f.write('docs/new-consumer.md', '# Consumer\n\nUpdated lifecycle owner.\n');
  f.write('scripts/README.md', '# Gate\n\nConsumer ownership migrated.\n');
  f.write(decision, '# Decision\n\nNew consumer lifecycle.\n');
  f.git('add', '-A');
  failed(f.run(), /boundary consumer requires an update to docs\/consumer.md/);
  f.write(guide, '# Consumer moved\n\nSee [current owner](new-consumer.md).\n');
  f.git('add', guide);
  passed(f.run());
});

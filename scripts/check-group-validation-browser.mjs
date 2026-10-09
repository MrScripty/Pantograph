import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readdirSync, readFileSync, writeFileSync, copyFileSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const cwd = fileURLToPath(new URL('../', import.meta.url));
const evidence = process.env.GROUP_VALIDATION_BROWSER_EVIDENCE_DIR ?? mkdtempSync(join(tmpdir(), 'pantograph-group-ui-'));
mkdirSync(evidence, { recursive: true });
if (readdirSync(evidence).some(name => name.endsWith('.json'))) throw new Error('Use a fresh evidence directory; stale responses are refused');
const env = { ...process.env, ORT_SKIP_DOWNLOAD: '1', HF_HUB_OFFLINE: '1', TRANSFORMERS_OFFLINE: '1',
  GROUP_PREFLIGHT_EVIDENCE_DIR: evidence, GROUP_VALIDATION_BROWSER_EVIDENCE_DIR: evidence };
console.log(`Fresh producer/browser evidence: ${evidence}`);
const producerArgs = ['test', '--locked', '--offline', '-p', 'pantograph-workflow-service',
  '--features', 'test-support', '--test', 'group_preflight_public_validation', '--', '--test-threads=1'];
const producer = spawnSync('cargo', producerArgs, { cwd, env, encoding: 'utf8' });
writeFileSync(join(evidence, 'producer.log'), `${producer.stdout ?? ''}${producer.stderr ?? ''}`);
if (producer.error) throw producer.error;
if (producer.status !== 0) { console.error(producer.stdout, producer.stderr); process.exit(producer.status ?? 1); }
const match = producer.stderr.match(/Running tests\/group_preflight_public_validation.rs \(([^)]+)\)/);
if (!match) throw new Error('Producer executable identity missing from Cargo output');
const executable = match[1];
const owned = join(evidence, 'producer-test-executable');
copyFileSync(executable, owned);
const binaryBytes = readFileSync(owned);
const binary = { build_path: executable, owned_path: owned, bytes: statSync(owned).size,
  sha256: createHash('sha256').update(binaryBytes).digest('hex') };
const cases = readdirSync(evidence).filter(name => name.endsWith('.json')).sort().map(name => {
  const bytes = readFileSync(join(evidence, name));
  return { name, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') };
});
writeFileSync(join(evidence, 'producer-response-manifest.json'), JSON.stringify({
  command: ['cargo', ...producerArgs], binary, cases, qualification: 'Fresh public Rust API responses; browser replay uses a controlled transport, not live Tauri RPC.' }, null, 2));
const frontend = spawnSync(process.execPath, ['scripts/run-frontend-tests.mjs'], { cwd, env, stdio: 'inherit' });
if (frontend.error) throw frontend.error;
process.exitCode = frontend.status ?? 1;

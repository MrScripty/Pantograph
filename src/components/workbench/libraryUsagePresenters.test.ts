import test from 'node:test';
import assert from 'node:assert/strict';

import type { LibraryUsageProjectionRecord } from '../../services/diagnostics/types';
import type { PortOption } from '../../services/workflow/types';
import {
  buildHfDownloadRequest,
  buildHfSearchRequest,
  buildLibraryModelRows,
  classifyLibraryAsset,
  classifyLibraryModelKind,
  createLibraryRequestGate,
  filterLibraryModelRows,
  filterLibraryModelRowsByKind,
  formatHfDownloadStatus,
  formatHfResultCountLine,
  formatHfSearchStatus,
  formatLibraryAssetCategory,
  formatLibraryBytes,
  formatLibraryDimensionLine,
  formatLibraryModelSubline,
  formatLibraryProjectionFreshness,
  formatLibraryTimeShort,
  formatPumasDeleteStatus,
  groupLibraryModelRows,
  isLibraryAssetLastUsedByRun,
  libraryDeleteBlockReason,
  libraryKindOptions,
  libraryModelChips,
  libraryModelIdFromOption,
  libraryModelLocationRows,
  libraryModelSourceRows,
  libraryModelUsageRows,
  libraryWorkflowUseRows,
  normalizeHfSearchLimit,
  projectHfSearchResults,
} from './libraryUsagePresenters.ts';

test('classifyLibraryAsset uses explicit asset id prefixes only', () => {
  assert.equal(classifyLibraryAsset('model:llama'), 'model');
  assert.equal(classifyLibraryAsset('runtimes/llama.cpp'), 'runtime');
  assert.equal(classifyLibraryAsset('workflow:caption'), 'workflow');
  assert.equal(classifyLibraryAsset('nodes:resize'), 'node');
  assert.equal(classifyLibraryAsset('pumas:model-id'), 'pumas');
  assert.equal(classifyLibraryAsset('unknown-asset'), 'unclassified');
});

test('formatLibraryAssetCategory exposes readable labels', () => {
  assert.equal(formatLibraryAssetCategory('connector:hf'), 'Connector');
  assert.equal(formatLibraryAssetCategory('pantograph:starter'), 'Pantograph');
  assert.equal(formatLibraryAssetCategory('opaque-id'), 'Unclassified');
});

test('isLibraryAssetLastUsedByRun highlights only exact last-run matches', () => {
  assert.equal(isLibraryAssetLastUsedByRun({ last_workflow_run_id: 'run-a' }, 'run-a'), true);
  assert.equal(isLibraryAssetLastUsedByRun({ last_workflow_run_id: 'run-a' }, 'run-b'), false);
  assert.equal(isLibraryAssetLastUsedByRun({ last_workflow_run_id: null }, 'run-a'), false);
  assert.equal(isLibraryAssetLastUsedByRun({ last_workflow_run_id: 'run-a' }, null), false);
});

test('formatLibraryBytes renders compact network totals', () => {
  assert.equal(formatLibraryBytes(512), '512 B');
  assert.equal(formatLibraryBytes(2_048), '2.0 KiB');
  assert.equal(formatLibraryBytes(2_097_152), '2.0 MiB');
});

test('formatLibraryProjectionFreshness keeps projection status visible', () => {
  assert.equal(formatLibraryProjectionFreshness(null), 'Projection unavailable');
  assert.equal(
    formatLibraryProjectionFreshness({
      projection_name: 'library_usage',
      projection_version: 1,
      last_applied_event_seq: 7,
      status: 'current',
      rebuilt_at_ms: null,
      updated_at_ms: 100,
    }),
    'Current at seq 7',
  );
});

// --- Library list, detail and search projections ---------------------------

const DAY_MS = 24 * 60 * 60 * 1000;
// Local-time construction keeps the time-of-day assertions independent of TZ.
const NOW = new Date(2026, 10, 3, 20, 5).getTime();

function option(id: string, metadata: Record<string, unknown> = {}, extra: Partial<PortOption> = {}): PortOption {
  return {
    value: { model_id: id },
    label: id.split('/').pop() ?? id,
    metadata: { id, ...metadata },
    ...extra,
  };
}

function usage(assetId: string, extra: Partial<LibraryUsageProjectionRecord> = {}): LibraryUsageProjectionRecord {
  return {
    asset_id: assetId,
    total_access_count: 3,
    run_access_count: 2,
    total_network_bytes: 0,
    last_accessed_at_ms: NOW - 60_000,
    last_operation: 'run_usage',
    last_workflow_run_id: 'run_a',
    last_workflow_id: 'ingest-docs-rag',
    last_workflow_semantic_version: '1.4.0',
    last_event_seq: 9,
    last_updated_at_ms: NOW - 60_000,
    ...extra,
  };
}

test('buildLibraryModelRows joins usage by pumas asset id and leaves the rest unjoined', () => {
  const rows = buildLibraryModelRows(
    [
      option('llm/qwen/qwen3-8b', { model_type: 'llm', repo_id: 'Qwen/Qwen3-8B', recommended_backend: 'llama.cpp' }),
      option('diffusion/sdxl', { model_type: 'diffusion', display_entry_path: '/models/sdxl' }),
      option('llm/qwen/qwen3-8b'),
      { value: null, label: 'no id' },
    ],
    [
      usage('pumas://models/llm/qwen/qwen3-8b'),
      usage('hf://models/Qwen/Qwen3-8B', { last_operation: 'download', last_workflow_id: null }),
      usage('pumas://models/deleted-model'),
    ],
  );
  assert.deepEqual(
    rows.map((row) => [row.modelId, row.assetId, row.usage?.asset_id ?? null]),
    [
      ['llm/qwen/qwen3-8b', 'pumas://models/llm/qwen/qwen3-8b', 'pumas://models/llm/qwen/qwen3-8b'],
      ['diffusion/sdxl', 'pumas://models/diffusion/sdxl', null],
    ],
  );
  assert.equal(rows[0].downloadUsage?.asset_id, 'hf://models/Qwen/Qwen3-8B');
  assert.equal(rows[0].backend, 'llama.cpp');
  assert.equal(rows[1].path, '/models/sdxl');
});

test('libraryModelIdFromOption reads metadata id, then model_ref value, then a string value', () => {
  assert.equal(libraryModelIdFromOption({ value: 'x', label: 'x', metadata: { id: 'meta-id' } }), 'meta-id');
  assert.equal(libraryModelIdFromOption({ value: { model_id: 'ref-id' }, label: 'x' }), 'ref-id');
  assert.equal(libraryModelIdFromOption({ value: 'plain-id', label: 'x' }), 'plain-id');
  assert.equal(libraryModelIdFromOption({ value: { other: 1 }, label: 'x' }), null);
});

test('classifyLibraryModelKind uses model type, then task, then pipeline tag, else no kind', () => {
  assert.equal(classifyLibraryModelKind({ modelType: 'embedding', taskType: null, pipelineTag: null }), 'text');
  assert.equal(classifyLibraryModelKind({ modelType: 'unknown', taskType: 'image_generation', pipelineTag: null }), 'image');
  assert.equal(
    classifyLibraryModelKind({ modelType: null, taskType: null, pipelineTag: 'automatic-speech-recognition' }),
    'audio',
  );
  assert.equal(classifyLibraryModelKind({ modelType: 'vlm', taskType: null, pipelineTag: null }), null);
});

test('libraryKindOptions is hidden when no kind is known and counts kinds otherwise', () => {
  const unknown = buildLibraryModelRows([option('a'), option('b')], []);
  assert.equal(libraryKindOptions(unknown), null);
  const known = buildLibraryModelRows(
    [option('a', { model_type: 'llm' }), option('b', { model_type: 'diffusion' }), option('c')],
    [],
  );
  assert.deepEqual(
    libraryKindOptions(known)?.map((segment) => [segment.value, segment.count]),
    [['all', 3], ['text', 1], ['image', 1], ['audio', 0]],
  );
  assert.deepEqual(filterLibraryModelRowsByKind(known, 'image').map((row) => row.modelId), ['b']);
});

test('filterLibraryModelRows matches name, id, repository and model type', () => {
  const rows = buildLibraryModelRows(
    [option('llm/qwen3', { repo_id: 'Qwen/Qwen3-8B', model_type: 'llm' }), option('audio/whisper')],
    [],
  );
  assert.deepEqual(filterLibraryModelRows(rows, 'QWEN/').map((row) => row.modelId), ['llm/qwen3']);
  assert.deepEqual(filterLibraryModelRows(rows, 'whisper').map((row) => row.modelId), ['audio/whisper']);
  assert.equal(filterLibraryModelRows(rows, '  ').length, 2);
});

test('groupLibraryModelRows groups by last access and falls back to one Models group', () => {
  const noUsage = buildLibraryModelRows([option('a'), option('b')], []);
  assert.deepEqual(
    groupLibraryModelRows(noUsage, NOW).map((group) => [group.label, group.rows.length]),
    [['Models', 2]],
  );
  const rows = buildLibraryModelRows(
    [option('recent'), option('old'), option('never')],
    [
      usage('pumas://models/recent', { last_accessed_at_ms: NOW - 2 * DAY_MS }),
      usage('pumas://models/old', { last_accessed_at_ms: NOW - 31 * DAY_MS }),
    ],
  );
  assert.deepEqual(
    groupLibraryModelRows(rows, NOW).map((group) => [group.label, group.rows.map((row) => row.modelId)]),
    [
      ['Used this month', ['recent']],
      ['Unused for 30 days', ['old']],
      ['No usage recorded', ['never']],
    ],
  );
  assert.equal(groupLibraryModelRows([], NOW).length, 0);
  assert.equal(
    formatLibraryDimensionLine(rows, NOW),
    '3 models, 1 used this month, 1 unused for 30 days, 1 with no usage recorded',
  );
  assert.equal(formatLibraryDimensionLine(noUsage, NOW), '2 models');
  assert.equal(formatLibraryDimensionLine([], NOW, true), 'Pumas model list unavailable');
  assert.equal(formatLibraryDimensionLine(noUsage, NOW, true), '2 models');
});

test('formatLibraryModelSubline says "No usage recorded" when the projection has no row', () => {
  const [withUsage, withoutUsage, noBackend] = buildLibraryModelRows(
    [option('a', { recommended_backend: 'llama.cpp' }), option('b', { recommended_backend: 'diffusers' }), option('c')],
    [usage('pumas://models/a')],
  );
  assert.equal(formatLibraryModelSubline(withUsage), 'llama.cpp, last used by ingest-docs-rag');
  assert.equal(formatLibraryModelSubline(withoutUsage), 'diffusers, no usage recorded');
  assert.equal(formatLibraryModelSubline(noBackend), 'No usage recorded');
});

test('formatLibraryTimeShort shows time today, month and day this year, and the year otherwise', () => {
  assert.equal(formatLibraryTimeShort(new Date(2026, 10, 3, 9, 7).getTime(), NOW), '09:07');
  assert.equal(formatLibraryTimeShort(new Date(2026, 9, 29, 9, 7).getTime(), NOW), 'Oct 29');
  assert.equal(formatLibraryTimeShort(new Date(2025, 9, 29, 9, 7).getTime(), NOW), 'Oct 29, 2025');
});

test('detail rows omit facts the projections lack', () => {
  const [row] = buildLibraryModelRows([option('a')], []);
  assert.deepEqual(libraryModelLocationRows(row), [{ key: 'Readiness', value: 'Ready to use in a graph' }]);
  assert.deepEqual(libraryModelUsageRows(row, NOW), []);
  assert.deepEqual(libraryModelSourceRows(row, NOW), []);
  assert.deepEqual(libraryWorkflowUseRows(row, 'run_a', NOW), []);

  const [full] = buildLibraryModelRows(
    [
      option(
        'b',
        { repo_id: 'Org/B', model_type: 'llm', task_type_primary: 'text-generation', storage_kind: 'library_owned' },
        { disabled: true, unavailableReason: 'entry path is Missing' },
      ),
    ],
    [
      usage('pumas://models/b', { total_network_bytes: 2_048, last_cache_status: 'hit' }),
      usage('hf://models/Org/B', { last_operation: 'download', last_accessed_at_ms: new Date(2026, 9, 1).getTime() }),
    ],
  );
  assert.deepEqual(libraryModelLocationRows(full), [
    { key: 'Storage', value: 'library_owned' },
    { key: 'Readiness', value: 'entry path is Missing' },
  ]);
  assert.deepEqual(
    libraryModelUsageRows(full, NOW).map((entry) => [entry.key, entry.value]),
    [
      ['Runs', '2 runs'],
      ['Accesses', '3'],
      ['Last used', '20:04, by ingest-docs-rag v1.4.0'],
      ['Last operation', 'run_usage'],
      ['Read from network', '2.0 KiB'],
      ['Cache', 'hit'],
    ],
  );
  assert.deepEqual(
    libraryModelSourceRows(full, NOW).map((entry) => [entry.key, entry.value]),
    [
      ['Repository', 'Org/B'],
      ['Model type', 'llm'],
      ['Task', 'text-generation'],
      ['Download recorded', 'Oct 1'],
    ],
  );
  assert.deepEqual(libraryModelChips(full, 'run_a'), ['Last used by active run', 'Not ready', '2 runs']);
});

test('libraryWorkflowUseRows marks the active run and libraryDeleteBlockReason blocks delete', () => {
  const [row] = buildLibraryModelRows([option('a')], [usage('pumas://models/a')]);
  const [active] = libraryWorkflowUseRows(row, 'run_a', NOW);
  assert.equal(active.isActiveRun, true);
  assert.equal(active.workflowId, 'ingest-docs-rag');
  assert.equal(active.version, '1.4.0');
  assert.equal(active.runCount, 2);
  assert.equal(libraryWorkflowUseRows(row, 'run_b', NOW)[0].isActiveRun, false);

  assert.match(libraryDeleteBlockReason(row, 'run_a', false) ?? '', /active run/);
  assert.match(libraryDeleteBlockReason(row, 'run_b', true) ?? '', /current library action/);
  assert.equal(libraryDeleteBlockReason(row, 'run_b', false), null);
  assert.equal(libraryDeleteBlockReason(row, null, false), null);
});

test('buildHfSearchRequest trims inputs and clamps the limit to the host range', () => {
  assert.deepEqual(buildHfSearchRequest({ query: ' qwen ', kind: '  ', limit: 25 }), {
    query: 'qwen',
    kind: null,
    limit: 25,
    hydrateLimit: 10,
  });
  assert.equal(buildHfSearchRequest({ query: '', kind: 'text-generation', limit: 500 }).limit, 100);
  assert.equal(normalizeHfSearchLimit(0), 1);
  assert.equal(normalizeHfSearchLimit('7'), 7);
  assert.equal(normalizeHfSearchLimit(Number.NaN), 25);
  assert.equal(normalizeHfSearchLimit(null), 25);
  assert.equal(normalizeHfSearchLimit(''), 25);
  assert.equal(buildHfSearchRequest({ query: 'x', kind: '', limit: 4 }).hydrateLimit, 4);
});

test('buildHfDownloadRequest keeps the existing defaults for family and official name', () => {
  assert.deepEqual(buildHfDownloadRequest('Org/Model', { officialName: '', family: ' ', modelType: '' }), {
    repo_id: 'Org/Model',
    family: 'model',
    official_name: 'Org/Model',
    model_type: null,
  });
  assert.deepEqual(
    buildHfDownloadRequest('Org/Model', { officialName: ' My model ', family: 'qwen', modelType: 'llm' }),
    { repo_id: 'Org/Model', family: 'qwen', official_name: 'My model', model_type: 'llm' },
  );
});

test('projectHfSearchResults reads the camelCase host fields and drops rows without a repository', () => {
  const rows = projectHfSearchResults([
    { repoId: 'Qwen/Qwen3-14B-GGUF', kind: 'text-generation', formats: ['gguf'], downloads: 1_234, totalSizeBytes: 2_147_483_648 },
    { repoId: 'Qwen/Qwen3-14B-GGUF' },
    { name: 'no repo' },
  ]);
  assert.deepEqual(rows, [
    { repoId: 'Qwen/Qwen3-14B-GGUF', sub: 'text-generation, gguf, 1.2k downloads', size: '2.0 GiB' },
  ]);
  assert.equal(formatHfResultCountLine(0, 25), 'No Hugging Face models match. Try a shorter or different query.');
  assert.equal(formatHfResultCountLine(3, 25), '3 results');
  assert.match(formatHfResultCountLine(25, 25), /^Showing the first 25\./);
});

test('status sentences name the audit event when the host returns one', () => {
  assert.equal(formatHfSearchStatus({ models: [], auditEventSeq: 12 }), 'Search recorded at event 12');
  assert.equal(formatHfSearchStatus({ models: [] }), 'Search finished');
  assert.equal(
    formatHfDownloadStatus('Org/M', { downloadId: 'dl-1', auditEventSeq: 13 }),
    'Download dl-1 of Org/M recorded at event 13',
  );
  assert.equal(formatHfDownloadStatus('Org/M', { downloadId: 'dl-1' }), 'Download dl-1 of Org/M started');
  assert.equal(formatPumasDeleteStatus({ success: true, auditEventSeq: 14 }), 'Delete recorded at event 14');
  assert.equal(formatPumasDeleteStatus({ success: false, error: 'busy' }), 'busy');
  assert.equal(formatPumasDeleteStatus({ success: false }), 'Delete did not complete');
});

test('createLibraryRequestGate applies only the newest invocation and nothing after dispose', async () => {
  const gate = createLibraryRequestGate();
  const applied: string[] = [];
  const run = async (label: string, delayMs: number) => {
    const token = gate.begin();
    await new Promise((resolve) => setTimeout(resolve, delayMs));
    if (gate.isCurrent(token)) applied.push(label);
  };
  await Promise.all([run('slow-first', 20), run('fast-second', 1)]);
  assert.deepEqual(applied, ['fast-second']);

  const token = gate.begin();
  assert.equal(gate.isCurrent(token), true);
  gate.dispose();
  assert.equal(gate.isCurrent(token), false);
  assert.equal(gate.isCurrent(gate.begin()), false);

  const other = createLibraryRequestGate();
  assert.equal(other.isCurrent(other.begin()), true, 'gates are scoped to their owner');
});

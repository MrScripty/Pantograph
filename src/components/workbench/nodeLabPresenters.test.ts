import test from 'node:test';
import assert from 'node:assert/strict';
import type { CustomNodeAuthoringEntry, NodeLabDraft } from '../../services/workflow/customNodeAuthoring.ts';
import type { NodeLabView } from './nodeLabController.ts';
import {
  buildNodeLabCatalogRows,
  buildNodeLabChecks,
  buildNodeLabIdentityRows,
  buildNodeLabStatusRows,
  nodeLabDraftPlaceholder,
  nodeLabErrorRegion,
  nodeLabShowsSearch,
  nodeLabSummaryLine,
  nodeLabTrustMessage,
  projectNodeLabDraft,
  resolveNodeLabSelection,
} from './nodeLabPresenters.ts';

test('NodeLab distinguishes admitted graph authoring from unsupported code publishing', () => {
  assert.match(nodeLabTrustMessage(), /trusted in-process/);
  assert.match(nodeLabTrustMessage(), /cannot install or publish code/);
  assert.deepEqual(buildNodeLabStatusRows(), [
    { label: 'Graph authoring', value: 'Host-admitted nodes' },
    { label: 'Local Agent', value: 'Unavailable' },
    { label: 'Runtime Publishing', value: 'Unsupported' },
  ]);
});

const identity = (nodeType: string) => ({
  node_type: nodeType,
  contract_version: '1.0.0',
  contract_digest: `contract-blake3:${'c'.repeat(64)}`,
  implementation_digest: `blake3:${'a'.repeat(64)}`,
  environment_digest: `blake3:${'b'.repeat(64)}`,
  adapter_version: 1 as const,
});
const entry = (nodeType: string, label: string, description = 'Host transform'): CustomNodeAuthoringEntry => ({
  identity: identity(nodeType),
  label,
  description,
  trust: 'trusted_in_process',
  inputs: [{ id: 'input', label: 'Input' }],
  outputs: [{ id: 'output', label: 'Output' }, { id: 'count', label: 'Count' }],
});
const upper = entry('host-uppercase', 'Host uppercase');
const lower = entry('host-lowercase', 'Host lowercase', 'Folds text to lower case');
const draft = (diagnostics: NodeLabDraft['graph_diagnostics'] = []): NodeLabDraft => ({
  session_id: 'edit-1',
  graph_revision: 'r1',
  graph: {
    nodes: [
      { id: 'custom', node_type: 'host-uppercase', position: { x: 320, y: 0 }, data: { label: 'Host uppercase', custom_node_identity: identity('host-uppercase') } },
      { id: 'input-0', node_type: 'text-input', position: { x: 0, y: 0 }, data: { label: 'Input', text: 'hello' } },
      { id: 'input-1', node_type: 'text-input', position: { x: 0, y: 160 }, data: {} },
      { id: 'output-0', node_type: 'text-output', position: { x: 640, y: 0 }, data: { label: 'Output', text: '' } },
    ],
    edges: [
      { id: 'edge-input-0', source: 'input-0', source_handle: 'text', target: 'custom', target_handle: 'input' },
      { id: 'edge-output-0', source: 'custom', source_handle: 'output', target: 'output-0', target_handle: 'text' },
    ],
  },
  graph_diagnostics: diagnostics,
});
const view = (patch: Partial<NodeLabView>): NodeLabView => ({ catalog: null, draft: null, savedPath: '', busy: false, error: null, notice: '', pendingCleanup: 0, ...patch });

test('catalog rows filter by label, type and description and mark the node the draft uses', () => {
  const all = buildNodeLabCatalogRows([upper, lower], draft(), '');
  assert.deepEqual(all.rows.map(row => [row.nodeType, row.glyph, row.time]), [
    ['host-uppercase', 'succeeded', 'in draft'],
    ['host-lowercase', 'idle', ''],
  ]);
  assert.equal(all.rows[0].statusLabel, '1 string input, 2 string outputs');
  assert.equal(all.rows[0].title, '1 string input, 2 string outputs. Trusted in-process, used by the open draft. Host transform');
  assert.equal(all.rows[1].title, '1 string input, 2 string outputs. Trusted in-process. Folds text to lower case');
  assert.equal(all.rows[1].detail, 'Folds text to lower case');
  assert.deepEqual([all.matched, all.total], [2, 2]);
  assert.deepEqual(buildNodeLabCatalogRows([upper, lower], null, 'LOWER CASE').rows.map(row => row.nodeType), ['host-lowercase']);
  assert.deepEqual(buildNodeLabCatalogRows([upper, lower], null, ' host-upper ').rows.map(row => row.nodeType), ['host-uppercase']);
  const none = buildNodeLabCatalogRows([upper, lower], null, 'zebra');
  assert.deepEqual([none.rows.length, none.matched, none.total], [0, 0, 2]);
  assert.deepEqual(buildNodeLabCatalogRows(null, null, ''), { rows: [], matched: 0, total: 0 });
});

test('search appears only for a catalog long enough to need it', () => {
  assert.equal(nodeLabShowsSearch(null), false);
  assert.equal(nodeLabShowsSearch([upper]), false);
  assert.equal(nodeLabShowsSearch(Array.from({ length: 6 }, (_, index) => entry(`node-${index}`, `Node ${index}`))), true);
});

test('selection prefers the picked row, then the draft node, then the first row, and never a missing node', () => {
  assert.equal(resolveNodeLabSelection(null, 'host-uppercase', null), null);
  assert.equal(resolveNodeLabSelection([], null, draft()), null);
  assert.equal(resolveNodeLabSelection([lower, upper], 'host-lowercase', draft()), lower);
  assert.equal(resolveNodeLabSelection([lower, upper], null, draft()), upper);
  assert.equal(resolveNodeLabSelection([lower, upper], 'gone', null), lower);
});

test('identity rows show every pinned identity field in mono', () => {
  const rows = buildNodeLabIdentityRows(upper);
  assert.deepEqual(rows.map(row => row.key), ['Node type', 'Contract version', 'Contract digest', 'Implementation', 'Environment', 'Adapter', 'Trust']);
  assert.equal(rows[1].value, '1.0.0');
  assert.equal(rows[2].value, `contract-blake3:${'c'.repeat(64)}`);
  assert.equal(rows[3].value, `blake3:${'a'.repeat(64)}`);
  assert.equal(rows[4].value, `blake3:${'b'.repeat(64)}`);
  assert.equal(rows[5].value, 'v1');
  assert.ok(rows.slice(0, 6).every(row => row.mono));
});

test('draft projection lists text sources with their editable value, outputs and the pinned node', () => {
  const projection = projectNodeLabDraft(draft());
  assert.equal(projection.countsText, '4 nodes · 2 connections');
  assert.deepEqual(projection.customNodes, [{ id: 'custom', label: 'Host uppercase', nodeType: 'host-uppercase' }]);
  assert.deepEqual(projection.sources.map(source => [source.node.id, source.label, source.text]), [['input-0', 'Input', 'hello'], ['input-1', 'input-1', '']]);
  assert.deepEqual(projection.outputs, [{ id: 'output-0', label: 'Output' }]);
});

test('checks say the contract is valid or prefix blocking diagnostics', () => {
  assert.deepEqual(buildNodeLabChecks(draft()), [{ glyph: 'succeeded', text: 'Graph contract valid' }]);
  assert.deepEqual(buildNodeLabChecks(draft([
    { message: 'host-uppercase is unavailable', blocking_submission: true },
    { message: 'Output is not connected', blocking_submission: false },
  ])), [
    { glyph: 'failed', text: 'Blocks execution: host-uppercase is unavailable' },
    { glyph: 'warn', text: 'Output is not connected' },
  ]);
});

test('summary line separates loading, failure and empty catalogs and describes the draft', () => {
  assert.equal(nodeLabSummaryLine(view({ busy: true })), 'Loading admitted nodes · no draft open');
  assert.equal(nodeLabSummaryLine(view({ error: 'Qualification host unavailable' })), 'Admitted nodes unavailable · no draft open');
  assert.equal(nodeLabSummaryLine(view({ catalog: [] })), 'No admitted nodes on this host · no draft open');
  assert.equal(nodeLabSummaryLine(view({ catalog: [upper], draft: draft() })), '1 admitted node · draft of 4 nodes, contract valid');
  assert.equal(
    nodeLabSummaryLine(view({ catalog: [upper, lower], draft: draft([{ message: 'x', blocking_submission: true }]), savedPath: '.pantograph/workflows/a.json' })),
    '2 admitted nodes · draft of 4 nodes, 1 blocking check, saved',
  );
});

test('the empty-catalog placeholder keeps the admission wording; otherwise it points at both ways in', () => {
  assert.equal(nodeLabDraftPlaceholder([]).question, 'This host has no admitted custom nodes.');
  assert.match(nodeLabDraftPlaceholder([]).sentence, /^A host maintainer must register a trusted implementation before creating a new draft\./);
  assert.equal(nodeLabDraftPlaceholder(null).question, 'No workflow draft is open.');
  assert.equal(nodeLabDraftPlaceholder([upper]).question, 'No workflow draft is open.');
});

test('errors are placed next to the control whose action raised them', () => {
  assert.equal(nodeLabErrorRegion('loadCatalog'), 'catalog');
  assert.equal(nodeLabErrorRegion('create'), 'catalog');
  assert.equal(nodeLabErrorRegion('reopen'), 'draft');
  assert.equal(nodeLabErrorRegion('update'), 'draft');
  assert.equal(nodeLabErrorRegion('save'), 'draft');
  assert.equal(nodeLabErrorRegion('retryCleanup'), 'sessions');
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { NodeLabService, parseCustomNodeCatalog, parseNodeLabDraft } from './customNodeAuthoring.ts';
import { NodeLabController } from '../../components/workbench/nodeLabController.ts';

const identity = { node_type: 'custom-uppercase', contract_version: '1.0.0', contract_digest: `contract-blake3:${'c'.repeat(64)}`, implementation_digest: `blake3:${'a'.repeat(64)}`, environment_digest: `blake3:${'b'.repeat(64)}`, adapter_version: 1 };
const entry = { identity, label: 'Uppercase', description: 'Host transform', trust: 'trusted_in_process', inputs: [{ id: 'input', label: 'Input' }], outputs: [{ id: 'output', label: 'Output' }] };
const catalog = () => ({ contract_version: 1, nodes: [structuredClone(entry)] });
const snapshot = (id = 'edit-1') => ({ session_id: id, graph_revision: 'revision-1', graph: { nodes: [{ id: 'source', node_type: 'text-input', position: { x: 0, y: 0 }, data: { text: 'draft' } }], edges: [] }, graph_diagnostics: [] });

test('catalog decoder requires complete v1 identity, trust, ports and extra-field policy', () => {
  assert.equal(parseCustomNodeCatalog(catalog())[0].identity.environment_digest, identity.environment_digest);
  assert.deepEqual(parseCustomNodeCatalog({ contract_version: 1, nodes: [] }), []);
  const invalid: unknown[] = [null, [], { ...catalog(), contract_version: 2 }, { ...catalog(), extra: true }, { ...catalog(), nodes: {} }];
  for (const patch of [
    { trust: 'sandbox' }, { inputs: [] }, { outputs: [{ id: 'output', label: 1 }] },
    { identity: { ...identity, adapter_version: 2 } }, { identity: { ...identity, implementation_digest: 'unverified' } },
    { identity: { ...identity, extra: true } }, { identity: { ...identity, contract_digest: 'bad' } }, { identity: { ...identity, contract_version: 'not-semver' } }, { identity: { ...identity, node_type: 'bad type' } }, { inputs: [{ id: 'x', label: 'X' }, { id: 'x', label: 'Again' }] },
  ]) invalid.push({ contract_version: 1, nodes: [{ ...entry, ...patch }] });
  for (const value of invalid) assert.throws(() => parseCustomNodeCatalog(value));
  assert.throws(() => parseCustomNodeCatalog({ contract_version: 1, nodes: [entry, entry] }));
});

test('draft decoder refuses malformed graph and diagnostics before exposing state', () => {
  assert.equal(parseNodeLabDraft(snapshot()).graph.nodes[0].data.text, 'draft');
  for (const value of [{ ...snapshot(), graph: { nodes: [{}], edges: [] } }, { ...snapshot(), graph_diagnostics: [{ message: 'invalid' }] }, { ...snapshot(), session_id: '' }, { ...snapshot(), graph_diagnostics: null }]) assert.throws(() => parseNodeLabDraft(value));
});

test('service forwards exact selection and refuses changed-session response', async () => {
  const calls: [string, Record<string, unknown> | undefined][] = [];
  const service = new NodeLabService(async (command, args) => { calls.push([command, args]); return snapshot(); });
  await service.create(identity);
  assert.deepEqual(calls[0], ['create_custom_node_draft', { request: { identity } }]);
  await assert.rejects(service.snapshot('other-session'), /correlation mismatch/);
});

test('invalid newly created response closes its session instead of leaking it', async () => {
  const closed: string[] = [];
  const service = new NodeLabService(async (command, args) => {
    if (command === 'remove_execution') { closed.push(String(args?.executionId)); return null; }
    return { session_id: 'created', graph: 'broken' };
  });
  await assert.rejects(service.create(identity), /Invalid NodeLab text/);
  assert.deepEqual(closed, ['created']);
});

test('controller distinguishes empty catalog from failure and closes superseded drafts', async () => {
  let fail = false;
  let counter = 0;
  const closed: string[] = [];
  const service = new NodeLabService(async (command, args) => {
    if (command === 'get_custom_node_authoring_catalog') {
      if (fail) throw new Error('host unavailable');
      return { contract_version: 1, nodes: [] };
    }
    if (command === 'remove_execution') { closed.push(String(args?.executionId)); return null; }
    return snapshot(`edit-${++counter}`);
  });
  const controller = new NodeLabController(service, () => {});
  await controller.loadCatalog(); assert.deepEqual(controller.view.catalog, []); assert.equal(controller.view.error, null);
  fail = true; await controller.loadCatalog(); assert.equal(controller.view.error, 'host unavailable');
  const selected = parseCustomNodeCatalog(catalog())[0];
  await controller.create(selected); await controller.create(selected);
  assert.deepEqual(closed, ['edit-1']);
  await controller.dispose(); assert.deepEqual(closed, ['edit-1', 'edit-2']);
});

test('dispose observes in-flight creation, prevents obsolete publication and releases the late session', async () => {
  let complete!: (value: unknown) => void;
  const pending = new Promise(resolve => { complete = resolve; });
  const closed: string[] = [];
  let published = 0;
  const service = new NodeLabService(async (command, args) => {
    if (command === 'remove_execution') { closed.push(String(args?.executionId)); return null; }
    return pending;
  });
  const controller = new NodeLabController(service, () => { published++; });
  const create = controller.create(parseCustomNodeCatalog(catalog())[0]);
  const before = published;
  const disposed = controller.dispose();
  complete(snapshot('late'));
  await Promise.all([create, disposed]);
  assert.equal(published, before); assert.deepEqual(closed, ['late']); assert.equal(controller.cleanupError, null);
});

test('close failure compensates the new session and retains the old draft', async () => {
  let created = 0;
  const closed: unknown[] = [];
  const service = new NodeLabService(async (command, args) => {
    if (command === 'remove_execution') {
      closed.push(args?.executionId);
      if (args?.executionId === 'edit-1') throw new Error('close refused');
      return null;
    }
    return snapshot(`edit-${++created}`);
  });
  const controller = new NodeLabController(service, () => {});
  const selected = parseCustomNodeCatalog(catalog())[0];
  await controller.create(selected); await controller.create(selected);
  assert.equal(created, 2); assert.equal(controller.view.error, 'close refused');
  assert.equal(controller.view.draft?.session_id, 'edit-1');
  assert.deepEqual(closed, ['edit-1', 'edit-2']);
  await controller.dispose(); assert.match(String(controller.cleanupError), /close refused/);
});

test('failed stale selection or reopen retains current edited draft and save path', async () => {
  let refuse = false;
  const closed: unknown[] = [];
  const service = new NodeLabService(async (command, args) => {
    if (command === 'remove_execution') { closed.push(args?.executionId); return null; }
    if (refuse) throw new Error('selection/path unavailable');
    return snapshot('current');
  });
  const controller = new NodeLabController(service, () => {});
  const selected = parseCustomNodeCatalog(catalog())[0];
  await controller.create(selected);
  controller.view.savedPath = 'existing.json';
  const before = structuredClone(controller.view.draft);
  refuse = true;
  await controller.create(selected);
  assert.deepEqual(controller.view.draft, before); assert.equal(controller.view.savedPath, 'existing.json');
  await controller.reopen('missing.json');
  assert.deepEqual(controller.view.draft, before); assert.equal(controller.view.savedPath, 'existing.json');
  assert.deepEqual(closed, []);
  await controller.dispose(); assert.deepEqual(closed, ['current']);
});

test('app service retains failed malformed-response cleanup across page owners until explicit recovery', async () => {
  let closeFails = true;
  const service = new NodeLabService(async command => {
    if (command === 'remove_execution') { if (closeFails) throw new Error('offline'); return null; }
    return { session_id: 'known-created', graph_revision: 'revision', graph: null };
  });
  await assert.rejects(service.create(identity), /offline/);
  assert.equal(service.pendingCleanupCount, 1);
  const nextPage = new NodeLabController(service, () => {});
  assert.equal(nextPage.view.pendingCleanup, 1);
  await assert.rejects(service.create(identity), /Close previous/);
  closeFails = false;
  await nextPage.retryCleanup();
  assert.equal(nextPage.view.pendingCleanup, 0); assert.equal(service.pendingCleanupCount, 0);
  await nextPage.dispose();
});

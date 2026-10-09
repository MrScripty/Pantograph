import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import vm from 'node:vm';
import { parse } from 'svelte/compiler';
import { jsonFilterApplyError, jsonFilterPathState, type JsonFilterMutationResult } from './jsonFilterAuthoring.ts';

// Actual Svelte handlers with controlled mutation responses. This does not claim
// browser rendering/Tauri IPC; the Rust regression covers real graph persistence.
const source = readFileSync(new URL('./JsonFilterPathEditor.svelte', import.meta.url), 'utf8');
const script = parse(source, { modern: true }).instance!.content.body;
const handlers = stripTypeScriptTypes(script.filter(node =>
  node.type === 'FunctionDeclaration'
  || (node.type === 'ExpressionStatement' && node.expression.type === 'CallExpression'
    && node.expression.callee.type === 'Identifier' && node.expression.callee.name === 'onDestroy'),
).map(node => {
  const { start, end } = node as unknown as { start: number; end: number };
  return source.slice(start, end);
}).join('\n') + '\nglobalThis.sync = syncSavedPath; globalThis.input = handleInput; globalThis.apply = applyPath; globalThis.reload = reloadSavedPath;');

function fixture(path: unknown = undefined) {
  const writes: { nodeId: string; data: Record<string, unknown> }[] = [];
  let response = Promise.resolve<JsonFilterMutationResult>({
    action: 'update node data', status: 'applied', sessionId: 'session',
  });
  const context = vm.createContext({
    invalidData: false, id: 'filter', data: { path }, draftPath: '', dirty: false, conflict: false,
    busy: false, applied: false, error: null, observedId: null, observedPath: undefined,
    pendingPath: null, appliedPath: null, generation: 0, disposed: false,
    jsonFilterPathState, jsonFilterApplyError,
    onDestroy: (callback: () => void) => { context.destroy = callback; },
    updateNodeData: async (nodeId: string, data: Record<string, unknown>) => {
      writes.push({ nodeId, data: JSON.parse(JSON.stringify(data)) });
      return response;
    },
  });
  context.applyNodeData = context.updateNodeData;
  Object.defineProperty(context, 'savedPath', { get: () => context.invalidData ? {text:'',error:'The saved node data must be an object. Apply a path to repair it.'} : jsonFilterPathState(context.data.path) });
  vm.runInContext(handlers, context);
  context.sync(context.id, path);
  return {
    context, writes,
    input: (value: string) => context.input({ currentTarget: { value } }),
    respond: (next: Promise<JsonFilterMutationResult>) => { response = next; },
    publish: (saved: unknown, id = context.id) => {
      context.data = { path: saved }; context.id = id; context.sync(id, saved);
    },
  };
}

test('missing path displays existing whole-value behavior without an implicit mutation', async () => {
  const f = fixture();
  assert.equal(f.context.draftPath, '');
  assert.equal(f.context.savedPath.error, null);
  await f.context.apply();
  assert.equal(f.writes.length, 0);
});

test('typing changes only the draft; Apply forwards the exact string and path-only patch', async () => {
  const f = fixture('old.path');
  const path = '  results[0].document.é\n';
  f.input(path);
  assert.equal(f.writes.length, 0);
  await f.context.apply();
  assert.deepEqual(f.writes, [{ nodeId: 'filter', data: { path } }]);
  assert.equal(f.context.applied, true);
  assert.equal(f.context.busy, false);
});

test('malformed saved types remain invalid until the user explicitly replaces them', async () => {
  for (const malformed of [null, 42, {}, []]) {
    const f = fixture(malformed);
    assert.match(f.context.savedPath.error, /must be a string/);
    assert.equal(f.context.data.path, malformed);
    assert.equal(f.writes.length, 0);
    await f.context.apply(); // explicit empty string repairs to whole-value semantics
    assert.deepEqual(f.writes, [{ nodeId: 'filter', data: { path: '' } }]);
  }
});

test('all non-applied mutation outcomes retain the draft without false saved success', async () => {
  for (const status of ['failed', 'skipped', 'stale'] as const) {
    const f = fixture('old'); f.input('new');
    f.respond(Promise.resolve({ action: 'update node data', status, sessionId: null, error: 'producer rejection' }));
    await f.context.apply();
    assert.equal(f.context.applied, false);
    assert.equal(f.context.dirty, true);
    assert.equal(f.context.draftPath, 'new');
    assert.ok(f.context.error);
  }
  const f = fixture('old'); f.input('new');
  f.respond(Promise.reject(new Error('bridge failure')));
  await f.context.apply();
  assert.match(f.context.error, /bridge failure/);
  assert.equal(f.context.busy, false);
});

test('a different saved path preserves the dirty draft and requires explicit reload', async () => {
  const f = fixture('old'); f.input('draft'); f.publish('other.saved');
  assert.equal(f.context.draftPath, 'draft');
  assert.equal(f.context.conflict, true);
  await f.context.apply(); assert.equal(f.writes.length, 0);
  f.context.reload();
  assert.equal(f.context.draftPath, 'other.saved');
  assert.equal(f.context.conflict, false);
  assert.equal(f.context.dirty, false);
  assert.equal(f.writes.length, 0);
});

test('a clean form follows saved path changes, including undo and exact empty path', () => {
  const f = fixture('initial');
  f.publish('items[0].text'); assert.equal(f.context.draftPath, 'items[0].text');
  f.publish(''); assert.equal(f.context.draftPath, '');
  f.publish(undefined); assert.equal(f.context.savedPath.error, null);
  assert.equal(f.writes.length, 0);
});

test('an in-flight Apply is not duplicated and its own saved response is accepted', async () => {
  const f = fixture('old'); f.input('new');
  let finish!: (value: JsonFilterMutationResult) => void;
  f.respond(new Promise(resolve => { finish = resolve; }));
  const pending = f.context.apply();
  f.input('later'); await f.context.apply();
  assert.equal(f.context.draftPath, 'new');
  assert.equal(f.writes.length, 1);
  f.publish('new');
  finish({ action: 'update node data', status: 'applied', sessionId: 'session' });
  await pending;
  assert.equal(f.context.applied, true);
  assert.equal(f.context.conflict, false);
});

test('own saved response arriving after the awaited mutation retains its acknowledgement', async () => {
  const f = fixture('old'); f.input('new');
  await f.context.apply();
  f.publish('new');
  assert.equal(f.context.applied, true);
  f.publish('different');
  assert.equal(f.context.applied, false);
});

test('node replacement and unmount ignore an old mutation completion', async () => {
  for (const change of ['replace', 'destroy']) {
    const f = fixture('old'); f.input('submitted');
    let finish!: (value: JsonFilterMutationResult) => void;
    f.respond(new Promise(resolve => { finish = resolve; }));
    const pending = f.context.apply();
    if (change === 'replace') f.publish('other.path', 'other-filter'); else f.context.destroy();
    finish({ action: 'update node data', status: 'failed', sessionId: 'session', error: 'old failure' });
    await pending;
    assert.equal(f.context.applied, false);
    assert.equal(f.context.error, null);
    if (change === 'replace') assert.equal(f.context.draftPath, 'other.path');
  }
});

test('saved path changes during Apply cannot be presented as the requested path success', async () => {
  const f = fixture('old'); f.input('submitted');
  let finish!: (value: JsonFilterMutationResult) => void;
  f.respond(new Promise(resolve => { finish = resolve; }));
  const pending = f.context.apply(); f.publish('other.saved');
  finish({ action: 'update node data', status: 'applied', sessionId: 'session' });
  await pending;
  assert.equal(f.context.applied, false);
  assert.equal(f.context.conflict, true);
  assert.equal(f.context.draftPath, 'submitted');
});

test('the latest saved path determines conflict even after an own response while Apply is pending', async () => {
  for (const paths of [['submitted', 'other.saved'], ['other.saved', 'submitted']]) {
    const f = fixture('old'); f.input('submitted');
    let finish!: (value: JsonFilterMutationResult) => void;
    f.respond(new Promise(resolve => { finish = resolve; }));
    const pending = f.context.apply();
    for (const path of paths) f.publish(path);
    finish({ action: 'update node data', status: 'applied', sessionId: 'session' });
    await pending;
    assert.equal(f.context.draftPath, 'submitted');
    assert.equal(f.context.conflict, paths.at(-1) !== 'submitted');
    assert.equal(f.context.applied, paths.at(-1) === 'submitted');
  }
});

test('the shared property form accepts the scoped group Apply callback without a root-node mutation', async () => {
  const f=fixture('old');const scoped:unknown[]=[];
  f.context.applyNodeData=async (id:string,patch:Record<string,unknown>)=>{
    scoped.push({id,patch});return {action:'update group node data',status:'applied',sessionId:'owner'};
  };
  f.input('nested.text');await f.context.apply();
  assert.deepEqual(JSON.parse(JSON.stringify(scoped)),[{id:'filter',patch:{path:'nested.text'}}]);
  assert.equal(f.writes.length,0);assert.equal(f.context.applied,true);
});

test('an invalid whole node-data value requires explicit Apply even for an empty repair path',async()=>{
  const f=fixture();f.context.invalidData=true;
  assert.equal(f.writes.length,0);assert.match(f.context.savedPath.error,/must be an object/);
  await f.context.apply();
  assert.deepEqual(f.writes,[{nodeId:'filter',data:{path:''}}]);
});

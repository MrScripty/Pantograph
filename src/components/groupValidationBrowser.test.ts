import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, existsSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { compile } from 'svelte/compiler';

const evidence = process.env.GROUP_VALIDATION_BROWSER_EVIDENCE_DIR;
interface CdpResult { result?: { value?: unknown }; exceptionDetails?: unknown }
const root = fileURLToPath(new URL('../../', import.meta.url));

// The only summaries and lifecycle events below come from the freshly executed Rust public tests.
// Store/IPC/flow-handle stubs isolate the actual toolbar/panel/card in a browser; no desktop claim.
const entry = `
import { mount, unmount, tick } from 'svelte';
import Toolbar from './src/components/WorkflowToolbar.svelte';
import Card from './src/components/nodes/workflow/NodeGroupNode.svelte';
import * as stores from 'test-stores';
const cases = await (await fetch('/cases')).json();
const checks = [];
const check = (condition, message) => { if (!condition) throw new Error(message); checks.push(message); };
const settle = async () => { await Promise.resolve(); await tick(); await new Promise(r => setTimeout(r, 0)); await tick(); };
let mounted = [];
window.harness = { requests: [], overlays: [], lifecycle: null, closeCalls: 0, authored: null };
const h = window.harness;
h.request = (method, body) => new Promise((resolve, reject) => h.requests.push({method, body, resolve, reject, done:false}));
const pending = method => h.requests.find(r => r.method === method && !r.done);
const resolve = (method, summary) => { const r = pending(method); check(!!r, 'pending '+method); r.done = true; r.resolve({summary, node_projections:[]}); };
const reject = method => { const r = pending(method); r.done = true; r.reject(new Error('Controlled old transport failure')); };
const button = () => document.querySelector('[data-testid=workflow-submit-button]');
const emit = event => { check(!!h.lifecycle, 'actual lifecycle subscription installed'); h.lifecycle({payload:{event}}); };
async function reset(data, summary) {
  for (const item of mounted) await unmount(item);
  mounted = []; document.body.innerHTML = '<main id="toolbar"></main><aside id="card"></aside>';
  h.requests = []; h.overlays = []; h.closeCalls = 0; h.authored = JSON.stringify(data.authored);
  stores.currentSessionId.set(summary.graph_session_id); stores.currentGraphId.set('preflight'); stores.currentGraphType.set('workflow');
  check(data.authored.derived_graph.graph_fingerprint === summary.current_graph_revision, 'producer authored fingerprint matches current response');
  stores.workflowGraph.set(data.authored);
  stores.availableWorkflows.set([{id:'preflight', name:'preflight'}]); stores.isDirty.set(false); stores.isExecuting.set(false);
  mounted.push(mount(Toolbar, {target:document.getElementById('toolbar')}));
  const group = data.authored.nodes.find(n => n.node_type === 'node-group');
  if (group) mounted.push(mount(Card, {target:document.getElementById('card'), props:{id:group.id, data:group.data}}));
  await settle(); check(button().disabled, 'submit disabled before producer read');
}
async function accept(data, summary) { await reset(data, summary); resolve('refresh', summary); await settle(); }
const eventOf = (data, kind, generation) => data.events.events.find(e => e.kind.kind === kind && (!generation || e.validation_session_id === generation));
window.runQualification = async () => {
  const valid = cases['valid-fanout'];
  await accept(valid, valid.summary);
  check(!button().disabled, 'real saved fanout enables Submit');
  const output = valid.authored.nodes.find(n => n.node_type === 'node-group').data.group.exposed_outputs;
  check(output.length === 2, 'producer retained both fanout mappings');
  check(document.querySelectorAll('[data-port="'+output[0].group_port_id+'"]').length === 1, 'actual card draws one fanout handle');
  check(JSON.stringify(valid.authored) === h.authored, 'card retains actual authored bytes');
  for (const name of Object.keys(cases).filter(name => name.startsWith('ui-'))) {
    const data = cases[name]; await accept(data, data.summary);
    const failure = data.summary.group_preflight.failures[0];
    const panel = document.querySelector('[data-testid=workflow-group-diagnostic]');
    check(button().disabled, name+' disables Submit');
    check(!!panel && panel.textContent.includes(failure.code) && panel.textContent.includes(failure.field), name+' renders typed code/field');
    check(panel.textContent.includes(failure.repair_hint), name+' renders original repair hint');
    if (failure.child_id) check(panel.textContent.includes(failure.child_id), name+' renders exact child');
    if (failure.group_id) check(panel.textContent.includes(failure.group_id), name+' renders exact group');
    button().click(); button().dispatchEvent(new MouseEvent('click',{bubbles:true})); await settle();
    check(!h.requests.some(r => ['create','publish','run'].includes(r.method)), name+' refuses before frontend effects');
    check(JSON.stringify(data.authored) === h.authored, name+' does not silently repair authored graph');
  }
  const canceled = cases['start-and-cancel'];
  const cancelEvent = eventOf(canceled, 'validation_cancelled');
  await accept(canceled, canceled.previous);
  emit(cancelEvent); await settle();
  check(button().disabled, 'cancel revokes allowed gate before projection returns');
  resolve('projection', canceled.canceled); await settle();
  check(button().disabled, 'real canceled response stays blocked');
  const generations = cases['canceled-scheduler-projections'];
  const pendingEvent = eventOf(generations, 'validation_superseded', generations.current.validation_session_id);
  check(!!pendingEvent, 'producer captured supersession');
  for (const late of ['success','error']) {
    await reset(generations, generations.previous);
    emit(pendingEvent); await settle(); resolve('projection', generations.current); await settle();
    check(!button().disabled, 'new real generation accepted before old '+late);
    if (late === 'success') resolve('refresh', generations.previous); else reject('refresh');
    await settle(); check(!button().disabled, 'old '+late+' cannot replace latest read');
    button().click(); await settle();
    const create = pending('create'); check(!!create, 'new generation may create metadata');
    create.done = true; create.resolve({session_id:'controlled-metadata'}); await settle();
    const publish = pending('publish'); check(!!publish, 'new generation reaches guarded publish');
    check(publish.body.validation_session_id === generations.current.validation_session_id, 'late '+late+' cannot restore old generation');
    reject('publish'); await settle(); check(h.closeCalls === 1, 'metadata closes after refused publication');
    check(!h.requests.some(r => r.method === 'run'), 'publication refusal prevents Run');
  }
  for (const late of ['success','error']) {
    await accept(generations, generations.previous);
    const oldAccepted = eventOf(generations, 'publication_accepted', generations.previous.validation_session_id);
    emit(oldAccepted); await settle(); const oldRead = pending('projection');
    emit(pendingEvent); await settle();
    const newRead = h.requests.filter(r => r.method === 'projection' && !r.done).at(-1);
    check(oldRead !== newRead, 'actual lifecycle events start distinct reads');
    newRead.done = true; newRead.resolve({summary:generations.current,node_projections:[]}); await settle();
    if (late === 'success') oldRead.resolve({summary:generations.previous,node_projections:[]});
    else oldRead.reject(new Error('Controlled old projection failure'));
    oldRead.done = true; await settle();
    check(!button().disabled, 'old projection '+late+' cannot replace new generation');
    const reads = h.requests.length; emit(oldAccepted); emit(pendingEvent); await settle();
    check(h.requests.length === reads && !button().disabled, 'real subscription rejects duplicate/out-of-order events');
    emit(cancelEvent); await settle();
    check(h.requests.length === reads && !button().disabled, 'real subscription rejects another graph session');
    button().click(); await settle(); const create = pending('create');
    create.done = true; create.resolve({session_id:'controlled-metadata'}); await settle();
    check(pending('publish').body.validation_session_id === generations.current.validation_session_id,
      'old projection '+late+' cannot restore previous publication ID');
    reject('publish'); await settle();
  }
  for (const boundary of ['create','publish']) {
    await accept(canceled, canceled.previous); button().click(); await settle();
    if (boundary === 'publish') { const create = pending('create'); create.done = true; create.resolve({session_id:'controlled-metadata'}); await settle(); }
    emit(cancelEvent); await settle(); resolve('projection', canceled.canceled); await settle();
    const held = pending(boundary); check(!!held, boundary+' await was held'); held.done = true;
    held.resolve(boundary === 'create' ? {session_id:'controlled-metadata'} : {}); await settle();
    check(!h.requests.some(r => r.method === 'run'), 'cancel during '+boundary+' prevents Run');
    if (boundary === 'create') check(!h.requests.some(r => r.method === 'publish'), 'cancel during create prevents publication');
    check(h.closeCalls === 1, 'cancel during '+boundary+' closes metadata');
  }
  for (const boundary of ['create','publish']) {
    await accept(generations, generations.previous);
    emit(pendingEvent); await settle(); resolve('projection', generations.current); await settle();
    check(!button().disabled, 'lifecycle-owned validation accepted before teardown');
    button().click(); await settle();
    if (boundary === 'publish') { const create = pending('create'); create.done = true; create.resolve({session_id:'controlled-metadata'}); await settle(); }
    await unmount(mounted.shift()); await settle();
    const held = pending(boundary); check(!!held, 'teardown holds '+boundary); held.done = true;
    held.resolve(boundary === 'create' ? {session_id:'controlled-metadata'} : {}); await settle();
    check(!h.requests.some(r => r.method === 'run'), 'teardown during '+boundary+' prevents Run');
    if (boundary === 'create') check(!h.requests.some(r => r.method === 'publish'), 'teardown during create prevents publication');
    check(h.closeCalls === 1, 'teardown during '+boundary+' closes metadata');
  }
  const edit = cases['semantic-edit'];
  await accept({authored:edit.previous_authored}, edit.previous);
  stores.workflowGraph.set(edit.current_authored);
  await settle(); check(button().disabled, 'actual semantic edit revokes Submit');
  resolve('refresh', edit.stale); await settle(); check(button().disabled, 'stale authored revision never enables Submit');
  const aba = cases['semantic-aba'];
  await reset(aba, aba.previous);
  const old = pending('refresh'); check(!!old && !old.done, 'original A read is genuinely pending');
  stores.workflowGraph.set(aba.changed_authored); await settle();
  const changedRead = h.requests.find(r => r.method === 'refresh' && r.body.graph_revision === aba.changed.current_graph_revision);
  changedRead.done = true; changedRead.resolve({summary:aba.changed,node_projections:[]}); await settle();
  check(!button().disabled, 'real edited B response accepted');
  stores.workflowGraph.set(aba.returned_authored); await settle();
  check(button().disabled, 'A-B-A edit waits for a new read');
  old.done = true; old.resolve({summary:aba.previous,node_projections:[]});
  await settle(); check(button().disabled, 'A-B-A old pending promise cannot enable Submit');
  resolve('refresh', aba.returned); await settle();
  check(!button().disabled, 'fresh returned A generation can enable Submit');
  check(aba.old_snapshot_refused, 'actual backend refuses original A publication after ABA');
  button().click(); await settle();
  const created = pending('create'); created.done = true; created.resolve({session_id:'controlled-metadata'}); await settle();
  check(pending('publish').body.validation_session_id === aba.returned.validation_session_id, 'ABA Submit captures the fresh returned A generation');
  reject('publish'); await settle();
  check(cases['keepalive-refusal'].runtime_load_calls === 0 && cases['forged-publication-refused'].snapshot_refused,
    'fresh backend tests independently refused acquisition/publication');
  check(cases['capacity-boundary-refusal'].changed_target_load_calls === 0 && cases['ephemeral-metadata-refusal'].queue_items === 0,
    'fresh backend tests refused before load/enqueue');
  return {checks, claims:'Fresh Rust public response to compiled Svelte Chromium replay; controlled IPC/stores/flow handles. No live Tauri RPC/native execution claim.'};
};
`;

test('fresh Rust public responses render in Chromium with generation-fenced Submit and backend refusal evidence', {
  skip: !evidence && 'Run scripts/check-group-validation-browser.mjs to create fresh producer evidence', timeout: 120_000,
}, async () => {
  assert.ok(evidence);
  const producer = readFileSync(join(evidence, 'producer.log'), 'utf8');
  assert.match(producer, /14 passed; 0 failed/);
  const manifest = JSON.parse(readFileSync(join(evidence, 'producer-response-manifest.json'), 'utf8')) as {cases:Array<{name:string;sha256:string}>};
  const cases: Record<string, unknown> = {};
  for (const file of manifest.cases) {
    const bytes: Buffer = readFileSync(join(evidence, file.name));
    assert.equal(createHash('sha256').update(bytes).digest('hex'), file.sha256);
    cases[file.name.replace(/\.json$/, '')] = JSON.parse(bytes.toString());
  }
  const mocks: Record<string, string> = {
    'test-stores': `import {writable} from 'svelte/store';
      export const isDirty=writable(false),isExecuting=writable(false),isReadOnly=writable(false),isEditing=writable(true),expandedGroupId=writable(null);
      export const edges=writable([]),workflowGraph=writable({nodes:[],edges:[]}),currentGraphId=writable(null),currentGraphType=writable('workflow'),currentSessionId=writable(null),availableWorkflows=writable([]);
      export const resetExecutionStates=()=>{},clearNodeRuntimeData=()=>{},clearStreamContent=()=>{},setNodeExecutionState=()=>{},appendStreamContent=()=>{},setStreamContent=()=>{},tabIntoGroup=()=>{};
      export const updateNodeData=()=>{throw new Error('Unexpected node edit')},updateGroupNodeData=()=>{throw new Error('Unexpected group edit')};
      export const updateNodeRuntimeData=(id,data)=>window.harness.overlays.push({id,data});
      export const focusWorkflowDiagnostics=()=>{},selectActiveWorkflowRun=()=>{},setWorkbenchPage=()=>{};`,
    'test-service': `export const workflowService={
      refreshCurrentGraphValidationSummary:body=>window.harness.request('refresh',body),currentGraphValidationProjection:body=>window.harness.request('projection',body),
      createWorkflowExecutionSession:body=>window.harness.request('create',body),publishGraphSessionExecutableValidationSnapshot:body=>window.harness.request('publish',body),
      runWorkflowExecutionSession:body=>window.harness.request('run',body),closeWorkflowExecutionSession:async()=>{window.harness.closeCalls++},subscribeEvents:()=>()=>{}};`,
    'test-event': `export async function listen(name,handler){window.harness.lifecycle=handler;return()=>{if(window.harness.lifecycle===handler)window.harness.lifecycle=null}}`,
    'test-persistence': `export default function Persistence(){}`,
    'test-flow': `export {default as Handle} from 'test-handle';export const Position={Left:'left',Right:'right'};`,
    'test-handle': compile('<script>export let id;</script><i data-port={id}></i>', { generate: 'client' }).js.code,
  };
  const bundle = await build({ stdin: { contents: entry, resolveDir: root, sourcefile: 'group-ui-replay-entry.js' }, absWorkingDir: root,
    bundle: true, write: false, format: 'esm', platform: 'browser', conditions: ['browser'], plugins: [{ name: 'controlled-boundaries', setup(plugin) {
      plugin.onResolve({ filter: /^(test-|@tauri-apps\/api\/event$|@xyflow\/svelte$)|\/stores\/(workflowStore|graphSessionStore|workbenchStore)$|\/WorkflowService$|WorkflowPersistenceControls\.svelte$/ }, args => {
        const path = args.path.startsWith('test-') ? args.path : args.path === '@tauri-apps/api/event' ? 'test-event' : args.path === '@xyflow/svelte' ? 'test-flow'
          : args.path.endsWith('/WorkflowService') ? 'test-service' : args.path.endsWith('WorkflowPersistenceControls.svelte') ? 'test-persistence' : 'test-stores';
        return { path, namespace: 'controlled' };
      });
      plugin.onLoad({ filter: /.*/, namespace: 'controlled' }, args => ({ contents: mocks[args.path], loader: 'js', resolveDir: root }));
      plugin.onLoad({ filter: /\.svelte$/ }, args => ({ contents: compile(readFileSync(args.path, 'utf8'), { filename: args.path, generate: 'client', css: 'injected' }).js.code, loader: 'js', resolveDir: join(args.path, '..') }));
    } }] });
  const javascript = bundle.outputFiles[0].contents;
  writeFileSync(join(evidence, 'compiled-browser-ui.js'), javascript);
  const server = createServer((request, response) => {
    response.setHeader('Content-Type', request.url === '/ui.js' ? 'text/javascript' : request.url === '/cases' ? 'application/json' : 'text/html');
    response.end(request.url === '/ui.js' ? javascript : request.url === '/cases' ? JSON.stringify(cases) : '<!doctype html><script type="module" src="/ui.js"></script>');
  });
  await new Promise<void>(done => server.listen(0, '127.0.0.1', done));
  const profile = mkdtempSync(join(tmpdir(), 'pantograph-chromium-'));
  const browser = spawn(process.env.CHROMIUM_PATH ?? '/usr/bin/chromium', ['--headless', '--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], { stdio: ['ignore','ignore','pipe'] });
  let stderr = ''; browser.stderr.on('data', chunk => { stderr += chunk; });
  let socket: WebSocket | undefined;
  try {
    const portFile = join(profile, 'DevToolsActivePort');
    for (let n = 0; !existsSync(portFile) && n < 100; n++) await new Promise(done => setTimeout(done, 50));
    assert.ok(existsSync(portFile), stderr);
    const port = readFileSync(portFile, 'utf8').split('\n')[0];
    const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    socket = new WebSocket(targets[0].webSocketDebuggerUrl);
    await new Promise<void>((done, fail) => { socket!.onopen = () => done(); socket!.onerror = () => fail(new Error(stderr)); });
    let next = 0;
    const pending = new Map<number, {resolve:(value:CdpResult)=>void;reject:(error:Error)=>void}>();
    socket.onmessage = event => { const message = JSON.parse(String(event.data)); if (message.id) {
      const action = pending.get(message.id); pending.delete(message.id);
      if (message.error) action?.reject(new Error(JSON.stringify(message.error))); else action?.resolve(message.result);
    } };
    const cdp = (method: string, params: object = {}) => new Promise<CdpResult>((resolve, reject) => { const id = ++next; pending.set(id,{resolve,reject}); socket!.send(JSON.stringify({id,method,params})); });
    await cdp('Page.enable');
    const address = server.address(); assert.ok(address && typeof address !== 'string');
    await cdp('Page.navigate', {url:`http://127.0.0.1:${address.port}`});
    let ready = false;
    for (let n = 0; n < 100; n++) { const result = await cdp('Runtime.evaluate', {expression:'typeof window.runQualification === "function"', returnByValue:true});
      if (result.result?.value === true) {ready=true;break;} await new Promise(done => setTimeout(done,50)); }
    assert.ok(ready, 'Actual Svelte browser bundle did not initialize');
    const result = await cdp('Runtime.evaluate', {expression:'window.runQualification()', awaitPromise:true, returnByValue:true});
    writeFileSync(join(evidence, 'browser-result.json'), JSON.stringify(result,null,2));
    assert.equal(result.exceptionDetails, undefined, JSON.stringify(result.exceptionDetails));
    const value = result.result?.value as { checks: string[] };
    assert.ok(Array.isArray(value?.checks) && value.checks.length > 100);
    console.log(`Chromium producer replay: ${value.checks.length} directed checks passed`);
  } finally { socket?.close(); browser.kill('SIGTERM'); server.close(); writeFileSync(join(evidence,'chromium.log'),stderr); }
});

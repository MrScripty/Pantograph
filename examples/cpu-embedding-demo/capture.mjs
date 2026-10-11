// Genuine DOM interaction with an explicitly approved, isolated local browser.
// No store injection, fake IPC, recorded responses, or replacement executor.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const output = path.resolve(process.argv[2] ?? path.join(root, 'docs/images/cpu-embedding-demo'));
const textOnly = process.argv.includes('--text');
const host = `http://127.0.0.1:${textOnly ? 8765 : 8766}`;
const workflow = textOnly ? 'editor-demo' : 'cpu-embedding-probe';
const inputText = textOnly ? 'Inspect fixture alignment before the next operation.' : 'hello';
const outputNode = textOnly ? 'out' : 'vectors';
const tabs = await (await fetch('http://127.0.0.1:9222/json')).json();
const ws = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
await new Promise(resolve => ws.addEventListener('open', resolve, { once: true }));
let nextId = 0;
const pending = new Map();
const requests = new Map();
const receipts = [];
const consoleErrors = [];
const exceptions = [];
const blockedRequests = [];
const screenshots = [];
const call = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++nextId;
  pending.set(id, { resolve, reject });
  ws.send(JSON.stringify({ id, method, params }));
});
ws.onmessage = async event => {
  const message = JSON.parse(event.data);
  const item = pending.get(message.id);
  if (item) {
    pending.delete(message.id);
    if (message.error) item.reject(new Error(JSON.stringify(message.error)));
    else item.resolve(message.result);
    return;
  }
  if (message.method === 'Page.javascriptDialogOpening') {
    await call('Page.handleJavaScriptDialog', { accept: true, promptText: workflow });
  }
  if (message.method === 'Fetch.requestPaused') {
    const { requestId, request } = message.params;
    const url = new URL(request.url);
    if (['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname) || ['data:', 'blob:'].includes(url.protocol)) {
      await call('Fetch.continueRequest', { requestId });
    } else {
      blockedRequests.push(request.url);
      await call('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' });
    }
  }
  if (message.method === 'Runtime.exceptionThrown') exceptions.push(message.params.exceptionDetails);
  if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') {
    consoleErrors.push(message.params.args.map(arg => arg.value ?? arg.description));
  }
  if (message.method === 'Network.requestWillBeSent' && message.params.request.method === 'POST' && message.params.request.url.includes('/api/workflow/commands/')) {
    requests.set(message.params.requestId, { url: message.params.request.url, request_body: message.params.request.postData, started_at: new Date().toISOString(), started_ms: performance.now() });
  }
  if (message.method === 'Network.responseReceived' && requests.has(message.params.requestId)) {
    Object.assign(requests.get(message.params.requestId), { status: message.params.response.status });
  }
  if (message.method === 'Network.loadingFinished' && requests.has(message.params.requestId)) {
    const response = requests.get(message.params.requestId);
    requests.delete(message.params.requestId);
    response.elapsed_ms = performance.now() - response.started_ms;
    delete response.started_ms;
    try {
      const body = await call('Network.getResponseBody', { requestId: message.params.requestId });
      receipts.push({ ...response, body: JSON.parse(body.body) });
    } catch (error) { receipts.push({ ...response, capture_error: String(error) }); }
  }
};
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
async function evaluate(expression) {
  const result = await call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function until(predicate, description) {
  for (let attempt = 0; attempt < 200; attempt++) {
    if (await predicate()) return;
    await wait(100);
  }
  throw new Error(`Timed out: ${description}`);
}
async function click(selector) {
  await until(() => evaluate(`Boolean(document.querySelector(${JSON.stringify(selector)}))`), selector);
  await evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`);
}
async function healthy() {
  const text = await evaluate('document.body.innerText');
  assert.ok(!/Transport error|Cannot read properties|Validation updates failed|Workflow submit failed/.test(text), text);
  const alerts = await evaluate(`[...document.querySelectorAll('[role="alert"]')].map(node => node.textContent.trim()).filter(Boolean)`);
  assert.deepEqual(alerts, [], 'visible application alerts');
}
async function screenshot(name) {
  await healthy();
  const result = await call('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
  const bytes = Buffer.from(result.data, 'base64');
  await fs.writeFile(path.join(output, name + '.png'), bytes);
  screenshots.push({ file: name + '.png', sha256: createHash('sha256').update(bytes).digest('hex'), captured_at: new Date().toISOString(), visible_text: await evaluate('document.body.innerText') });
}
const received = command => receipts.findLast(item => item.url.endsWith('/' + command));
try {
  await fs.mkdir(path.dirname(output), { recursive: true });
  await fs.mkdir(output);
  await call('Page.enable');
  await call('Runtime.enable');
  await call('Network.enable');
  await call('Fetch.enable', { patterns: [{ urlPattern: '*' }] });
  const browser = await call('Browser.getVersion');
  await call('Emulation.setDeviceMetricsOverride', { width: 1800, height: 1200, deviceScaleFactor: 1, mobile: false });
  await call('Page.navigate', { url: `http://127.0.0.1:3001/?workflowHost=${encodeURIComponent(host)}` });
  await click('[data-testid="workbench-nav-graph"]');
  await until(() => received('create_workflow_execution_session')?.status === 200, 'host-scoped startup graph');
  const startupSession = received('create_workflow_execution_session');
  await click('[data-testid="workflow-graph-selector-toggle"]');
  await click(`[data-workflow-id="${workflow}"]`);
  await until(() => received('create_workflow_execution_session') !== startupSession, 'selected graph session');
  await wait(300);
  const input = '.text-input-node-wrapper textarea';
  await until(() => evaluate(`Boolean(document.querySelector(${JSON.stringify(input)}))`), 'real prompt editor');
  await evaluate(`{ const input = document.querySelector(${JSON.stringify(input)}); input.value = ${JSON.stringify(inputText)}; input.dispatchEvent(new Event('input', { bubbles: true })); }`);
  await until(() => received('update_node_data')?.status === 200, 'backend prompt mutation');
  await until(() => {
    const summary = received('refresh_current_graph_validation_summary')?.body.summary;
    return summary && summary.state !== 'stale' && summary.current_graph_revision === received('update_node_data').body.graph_revision;
  }, 'current authored graph validation');
  if (!textOnly) {
    await evaluate(`[...document.querySelectorAll('.svelte-flow__node[data-id="deps"] button')].find(button => button.textContent.trim() === 'Resolve')?.click()`);
    await until(() => received('resolve_dependency_environment_action_intent')?.status === 200, 'real dependency Resolve');
    assert.notEqual(received('resolve_dependency_environment_action_intent').body.status, 'blocked');
    await wait(400);
  }
  await evaluate(`[...document.querySelectorAll('button')].find(button => button.title === 'Save Workflow').click()`);
  await until(() => received('save_workflow')?.status === 200, 'backend workflow save');
  await until(() => evaluate(`document.querySelector('[data-testid="workflow-submit-button"]')?.disabled === false`), 'backend executable validation');
  await evaluate(`document.querySelector('.svelte-flow__controls-fitview')?.click()`);
  await wait(500);
  await screenshot('graph-before-submit');
  await click('[data-testid="workflow-submit-button"]');
  await until(() => {
    const publication = received('publish_graph_session_executable_validation_snapshot');
    if (publication?.status >= 400) throw new Error(JSON.stringify(publication.body));
    return received('workflow_run_execution_session') !== undefined;
  }, 'real scheduler response');
  const runReceipt = received('workflow_run_execution_session');
  assert.equal(runReceipt.status, 200, JSON.stringify(runReceipt));
  const result = runReceipt.body;
  const actual = result.outputs.find(item => item.node_id === outputNode)?.value;
  const golden = textOnly ? inputText : JSON.parse(await fs.readFile(path.join(root, 'crates/inference/tests/fixtures/candle_bert/bert-8/golden.json'), 'utf8')).single_vectors[1];
  if (textOnly) assert.equal(actual, golden);
  else {
    assert.equal(actual.length, 8);
    assert.ok(actual.every((value, index) => Math.abs(value - golden[index]) < 1e-4));
  }
  await until(() => {
    const nodes = received('workflow_node_status_query')?.body.nodes ?? [];
    return (textOnly ? [outputNode] : ['infer', outputNode]).every(id => nodes.some(node => node.node_id === id && node.status === 'completed' && node.workflow_run_id === result.workflow_run_id));
  }, 'actual completed node statuses');
  // Submit selects the completed run in Scheduler; return via the real sidebar
  // to verify its output overlay on the original authored Graph.
  await click('[data-testid="workbench-nav-graph"]');
  const graphPreview = textOnly ? actual : actual.slice(0, 8).map(value => value.toFixed(4)).join(', ');
  await until(() => evaluate(`document.querySelector('.svelte-flow__node[data-id="${outputNode}"] .copyable-output')?.textContent.includes(${JSON.stringify(graphPreview)})`), 'actual output rendered in Graph');
  await screenshot('graph-completed');
  await click('[data-testid="workbench-nav-scheduler"]');
  await until(() => received('workflow_run_list_query')?.body.runs?.some(run => run.workflow_run_id === result.workflow_run_id && run.status === 'completed'), 'actual completed scheduler run');
  await wait(500);
  await screenshot('scheduler');
  await click('[data-testid="workbench-nav-network"]');
  await until(() => received('workflow_local_network_status_query')?.status === 200, 'actual local Network metrics');
  const network = received('workflow_local_network_status_query').body;
  assert.equal(network.local_node.transport_state, 'local_only');
  assert.ok(network.local_node.system.memory.total_bytes > 0);
  await wait(500);
  await screenshot('network');
  await click('[data-testid="workbench-nav-io_inspector"]');
  await until(() => evaluate(`[...document.querySelectorAll('svg g[role="button"]')].some(node => node.textContent.trim().startsWith(${JSON.stringify(outputNode)}))`), 'real run graph');
  await evaluate(`[...document.querySelectorAll('svg g[role="button"]')].find(node => node.textContent.trim().startsWith(${JSON.stringify(outputNode)})).dispatchEvent(new MouseEvent('click', { bubbles: true }))`);
  await wait(300);
  await evaluate(`[...document.querySelectorAll('button')].find(button => button.textContent.trim() === 'Read' && !button.disabled).click()`);
  await until(() => received('workflow_read_artifact_body')?.status === 200, 'retained output body');
  const retainedBytes = Buffer.from(received('workflow_read_artifact_body').body.body).toString();
  const retained = textOnly ? retainedBytes : JSON.parse(retainedBytes);
  assert.deepEqual(retained, actual);
  await until(() => evaluate(`document.body.innerText.includes(${JSON.stringify(String(textOnly ? actual : actual[0]))})`), 'visible retained output');
  await screenshot('retained-output');
  await click('[data-testid="workbench-nav-diagnostics"]');
  await wait(700);
  await screenshot('diagnostics');
  assert.deepEqual(exceptions, [], 'uncaught browser exceptions');
  assert.deepEqual(consoleErrors, [], 'browser console errors');
  assert.deepEqual(blockedRequests, [], 'no external requests from the local application');
  assert.deepEqual(receipts.filter(item => item.status >= 400 || item.capture_error), [], 'all observed application commands succeeded');
  await fs.writeFile(path.join(output, 'receipt.json'), JSON.stringify({
    editor_qualified: true, scope: 'actual Chromium Graph edit, Save, Submit, output, Scheduler, Network and retained output',
    source_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    source_worktree_status: execFileSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }),
    browser, browser_sandbox_disabled_with_user_approval: true, isolated_profile: true,
    local_only_navigation: true, model_training: textOnly ? null : 'untrained', model_acquisition_bytes: 0,
    model: textOnly ? null : {
      id: 'embedding/qualification/synthetic-bert-8', architecture: 'BERT, one layer, hidden size 8',
      weights_bytes: (await fs.stat(path.join(root, 'crates/inference/tests/fixtures/candle_bert/bert-8/model.safetensors'))).size,
      weights_sha256: createHash('sha256').update(await fs.readFile(path.join(root, 'crates/inference/tests/fixtures/candle_bert/bert-8/model.safetensors'))).digest('hex'),
      backend: 'Candle', version: '0.9.2-alpha.2', commit: '88ed7911de9e88196b1f55b199145d22647f415e', device: 'CPU', dtype: 'F32',
      pumas_commit: 'aa1d79ae2d53f0acb088bc85223ecc2ff46af12b',
    },
    input: inputText, result, retained, max_reference_error: textOnly ? 0 : Math.max(...actual.map((value, index) => Math.abs(value - golden[index]))),
    screenshots, consoleErrors, exceptions, blockedRequests, receipts,
  }, null, 2) + '\n');
  console.log(`Qualified browser run ${result.workflow_run_id}; screenshots and actual receipts: ${output}`);
} catch (error) {
  try {
    await fs.writeFile(path.join(output, 'failure.json'), JSON.stringify({ error: String(error), visible_text: await evaluate('document.body.innerText'), receipts, exceptions, consoleErrors, blockedRequests }, null, 2));
    const shot = await call('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
    await fs.writeFile(path.join(output, 'failure.png'), Buffer.from(shot.data, 'base64'));
  } catch { /* Preserve the original failure. */ }
  throw error;
} finally { ws.close(); }

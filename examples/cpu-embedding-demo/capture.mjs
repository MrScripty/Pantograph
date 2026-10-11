// Genuine workbench interaction over CDP. Requires an approved sandboxed browser.
// No store injection, recorded responses, fake IPC, or replacement service.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const output = path.resolve(process.argv[2] ?? path.join(root, 'docs/images/cpu-embedding-demo'));
const tabs = await (await fetch('http://127.0.0.1:9222/json')).json();
const ws = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
await new Promise(resolve => ws.addEventListener('open', resolve, { once: true }));
let nextId = 0;
const pending = new Map();
const requests = new Map();
const receipts = [];
const call = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++nextId;
  pending.set(id, { resolve, reject });
  ws.send(JSON.stringify({ id, method, params }));
});
ws.onmessage = async event => {
  const message = JSON.parse(event.data);
  if (message.method === 'Page.javascriptDialogOpening') {
    await call('Page.handleJavaScriptDialog', { accept: true, promptText: 'cpu-embedding-probe' });
  }
  if (message.method === 'Network.responseReceived' && message.params.response.url.includes('/api/workflow/commands/')) {
    requests.set(message.params.requestId, message.params.response);
  }
  if (message.method === 'Network.loadingFinished' && requests.has(message.params.requestId)) {
    const response = requests.get(message.params.requestId);
    requests.delete(message.params.requestId);
    try {
      const body = await call('Network.getResponseBody', { requestId: message.params.requestId });
      receipts.push({ url: response.url, status: response.status, body: JSON.parse(body.body) });
    } catch (error) { receipts.push({ url: response.url, capture_error: String(error) }); }
  }
  const item = pending.get(message.id);
  if (item) {
    pending.delete(message.id);
    if (message.error) item.reject(new Error(JSON.stringify(message.error)));
    else item.resolve(message.result);
  }
};
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
async function evaluate(expression) {
  const result = await call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function until(predicate, description) {
  for (let attempt = 0; attempt < 150; attempt++) {
    if (await predicate()) return;
    await wait(100);
  }
  throw new Error(`Timed out: ${description}`);
}
async function click(selector) {
  await until(() => evaluate(`Boolean(document.querySelector(${JSON.stringify(selector)}))`), selector);
  await evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`);
}
async function screenshot(name) {
  const result = await call('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
  await fs.writeFile(path.join(output, name + '.png'), Buffer.from(result.data, 'base64'));
}
const received = command => receipts.findLast(item => item.url.endsWith('/' + command));
try {
  await fs.mkdir(output, { recursive: true });
  await call('Page.enable');
  await call('Network.enable');
  await call('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1100, deviceScaleFactor: 1, mobile: false });
  await call('Page.navigate', { url: 'http://127.0.0.1:3001/?workflowHost=http%3A%2F%2F127.0.0.1%3A8766' });
  await click('[data-testid="workbench-nav-graph"]');
  await click('[data-testid="workflow-graph-selector-toggle"]');
  await click('[data-workflow-id="cpu-embedding-probe"]');
  const input = '.svelte-flow__node[data-id="prompt"] textarea';
  await until(() => evaluate(`Boolean(document.querySelector(${JSON.stringify(input)}))`), 'real prompt editor');
  await evaluate(`{ const input = document.querySelector(${JSON.stringify(input)}); input.value = 'hello'; input.dispatchEvent(new Event('input', { bubbles: true })); }`);
  await until(() => received('update_node_data')?.status === 200, 'backend prompt mutation');
  await evaluate(`document.querySelector('.svelte-flow__controls-fitview')?.click()`);
  await screenshot('editor-before-submit');
  await evaluate(`[...document.querySelectorAll('.svelte-flow__node[data-id="deps"] button')].find(button => button.textContent.trim() === 'Resolve')?.click()`);
  await until(() => received('resolve_dependency_environment_action_intent')?.status === 200, 'real dependency Resolve');
  await wait(400);
  await evaluate(`[...document.querySelectorAll('button')].find(button => button.title === 'Save Workflow').click()`);
  await until(() => received('save_workflow')?.status === 200, 'backend workflow save');
  await until(() => evaluate(`document.querySelector('[data-testid="workflow-submit-button"]')?.disabled === false`), 'backend executable validation');
  await click('[data-testid="workflow-submit-button"]');
  await until(() => received('workflow_run_execution_session') !== undefined, 'real scheduler response');
  const runReceipt = received('workflow_run_execution_session');
  assert.equal(runReceipt.status, 200, JSON.stringify(runReceipt));
  const result = runReceipt.body;
  const vector = result.outputs.find(item => item.node_id === 'vectors').value;
  const golden = JSON.parse(await fs.readFile(path.join(root, 'crates/inference/tests/fixtures/candle_bert/bert-8/golden.json'), 'utf8')).single_vectors[1];
  assert.equal(vector.length, 8);
  assert.ok(vector.every((value, index) => Math.abs(value - golden[index]) < 1e-4));
  await click('[data-testid="workbench-nav-io_inspector"]');
  await until(() => {
    const nodes = received('workflow_node_status_query')?.body.nodes ?? [];
    return ['infer', 'vectors'].every(id => nodes.some(node => node.node_id === id && node.status === 'completed'));
  }, 'actual inference and output node completion');
  await until(() => evaluate(`[...document.querySelectorAll('svg g[role="button"]')].some(node => node.textContent.trim().startsWith('vectors'))`), 'real run graph');
  await evaluate(`[...document.querySelectorAll('svg g[role="button"]')].find(node => node.textContent.trim().startsWith('vectors')).dispatchEvent(new MouseEvent('click', { bubbles: true }))`);
  await wait(300);
  await evaluate(`[...document.querySelectorAll('button')].find(button => button.textContent.trim() === 'Read' && !button.disabled).click()`);
  await until(() => received('workflow_read_artifact_body')?.status === 200, 'retained output body');
  const retained = JSON.parse(Buffer.from(received('workflow_read_artifact_body').body.body).toString());
  assert.deepEqual(retained, vector);
  await until(() => evaluate(`document.body.innerText.includes(${JSON.stringify(String(vector[0]))})`), 'visible actual vector');
  const visibleText = await evaluate('document.body.innerText');
  assert.ok(!/Transport error|Cannot read properties|Validation updates failed|Workflow submit failed/.test(visibleText));
  await screenshot('visible-vector');
  await click('[data-testid="workbench-nav-diagnostics"]');
  await wait(700);
  await screenshot('scheduler-diagnostics');
  await fs.writeFile(path.join(output, 'receipt.json'), JSON.stringify({
    editor_qualified: true, scope: 'actual Chromium editor Submit and visible retained output',
    model_training: 'untrained', model_acquisition_bytes: 0, result, retained, receipts,
  }, null, 2) + '\n');
  console.log(`Qualified browser run ${result.workflow_run_id}; screenshots and real receipts: ${output}`);
} finally { ws.close(); }

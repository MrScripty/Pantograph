// Genuine Chromium captures. Requires loopback demo, Vite, and a CDP browser on 9222.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const output = path.join(root, 'docs/images');
const tabs = await (await fetch('http://127.0.0.1:9222/json')).json();
const ws = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
await new Promise(resolve => ws.addEventListener('open', resolve, { once: true }));
let nextId = 0;
const pending = new Map();
ws.onmessage = event => {
  const message = JSON.parse(event.data);
  const item = pending.get(message.id);
  if (!item) return;
  pending.delete(message.id);
  if (message.error) item.reject(new Error(JSON.stringify(message.error)));
  else item.resolve(message.result);
};
const call = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++nextId;
  pending.set(id, { resolve, reject });
  ws.send(JSON.stringify({ id, method, params }));
});
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
async function evaluate(expression) {
  const result = await call('Runtime.evaluate', {
    expression: `(async () => { ${expression} })()`, awaitPromise: true, returnByValue: true,
  });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function navigate(url) {
  await call('Page.navigate', { url });
  await wait(2500);
}
async function screenshot(name, fullPage = false, cropHeight) {
  const { cssContentSize } = await call('Page.getLayoutMetrics');
  const clip = fullPage || cropHeight ? { x: 0, y: 0, width: 1600, height: cropHeight ?? cssContentSize.height, scale: 1 } : undefined;
  const image = await call('Page.captureScreenshot', { format: 'png', captureBeyondViewport: fullPage, ...(clip ? { clip } : {}) });
  await fs.writeFile(path.join(output, `${name}.png`), Buffer.from(image.data, 'base64'));
}
// This script drives existing application actions; never inject execution results.
const message = 'Inspect fixture alignment before the next operation.';
async function until(expression, description) {
  for (let attempt = 0; attempt < 100; attempt++) {
    if (await evaluate(`return Boolean(${expression});`)) return;
    await wait(200);
  }
  throw new Error(`Timed out: ${description}`);
}
async function healthy() {
  const text = await evaluate('return document.body.innerText;');
  assert.ok(!/Transport error|Cannot read properties|Validation updates failed/.test(text), text);
  const alerts = await evaluate("return [...document.querySelectorAll('[role=alert]')].map(node => node.textContent.trim()).filter(Boolean);");
  assert.deepEqual(alerts, []);
}
try {
  await call('Page.enable');
  await call('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await fs.mkdir(output, { recursive: true });
  await navigate('http://127.0.0.1:3001/?workflowHost=http%3A%2F%2F127.0.0.1%3A8765');
  await evaluate(`
    const session = await import('/src/stores/graphSessionStore.ts');
    if (!await session.loadWorkflowByName('editor-demo')) throw new Error('Saved editor demo did not load');
    (await import('/src/stores/workbenchStore.ts')).setWorkbenchPage('graph');
  `);
  await until("document.querySelector('.text-input-node-wrapper textarea')", 'editable Text Input');
  await evaluate(`
    const input = document.querySelector('.text-input-node-wrapper textarea');
    input.value = ${JSON.stringify(message)};
    input.dispatchEvent(new Event('input', { bubbles: true }));
  `);
  await wait(500);
  await evaluate(`
    const save = [...document.querySelectorAll('button')].find(button => button.textContent.trim() === 'Save');
    if (!save || save.disabled) throw new Error('Save action unavailable');
    save.click();
  `);
  await until("[...document.querySelectorAll('button')].some(button => button.textContent.trim() === 'Submit' && !button.disabled)", 'validated saved graph with enabled Submit');
  await healthy();
  await evaluate("[...document.querySelectorAll('button')].find(button => button.textContent.trim() === 'Submit').click();");
  await until("document.body.innerText.includes('completed') && !document.body.innerText.includes('Submitting...')", 'real completed submission');
  await evaluate("(await import('/src/stores/workbenchStore.ts')).setWorkbenchPage('scheduler');");
  await wait(1000);
  await healthy();
  await screenshot('scheduler-workbench');
  const report = await evaluate(`
    const { activeWorkflowRun } = await import('/src/stores/workbenchStore.ts');
    let run; const unsubscribe = activeWorkflowRun.subscribe(value => { run = value; }); unsubscribe();
    if (!run) throw new Error('No actual submitted run selected');
    const { workflowService } = await import('/src/services/workflow/WorkflowService.ts');
    return { run, detail: await workflowService.queryRunDetail({ workflow_run_id: run.workflow_run_id }),
      artifacts: await workflowService.queryIoArtifacts({ workflow_run_id: run.workflow_run_id }) };
  `);
  assert.equal(report.detail.run.status, 'completed');
  await fs.writeFile(path.join(output, 'editor-workflow-run.json'), JSON.stringify(report, null, 2) + '\n');
  await evaluate("(await import('/src/stores/workbenchStore.ts')).setWorkbenchPage('graph');");
  await until(`document.querySelector('.copyable-output')?.textContent.includes(${JSON.stringify(message)})`, 'actual submitted output rendered in Graph');
  await healthy();
  assert.equal(await evaluate("return document.querySelector('[data-testid=workflow-submit-disabled-reason]')?.textContent ?? null;"), null);
  await evaluate("document.querySelector('.svelte-flow__controls-fitview')?.click();");
  await wait(500);
  await screenshot('editor-workflow-completed');
  await evaluate("(await import('/src/stores/workbenchStore.ts')).setWorkbenchPage('network');");
  await until("document.body.innerText.includes('Local')", 'real local Network status');
  await wait(1200);
  await healthy();
  await screenshot('network-workbench');
  console.log(`Captured actual saved editor submission, output, Scheduler and local Network: ${report.run.workflow_run_id}`);
} finally { ws.close(); }

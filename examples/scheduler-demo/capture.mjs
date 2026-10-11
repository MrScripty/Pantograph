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
try {
  await call('Page.enable');
  await call('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await fs.mkdir(output, { recursive: true });
  let success;
  for (const [name, invalid] of [['success', false], ['rejection', true]]) {
    await navigate('http://127.0.0.1:8765');
    const report = await evaluate(`
      ${invalid ? "document.getElementById('invalid').click();" : ''}
      await document.getElementById('run').onclick();
      return JSON.parse(document.getElementById('raw').textContent);
    `);
    assert.equal(report.execution.ok, !invalid);
    assert.ok(report.diagnostics.some(event => event.event_kind === 'run_terminal'));
    if (!invalid) {
      success = report;
      assert.equal(report.execution.result.outputs[0].value, report.input.items[0].prompt);
      assert.ok(report.diagnostics.some(event => event.node_id === 'filter' && event.event_kind === 'scheduler_task_attempt_lifecycle_changed'));
      assert.equal(report.projections.queryRunDetail.run.workflow_run_id, report.execution.result.workflow_run_id);
      assert.ok(report.projections.queryIoArtifacts.artifacts.length >= 3);
      assert.ok(report.projections.queryRunInspection.run_graph);
    }
    await fs.writeFile(path.join(output, `scheduler-demo-${name}.json`), JSON.stringify(report, null, 2) + '\n');
    await screenshot(`scheduler-demo-${name}`, true);
  }
  await navigate('http://127.0.0.1:3001');
  // Load actual backend graph/definitions through the app's normal store action.
  // Only layout is adjusted; do not inject execution states, IPC, or output overlays.
  const counts = await evaluate(`
    const graph = ${JSON.stringify(success.graph)};
    graph.nodes.forEach((node, index) => { node.position = { x: index * 500, y: 180 }; });
    const stores = await import('/src/stores/workflowStore.ts');
    const session = await import('/src/stores/graphSessionStore.ts');
    stores.nodeDefinitions.set(graph.nodes.map(node => node.data.definition));
    stores.loadWorkflow(graph);
    session.currentGraphName.set('Scheduler JSON extraction · browser rendering');
    session.currentGraphType.set('workflow');
    (await import('/src/stores/workbenchStore.ts')).setWorkbenchPage('graph');
    await new Promise(resolve => setTimeout(resolve, 1200));
    document.querySelector('.svelte-flow__controls-fitview')?.click();
    await new Promise(resolve => setTimeout(resolve, 700));
    return { nodes: document.querySelectorAll('.svelte-flow__node').length, edges: document.querySelectorAll('.svelte-flow__edge').length, alignedFilters: [...document.querySelectorAll('.svelte-flow__edge path[id]')].map(edge => edge.getAttribute('filter')) };
  `);
  assert.equal(counts.nodes, 3);
  assert.equal(counts.edges, 2);
  assert.deepEqual(counts.alignedFilters, [null, null], 'aligned wires must not disappear inside zero-area SVG filters');
  await screenshot('node-editor');
  // Replay only native query responses into the real workbench components.
  // This is recorded backend evidence, not a live Tauri connection. Native
  // event subscriptions and Pumas IPC remain unavailable and visibly report it.
  await evaluate(`
    const projections = ${JSON.stringify(success.projections)};
    const { workflowService } = await import('/src/services/workflow/WorkflowService.ts');
    for (const [method, response] of Object.entries(projections)) {
      workflowService[method] = async () => structuredClone(response);
    }
    const originalInspection = workflowService.queryRunInspection;
    workflowService.queryRunInspection = async () => {
      const response = await originalInspection();
      return { ...response, node_statuses: response.node_statuses ?? [],
        io_artifacts: response.io_artifacts ?? [], retention_summary: response.retention_summary ?? [] };
    };
    const workbench = await import('/src/stores/workbenchStore.ts');
    workbench.selectActiveWorkflowRun(projections.queryRunDetail.run);
  `);
  for (const [page, name, expected] of [
    ['diagnostics', 'execution-ledger', 'Diagnostics'],
    ['io_inspector', 'workflow-run-inspector', 'I/O Inspector'],
    ['library', 'model-library', 'Pumas Models'],
    ['node_lab', 'node-editor-status', 'Node Editor'],
  ]) {
    const text = await evaluate(`
      (await import('/src/stores/workbenchStore.ts')).setWorkbenchPage('${page}');
      await new Promise(resolve => setTimeout(resolve, 1000));
      return document.body.innerText;
    `);
    assert.ok(text.includes(expected), name + ' page did not render');
    if (page === 'io_inspector') {
      await evaluate(`
        [...document.querySelectorAll('svg g[role="button"]')].find(node => node.textContent.trim().startsWith('out'))?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
        await new Promise(resolve => setTimeout(resolve, 300));
      `);
    }
    await screenshot(name, false, page === 'node_lab' ? 400 : page === 'library' ? 700 : undefined);
    console.log(name + ': ' + text.slice(0, 350));
    if (page === 'diagnostics') {
      await evaluate(`
        [...document.querySelectorAll('h2')].find(node => node.textContent === 'Scheduler Timeline')?.scrollIntoView({ block: 'start' });
        await new Promise(resolve => setTimeout(resolve, 300));
      `);
      await screenshot('execution-ledger-timeline');
    }
  }
  console.log('Captured native success/rejection with raw JSON and real editor rendering (3 nodes, 2 edges).');
} finally { ws.close(); }

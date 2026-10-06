import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const evidence = process.env.PANTOGRAPH_NATIVE_CPU_EVIDENCE_DIR;
const fixtureRoot = process.env.PANTOGRAPH_NATIVE_CPU_FIXTURE_ROOT;
const selector = (id) => `[data-testid="${id}"]`;

async function invoke(command, args) {
  const response = await browser.executeAsync((name, parameters, done) => {
    const native = window.__TAURI_INTERNALS__;
    if (!native || typeof native.invoke !== 'function') {
      done({ error: 'Real Tauri IPC is unavailable; browser mocks are unsupported' });
      return;
    }
    native.invoke(name, parameters).then((value) => done({ value })).catch((error) => done({ error: String(error) }));
  }, command, args);
  assert.equal(response.error, undefined, response.error);
  return response.value;
}

async function openGraph(workflowId) {
  const navigation = await $(selector('workbench-nav-graph'));
  await navigation.waitForDisplayed({ timeout: 30000 });
  await navigation.click();
  await $(selector('workflow-editor-graph-page')).waitForDisplayed({ timeout: 30000 });
  await $(selector('workflow-graph-selector-toggle')).click();
  const option = await $(`${selector('workflow-graph-selector-option')}[data-workflow-id="${workflowId}"]`);
  await option.waitForDisplayed({ timeout: 30000 });
  await option.click();
}

async function readJsonArtifact(artifact) {
  const response = await invoke('workflow_read_artifact_body', { request: { artifact_id: artifact.artifact_id } });
  assert.ok(Array.isArray(response.body), 'Native retained artifact must contain bytes');
  return JSON.parse(Buffer.from(response.body).toString('utf8'));
}

describe('actual native Tauri saved CPU embedding graph', () => {
  it('saves, reopens, displays typed ports and submits synthetic weights through the public scheduler', async () => {
    assert.ok(evidence && fixtureRoot, 'Isolated evidence and fixture roots are required');
    const fixture = JSON.parse(readFileSync(path.join(fixtureRoot, 'fixture.json'), 'utf8'));
    const graph = JSON.parse(readFileSync(path.join(fixtureRoot, 'graph.json'), 'utf8'));
    await $(selector('workbench-nav-graph')).waitForDisplayed({ timeout: 30000 });
    // Observe the real owner responses without replacing commands, results or gates.
    await browser.execute(() => {
      const native = window.__TAURI_INTERNALS__;
      const original = native.invoke.bind(native);
      const commands = new Set(['get_execution_graph', 'current_graph_validation_summary',
        'current_graph_validation_projection', 'refresh_current_graph_validation_summary',
        'start_current_graph_validation_task']);
      window.__nativeCpuOwnerObservations = [];
      native.invoke = (command, args, ...rest) => {
        const pending = original(command, args, ...rest);
        if (!commands.has(command)) return pending;
        return pending.then((response) => {
          window.__nativeCpuOwnerObservations.push({ command, args, response });
          return response;
        }, (error) => {
          window.__nativeCpuOwnerObservations.push({ command, args, error: String(error) });
          throw error;
        });
      };
    });
    const savedPath = await invoke('save_workflow', { name: 'Synthetic CPU Embedding Qualification', graph });
    const restored = await invoke('load_workflow', { path: savedPath });
    assert.deepEqual(restored.graph.nodes, graph.nodes);
    assert.deepEqual(restored.graph.edges, graph.edges);
    const workflowId = path.basename(savedPath, '.json');
    writeFileSync(path.join(evidence, 'native-save-reopen.json'), JSON.stringify({ savedPath, workflowId, restored, fixture }, null, 2));
    await openGraph(workflowId);
    const infer = await $('[data-id="infer"]');
    await infer.waitForDisplayed({ timeout: 30000 });
    assert.match(await infer.getText(), /Synthetic CPU embedding/);
    await infer.$('[data-handleid="text"]').waitForDisplayed({ timeout: 30000 });
    await infer.$('[data-handleid="embedding"]').waitForDisplayed({ timeout: 30000 });
    const vector = await $('[data-id="vectors"]');
    await vector.waitForDisplayed({ timeout: 30000 });
    await vector.$('[data-handleid="vector"]').waitForDisplayed({ timeout: 30000 });
    const renderedEdges = await browser.execute(() =>
      Array.from(document.querySelectorAll('.svelte-flow__edge path[id]')).map((edge) => {
        const bounds = edge.getBBox();
        const style = getComputedStyle(edge);
        return { id: edge.id, length: edge.getTotalLength(), width: bounds.width,
          height: bounds.height, stroke: style.stroke, filter: style.filter };
      }));
    writeFileSync(path.join(evidence, 'native-rendered-edges.json'), JSON.stringify(renderedEdges, null, 2));
    assert.deepEqual(renderedEdges.map((edge) => edge.id).sort(), graph.edges.map((edge) => edge.id).sort());
    for (const edge of renderedEdges) {
      assert.ok(edge.length > 0 && edge.width > 0 && edge.height === 0, 'Aligned fixture edges must exercise zero-height geometry');
      assert.notEqual(edge.stroke, 'none');
      assert.match(edge.filter, /^drop-shadow\(/, 'Glow must avoid a zero-height objectBoundingBox filter region');
    }
    await browser.saveScreenshot(path.join(evidence, 'native-configured-graph.png'));

    // A seeded authored descriptor may require the normal visible update review
    // when the actual owner computes its current fingerprint. No gate is bypassed.
    let lastReason = null;
    let applied = false;
    await browser.waitUntil(async () => {
      const reason = await $(selector('workflow-submit-disabled-reason'));
      lastReason = await reason.isExisting() ? await reason.getText() : null;
      const update = await infer.$('.inference-update-button');
      if (!applied && await update.isExisting() && await update.isClickable()) {
        await update.click();
        applied = true;
        const save = await $('button[title="Save Workflow"]');
        await save.waitForClickable({ timeout: 30000 });
        await save.click();
        await browser.sendAlertText('Synthetic CPU Embedding Qualification');
        await browser.acceptAlert();
      }
      const submit = await $(selector('workflow-submit-button'));
      return await submit.isExisting() && !(await submit.getAttribute('disabled'));
    }, { timeout: 120000, timeoutMsg: 'Real desktop submit gate did not admit the saved graph' }).catch((error) => {
      writeFileSync(path.join(evidence, 'native-submit-blocker.json'), JSON.stringify({ lastReason, applied }, null, 2));
      throw error;
    });

    // The existing production GUI publishes validation and invokes
    // workflow_run_execution_session; no qualification executor is substituted.
    await $(selector('workflow-submit-button')).click();
    await $(selector('io-inspector-page')).waitForDisplayed({ timeout: 300000 });
    const runs = await invoke('workflow_run_list_query', { request: { workflow_id: workflowId, limit: 8 } });
    assert.equal(runs.runs.length, 1, 'One isolated native submission must produce one scoped run');
    const run = runs.runs[0];
    const runId = run.workflow_run_id;
    assert.equal(typeof runId, 'string');
    const inspection = await invoke('workflow_run_inspection_query', { request: { workflow_run_id: runId, artifact_limit: 64 } });
    const embedding = inspection.io_artifacts.find((item) => item.producer_node_id === 'infer' && item.producer_port_id === 'embedding');
    const metadata = inspection.io_artifacts.find((item) => item.producer_node_id === 'infer' && item.producer_port_id === 'metadata');
    assert.ok(embedding && metadata, 'Actual native owner must retain scoped vector and selection metadata');
    assert.equal(embedding.workflow_run_id, runId);
    const output = await readJsonArtifact(embedding);
    const selected = await readJsonArtifact(metadata);
    assert.ok(Array.isArray(output) && output.length === 8);
    output.forEach((value, index) => {
      assert.ok(Number.isFinite(value));
      assert.ok(Math.abs(value - fixture.expected_vector[index]) <= 1e-5, `Native CPU oracle mismatch at ${index}`);
    });
    assert.equal(selected.runtime_variant_id, 'candle.cpu');
    assert.deepEqual(selected.device_ids, ['cpu']);
    assert.equal(selected.model_ref.model_id, fixture.model_id);
    writeFileSync(path.join(evidence, 'native-output.json'), JSON.stringify({ synthetic_untrained: true, discovery: fixture.discovery, runId, output, selected, inspection }, null, 2));
    const cards = await $$(selector('io-artifact-card'));
    for (const card of cards) {
      const read = await card.$(selector('io-artifact-read-button'));
      if (await read.isExisting() && await read.isClickable()) await read.click();
    }
    await browser.saveScreenshot(path.join(evidence, 'native-cpu-output.png'));
    const finalGraph = await invoke('load_workflow', { path: savedPath });
    assert.deepEqual(finalGraph.graph.edges, graph.edges);
    writeFileSync(path.join(evidence, 'native-final-saved-graph.json'), JSON.stringify(finalGraph, null, 2));
  });
});

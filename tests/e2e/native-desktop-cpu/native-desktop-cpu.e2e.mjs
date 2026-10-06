import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { defaultVectorArtifact, completedCpuAttempt } from './native-output-contract.mjs';

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
    // Subscribe through the supported native event API; invoke is read-only.
    const validationHandler = await browser.execute(() => {
      const native = window.__TAURI_INTERNALS__;
      window.__nativeCpuValidationEvents = [];
      return native.transformCallback((event) => {
        window.__nativeCpuValidationEvents.push(event.payload.event);
      });
    });
    await invoke('plugin:event|listen', { event: 'workflow://graph-validation/lifecycle-event',
      target: { kind: 'Any' }, handler: validationHandler });
    // Compiled capability is discoverable before the typed scheduler loads it.
    const coldRegistry = await invoke('get_runtime_registry_snapshot');
    writeFileSync(path.join(evidence, 'native-cold-candle.json'), JSON.stringify(coldRegistry, null, 2));
    const candle = coldRegistry.runtimes.find((runtime) => runtime.runtime_id === 'candle');
    assert.ok(candle, 'The compiled Candle owner must be registered for typed scheduler loading');
    assert.equal(candle.status, 'stopped');
    assert.equal(candle.runtime_instance_id, null);
    assert.deepEqual(candle.models, []);
    assert.deepEqual(candle.active_reservation_ids, []);
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
      assert.ok(edge.length > 0 && edge.width > 0, 'Every saved edge must have visible geometry');
      if (edge.id !== 'deps-to-infer') assert.equal(edge.height, 0, 'Aligned pipeline edges must exercise zero-height geometry');
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
    const submissionEvents = await browser.execute(() => window.__nativeCpuValidationEvents || []);
    const activeValidation = submissionEvents.at(-1);
    if (activeValidation) {
      const projection = await invoke('current_graph_validation_projection', { request: {
        graph_session_id: activeValidation.graph_session_id, graph_revision: activeValidation.graph_revision,
      } });
      writeFileSync(path.join(evidence, 'native-submission-validation.json'), JSON.stringify({ projection, submissionEvents }, null, 2));
      // Resolve through the existing graph-associated producer; its owner records
      // the revision/session/model-scoped proof required by snapshot publication.
      const resolved = await invoke('resolve_dependency_environment_action_intent', { request: {
        contract_version: 1, graph_session_id: activeValidation.graph_session_id,
        graph_revision: activeValidation.graph_revision,
        validation_session_id: projection.summary.validation_session_id,
        target_node_id: 'deps', action: 'resolve',
      } });
      writeFileSync(path.join(evidence, 'native-dependency-resolution.json'), JSON.stringify(resolved, null, 2));
      assert.equal(resolved.status, 'request_ready', JSON.stringify(resolved));
      assert.equal(resolved.graph_session_id, activeValidation.graph_session_id);
      assert.equal(resolved.graph_revision, activeValidation.graph_revision);
      assert.equal(resolved.validation_session_id, projection.summary.validation_session_id);
      assert.equal(resolved.target_node_id, 'deps');
      assert.equal(resolved.action, 'resolve');
      // The contract omits empty diagnostics during serialization.
      assert.deepEqual(resolved.diagnostics ?? [], []);

    }
    await $(selector('workflow-submit-button')).click();
    let submissionError = null;
    try {
      await browser.waitUntil(async () => {
        const error = await $(selector('workflow-submit-error'));
        if (await error.isExisting()) {
          submissionError = await error.getAttribute('title') || await error.getText();
          return Boolean(submissionError);
        }
        const inspector = await $(selector('io-inspector-page'));
        if (await inspector.isExisting() && await inspector.isDisplayed()) return true;
        // Successful non-image runs use the existing Scheduler destination.
        return await $(selector('workbench-nav-scheduler')).getAttribute('aria-current') === 'page';
      }, { timeout: 30000, timeoutMsg: 'Native submission did not report success or its GUI error' });
      if (submissionError && !submissionError.includes('runtime dependency readiness is pending')) {
        assert.fail(submissionError);
      }
    } catch (error) {
      const runs = await invoke('workflow_run_list_query', { request: { workflow_id: workflowId, limit: 8 } })
        .catch((readError) => ({ error: String(readError) }));
      const body = await browser.execute(() => document.body.innerText);
      writeFileSync(path.join(evidence, 'native-submission-failure.json'), JSON.stringify({ submissionError,
        error: String(error), runs, body }, null, 2));
      // Retain the original failure, then observe the real producer's 60-second
      // poll without resubmitting or manufacturing a readiness result.
      if (submissionError?.includes('runtime dependency readiness is pending') && runs.runs?.length === 1) {
        const run = runs.runs[0];
        const samples = [];
        for (const delay of [0, 35000, 35000]) {
          if (delay) await browser.pause(delay);
          samples.push({
            capturedAt: new Date().toISOString(),
            runs: await invoke('workflow_run_list_query', { request: { workflow_id: workflowId, limit: 8 } }),
            scheduler: await invoke('workflow_get_scheduler_snapshot', {
              request: { session_id: run.workflow_execution_session_id },
            }).catch((readError) => ({ error: String(readError) })),
            inspection: await invoke('workflow_run_inspection_query', {
              request: { workflow_run_id: run.workflow_run_id, artifact_limit: 64 },
            }).catch((readError) => ({ error: String(readError) })),
          });
          writeFileSync(path.join(evidence, 'native-bootstrap-observation.json'), JSON.stringify(samples, null, 2));
        }
      }
      throw error;
    }
    const runs = await invoke('workflow_run_list_query', { request: { workflow_id: workflowId, limit: 8 } });
    assert.equal(runs.runs.length, 1, 'One isolated native submission must produce one scoped run');
    const run = runs.runs[0];
    const runId = run.workflow_run_id;
    assert.equal(typeof runId, 'string');
    if (submissionError) {
      assert.equal(run.workflow_execution_session_resume_state, 'dependency_readiness_pending', 'Pending GUI response must refer to the real deferred run');
      writeFileSync(path.join(evidence, 'native-initial-submission.json'), JSON.stringify({ submissionError, run }, null, 2));
    }
    let inspection;
    let artifactQuery;
    const samples = [];
    await browser.waitUntil(async () => {
      const current = await invoke('workflow_run_list_query', { request: { workflow_id: workflowId, limit: 8 } });
      assert.equal(current.runs.length, 1, 'Automatic bootstrap must preserve the single submitted run');
      assert.equal(current.runs[0].workflow_run_id, runId);
      assert.equal(current.runs[0].workflow_execution_session_id, run.workflow_execution_session_id);
      inspection = await invoke('workflow_run_inspection_query', { request: { workflow_run_id: runId, artifact_limit: 64 } });
      artifactQuery = await invoke('workflow_io_artifact_query', { request: { workflow_run_id: runId, limit: 64 } });
      assert.ok(Array.isArray(artifactQuery.artifacts), 'Public artifact query must return its canonical array');
      const scheduler = await invoke('workflow_get_scheduler_snapshot', { request: { session_id: run.workflow_execution_session_id } });
      samples.push({ capturedAt: new Date().toISOString(), runs: current, scheduler, inspection, artifactQuery });
      writeFileSync(path.join(evidence, 'native-bootstrap-observation.json'), JSON.stringify(samples, null, 2));
      return inspection.run.status === 'completed'
        && Boolean(defaultVectorArtifact(artifactQuery.artifacts, runId));
    }, { timeout: 120000, interval: 2000, timeoutMsg: 'The same native submitted run did not retain CPU output after automatic dependency bootstrap' });
    const vectorArtifact = defaultVectorArtifact(artifactQuery.artifacts, runId);
    assert.ok(vectorArtifact, 'Actual default GUI output must retain its scoped vector sink');
    const output = await readJsonArtifact(vectorArtifact);
    writeFileSync(path.join(evidence, 'native-vector-body.json'), JSON.stringify({ runId, vectorArtifact, output }, null, 2));
    assert.ok(Array.isArray(output) && output.length === 8);
    output.forEach((value, index) => {
      assert.ok(Number.isFinite(value));
      assert.ok(Math.abs(value - fixture.expected_vector[index]) <= 1e-5, `Native CPU oracle mismatch at ${index}`);
    });
    const timeline = await invoke('workflow_scheduler_timeline_query', { request: { workflow_run_id: runId, limit: 64 } });
    const selectionAttempt = completedCpuAttempt(timeline.events, runId);
    assert.ok(selectionAttempt, 'The same run must contain an actual completed Candle CPU runtime attempt');
    const boundModelRef = inspection.run_graph.graph.nodes.find((node) => node.id === 'infer').data.pumas_model_ref;
    assert.equal(boundModelRef.model_id, fixture.model_id);
    writeFileSync(path.join(evidence, 'native-output.json'), JSON.stringify({ synthetic_untrained: true,
      discovery: fixture.discovery, runId, output, vectorArtifact, selectionAttempt, timeline,
      bound_model_ref: boundModelRef, model_ref_source: 'saved executable graph; host selection/loader identity checks apply',
      internal_inference_metadata_exported: artifactQuery.artifacts.some((item) => item.producer_node_id === 'infer' && item.producer_port_id === 'metadata'),
      inspection, artifactQuery }, null, 2));
    await $(selector('workbench-nav-io_inspector')).click();
    await $(selector('io-inspector-page')).waitForDisplayed({ timeout: 30000 });
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

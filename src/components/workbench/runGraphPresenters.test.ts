import test from 'node:test';
import assert from 'node:assert/strict';

import type { WorkflowRunGraphProjection } from '../../services/workflow/types.ts';
import {
  buildRunGraphCanvasModel,
  buildRunGraphNodeStatusMap,
  formatRunGraphNodeCacheStatusLabel,
  formatRunGraphNodeStatusLabel,
  resolveRunGraphPresentationLabel,
  buildRunGraphNodeFacts,
  buildRunGraphSnapshotLayout,
  createRunGraphRequestGate,
  formatRunGraphDuration,
  formatRunGraphNodeRunSentence,
  resolveRunGraphSelectedNodeId,
  runGraphNodeGlyph,
  runGraphSettingRows,
  runGraphWireKind,
  clampRunGraphScale,
  fitRunGraphScale,
  RUN_GRAPH_CARD_HEADER,
  RUN_GRAPH_CARD_ROW,
  type RunGraphNodeStatusByNodeId,
} from './runGraphPresenters.ts';
import type { NodeStatusProjectionRecord } from '../../services/diagnostics/types.ts';

function createRunGraph(): WorkflowRunGraphProjection {
  return {
    workflow_run_id: 'run-1',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-1',
    workflow_presentation_revision_id: 'wfpres-1',
    workflow_semantic_version: '1.2.3',
    workflow_execution_fingerprint: 'fingerprint-1',
    snapshot_created_at_ms: 1_000,
    workflow_version_created_at_ms: 900,
    presentation_revision_created_at_ms: 950,
    graph: {
      nodes: [
        {
          id: 'input-1',
          node_type: 'text-input',
          position: { x: 10, y: 20 },
          data: { prompt: 'hello' },
        },
        {
          id: 'output-1',
          node_type: 'text-output',
          position: { x: 300, y: 20 },
          data: {},
        },
      ],
      edges: [
        {
          id: 'edge-1',
          source: 'input-1',
          source_handle: 'text',
          target: 'output-1',
          target_handle: 'value',
        },
      ],
    },
    executable_topology: {
      schema_version: 1,
      nodes: [
        {
          node_id: 'input-1',
          node_type: 'text-input',
          contract_version: '1.0.0',
          behavior_digest: 'digest-input',
        },
        {
          node_id: 'output-1',
          node_type: 'text-output',
          contract_version: '1.0.1',
          behavior_digest: 'digest-output',
        },
      ],
      edges: [
        {
          source_node_id: 'input-1',
          source_port_id: 'text',
          target_node_id: 'output-1',
          target_port_id: 'value',
        },
      ],
    },
    presentation_metadata: {
      schema_version: 1,
      nodes: [
        { node_id: 'input-1', position: { x: 10, y: 20 } },
        { node_id: 'output-1', position: { x: 300, y: 20 } },
      ],
      edges: [
        {
          edge_id: 'edge-1',
          source_node_id: 'input-1',
          source_port_id: 'text',
          target_node_id: 'output-1',
          target_port_id: 'value',
        },
      ],
    },
    graph_settings: {
      schema_version: 1,
      nodes: [
        { node_id: 'input-1', node_type: 'text-input', data: { prompt: 'hello' } },
        { node_id: 'output-1', node_type: 'text-output', data: {} },
      ],
    },
  };
}

test('resolveRunGraphPresentationLabel distinguishes complete and fallback layouts', () => {
  const runGraph = createRunGraph();
  assert.equal(resolveRunGraphPresentationLabel(runGraph), 'Presentation revision');

  runGraph.presentation_metadata.edges = [];
  assert.equal(resolveRunGraphPresentationLabel(runGraph), 'Generated layout fallback');
});

test('buildRunGraphCanvasModel derives stable viewbox and skips broken edges', () => {
  const runGraph = createRunGraph();
  runGraph.graph.edges.push({
    id: 'broken-edge',
    source: 'missing',
    source_handle: 'text',
    target: 'output-1',
    target_handle: 'value',
  });

  const canvas = buildRunGraphCanvasModel(runGraph.graph);

  assert.equal(canvas.viewBox, '-86 -76 672 296');
  assert.deepEqual(canvas.nodes.map((node) => node.id), ['input-1', 'output-1']);
  assert.deepEqual(canvas.edges.map((edge) => edge.id), ['edge-1']);
});

test('run graph presenters expose backend stale graph diagnostics', () => {
  const runGraph = createRunGraph();
  runGraph.graph_diagnostics = [
    {
      code: 'retired_node_type',
      severity: 'error',
      scope: 'node',
      node_id: 'output-1',
      node_type: 'diffusion-inference',
      message: 'node type diffusion-inference is retired',
      blocking_submission: true,
    },
    {
      code: 'missing_edge_target_node',
      severity: 'error',
      scope: 'edge',
      message: 'edge target node is missing',
      blocking_submission: true,
      details: { edge_id: 'edge-1', target_node_id: 'missing' },
    },
  ];

  const canvas = buildRunGraphCanvasModel(runGraph.graph, {}, {}, runGraph.graph_diagnostics);
  const outputNode = canvas.nodes.find((node) => node.id === 'output-1');
  assert.equal(outputNode?.staleDiagnosticCount, 1);
  assert.equal(outputNode?.staleBadgeLabel, '1 stale graph fact');
  assert.equal(canvas.edges[0].staleDiagnosticCount, 1);
  assert.equal(canvas.edges[0].staleSeverity, 'error');
});

test('buildRunGraphCanvasModel exposes artifact availability', () => {
  const summaries = {
    'output-1': {
      nodeId: 'output-1',
      inputCount: 0,
      outputCount: 1,
      artifactCount: 1,
      payloadRefCount: 1,
      latestEventSeq: 14,
      mediaTypes: ['image/png'],
    },
  };

  const canvas = buildRunGraphCanvasModel(createRunGraph().graph, summaries);
  const outputNode = canvas.nodes.find((node) => node.id === 'output-1');
  assert.equal(outputNode?.artifactCount, 1);
  assert.equal(outputNode?.artifactSummaryLabel, '1 output / 0 inputs');
  const inputNode = canvas.nodes.find((node) => node.id === 'input-1');
  assert.equal(inputNode?.artifactSummaryLabel, 'No retained I/O');
  assert.equal(outputNode?.hasOutputArtifacts, true);
});

test('buildRunGraphNodeStatusMap keeps the latest node status by event sequence', () => {
  const statuses = buildRunGraphNodeStatusMap([
    {
      workflow_run_id: 'run-1',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-1',
      workflow_semantic_version: '1.2.3',
      node_id: 'output-1',
      node_type: 'text-output',
      node_version: '1.0.1',
      runtime_id: null,
      runtime_version: null,
      model_id: null,
      model_version: null,
      execution_cache_status: null,
      status: 'running',
      started_at_ms: 1_000,
      completed_at_ms: null,
      duration_ms: null,
      error: null,
      last_event_seq: 10,
      last_updated_at_ms: 1_000,
    },
    {
      workflow_run_id: 'run-1',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-1',
      workflow_semantic_version: '1.2.3',
      node_id: 'output-1',
      node_type: 'text-output',
      node_version: '1.0.1',
      runtime_id: null,
      runtime_version: null,
      model_id: null,
      model_version: null,
      execution_cache_status: 'cache_hit',
      status: 'completed',
      started_at_ms: 1_000,
      completed_at_ms: 1_500,
      duration_ms: 500,
      error: null,
      last_event_seq: 12,
      last_updated_at_ms: 1_500,
    },
  ]);

  assert.equal(statuses['output-1'].status, 'completed');
  assert.equal(formatRunGraphNodeCacheStatusLabel(statuses['output-1'].execution_cache_status), 'Cache hit');
  assert.equal(formatRunGraphNodeStatusLabel(statuses['output-1'].status), 'Completed');

  const canvas = buildRunGraphCanvasModel(createRunGraph().graph, {}, statuses);
  const outputNode = canvas.nodes.find((node) => node.id === 'output-1');
  assert.equal(outputNode?.statusClass, 'completed');
  assert.equal(outputNode?.cacheStatusLabel, 'Cache hit');
});

test('run graph presenters expose node diagnostic error badges', () => {
  const statuses = buildRunGraphNodeStatusMap([
    {
      workflow_run_id: 'run-1',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-1',
      workflow_semantic_version: '1.2.3',
      node_id: 'output-1',
      node_type: 'text-output',
      node_version: '1.0.1',
      runtime_id: null,
      runtime_version: null,
      model_id: null,
      model_version: null,
      status: 'failed',
      started_at_ms: 1_000,
      completed_at_ms: 1_500,
      duration_ms: 500,
      error: 'node failed',
      error_event_id: 'error-event-1',
      error_severity: 'fatal',
      error_phase: 'node_execution',
      error_code: 'node_execution_failed',
      last_event_seq: 12,
      last_updated_at_ms: 1_500,
    },
  ]);

  const canvas = buildRunGraphCanvasModel(createRunGraph().graph, {}, statuses);
  const outputNode = canvas.nodes.find((node) => node.id === 'output-1');
  assert.equal(outputNode?.errorBadgeLabel, 'Fatal: node_execution_failed');
});

function nodeStatus(
  nodeId: string,
  status: NodeStatusProjectionRecord['status'],
  extra: Partial<NodeStatusProjectionRecord> = {},
): NodeStatusProjectionRecord {
  return {
    workflow_run_id: 'run-1',
    workflow_id: 'workflow-a',
    node_id: nodeId,
    status,
    last_event_seq: 1,
    last_updated_at_ms: 1_000,
    ...extra,
  };
}

function createDefinedRunGraph(): WorkflowRunGraphProjection {
  const runGraph = createRunGraph();
  runGraph.graph.nodes[0].data = {
    label: 'Prompt',
    definition: {
      label: 'Text Input',
      inputs: [],
      outputs: [
        { id: 'text', label: 'Text', data_type: 'string' },
        { id: 'unused', label: 'Unused', data_type: 'string' },
      ],
    },
  };
  runGraph.graph.nodes[1].data = {
    definition: {
      label: 'Text Output',
      inputs: [{ id: 'value', label: 'Value', data_type: 'string' }],
      outputs: [],
    },
  };
  return runGraph;
}

test('run node glyphs follow the status vocabulary and never guess without statuses', () => {
  assert.deepEqual(runGraphNodeGlyph(nodeStatus('a', 'running'), true), { glyph: 'running', label: 'Running' });
  assert.deepEqual(runGraphNodeGlyph(nodeStatus('a', 'completed'), true), { glyph: 'succeeded', label: 'Completed' });
  assert.deepEqual(runGraphNodeGlyph(nodeStatus('a', 'failed'), true), { glyph: 'failed', label: 'Failed' });
  assert.deepEqual(runGraphNodeGlyph(nodeStatus('a', 'cancelled'), true), { glyph: 'cancelled', label: 'Cancelled' });
  assert.deepEqual(runGraphNodeGlyph(nodeStatus('a', 'waiting'), true), { glyph: 'waiting', label: 'Waiting' });
  assert.deepEqual(runGraphNodeGlyph(null, true), { glyph: 'queued', label: 'Not reached' });
  assert.deepEqual(runGraphNodeGlyph(null, false), { glyph: 'idle', label: 'No status' });
});

test('wires leaving a running node are live and wires leaving a failed node are failed', () => {
  assert.equal(runGraphWireKind('running', 0), 'live');
  assert.equal(runGraphWireKind('failed', 2), 'failed');
  assert.equal(runGraphWireKind('completed', 1), 'stale');
  assert.equal(runGraphWireKind(null, 0), 'idle');
});

test('snapshot layout shifts positions, draws connected ports and anchors wires on them', () => {
  const runGraph = createDefinedRunGraph();
  const statuses: RunGraphNodeStatusByNodeId = { 'input-1': nodeStatus('input-1', 'running') };
  const layout = buildRunGraphSnapshotLayout(runGraph.graph, statuses, [], true);

  const input = layout.nodes.find((node) => node.id === 'input-1');
  const output = layout.nodes.find((node) => node.id === 'output-1');
  assert.ok(input && output);
  assert.equal(input.label, 'Prompt');
  assert.equal(output.label, 'Text Output');
  assert.equal(input.x, 48);
  assert.equal(input.y, 48);
  assert.equal(output.x, 338);
  assert.deepEqual(input.outputs.map((port) => port.id), ['text']);
  assert.equal(input.hiddenPortCount, 1);
  assert.equal(input.glyph, 'running');
  assert.equal(output.glyph, 'queued');
  assert.equal(output.stateLabel, 'Not reached');

  const [wire] = layout.wires;
  assert.equal(wire.kind, 'live');
  const sourceY = input.y + RUN_GRAPH_CARD_HEADER + 6 + RUN_GRAPH_CARD_ROW / 2;
  const targetY = output.y + RUN_GRAPH_CARD_HEADER + 6 + RUN_GRAPH_CARD_ROW / 2;
  assert.ok(wire.d.startsWith(`M ${input.x + input.width} ${sourceY} `));
  assert.ok(wire.d.endsWith(`${output.x} ${targetY}`));
  assert.ok(layout.width >= output.x + output.width);
});

test('snapshot layout falls back to edge handles when nodes carry no definition', () => {
  const runGraph = createRunGraph();
  const layout = buildRunGraphSnapshotLayout(
    runGraph.graph,
    { 'input-1': nodeStatus('input-1', 'failed', { error_code: 'oom', error_severity: 'fatal' }) },
    [],
    true,
  );
  const input = layout.nodes.find((node) => node.id === 'input-1');
  assert.deepEqual(input?.outputs, [{ id: 'text', label: 'text', dataType: null }]);
  assert.equal(input?.note, 'Fatal: oom');
  assert.equal(input?.noteDanger, true);
  assert.equal(layout.wires[0].kind, 'failed');
  assert.deepEqual(buildRunGraphSnapshotLayout({ nodes: [], edges: [] }), {
    width: 0,
    height: 0,
    nodes: [],
    wires: [],
  });
});

test('selected run node defaults to the running node, then the failed node, then the first', () => {
  const graph = createRunGraph().graph;
  assert.equal(resolveRunGraphSelectedNodeId('output-1', graph, {}), 'output-1');
  assert.equal(resolveRunGraphSelectedNodeId('missing', graph, {}), 'input-1');
  assert.equal(
    resolveRunGraphSelectedNodeId(null, graph, { 'output-1': nodeStatus('output-1', 'failed') }),
    'output-1',
  );
  assert.equal(
    resolveRunGraphSelectedNodeId(null, graph, {
      'input-1': nodeStatus('input-1', 'failed'),
      'output-1': nodeStatus('output-1', 'running'),
    }),
    'output-1',
  );
  assert.equal(resolveRunGraphSelectedNodeId(null, { nodes: [], edges: [] }, {}), null);
});

test('run node facts read captured settings, ports, contract and run status', () => {
  const runGraph = createDefinedRunGraph();
  runGraph.graph_settings.nodes[0].data = { prompt: 'hello', batch_size: 64, nested: { a: 1 } };
  const facts = buildRunGraphNodeFacts(runGraph, 'input-1', {
    'input-1': nodeStatus('input-1', 'completed', { duration_ms: 1_200, execution_cache_status: 'cache_hit' }),
  });
  assert.ok(facts);
  assert.equal(facts.label, 'Prompt');
  assert.equal(facts.glyph, 'succeeded');
  assert.deepEqual(facts.settings, [
    { key: 'Prompt', value: 'hello' },
    { key: 'Batch size', value: '64', mono: true },
  ]);
  assert.equal(facts.settingsNote, null);
  assert.deepEqual(facts.ports.map((port) => `${port.direction}:${port.id}`), ['out:text', 'out:unused']);
  assert.equal(facts.contractRows[0].value, '1.0.0');
  assert.equal(facts.runSentence, 'Completed in 1.2 s.');
  assert.deepEqual(facts.runRows, [{ key: 'Cache', value: 'Cache hit' }]);

  const output = buildRunGraphNodeFacts(runGraph, 'output-1', {});
  assert.equal(output?.settingsNote, 'No plain settings were captured for this node.');
  assert.equal(output?.runSentence, 'Not reached in this run.');
  assert.equal(buildRunGraphNodeFacts(runGraph, 'missing', {}), null);

  runGraph.graph_settings.nodes = [];
  assert.equal(
    buildRunGraphNodeFacts(runGraph, 'input-1', {}, false)?.settingsNote,
    'This run did not capture settings for this node.',
  );
});

test('run sentences explain failures in words and durations read naturally', () => {
  assert.equal(
    formatRunGraphNodeRunSentence(nodeStatus('a', 'failed', { error: 'out of memory, needed 14.2 GB.' })),
    'Failed: out of memory, needed 14.2 GB.',
  );
  assert.equal(
    formatRunGraphNodeRunSentence(nodeStatus('a', 'failed', { error_code: 'oom' })),
    'Failed: Error: oom.',
  );
  assert.equal(formatRunGraphNodeRunSentence(null, false), 'Node status is unavailable for this run.');
  assert.equal(formatRunGraphDuration(420), '420 ms');
  assert.equal(formatRunGraphDuration(38_000), '38.0 s');
  assert.equal(formatRunGraphDuration(663_000), '11m 03s');
});

test('setting rows keep plain scalars and shorten long text', () => {
  const long = 'x'.repeat(200);
  const rows = runGraphSettingRows({ label: 'skip', definition: {}, empty: ' ', flag: true, text: long });
  assert.deepEqual(rows.map((row) => row.key), ['Flag', 'Text']);
  assert.equal(rows[1].value.length, 120);
  assert.ok(rows[1].value.endsWith('…'));
  assert.deepEqual(runGraphSettingRows(null), []);
  assert.deepEqual(runGraphSettingRows(['a']), []);
});

test('request gate applies only the newest load and nothing after invalidation', () => {
  const gate = createRunGraphRequestGate();
  const first = gate.begin();
  const second = gate.begin();
  assert.equal(gate.isCurrent(first), false);
  assert.equal(gate.isCurrent(second), true);
  gate.invalidate();
  assert.equal(gate.isCurrent(second), false);

  const other = createRunGraphRequestGate();
  const otherToken = other.begin();
  assert.equal(other.isCurrent(otherToken), true, 'gates are scoped to their owner');
});

test('fit scale shrinks large snapshots to the viewport and never enlarges small ones', () => {
  assert.equal(fitRunGraphScale({ width: 1024, height: 524 }, { width: 2000, height: 500 }), 0.5);
  assert.equal(fitRunGraphScale({ width: 1200, height: 800 }, { width: 400, height: 300 }), 1);
  assert.equal(fitRunGraphScale({ width: 0, height: 0 }, { width: 400, height: 300 }), 1);
  assert.equal(fitRunGraphScale({ width: 124, height: 124 }, { width: 10_000, height: 10_000 }), 0.35);
  assert.equal(clampRunGraphScale(9), 1.5);
});

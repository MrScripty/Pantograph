import test from 'node:test';
import assert from 'node:assert/strict';

import type { WorkflowRunGraphProjection } from '../../services/workflow/types.ts';
import {
  buildRunGraphCanvasModel,
  buildRunGraphEdgeRows,
  buildRunGraphNodeArtifactSummaries,
  buildRunGraphNodeRows,
  formatRunGraphArtifactDetail,
  formatRunGraphArtifactSummary,
  formatRunGraphCountLabel,
  resolveRunGraphCounts,
  resolveRunGraphPresentationLabel,
} from './runGraphPresenters.ts';

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

test('resolveRunGraphCounts and labels use the immutable graph snapshot', () => {
  const runGraph = createRunGraph();

  assert.deepEqual(resolveRunGraphCounts(runGraph.graph), {
    nodeCount: 2,
    edgeCount: 1,
  });
  assert.equal(formatRunGraphCountLabel({ nodeCount: 2, edgeCount: 1 }), '2 nodes / 1 edges');
});

test('buildRunGraphNodeRows joins topology versions without editor state', () => {
  const rows = buildRunGraphNodeRows(createRunGraph());

  assert.deepEqual(rows.map((row) => row.nodeId), ['input-1', 'output-1']);
  assert.equal(rows[0].contractVersion, '1.0.0');
  assert.equal(rows[0].behaviorDigest, 'digest-input');
  assert.equal(rows[0].positionLabel, '10, 20');
  assert.equal(rows[0].settingsState, 'Run settings captured');
  assert.equal(rows[0].artifactSummaryLabel, 'No retained I/O');
});

test('buildRunGraphEdgeRows renders captured graph edges before topology fallback', () => {
  const rows = buildRunGraphEdgeRows(createRunGraph());

  assert.deepEqual(rows, [
    {
      edgeId: 'edge-1',
      source: 'input-1:text',
      target: 'output-1:value',
    },
  ]);
});

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

  assert.equal(canvas.viewBox, '-86 -76 672 276');
  assert.deepEqual(canvas.nodes.map((node) => node.id), ['input-1', 'output-1']);
  assert.deepEqual(canvas.edges.map((edge) => edge.id), ['edge-1']);
});

test('buildRunGraphNodeArtifactSummaries groups retained node io facts', () => {
  const summaries = buildRunGraphNodeArtifactSummaries([
    {
      node_id: 'input-1',
      artifact_role: 'node_input',
      event_seq: 10,
      payload_ref: 'artifact://input',
      media_type: 'text/plain',
    },
    {
      node_id: 'input-1',
      artifact_role: 'node_output',
      event_seq: 12,
      payload_ref: null,
      media_type: 'application/json',
    },
    {
      node_id: 'output-1',
      artifact_role: 'node_output',
      event_seq: 14,
      payload_ref: 'artifact://output',
      media_type: 'image/png',
    },
    {
      node_id: null,
      artifact_role: 'workflow_output',
      event_seq: 15,
      payload_ref: 'artifact://workflow-output',
      media_type: 'text/plain',
    },
  ]);

  assert.deepEqual(summaries['input-1'], {
    nodeId: 'input-1',
    inputCount: 1,
    outputCount: 1,
    artifactCount: 2,
    payloadRefCount: 1,
    latestEventSeq: 12,
    mediaTypes: ['application/json', 'text/plain'],
  });
  assert.equal(summaries['output-1'].outputCount, 1);
  assert.equal(summaries['output-1'].payloadRefCount, 1);
  assert.equal(Object.hasOwn(summaries, 'workflow-output'), false);
});

test('buildRunGraphNodeRows and canvas expose artifact availability labels', () => {
  const summaries = buildRunGraphNodeArtifactSummaries([
    {
      node_id: 'output-1',
      artifact_role: 'node_output',
      event_seq: 14,
      payload_ref: 'artifact://output',
      media_type: 'image/png',
    },
  ]);

  const rows = buildRunGraphNodeRows(createRunGraph(), summaries);
  const outputRow = rows.find((row) => row.nodeId === 'output-1');
  assert.equal(outputRow?.artifactSummaryLabel, '1 output / 0 inputs');
  assert.equal(outputRow?.artifactDetailLabel, '1 artifact, 1 payload reference, image/png');
  assert.equal(outputRow?.hasOutputArtifacts, true);

  const canvas = buildRunGraphCanvasModel(createRunGraph().graph, summaries);
  const outputNode = canvas.nodes.find((node) => node.id === 'output-1');
  assert.equal(outputNode?.artifactCount, 1);
  assert.equal(outputNode?.hasOutputArtifacts, true);
});

test('formatRunGraphArtifactSummary distinguishes metadata-only nodes', () => {
  const summary = buildRunGraphNodeArtifactSummaries([
    {
      node_id: 'input-1',
      artifact_role: 'node_input',
      event_seq: 10,
      payload_ref: null,
      media_type: null,
    },
  ])['input-1'];

  assert.equal(formatRunGraphArtifactSummary(null), 'No retained I/O');
  assert.equal(formatRunGraphArtifactSummary(summary), '0 outputs / 1 input');
  assert.equal(formatRunGraphArtifactDetail(summary), '1 artifact, 0 payload references, media unknown');
});

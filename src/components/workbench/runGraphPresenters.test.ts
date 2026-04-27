import test from 'node:test';
import assert from 'node:assert/strict';

import type { WorkflowRunGraphProjection } from '../../services/workflow/types.ts';
import {
  buildRunGraphCanvasModel,
  buildRunGraphEdgeRows,
  buildRunGraphNodeRows,
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

  assert.equal(canvas.viewBox, '-86 -76 672 256');
  assert.deepEqual(canvas.nodes.map((node) => node.id), ['input-1', 'output-1']);
  assert.deepEqual(canvas.edges.map((edge) => edge.id), ['edge-1']);
});

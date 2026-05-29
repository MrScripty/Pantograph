import test from 'node:test';
import assert from 'node:assert/strict';
import type { Edge, Node } from '@xyflow/svelte';

import {
  INFERENCE_INTERFACE_DRIFT_EDGE_ACTIVE_KEY,
  INFERENCE_INTERFACE_DRIFT_EDGE_TITLE_KEY,
  applyInferenceDriftEdgeOverlays,
  inferenceProposalAffectsPort,
} from './workflowInferenceDriftEdgeOverlays.ts';

function proposal(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    proposal_id: 'inference-interface-update/infer-1',
    node_id: 'infer-1',
    current_descriptor_fingerprint: 'descriptor.image_generation.2',
    drift_report: {
      authored_fingerprint: 'descriptor.image_generation.1',
      current_fingerprint: 'descriptor.image_generation.2',
      severity: 'blocking',
      blocking: true,
      diagnostics: [],
    },
    operations: [],
    affected_edges: [],
    requires_confirmation: true,
    destructive: true,
    ...overrides,
  };
}

function edge(id: string, data?: Record<string, unknown>): Edge {
  return {
    id,
    source: 'source-1',
    sourceHandle: 'image',
    target: 'infer-1',
    targetHandle: 'image',
    data,
  } as Edge;
}

function node(id: string, data?: Record<string, unknown>): Node {
  return {
    id,
    type: 'llm-inference',
    position: { x: 0, y: 0 },
    data: data ?? {},
  } as Node;
}

test('applyInferenceDriftEdgeOverlays marks backend-affected edges without changing structural endpoints', () => {
  const edges = [edge('edge-affected'), edge('edge-current')];
  const nodes = [
    node('infer-1', {
      inference_interface_update_proposal: proposal({
        affected_edges: [
          {
            edge_id: 'edge-affected',
            source_node_id: 'source-1',
            source_port_id: 'image',
            target_node_id: 'infer-1',
            target_port_id: 'image',
          },
        ],
      }),
    }),
  ];

  const result = applyInferenceDriftEdgeOverlays(edges, nodes);

  assert.equal(result.changed, true);
  assert.equal(result.edges[0]?.source, 'source-1');
  assert.equal(result.edges[0]?.target, 'infer-1');
  assert.equal(
    result.edges[0]?.data?.[INFERENCE_INTERFACE_DRIFT_EDGE_ACTIVE_KEY],
    true,
  );
  assert.equal(
    result.edges[0]?.data?.[INFERENCE_INTERFACE_DRIFT_EDGE_TITLE_KEY],
    'Blocking inference interface drift: source-1.image -> infer-1.image',
  );
  assert.equal(
    result.edges[1]?.data?.[INFERENCE_INTERFACE_DRIFT_EDGE_ACTIVE_KEY],
    undefined,
  );
});

test('applyInferenceDriftEdgeOverlays removes stale display-only edge markers', () => {
  const staleEdge = edge('edge-current', {
    [INFERENCE_INTERFACE_DRIFT_EDGE_ACTIVE_KEY]: true,
    [INFERENCE_INTERFACE_DRIFT_EDGE_TITLE_KEY]: 'stale',
    edgeInsertPreviewActive: true,
  });

  const result = applyInferenceDriftEdgeOverlays([staleEdge], []);

  assert.equal(result.changed, true);
  assert.deepEqual(result.edges[0]?.data, { edgeInsertPreviewActive: true });
});

test('inferenceProposalAffectsPort reads backend affected edge facts from node data', () => {
  const data = {
    inference_interface_update_proposal: proposal({
      operations: [
        {
          operation: 'remove_invalid_edge',
          value: {
            reason: 'target_port_removed',
            edge: {
              edge_id: 'edge-affected',
              source_node_id: 'source-1',
              source_port_id: 'image',
              target_node_id: 'infer-1',
              target_port_id: 'image',
            },
          },
        },
      ],
    }),
  };

  assert.equal(
    inferenceProposalAffectsPort({ data, nodeId: 'infer-1', portId: 'image' }),
    true,
  );
  assert.equal(
    inferenceProposalAffectsPort({ data, nodeId: 'infer-1', portId: 'prompt' }),
    false,
  );
});

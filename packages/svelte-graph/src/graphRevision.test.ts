import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildDerivedGraph,
  computeConsumerCountMap,
  computeGraphFingerprint,
} from './graphRevision.ts';
import type { WorkflowGraph } from './types/workflow.js';

function graph(): WorkflowGraph {
  return {
    nodes: [
      { id: 'input', node_type: 'text-input', position: { x: 0, y: 0 }, data: {} },
      { id: 'llm', node_type: 'llm-inference', position: { x: 100, y: 0 }, data: {} },
      { id: 'output', node_type: 'text-output', position: { x: 200, y: 0 }, data: {} },
    ],
    edges: [
      {
        id: 'e1',
        source: 'input',
        source_handle: 'text',
        target: 'llm',
        target_handle: 'prompt',
      },
      {
        id: 'e2',
        source: 'llm',
        source_handle: 'response',
        target: 'output',
        target_handle: 'text',
      },
    ],
  };
}

test('computeGraphFingerprint is deterministic for graph structure', () => {
  const first = computeGraphFingerprint(graph());
  const second = computeGraphFingerprint(graph());

  assert.equal(first, second);
});

test('computeConsumerCountMap counts outgoing consumers by source port', () => {
  const counts = computeConsumerCountMap(graph());

  assert.deepEqual(counts, {
    'input:text': 1,
    'llm:response': 1,
  });
});

test('buildDerivedGraph includes schema version and fingerprint', () => {
  const derived = buildDerivedGraph(graph());

  assert.equal(derived.schema_version, 1);
  assert.equal(derived.graph_fingerprint.length, 16);
});

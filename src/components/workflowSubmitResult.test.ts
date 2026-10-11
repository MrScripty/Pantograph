import test from 'node:test';
import assert from 'node:assert/strict';
import { workflowSubmitOutputOverlays, workflowSubmitStatusOverlays } from './workflowSubmitResult.ts';
import type { NodeStatusProjectionRecord } from '../services/diagnostics/types.ts';

const graph = {
  nodes: [
    { id: 'input', node_type: 'text-input', position: { x: 0, y: 0 }, data: { text: 'authored' } },
    { id: 'output', node_type: 'text-output', position: { x: 300, y: 0 }, data: {} },
  ], edges: [],
};

test('returned outputs update only their actual matching node and preserve empty text', () => {
  assert.deepEqual(workflowSubmitOutputOverlays(graph, {
    workflow_run_id: 'run', timing_ms: 1, outputs: [
      { node_id: 'output', port_id: 'text', value: '' },
      { node_id: 'other-graph', port_id: 'text', value: 'must not appear' },
    ],
  }), [{ nodeId: 'output', data: { text: '' } }]);
  assert.equal(graph.nodes[0].data.text, 'authored');
});

test('only real matching run node statuses become overlays; materialized input remains unmarked', () => {
  const status = (node_id: string, workflow_run_id = 'run'): NodeStatusProjectionRecord => ({
    node_id, workflow_run_id, workflow_id: 'editor-demo', status: 'completed', last_event_seq: 1, last_updated_at_ms: 1,
  });
  assert.deepEqual(workflowSubmitStatusOverlays(graph, 'run', [
    status('output'), status('input', 'older-run'), status('missing'),
  ]), [{ nodeId: 'output', state: 'success', message: undefined }]);
  assert.deepEqual(workflowSubmitStatusOverlays(graph, 'run', []), []);
  assert.deepEqual(workflowSubmitStatusOverlays(graph, 'run', [
    { ...status('output'), status: 'failed', error: 'real failure' },
  ]), [{ nodeId: 'output', state: 'error', message: 'real failure' }]);
});

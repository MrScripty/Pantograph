import test from 'node:test';
import assert from 'node:assert/strict';

import { WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS } from './workflowGraphEdgeOptions.ts';

test('WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS preserves shared reconnectable edge defaults', () => {
  assert.deepEqual(WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS, {
    type: 'reconnectable',
    animated: false,
    style: 'stroke: #525252; stroke-width: 2px;',
    interactionWidth: 20,
    selectable: true,
    focusable: true,
  });
});

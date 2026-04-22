import test from 'node:test';
import assert from 'node:assert/strict';

import {
  WORKFLOW_GRAPH_FIT_VIEW_OPTIONS,
  WORKFLOW_GRAPH_MAX_ZOOM,
  WORKFLOW_GRAPH_MINIMAP_MASK_COLOR,
  WORKFLOW_GRAPH_MIN_ZOOM,
  WORKFLOW_GRAPH_PAN_ACTIVATION_KEY,
} from './workflowGraphViewport.ts';

test('workflow graph viewport constants preserve shared SvelteFlow canvas defaults', () => {
  assert.deepEqual(WORKFLOW_GRAPH_FIT_VIEW_OPTIONS, { maxZoom: 1 });
  assert.equal(WORKFLOW_GRAPH_MIN_ZOOM, 0.25);
  assert.equal(WORKFLOW_GRAPH_MAX_ZOOM, 2);
  assert.equal(WORKFLOW_GRAPH_MINIMAP_MASK_COLOR, 'rgba(0, 0, 0, 0.8)');
  assert.equal(WORKFLOW_GRAPH_PAN_ACTIVATION_KEY, null);
});

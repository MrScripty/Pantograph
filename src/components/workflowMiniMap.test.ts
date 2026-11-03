import test from 'node:test';
import assert from 'node:assert/strict';

import {
  getWorkflowMiniMapNodeColor,
  WORKFLOW_MINIMAP_GROUP_COLOR,
  WORKFLOW_MINIMAP_NODE_COLOR,
} from './workflowMiniMap.ts';

test('getWorkflowMiniMapNodeColor prioritizes group nodes', () => {
  assert.equal(getWorkflowMiniMapNodeColor({ type: 'node-group' }), WORKFLOW_MINIMAP_GROUP_COLOR);
  assert.equal(getWorkflowMiniMapNodeColor({ data: { isGroup: true } }), WORKFLOW_MINIMAP_GROUP_COLOR);
});

test('getWorkflowMiniMapNodeColor uses one token colour for every category', () => {
  for (const category of ['input', 'processing', 'tool', 'output', 'control', 'custom']) {
    assert.equal(
      getWorkflowMiniMapNodeColor({ data: { definition: { category } } }),
      WORKFLOW_MINIMAP_NODE_COLOR,
    );
  }
  assert.equal(getWorkflowMiniMapNodeColor({}), WORKFLOW_MINIMAP_NODE_COLOR);
});

test('minimap colours are workbench token references, not literals', () => {
  assert.equal(WORKFLOW_MINIMAP_NODE_COLOR, 'var(--pg-border-idle)');
  assert.equal(WORKFLOW_MINIMAP_GROUP_COLOR, 'var(--pg-text-muted)');
  assert.notEqual(WORKFLOW_MINIMAP_NODE_COLOR, WORKFLOW_MINIMAP_GROUP_COLOR);
});

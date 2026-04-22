import test from 'node:test';
import assert from 'node:assert/strict';

import { getWorkflowMiniMapNodeColor } from './workflowMiniMap.ts';

test('getWorkflowMiniMapNodeColor prioritizes graph groups', () => {
  assert.equal(getWorkflowMiniMapNodeColor({ type: 'node-group' }), '#7c3aed');
  assert.equal(getWorkflowMiniMapNodeColor({ data: { isGroup: true } }), '#7c3aed');
});

test('getWorkflowMiniMapNodeColor maps backend categories to minimap colors', () => {
  assert.equal(getWorkflowMiniMapNodeColor({ data: { definition: { category: 'input' } } }), '#2563eb');
  assert.equal(getWorkflowMiniMapNodeColor({ data: { definition: { category: 'processing' } } }), '#16a34a');
  assert.equal(getWorkflowMiniMapNodeColor({ data: { definition: { category: 'tool' } } }), '#d97706');
  assert.equal(getWorkflowMiniMapNodeColor({ data: { definition: { category: 'output' } } }), '#0891b2');
  assert.equal(getWorkflowMiniMapNodeColor({ data: { definition: { category: 'control' } } }), '#9333ea');
});

test('getWorkflowMiniMapNodeColor uses fallback color for unknown categories', () => {
  assert.equal(getWorkflowMiniMapNodeColor({}), '#525252');
  assert.equal(getWorkflowMiniMapNodeColor({ data: { definition: { category: 'custom' } } }), '#525252');
});

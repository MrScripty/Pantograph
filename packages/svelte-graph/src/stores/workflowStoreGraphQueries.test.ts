import test from 'node:test';
import assert from 'node:assert/strict';
import type { Edge, Node } from '@xyflow/svelte';

import type { NodeGroup } from '../types/groups.ts';
import {
  extractWorkflowNodeGroups,
  findWorkflowGroupContainingNodeIds,
  getWorkflowConnectedNodes,
  getWorkflowNodesBounds,
  isWorkflowNodeGroupData,
} from './workflowStoreGraphQueries.ts';

const group = {
  id: 'group-a',
  name: 'Group A',
  nodes: [
    {
      id: 'a',
      node_type: 'text-input',
      position: { x: 0, y: 0 },
      data: {},
    },
    {
      id: 'b',
      node_type: 'text-output',
      position: { x: 120, y: 0 },
      data: {},
    },
  ],
  edges: [],
  exposed_inputs: [],
  exposed_outputs: [],
  position: { x: 10, y: 20 },
  collapsed: true,
} satisfies NodeGroup;

test('workflow store graph query helpers extract group node data', () => {
  assert.equal(isWorkflowNodeGroupData(group), true);
  assert.equal(isWorkflowNodeGroupData({ id: 'missing-shape' }), false);

  const groups = extractWorkflowNodeGroups([
    {
      id: 'group-a',
      type: 'node-group',
      position: { x: 0, y: 0 },
      data: { group },
    },
    {
      id: 'ordinary',
      type: 'text-input',
      position: { x: 100, y: 0 },
      data: {},
    },
  ] as Node[]);

  assert.equal(groups.get('group-a'), group);
  assert.equal(findWorkflowGroupContainingNodeIds(groups, ['a', 'b']), group);
  assert.equal(findWorkflowGroupContainingNodeIds(groups, ['a']), null);
});

test('workflow store graph query helpers project connected nodes', () => {
  const nodes = [
    { id: 'source', position: { x: 0, y: 0 }, data: {} },
    { id: 'target', position: { x: 100, y: 0 }, data: {} },
    { id: 'output', position: { x: 200, y: 0 }, data: {} },
  ] as Node[];
  const edges = [
    { id: 'source-target', source: 'source', target: 'target' },
    { id: 'target-output', source: 'target', target: 'output' },
  ] as Edge[];

  assert.deepEqual(
    getWorkflowConnectedNodes(nodes, edges, 'target'),
    {
      inputs: [nodes[0]],
      outputs: [nodes[2]],
    },
  );
});

test('workflow store graph query helpers calculate node bounds', () => {
  const nodes = [
    {
      id: 'a',
      position: { x: 10, y: 20 },
      width: 50,
      height: 60,
      data: {},
    },
    {
      id: 'b',
      position: { x: 100, y: 120 },
      measured: { width: 80, height: 90 },
      data: {},
    },
  ] as Node[];

  assert.deepEqual(getWorkflowNodesBounds(nodes, ['a', 'b']), {
    x: 10,
    y: 20,
    width: 170,
    height: 190,
  });
  assert.equal(getWorkflowNodesBounds(nodes, ['missing']), null);
});

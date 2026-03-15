import test from 'node:test';
import assert from 'node:assert/strict';

import type { Node } from '@xyflow/svelte';

import { applySelectedNodeIds, collectSelectedNodeIds } from './workflowSelection.ts';

function node(id: string, selected = false): Node {
  return {
    id,
    position: { x: 0, y: 0 },
    data: { label: id },
    selected,
  };
}

test('applySelectedNodeIds updates only nodes whose selection changed', () => {
  const a = node('a', false);
  const b = node('b', true);

  const result = applySelectedNodeIds([a, b], ['a']);

  assert.equal(result[0].selected, true);
  assert.equal(result[1].selected, false);
  assert.notEqual(result[0], a);
  assert.notEqual(result[1], b);
});

test('applySelectedNodeIds preserves node references when selection already matches', () => {
  const a = node('a', true);
  const b = node('b', false);

  const result = applySelectedNodeIds([a, b], ['a']);

  assert.equal(result[0], a);
  assert.equal(result[1], b);
});

test('collectSelectedNodeIds returns only selected ids', () => {
  assert.deepEqual(collectSelectedNodeIds([node('a', true), node('b', false), node('c', true)]), [
    'a',
    'c',
  ]);
});

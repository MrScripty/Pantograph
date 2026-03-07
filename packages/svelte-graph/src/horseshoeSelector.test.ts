import test from 'node:test';
import assert from 'node:assert/strict';

import {
  findBestInsertableMatchIndex,
  findNearestVisibleHorseshoeIndex,
  getHorseshoeItemPosition,
  getHorseshoeWindow,
  rotateHorseshoeIndex,
} from './horseshoeSelector.ts';
import type { InsertableNodeTypeCandidate } from './types/workflow.ts';

const candidates: InsertableNodeTypeCandidate[] = [
  {
    node_type: 'audio-output',
    category: 'output',
    label: 'Audio Output',
    description: '',
    matching_input_port_ids: ['audio'],
  },
  {
    node_type: 'llm-inference',
    category: 'processing',
    label: 'LLM Inference',
    description: '',
    matching_input_port_ids: ['prompt'],
  },
  {
    node_type: 'masked-text-input',
    category: 'input',
    label: 'Masked Text Input',
    description: '',
    matching_input_port_ids: ['text'],
  },
  {
    node_type: 'text-output',
    category: 'output',
    label: 'Text Output',
    description: '',
    matching_input_port_ids: ['text'],
  },
  {
    node_type: 'vector-output',
    category: 'output',
    label: 'Vector Output',
    description: '',
    matching_input_port_ids: ['vector'],
  },
];

test('getHorseshoeWindow centers the selected item when possible', () => {
  const window = getHorseshoeWindow(candidates, 2, 3);
  assert.deepEqual(window.visibleItems.map((entry) => entry.index), [1, 2, 3]);
  assert.equal(window.hiddenBefore, 1);
  assert.equal(window.hiddenAfter, 1);
});

test('rotateHorseshoeIndex clamps at the list boundaries', () => {
  assert.equal(rotateHorseshoeIndex(0, -1, candidates.length), 0);
  assert.equal(rotateHorseshoeIndex(4, 1, candidates.length), 4);
  assert.equal(rotateHorseshoeIndex(2, 1, candidates.length), 3);
});

test('findBestInsertableMatchIndex prefers prefix matches before substring matches', () => {
  assert.equal(findBestInsertableMatchIndex(candidates, 'mask'), 2);
  assert.equal(findBestInsertableMatchIndex(candidates, 'infer'), 1);
  assert.equal(findBestInsertableMatchIndex(candidates, 'output', 1), 0);
});

test('getHorseshoeItemPosition places a single item at the top of the arc', () => {
  assert.deepEqual(getHorseshoeItemPosition(0, 1), {
    x: 0,
    y: -126,
    angle: -90,
  });
});

test('findNearestVisibleHorseshoeIndex selects the closest visible item', () => {
  const anchorPosition = { x: 300, y: 240 };
  const itemPosition = getHorseshoeItemPosition(1, 3);

  assert.equal(
    findNearestVisibleHorseshoeIndex(
      candidates,
      2,
      {
        x: anchorPosition.x + itemPosition.x,
        y: anchorPosition.y + itemPosition.y,
      },
      anchorPosition,
      3,
    ),
    2,
  );
});

test('findNearestVisibleHorseshoeIndex ignores pointers outside the menu radius', () => {
  assert.equal(
    findNearestVisibleHorseshoeIndex(
      candidates,
      2,
      { x: 0, y: 0 },
      { x: 300, y: 240 },
      3,
    ),
    null,
  );
});

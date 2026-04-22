import test from 'node:test';
import assert from 'node:assert/strict';

import { startHorseshoeDrag } from './horseshoeDragSession.ts';
import {
  createHorseshoeInsertFeedbackState,
  startHorseshoeInsertFeedback,
} from './horseshoeInsertFeedback.ts';
import {
  normalizeWorkflowHorseshoeSelectedIndex,
  resolveWorkflowHorseshoeQueryUpdate,
  resolveWorkflowHorseshoeSelectionSnapshot,
  rotateWorkflowHorseshoeSelection,
} from './workflowHorseshoeSelection.ts';
import type { InsertableNodeTypeCandidate } from './types/workflow.ts';

const items = [
  { node_type: 'a' },
  { node_type: 'b' },
];

test('resolveWorkflowHorseshoeSelectionSnapshot returns selected candidate and keyboard context', () => {
  assert.deepEqual(
    resolveWorkflowHorseshoeSelectionSnapshot({
      session: {
        ...startHorseshoeDrag({ x: 1, y: 2 }),
        displayState: 'open',
      },
      feedback: createHorseshoeInsertFeedbackState(),
      items,
      selectedIndex: 1,
    }),
    {
      keyboardContext: {
        displayState: 'open',
        dragActive: true,
        pending: false,
        hasSelection: true,
      },
      selectedCandidate: { node_type: 'b' },
    },
  );
});

test('resolveWorkflowHorseshoeSelectionSnapshot reports no selection outside item bounds', () => {
  assert.deepEqual(
    resolveWorkflowHorseshoeSelectionSnapshot({
      session: {
        ...startHorseshoeDrag(null),
        displayState: 'pending',
      },
      feedback: startHorseshoeInsertFeedback(),
      items,
      selectedIndex: 5,
    }),
    {
      keyboardContext: {
        displayState: 'pending',
        dragActive: true,
        pending: true,
        hasSelection: false,
      },
      selectedCandidate: null,
    },
  );
});

test('resolveWorkflowHorseshoeSelectionSnapshot accepts missing item lists', () => {
  assert.equal(
    resolveWorkflowHorseshoeSelectionSnapshot({
      session: startHorseshoeDrag(null),
      feedback: createHorseshoeInsertFeedbackState(),
      items: undefined,
      selectedIndex: 0,
    }).selectedCandidate,
    null,
  );
});

test('normalizeWorkflowHorseshoeSelectedIndex clamps selected index to available items', () => {
  assert.equal(
    normalizeWorkflowHorseshoeSelectedIndex({
      selectedIndex: -1,
      itemCount: 2,
    }),
    0,
  );
  assert.equal(
    normalizeWorkflowHorseshoeSelectedIndex({
      selectedIndex: 5,
      itemCount: 2,
    }),
    1,
  );
  assert.equal(
    normalizeWorkflowHorseshoeSelectedIndex({
      selectedIndex: 5,
      itemCount: 0,
    }),
    0,
  );
});

test('rotateWorkflowHorseshoeSelection rotates within available items', () => {
  assert.equal(
    rotateWorkflowHorseshoeSelection({
      selectedIndex: 1,
      delta: 1,
      itemCount: 3,
    }),
    2,
  );
  assert.equal(
    rotateWorkflowHorseshoeSelection({
      selectedIndex: 0,
      delta: -1,
      itemCount: 3,
    }),
    0,
  );
  assert.equal(
    rotateWorkflowHorseshoeSelection({
      selectedIndex: 0,
      delta: 1,
      itemCount: 0,
    }),
    null,
  );
});

test('resolveWorkflowHorseshoeQueryUpdate selects best query match and reset action', () => {
  const candidates: InsertableNodeTypeCandidate[] = [
    {
      node_type: 'text-output',
      category: 'output',
      label: 'Text Output',
      description: 'Output text',
      matching_input_port_ids: ['input'],
    },
    {
      node_type: 'mask-image',
      category: 'processing',
      label: 'Mask Image',
      description: 'Mask an image',
      matching_input_port_ids: ['image'],
    },
  ];

  assert.deepEqual(
    resolveWorkflowHorseshoeQueryUpdate({
      items: candidates,
      query: 'mask',
      selectedIndex: 0,
    }),
    {
      query: 'mask',
      selectedIndex: 1,
      resetTimerAction: 'schedule',
    },
  );
  assert.deepEqual(
    resolveWorkflowHorseshoeQueryUpdate({
      items: candidates,
      query: '',
      selectedIndex: 5,
    }),
    {
      query: '',
      selectedIndex: 1,
      resetTimerAction: 'clear',
    },
  );
});

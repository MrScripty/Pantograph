import test from 'node:test';
import assert from 'node:assert/strict';

import {
  clearActiveWorkflowPaletteDragDefinition,
  readWorkflowPaletteDragDefinition,
  resolveWorkflowPaletteDropPosition,
  setActiveWorkflowPaletteDragDefinition,
} from './workflowPaletteDrag.ts';
import type { NodeDefinition } from './types/workflow.ts';

const paletteDefinition: NodeDefinition = {
  node_type: 'text_input',
  category: 'input',
  label: 'Text Input',
  description: 'Provides text',
  io_binding_origin: 'client_session',
  execution_mode: 'manual',
  inputs: [],
  outputs: [],
};

test('readWorkflowPaletteDragDefinition parses node definitions from drag data', () => {
  assert.deepEqual(
    readWorkflowPaletteDragDefinition({
      dataTransfer: {
        getData: () => JSON.stringify(paletteDefinition),
      },
    }),
    paletteDefinition,
  );
});

test('readWorkflowPaletteDragDefinition falls back to plain text drag data', () => {
  assert.deepEqual(
    readWorkflowPaletteDragDefinition({
      dataTransfer: {
        getData: (type) => type === 'text/plain' ? JSON.stringify(paletteDefinition) : '',
      },
    }),
    paletteDefinition,
  );
});

test('readWorkflowPaletteDragDefinition reports invalid palette drag data', () => {
  const parseErrors: unknown[] = [];

  assert.equal(
    readWorkflowPaletteDragDefinition(
      {
        dataTransfer: {
          getData: () => '{',
        },
      },
      (error) => parseErrors.push(error),
    ),
    null,
  );
  assert.equal(parseErrors.length, 1);
});

test('readWorkflowPaletteDragDefinition ignores empty drag data', () => {
  clearActiveWorkflowPaletteDragDefinition();
  assert.equal(
    readWorkflowPaletteDragDefinition({
      dataTransfer: {
        getData: () => '',
      },
    }),
    null,
  );
});

test('readWorkflowPaletteDragDefinition falls back to the active palette definition', () => {
  setActiveWorkflowPaletteDragDefinition(paletteDefinition);
  try {
    assert.deepEqual(
      readWorkflowPaletteDragDefinition({
        dataTransfer: {
          getData: () => '',
        },
      }),
      paletteDefinition,
    );
  } finally {
    clearActiveWorkflowPaletteDragDefinition();
  }
});

test('resolveWorkflowPaletteDropPosition projects client coordinates into graph space', () => {
  assert.deepEqual(
    resolveWorkflowPaletteDropPosition({
      clientPosition: { x: 340, y: 260 },
      containerBounds: { left: 40, top: 60 },
    }),
    {
      x: 200,
      y: 150,
    },
  );
});

test('resolveWorkflowPaletteDropPosition accepts custom node offsets', () => {
  assert.deepEqual(
    resolveWorkflowPaletteDropPosition({
      clientPosition: { x: 340, y: 260 },
      containerBounds: { left: 40, top: 60 },
      nodeOffset: { x: 20, y: 30 },
    }),
    {
      x: 280,
      y: 170,
    },
  );
});

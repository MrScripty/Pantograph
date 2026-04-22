import test from 'node:test';
import assert from 'node:assert/strict';

import {
  isWorkflowPaletteEdgeInsertEnabled,
  readWorkflowPaletteDragDefinition,
  resolveWorkflowPaletteDropPosition,
} from './workflowPaletteDrag.ts';
import type { NodeDefinition } from '../services/workflow/types.ts';

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

test('isWorkflowPaletteEdgeInsertEnabled disables edge insert on the architecture graph', () => {
  assert.equal(isWorkflowPaletteEdgeInsertEnabled('system', 'app-architecture'), false);
  assert.equal(isWorkflowPaletteEdgeInsertEnabled('workflow', 'main'), true);
  assert.equal(isWorkflowPaletteEdgeInsertEnabled(null, null), true);
});

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
  assert.equal(
    readWorkflowPaletteDragDefinition({
      dataTransfer: {
        getData: () => '',
      },
    }),
    null,
  );
});

test('resolveWorkflowPaletteDropPosition projects pointer coordinates into graph space', () => {
  assert.deepEqual(
    resolveWorkflowPaletteDropPosition({
      pointerPosition: { x: 340, y: 260 },
      viewport: { x: 40, y: 60, zoom: 2 },
    }),
    {
      x: 50,
      y: 50,
    },
  );
});

test('resolveWorkflowPaletteDropPosition uses the default viewport when unavailable', () => {
  assert.deepEqual(
    resolveWorkflowPaletteDropPosition({
      pointerPosition: { x: 140, y: 90 },
      viewport: null,
    }),
    {
      x: 40,
      y: 40,
    },
  );
});

test('resolveWorkflowPaletteDropPosition returns null without a pointer position', () => {
  assert.equal(
    resolveWorkflowPaletteDropPosition({
      pointerPosition: null,
      viewport: { x: 40, y: 60, zoom: 2 },
    }),
    null,
  );
});

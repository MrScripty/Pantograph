import test from 'node:test';
import assert from 'node:assert/strict';

import { resolveWorkflowInsertPositionHint } from './workflowInsertPosition.ts';

test('resolveWorkflowInsertPositionHint projects anchor coordinates through the viewport', () => {
  assert.deepEqual(
    resolveWorkflowInsertPositionHint({
      anchorPosition: { x: 340, y: 260 },
      viewport: { x: 40, y: 60, zoom: 2 },
    }),
    {
      position: {
        x: 150,
        y: 100,
      },
    },
  );
});

test('resolveWorkflowInsertPositionHint uses an identity viewport when unavailable', () => {
  assert.deepEqual(
    resolveWorkflowInsertPositionHint({
      anchorPosition: { x: 140, y: 90 },
      viewport: null,
    }),
    {
      position: {
        x: 140,
        y: 90,
      },
    },
  );
});

test('resolveWorkflowInsertPositionHint returns null without an anchor', () => {
  assert.equal(
    resolveWorkflowInsertPositionHint({
      anchorPosition: null,
      viewport: { x: 40, y: 60, zoom: 2 },
    }),
    null,
  );
});

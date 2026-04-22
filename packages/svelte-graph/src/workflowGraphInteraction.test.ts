import test from 'node:test';
import assert from 'node:assert/strict';

import { resolveWorkflowGraphInteractionState } from './workflowGraphInteraction.ts';

test('resolveWorkflowGraphInteractionState enables editing when editable and no palette drag is active', () => {
  assert.deepEqual(
    resolveWorkflowGraphInteractionState({
      canEdit: true,
      ctrlPressed: false,
      externalPaletteDragActive: false,
    }),
    {
      deleteKey: 'Delete',
      edgesReconnectable: true,
      elementsSelectable: true,
      nodesConnectable: true,
      nodesDraggable: true,
      panOnDrag: true,
    },
  );
});

test('resolveWorkflowGraphInteractionState disables edit commands when editing is unavailable', () => {
  assert.deepEqual(
    resolveWorkflowGraphInteractionState({
      canEdit: false,
      ctrlPressed: false,
      externalPaletteDragActive: false,
    }),
    {
      deleteKey: null,
      edgesReconnectable: false,
      elementsSelectable: true,
      nodesConnectable: false,
      nodesDraggable: false,
      panOnDrag: true,
    },
  );
});

test('resolveWorkflowGraphInteractionState suspends graph selection and edits during external palette drags', () => {
  assert.deepEqual(
    resolveWorkflowGraphInteractionState({
      canEdit: true,
      ctrlPressed: false,
      externalPaletteDragActive: true,
    }),
    {
      deleteKey: 'Delete',
      edgesReconnectable: false,
      elementsSelectable: false,
      nodesConnectable: false,
      nodesDraggable: false,
      panOnDrag: false,
    },
  );
});

test('resolveWorkflowGraphInteractionState disables pane panning while cut gesture modifier is pressed', () => {
  assert.equal(
    resolveWorkflowGraphInteractionState({
      canEdit: true,
      ctrlPressed: true,
      externalPaletteDragActive: false,
    }).panOnDrag,
    false,
  );
});

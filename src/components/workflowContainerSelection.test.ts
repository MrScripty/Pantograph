import test from 'node:test';
import assert from 'node:assert/strict';

import {
  clearWorkflowContainerSelection,
  resolveWorkflowContainerKeyboardAction,
  resolveWorkflowContainerSelectionAfterGraphSelection,
  toggleWorkflowContainerSelection,
} from './workflowContainerSelection.ts';

test('resolveWorkflowContainerKeyboardAction maps selected tab to orchestration zoom', () => {
  assert.deepEqual(
    resolveWorkflowContainerKeyboardAction({
      key: 'Tab',
      containerSelected: true,
    }),
    {
      type: 'zoom-to-orchestration',
      preventDefault: true,
    },
  );
});

test('resolveWorkflowContainerKeyboardAction maps selected escape to deselect', () => {
  assert.deepEqual(
    resolveWorkflowContainerKeyboardAction({
      key: 'Escape',
      containerSelected: true,
    }),
    {
      type: 'deselect-container',
      preventDefault: true,
    },
  );
});

test('resolveWorkflowContainerKeyboardAction ignores keys without selected container', () => {
  assert.deepEqual(
    resolveWorkflowContainerKeyboardAction({
      key: 'Tab',
      containerSelected: false,
    }),
    {
      type: 'noop',
    },
  );
});

test('resolveWorkflowContainerKeyboardAction ignores unrelated selected-container keys', () => {
  assert.deepEqual(
    resolveWorkflowContainerKeyboardAction({
      key: 'Enter',
      containerSelected: true,
    }),
    {
      type: 'noop',
    },
  );
});

test('resolveWorkflowContainerSelectionAfterGraphSelection clears selected container when nodes are selected', () => {
  assert.equal(
    resolveWorkflowContainerSelectionAfterGraphSelection({
      containerSelected: true,
      selectedNodeCount: 1,
    }),
    false,
  );
  assert.equal(
    resolveWorkflowContainerSelectionAfterGraphSelection({
      containerSelected: true,
      selectedNodeCount: 0,
    }),
    true,
  );
});

test('container selection helpers toggle and clear boundary selection', () => {
  assert.equal(toggleWorkflowContainerSelection(false), true);
  assert.equal(toggleWorkflowContainerSelection(true), false);
  assert.equal(clearWorkflowContainerSelection(), false);
});

import test from 'node:test';
import assert from 'node:assert/strict';

import { resolveWorkflowContainerKeyboardAction } from './workflowContainerSelection.ts';

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

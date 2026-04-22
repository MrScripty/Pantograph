import test from 'node:test';
import assert from 'node:assert/strict';

import {
  isEditableKeyboardTarget,
  resolveHorseshoeKeyboardAction,
} from './workflowHorseshoeKeyboard.ts';

test('isEditableKeyboardTarget recognizes form and editable targets', () => {
  assert.equal(isEditableKeyboardTarget(null), false);
  assert.equal(isEditableKeyboardTarget({ tagName: 'DIV' }), false);
  assert.equal(isEditableKeyboardTarget({ tagName: 'INPUT' }), true);
  assert.equal(isEditableKeyboardTarget({ tagName: 'TEXTAREA' }), true);
  assert.equal(isEditableKeyboardTarget({ tagName: 'SELECT' }), true);
  assert.equal(isEditableKeyboardTarget({ isContentEditable: true, tagName: 'DIV' }), true);
});

test('resolveHorseshoeKeyboardAction ignores idle hidden sessions', () => {
  assert.deepEqual(
    resolveHorseshoeKeyboardAction(
      { key: 'Escape' },
      {
        displayState: 'hidden',
        dragActive: false,
        pending: false,
        hasSelection: false,
      },
    ),
    { type: 'noop', preventDefault: false },
  );
});

test('resolveHorseshoeKeyboardAction maps space to request open or confirm', () => {
  assert.deepEqual(
    resolveHorseshoeKeyboardAction(
      { code: 'Space', key: ' ' },
      {
        displayState: 'hidden',
        dragActive: true,
        pending: false,
        hasSelection: false,
      },
    ),
    { type: 'request-open', preventDefault: true },
  );

  assert.deepEqual(
    resolveHorseshoeKeyboardAction(
      { code: 'Space', key: ' ' },
      {
        displayState: 'open',
        dragActive: true,
        pending: false,
        hasSelection: true,
      },
    ),
    { type: 'confirm-selection', preventDefault: true },
  );
});

test('resolveHorseshoeKeyboardAction maps open-menu navigation keys', () => {
  const context = {
    displayState: 'open' as const,
    dragActive: true,
    pending: false,
    hasSelection: true,
  };

  assert.deepEqual(resolveHorseshoeKeyboardAction({ key: 'Escape' }, context), {
    type: 'close',
    preventDefault: true,
  });
  assert.deepEqual(resolveHorseshoeKeyboardAction({ key: 'Enter' }, context), {
    type: 'confirm-selection',
    preventDefault: true,
  });
  assert.deepEqual(resolveHorseshoeKeyboardAction({ key: 'ArrowLeft' }, context), {
    type: 'rotate-selection',
    delta: -1,
    preventDefault: true,
  });
  assert.deepEqual(resolveHorseshoeKeyboardAction({ key: 'ArrowRight' }, context), {
    type: 'rotate-selection',
    delta: 1,
    preventDefault: true,
  });
  assert.deepEqual(resolveHorseshoeKeyboardAction({ key: 'Backspace' }, context), {
    type: 'remove-query-character',
    preventDefault: true,
  });
  assert.deepEqual(resolveHorseshoeKeyboardAction({ key: 'p' }, context), {
    type: 'append-query-character',
    character: 'p',
    preventDefault: true,
  });
});

test('resolveHorseshoeKeyboardAction ignores modified printable keys', () => {
  assert.deepEqual(
    resolveHorseshoeKeyboardAction(
      { key: 'p', ctrlKey: true },
      {
        displayState: 'open',
        dragActive: true,
        pending: false,
        hasSelection: true,
      },
    ),
    { type: 'noop', preventDefault: false },
  );
});

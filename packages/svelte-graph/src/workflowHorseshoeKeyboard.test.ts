import test from 'node:test';
import assert from 'node:assert/strict';

import {
  dispatchWorkflowHorseshoeKeyboardAction,
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

test('dispatchWorkflowHorseshoeKeyboardAction opens and confirms with trace updates', () => {
  const calls: string[] = [];

  dispatchWorkflowHorseshoeKeyboardAction({
    event: {
      code: 'Space',
      key: ' ',
      preventDefault: () => calls.push('prevent'),
    },
    query: '',
    selection: {
      keyboardContext: {
        displayState: 'hidden',
        dragActive: true,
        pending: false,
        hasSelection: false,
      },
      selectedCandidate: null,
    },
    handlers: {
      onClose: () => calls.push('close'),
      onConfirmSelection: (candidate) => calls.push(`confirm:${candidate}`),
      onQueryUpdate: (query) => calls.push(`query:${query}`),
      onRequestOpen: () => calls.push('open'),
      onRotateSelection: (delta) => calls.push(`rotate:${delta}`),
      onTrace: (trace) => calls.push(`trace:${trace}`),
    },
  });

  assert.deepEqual(calls, ['prevent', 'trace:keydown:space', 'open']);
  calls.length = 0;

  dispatchWorkflowHorseshoeKeyboardAction({
    event: {
      key: 'Enter',
      preventDefault: () => calls.push('prevent'),
    },
    query: '',
    selection: {
      keyboardContext: {
        displayState: 'open',
        dragActive: true,
        pending: false,
        hasSelection: true,
      },
      selectedCandidate: 'text',
    },
    handlers: {
      onClose: () => calls.push('close'),
      onConfirmSelection: (candidate) => calls.push(`confirm:${candidate}`),
      onQueryUpdate: (query) => calls.push(`query:${query}`),
      onRequestOpen: () => calls.push('open'),
      onRotateSelection: (delta) => calls.push(`rotate:${delta}`),
      onTrace: (trace) => calls.push(`trace:${trace}`),
    },
  });

  assert.deepEqual(calls, ['prevent', 'trace:keydown:enter', 'confirm:text']);
});

test('dispatchWorkflowHorseshoeKeyboardAction routes selector navigation and query editing', () => {
  const calls: string[] = [];
  const handlers = {
    onClose: () => calls.push('close'),
    onConfirmSelection: (candidate: string) => calls.push(`confirm:${candidate}`),
    onQueryUpdate: (query: string) => calls.push(`query:${query}`),
    onRequestOpen: () => calls.push('open'),
    onRotateSelection: (delta: -1 | 1) => calls.push(`rotate:${delta}`),
    onTrace: (trace: string) => calls.push(`trace:${trace}`),
  };
  const selection = {
    keyboardContext: {
      displayState: 'open' as const,
      dragActive: true,
      pending: false,
      hasSelection: true,
    },
    selectedCandidate: 'text',
  };

  dispatchWorkflowHorseshoeKeyboardAction({
    event: { key: 'ArrowRight', preventDefault: () => calls.push('prevent') },
    handlers,
    query: 'te',
    selection,
  });
  dispatchWorkflowHorseshoeKeyboardAction({
    event: { key: 'Backspace', preventDefault: () => calls.push('prevent') },
    handlers,
    query: 'te',
    selection,
  });
  dispatchWorkflowHorseshoeKeyboardAction({
    event: { key: 'x', preventDefault: () => calls.push('prevent') },
    handlers,
    query: 'te',
    selection,
  });
  dispatchWorkflowHorseshoeKeyboardAction({
    event: { key: 'Escape', preventDefault: () => calls.push('prevent') },
    handlers,
    query: 'te',
    selection,
  });

  assert.deepEqual(calls, [
    'prevent',
    'rotate:1',
    'prevent',
    'query:t',
    'prevent',
    'query:tex',
    'prevent',
    'close',
  ]);
});

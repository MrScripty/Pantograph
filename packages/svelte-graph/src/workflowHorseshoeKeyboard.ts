import type { HorseshoeDisplayState } from './horseshoeDragSession.js';
import { isSpaceKey, resolveHorseshoeSpaceKeyAction } from './horseshoeInvocation.ts';

type KeyboardTargetLike = {
  isContentEditable?: boolean;
  tagName?: string;
};

type KeyboardEventLike = {
  key: string;
  code?: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
  altKey?: boolean;
};

export type HorseshoeKeyboardAction =
  | { type: 'noop'; preventDefault: false }
  | { type: 'request-open'; preventDefault: true }
  | { type: 'confirm-selection'; preventDefault: true }
  | { type: 'close'; preventDefault: true }
  | { type: 'rotate-selection'; delta: -1 | 1; preventDefault: true }
  | { type: 'remove-query-character'; preventDefault: true }
  | { type: 'append-query-character'; character: string; preventDefault: true };

export interface HorseshoeKeyboardContext {
  displayState: HorseshoeDisplayState;
  dragActive: boolean;
  pending: boolean;
  hasSelection: boolean;
}

const NOOP_ACTION: HorseshoeKeyboardAction = {
  type: 'noop',
  preventDefault: false,
};

export function isEditableKeyboardTarget(target: KeyboardTargetLike | null): boolean {
  if (!target) return false;

  return (
    target.isContentEditable === true ||
    ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName ?? '')
  );
}

export function resolveHorseshoeKeyboardAction(
  event: KeyboardEventLike,
  context: HorseshoeKeyboardContext,
): HorseshoeKeyboardAction {
  if (!context.dragActive && context.displayState === 'hidden') {
    return NOOP_ACTION;
  }

  const spaceAction = isSpaceKey({ code: event.code ?? '', key: event.key })
    ? resolveHorseshoeSpaceKeyAction(context)
    : 'noop';

  if (spaceAction === 'open') {
    return {
      type: 'request-open',
      preventDefault: true,
    };
  }

  if (spaceAction === 'confirm') {
    return {
      type: 'confirm-selection',
      preventDefault: true,
    };
  }

  if (context.displayState === 'hidden') {
    return NOOP_ACTION;
  }

  if (event.key === 'Escape') {
    return {
      type: 'close',
      preventDefault: true,
    };
  }

  if (context.displayState !== 'open') {
    return NOOP_ACTION;
  }

  if (event.key === 'Enter') {
    return {
      type: 'confirm-selection',
      preventDefault: true,
    };
  }

  if (event.key === 'ArrowLeft') {
    return {
      type: 'rotate-selection',
      delta: -1,
      preventDefault: true,
    };
  }

  if (event.key === 'ArrowRight') {
    return {
      type: 'rotate-selection',
      delta: 1,
      preventDefault: true,
    };
  }

  if (event.key === 'Backspace') {
    return {
      type: 'remove-query-character',
      preventDefault: true,
    };
  }

  if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
    return {
      type: 'append-query-character',
      character: event.key,
      preventDefault: true,
    };
  }

  return NOOP_ACTION;
}

/**
 * Clipboard shortcuts for run ids (spec "Run-context picker", Clipboard).
 *
 * - Ctrl/Cmd+C while the pointer is over any element carrying
 *   `data-run-id="<run id>"` copies that id.
 * - Ctrl/Cmd+V while the pointer is over the run chip
 *   (`data-run-context-chip`) reads the clipboard and hands the text to
 *   `onPaste`, which selects a known run or opens the picker.
 *
 * Hover is tracked with one delegated `pointerover` listener, so pages only
 * add the attribute; they never register handlers. Keystrokes whose target is
 * an input, select, textarea or editable element are ignored, as is Ctrl+C
 * while the user has selected text (the browser's copy wins).
 *
 * Lifecycle: one controller per shell. `attach()` adds exactly one listener
 * per event type and throws if called twice; `detach()` removes them and
 * invalidates any clipboard read still in flight. A read that completes after
 * `cancelPendingPaste()`, a newer paste, or `detach()` is classified
 * `superseded` and is not applied.
 *
 * Fallbacks: if the clipboard cannot be written, the id is kept in memory for
 * this window and `onCopied` reports `in_app`. If it cannot be read, the last
 * id copied in this window is used; with none, `onPaste('')` lets the caller
 * open the picker.
 */

export const RUN_ID_ATTRIBUTE = 'data-run-id';
export const RUN_CONTEXT_CHIP_ATTRIBUTE = 'data-run-context-chip';

export type RunCopyOutcome = 'clipboard' | 'in_app';
export type RunPasteSource = 'clipboard' | 'in_app' | 'none';
export type RunPasteOutcome = 'applied' | 'superseded';

/** The subset of a DOM event target the controller needs. */
export interface ListenerTarget {
  addEventListener(type: string, listener: (event: Event) => void): void;
  removeEventListener(type: string, listener: (event: Event) => void): void;
}

export interface RunContextClipboardDeps {
  target: ListenerTarget;
  writeText?: (text: string) => Promise<void>;
  readText?: () => Promise<string>;
  /** True when the user has a non-empty text selection. */
  hasTextSelection?: () => boolean;
  onCopied: (runId: string, outcome: RunCopyOutcome) => void;
  onPaste: (text: string, source: RunPasteSource) => void;
}

export interface RunContextClipboard {
  attach(): void;
  detach(): void;
  isAttached(): boolean;
  /** Invalidate a clipboard read in flight (picker closed, run chosen). */
  cancelPendingPaste(): void;
  copyRunId(runId: string): Promise<RunCopyOutcome>;
  pasteIntoRunContext(): Promise<RunPasteOutcome>;
  /** Hover state, exposed for tests and diagnostics. */
  hoveredRunId(): string | null;
  isChipHovered(): boolean;
}

interface ElementLike {
  tagName?: string;
  isContentEditable?: boolean;
  closest?: (selector: string) => ElementLike | null;
  getAttribute?: (name: string) => string | null;
}

interface KeyEventLike {
  key?: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
  altKey?: boolean;
  target?: unknown;
  preventDefault?: () => void;
}

const FIELD_TAGS = new Set(['INPUT', 'TEXTAREA', 'SELECT']);

function asElement(value: unknown): ElementLike | null {
  return typeof value === 'object' && value !== null ? (value as ElementLike) : null;
}

export function isEditableTarget(target: unknown): boolean {
  const element = asElement(target);
  if (!element) return false;
  if (element.isContentEditable) return true;
  return typeof element.tagName === 'string' && FIELD_TAGS.has(element.tagName.toUpperCase());
}

/** The run id of the nearest `data-run-id` ancestor (inclusive), if any. */
export function runIdFromTarget(target: unknown): string | null {
  const element = asElement(target);
  const holder = element?.closest?.(`[${RUN_ID_ATTRIBUTE}]`) ?? null;
  const runId = holder?.getAttribute?.(RUN_ID_ATTRIBUTE)?.trim();
  return runId ? runId : null;
}

export function isRunContextChipTarget(target: unknown): boolean {
  const element = asElement(target);
  return Boolean(element?.closest?.(`[${RUN_CONTEXT_CHIP_ATTRIBUTE}]`));
}

export function createRunContextClipboard(deps: RunContextClipboardDeps): RunContextClipboard {
  let attached = false;
  let hoveredRunId: string | null = null;
  let chipHovered = false;
  let lastCopiedRunId: string | null = null;
  let pasteGeneration = 0;

  function onPointerOver(event: Event): void {
    hoveredRunId = runIdFromTarget(event.target);
    chipHovered = isRunContextChipTarget(event.target);
  }

  function onPointerOut(event: Event): void {
    // Leaving the window: relatedTarget is null.
    if ((event as Event & { relatedTarget?: unknown }).relatedTarget == null) {
      hoveredRunId = null;
      chipHovered = false;
    }
  }

  function onKeyDown(event: Event): void {
    const key = event as KeyEventLike;
    const withModifier = Boolean(key.ctrlKey || key.metaKey) && !key.altKey;
    if (!withModifier || isEditableTarget(key.target)) return;
    const letter = (key.key ?? '').toLowerCase();
    if (letter === 'c' && hoveredRunId) {
      if (deps.hasTextSelection?.()) return;
      key.preventDefault?.();
      void copyRunId(hoveredRunId);
      return;
    }
    if (letter === 'v' && chipHovered) {
      key.preventDefault?.();
      void pasteIntoRunContext();
    }
  }

  async function copyRunId(runId: string): Promise<RunCopyOutcome> {
    lastCopiedRunId = runId;
    let outcome: RunCopyOutcome = 'in_app';
    if (deps.writeText) {
      try {
        await deps.writeText(runId);
        outcome = 'clipboard';
      } catch {
        outcome = 'in_app';
      }
    }
    deps.onCopied(runId, outcome);
    return outcome;
  }

  async function pasteIntoRunContext(): Promise<RunPasteOutcome> {
    const invocation = ++pasteGeneration;
    let text = '';
    let source: RunPasteSource = 'none';
    if (deps.readText) {
      try {
        text = await deps.readText();
        source = 'clipboard';
      } catch {
        source = 'none';
      }
    }
    if (source === 'none' && lastCopiedRunId) {
      text = lastCopiedRunId;
      source = 'in_app';
    }
    if (!attached || invocation !== pasteGeneration) return 'superseded';
    deps.onPaste(text.trim(), source);
    return 'applied';
  }

  return {
    attach() {
      if (attached) {
        throw new Error('Run-context clipboard is already attached; the shell owns one attachment.');
      }
      attached = true;
      deps.target.addEventListener('pointerover', onPointerOver);
      deps.target.addEventListener('pointerout', onPointerOut);
      deps.target.addEventListener('keydown', onKeyDown);
    },
    detach() {
      if (!attached) return;
      attached = false;
      pasteGeneration += 1;
      hoveredRunId = null;
      chipHovered = false;
      deps.target.removeEventListener('pointerover', onPointerOver);
      deps.target.removeEventListener('pointerout', onPointerOut);
      deps.target.removeEventListener('keydown', onKeyDown);
    },
    isAttached: () => attached,
    cancelPendingPaste() {
      pasteGeneration += 1;
    },
    copyRunId,
    pasteIntoRunContext,
    hoveredRunId: () => hoveredRunId,
    isChipHovered: () => chipHovered,
  };
}

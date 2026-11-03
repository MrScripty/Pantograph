import test from 'node:test';
import assert from 'node:assert/strict';

import {
  createRunContextClipboard,
  isEditableTarget,
  runIdFromTarget,
  type ListenerTarget,
  type RunContextClipboardDeps,
} from './runContextClipboard.ts';

type Listener = (event: Event) => void;

class FakeTarget implements ListenerTarget {
  listeners = new Map<string, Listener[]>();

  addEventListener(type: string, listener: Listener): void {
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), listener]);
  }

  removeEventListener(type: string, listener: Listener): void {
    this.listeners.set(type, (this.listeners.get(type) ?? []).filter((entry) => entry !== listener));
  }

  count(): number {
    return [...this.listeners.values()].reduce((total, list) => total + list.length, 0);
  }

  dispatch(type: string, event: Record<string, unknown>): void {
    for (const listener of this.listeners.get(type) ?? []) listener(event as unknown as Event);
  }
}

interface FakeElement {
  tagName: string;
  parent: FakeElement | null;
  getAttribute(name: string): string | null;
  closest(selector: string): FakeElement | null;
}

/** Minimal element: `closest` walks a parent chain matching attribute selectors. */
function element(
  attributes: Record<string, string>,
  parent: FakeElement | null = null,
  tagName = 'DIV',
): FakeElement {
  const node: FakeElement = {
    tagName,
    parent,
    getAttribute: (name) => attributes[name] ?? null,
    closest(selector) {
      const attribute = selector.slice(1, -1);
      for (let current: FakeElement | null = node; current; current = current.parent) {
        if (current.getAttribute(attribute) !== null) return current;
      }
      return null;
    },
  };
  return node;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function harness(overrides: Partial<RunContextClipboardDeps> = {}) {
  const target = new FakeTarget();
  const copied: Array<[string, string]> = [];
  const pasted: Array<[string, string]> = [];
  const controller = createRunContextClipboard({
    target,
    onCopied: (runId, outcome) => copied.push([runId, outcome]),
    onPaste: (text, source) => pasted.push([text, source]),
    ...overrides,
  });
  return { target, controller, copied, pasted };
}

const flush = () => new Promise((resolve) => setImmediate(resolve));

test('attach adds one listener per event, refuses a second owner, detach removes them', () => {
  const { target, controller } = harness();
  controller.attach();
  assert.equal(target.count(), 3);
  assert.throws(() => controller.attach(), /already attached/);
  assert.equal(target.count(), 3);
  controller.detach();
  controller.detach();
  assert.equal(target.count(), 0);
});

test('hover over a data-run-id element and Ctrl+C copies that run id', async () => {
  const writes: string[] = [];
  const { target, controller, copied } = harness({
    writeText: async (text) => {
      writes.push(text);
    },
  });
  controller.attach();
  const row = element({ 'data-run-id': 'run_a' });
  const child = element({}, row, 'SPAN');
  target.dispatch('pointerover', { target: child });
  assert.equal(controller.hoveredRunId(), 'run_a');

  let prevented = false;
  target.dispatch('keydown', { key: 'c', ctrlKey: true, target: child, preventDefault: () => (prevented = true) });
  await flush();
  assert.equal(prevented, true);
  assert.deepEqual(writes, ['run_a']);
  assert.deepEqual(copied, [['run_a', 'clipboard']]);

  // Cmd+C works the same; moving off the row stops copying.
  target.dispatch('keydown', { key: 'C', metaKey: true, target: child });
  await flush();
  assert.equal(copied.length, 2);
  target.dispatch('pointerover', { target: element({}) });
  target.dispatch('keydown', { key: 'c', ctrlKey: true, target: child });
  await flush();
  assert.equal(copied.length, 2);
});

test('keystrokes in fields, with a text selection or without a modifier are ignored', async () => {
  let selection = false;
  const { target, controller, copied } = harness({ hasTextSelection: () => selection });
  controller.attach();
  target.dispatch('pointerover', { target: element({ 'data-run-id': 'run_a' }) });

  target.dispatch('keydown', { key: 'c', ctrlKey: true, target: { tagName: 'INPUT' } });
  target.dispatch('keydown', { key: 'c', ctrlKey: true, target: { tagName: 'div', isContentEditable: true } });
  target.dispatch('keydown', { key: 'c', target: element({}) });
  target.dispatch('keydown', { key: 'c', ctrlKey: true, altKey: true, target: element({}) });
  selection = true;
  target.dispatch('keydown', { key: 'c', ctrlKey: true, target: element({}) });
  await flush();
  assert.deepEqual(copied, []);
});

test('a failed clipboard write still copies for this window and the paste falls back to it', async () => {
  const { target, controller, copied, pasted } = harness({
    writeText: () => Promise.reject(new Error('denied')),
    readText: () => Promise.reject(new Error('denied')),
  });
  controller.attach();
  target.dispatch('pointerover', { target: element({ 'data-run-id': 'run_a' }) });
  target.dispatch('keydown', { key: 'c', ctrlKey: true, target: element({}) });
  await flush();
  assert.deepEqual(copied, [['run_a', 'in_app']]);

  target.dispatch('pointerover', { target: element({ 'data-run-context-chip': '' }) });
  assert.equal(controller.isChipHovered(), true);
  target.dispatch('keydown', { key: 'v', ctrlKey: true, target: element({}) });
  await flush();
  assert.deepEqual(pasted, [['run_a', 'in_app']]);
});

test('paste with no clipboard and nothing copied hands an empty string to open the picker', async () => {
  const { controller, pasted } = harness();
  controller.attach();
  assert.equal(await controller.pasteIntoRunContext(), 'applied');
  assert.deepEqual(pasted, [['', 'none']]);
});

test('Ctrl+V only acts while the pointer is over the chip', async () => {
  const reads: number[] = [];
  const { target, controller, pasted } = harness({
    readText: async () => {
      reads.push(1);
      return ' run_b ';
    },
  });
  controller.attach();
  target.dispatch('keydown', { key: 'v', ctrlKey: true, target: element({}) });
  await flush();
  assert.equal(reads.length, 0);

  target.dispatch('pointerover', { target: element({}, element({ 'data-run-context-chip': '' })) });
  target.dispatch('keydown', { key: 'v', ctrlKey: true, target: element({}) });
  await flush();
  assert.deepEqual(pasted, [['run_b', 'clipboard']]);
});

test('a clipboard read that resolves after cancel, a newer paste or detach is not applied', async () => {
  const reads: Array<ReturnType<typeof deferred<string>>> = [];
  const { controller, pasted } = harness({
    readText: () => {
      const next = deferred<string>();
      reads.push(next);
      return next.promise;
    },
  });
  controller.attach();

  const cancelled = controller.pasteIntoRunContext();
  controller.cancelPendingPaste();
  reads[0].resolve('run_cancelled');
  assert.equal(await cancelled, 'superseded');

  const older = controller.pasteIntoRunContext();
  const newer = controller.pasteIntoRunContext();
  reads[2].resolve('run_newer');
  reads[1].resolve('run_older');
  assert.equal(await newer, 'applied');
  assert.equal(await older, 'superseded');

  const late = controller.pasteIntoRunContext();
  controller.detach();
  reads[3].resolve('run_late');
  assert.equal(await late, 'superseded');

  assert.deepEqual(pasted, [['run_newer', 'clipboard']]);
});

test('target helpers find the nearest run id and recognise editable targets', () => {
  const row = element({ 'data-run-id': ' run_x ' });
  assert.equal(runIdFromTarget(element({}, row)), 'run_x');
  assert.equal(runIdFromTarget(element({ 'data-run-id': '' })), null);
  assert.equal(runIdFromTarget(null), null);
  assert.equal(isEditableTarget({ tagName: 'textarea' }), true);
  assert.equal(isEditableTarget({ tagName: 'SELECT' }), true);
  assert.equal(isEditableTarget({ tagName: 'BUTTON' }), false);
});

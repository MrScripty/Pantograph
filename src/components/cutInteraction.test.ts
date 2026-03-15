import test from 'node:test';
import assert from 'node:assert/strict';

import {
  findRenderedEdgePath,
  isCutModifierPressed,
  shouldStartCutGesture,
} from './cutInteraction.ts';

test('isCutModifierPressed accepts ctrl and meta shortcuts', () => {
  assert.equal(isCutModifierPressed({ ctrlKey: true }), true);
  assert.equal(isCutModifierPressed({ metaKey: true }), true);
  assert.equal(isCutModifierPressed({ key: 'Control' }), true);
  assert.equal(isCutModifierPressed({ key: 'Meta' }), true);
  assert.equal(isCutModifierPressed({ key: 'Shift' }), false);
});

test('shouldStartCutGesture rejects non-editable or node-bound drags', () => {
  const paneTarget = {
    closest: () => null,
  };
  const nodeTarget = {
    closest: (selector: string) => (selector === '.svelte-flow__node' ? ({} as Element) : null),
  };

  assert.equal(
    shouldStartCutGesture({
      canEdit: false,
      modifierPressed: true,
      target: paneTarget,
    }),
    false,
  );
  assert.equal(
    shouldStartCutGesture({
      canEdit: true,
      modifierPressed: false,
      target: paneTarget,
    }),
    false,
  );
  assert.equal(
    shouldStartCutGesture({
      canEdit: true,
      modifierPressed: true,
      target: nodeTarget,
    }),
    false,
  );
});

test('shouldStartCutGesture allows pane drags with the modifier pressed', () => {
  assert.equal(
    shouldStartCutGesture({
      canEdit: true,
      modifierPressed: true,
      target: {
        closest: () => null,
      },
    }),
    true,
  );
});

test('findRenderedEdgePath matches by data-id without relying on selector escaping', () => {
  const matchingPath = { id: 'path-1' } as SVGPathElement;
  const root = {
    querySelectorAll: () => [
      {
        dataset: { id: 'edge:1/output->target' },
        querySelector: () => matchingPath,
      },
      {
        dataset: { id: 'edge-2' },
        querySelector: () => ({ id: 'path-2' } as SVGPathElement),
      },
    ],
  };

  assert.equal(findRenderedEdgePath(root, 'edge:1/output->target'), matchingPath);
  assert.equal(findRenderedEdgePath(root, 'missing-edge'), null);
});

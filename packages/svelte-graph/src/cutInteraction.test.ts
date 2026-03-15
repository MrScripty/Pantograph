import test from 'node:test';
import assert from 'node:assert/strict';

import {
  applyMatrixToPoint,
  findRenderedEdgePath,
  isCutModifierPressed,
  shouldStartCutGesture,
  toContainerRelativePoint,
} from './cutInteraction.ts';

test('isCutModifierPressed accepts ctrl and meta shortcuts', () => {
  assert.equal(isCutModifierPressed({ ctrlKey: true }), true);
  assert.equal(isCutModifierPressed({ metaKey: true }), true);
  assert.equal(isCutModifierPressed({ key: 'Control' }), true);
  assert.equal(isCutModifierPressed({ key: 'Meta' }), true);
  assert.equal(isCutModifierPressed({ key: 'Shift' }), false);
});

test('shouldStartCutGesture rejects disabled or node-bound drags', () => {
  const paneTarget = {
    closest: () => null,
  };
  const nodeTarget = {
    closest: (selector: string) => (selector === '.svelte-flow__node' ? ({} as Element) : null),
  };

  assert.equal(
    shouldStartCutGesture({
      enabled: false,
      modifierPressed: true,
      target: paneTarget,
    }),
    false,
  );
  assert.equal(
    shouldStartCutGesture({
      enabled: true,
      modifierPressed: false,
      target: paneTarget,
    }),
    false,
  );
  assert.equal(
    shouldStartCutGesture({
      enabled: true,
      modifierPressed: true,
      target: nodeTarget,
    }),
    false,
  );
});

test('shouldStartCutGesture allows pane drags with the modifier pressed', () => {
  assert.equal(
    shouldStartCutGesture({
      enabled: true,
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

test('applyMatrixToPoint handles viewport scale and translation', () => {
  assert.deepEqual(
    applyMatrixToPoint(
      { x: 10, y: 20 },
      { a: 2, b: 0, c: 0, d: 2, e: 100, f: 50 },
    ),
    { x: 120, y: 90 },
  );
});

test('toContainerRelativePoint converts screen points into container space', () => {
  assert.deepEqual(
    toContainerRelativePoint(
      { x: 320, y: 180 },
      { left: 20, top: 30 },
    ),
    { x: 300, y: 150 },
  );
});

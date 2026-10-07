import test from 'node:test';
import assert from 'node:assert/strict';
import { readRunGraphHitDiagnostics } from './run-graph-hit-diagnostics.mjs';

function fixture(rects, hitFactory) {
  const element = (tag, label) => Object.freeze({ tagName: tag,
    getAttribute: () => label, getClientRects: () => rects,
    contains: (other) => other === node || other === target });
  const node = element('g', 'vectors vector-output');
  const target = element('rect', null);
  const calls = [];
  globalThis.window = Object.freeze({ innerWidth: 800, innerHeight: 600 });
  globalThis.document = Object.freeze({ elementsFromPoint: (x, y) => {
    calls.push({ x, y });
    return hitFactory(node, target);
  } });
  return { node, target, calls };
}

test('observes painted descendant hits separately from a missing SVG container hit', () => {
  const rects = [{ left: 600, top: 100, right: 790, bottom: 204, width: 190, height: 104 }];
  const { node, target, calls } = fixture(rects, (_node, body) => [body]);
  const result = readRunGraphHitDiagnostics(node, target);
  assert.deepEqual(calls, [{ x: 695, y: 152 }, { x: 695, y: 152 }]);
  assert.equal(result.node.hits.some(hit => hit.isNode), false);
  assert.equal(result.target.hits[0].isTarget, true);
  assert.equal(result.target.hits[0].inNode, true);
  assert.equal(result.node.label, 'vectors vector-output');
  assert.deepEqual(result.target.rects, rects);
});

test('records viewport-clipped centers and foreign occlusion without changing interaction', () => {
  const rects = [{ left: 700, top: 500, right: 890, bottom: 604, width: 190, height: 104 }];
  const overlay = Object.freeze({ tagName: 'DIV', getAttribute: () => 'overlay' });
  const { node, target, calls } = fixture(rects, (_node, body) => [overlay, body]);
  const result = readRunGraphHitDiagnostics(node, target);
  assert.deepEqual(calls, [{ x: 750, y: 550 }, { x: 750, y: 550 }]);
  assert.equal(result.target.hits[0].inNode, false);
  assert.equal(result.target.hits[1].isTarget, true);
});

test('empty SVG client rects are observable without inventing a pointer location', () => {
  const { node, target, calls } = fixture([], () => { throw new Error('No point should be hit-tested'); });
  const result = readRunGraphHitDiagnostics(node, target);
  assert.equal(result.node.center, null);
  assert.equal(result.target.center, null);
  assert.deepEqual(calls, []);
});

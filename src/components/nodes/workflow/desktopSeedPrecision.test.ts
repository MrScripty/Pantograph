import test from 'node:test';
import assert from 'node:assert/strict';

import {
  DESKTOP_SEED_ERROR,
  DESKTOP_SEED_MAX,
  assertDesktopSeedInputs,
  desktopSeedInputError,
  hasInferenceSeedTarget,
  numberInputDisplayValue,
  numberInputPersistedValue,
  numberInputState,
  parseDesktopSeedValue,
} from './primitiveInputMetadata.ts';

function graph(value: unknown) {
  return {
    nodes: [
      { id: 'number', node_type: 'number-input', data: { value } },
      { id: 'inference', node_type: 'llm-inference', data: {} },
    ],
    edges: [{ source: 'number', source_handle: 'value', target: 'inference', target_handle: 'seed' }],
  };
}

test('desktop seed accepts zero and its safe integer maximum exactly through save/load/replay', () => {
  for (const authored of ['0', '42', '9007199254740991']) {
    const stored = numberInputPersistedValue(authored, true);
    assert.equal(stored, Number(authored));
    const loaded = JSON.parse(JSON.stringify(graph(stored)));
    assert.equal(loaded.nodes[0].data.value, stored);
    assert.equal(numberInputDisplayValue(loaded.nodes[0].data.value, true), authored);
    assert.doesNotThrow(() => assertDesktopSeedInputs(loaded));
    assert.equal(numberInputPersistedValue(numberInputDisplayValue(stored, true), true), stored);
  }
  assert.equal(DESKTOP_SEED_MAX, 9007199254740991);
});

test('unsafe seed text including u64 maximum persists verbatim and blocks replay after save/load', () => {
  for (const authored of ['9007199254740992', '9007199254740993', '18446744073709551615']) {
    const stored = numberInputPersistedValue(authored, true);
    assert.equal(stored, authored);
    assert.equal(typeof stored, 'string');
    const saved = JSON.stringify(graph(stored));
    assert.ok(saved.includes(`"value":"${authored}"`));
    for (let replay = 0; replay < 2; replay += 1) {
      const loaded = JSON.parse(saved);
      assert.equal(numberInputDisplayValue(loaded.nodes[0].data.value, true), authored);
      assert.equal(numberInputState(loaded.nodes[0].data.value, true).error, DESKTOP_SEED_ERROR);
      assert.match(desktopSeedInputError(loaded) ?? '', /9007199254740991/);
      assert.throws(() => assertDesktopSeedInputs(loaded), /Seed must be a whole number/);
      assert.equal(numberInputPersistedValue(loaded.nodes[0].data.value, true), authored);
    }
  }
});

test('seed text rejects fractions before they can round into the supported range', () => {
  for (const authored of ['9007199254740991.1', '1.25', '-1', '-', '1e', '1e3', 'Infinity', 'abc']) {
    assert.equal(numberInputPersistedValue(authored, true), authored);
    assert.equal(parseDesktopSeedValue(authored).error, DESKTOP_SEED_ERROR);
    assert.throws(() => assertDesktopSeedInputs(graph(authored)), /Seed must be a whole number/);
  }
});

test('omission stays absent, clearing stays unset, and correcting a rejected seed permits replay', () => {
  for (const absent of [undefined, null, '']) {
    assert.deepEqual(parseDesktopSeedValue(absent), { value: null, error: null });
    const loaded = JSON.parse(JSON.stringify(graph(absent)));
    assert.doesNotThrow(() => assertDesktopSeedInputs(loaded));
  }
  const omitted = JSON.parse(JSON.stringify(graph(undefined)));
  assert.equal(Object.hasOwn(omitted.nodes[0].data, 'value'), false);
  assert.equal(numberInputPersistedValue('', true), null);
  const rejected = graph(numberInputPersistedValue('9007199254740993', true));
  assert.throws(() => assertDesktopSeedInputs(rejected));
  rejected.nodes[0].data.value = numberInputPersistedValue('0', true);
  assert.doesNotThrow(() => assertDesktopSeedInputs(JSON.parse(JSON.stringify(rejected))));
});

test('saved unsafe numbers are rejected when connected later or through a second fan-out edge', () => {
  for (const unsafe of [2 ** 53, Number('9007199254740993'), Number('18446744073709551615')]) {
    const saved = graph(unsafe);
    saved.nodes.push({ id: 'float-consumer', node_type: 'float-processing', data: {} });
    saved.edges.unshift({ source: 'number', source_handle: 'value', target: 'float-consumer', target_handle: 'speed' });
    assert.equal(hasInferenceSeedTarget('number', saved.nodes, saved.edges), true);
    assert.throws(() => assertDesktopSeedInputs(JSON.parse(JSON.stringify(saved))), /Seed must be a whole number/);
    saved.edges.pop();
    assert.doesNotThrow(() => assertDesktopSeedInputs(saved));
  }
});

test('seed detection supports editor edge names and ignores unrelated ports named seed', () => {
  const nodes = [{ id: 'infer', type: 'llm-inference' }];
  const edges = [{ source: 'input', sourceHandle: 'value', target: 'infer', targetHandle: 'seed' }];
  assert.equal(hasInferenceSeedTarget('input', nodes, edges), true);
  assert.equal(hasInferenceSeedTarget('other', nodes, edges), false);
  assert.equal(hasInferenceSeedTarget('input', [{ id: 'infer', type: 'float-processing' }], edges), false);
});

test('floating-point number nodes retain their existing parser and persistence behavior', () => {
  for (const [authored, expected] of [['1.25', 1.25], ['-2.5', -2.5], ['1e100', 1e100]] as const) {
    assert.equal(numberInputPersistedValue(authored, false), expected);
    assert.deepEqual(numberInputState(expected, false), { value: expected, error: null });
    const unrelated = graph(expected);
    unrelated.nodes[1].node_type = 'float-processing';
    assert.doesNotThrow(() => assertDesktopSeedInputs(unrelated));
  }
});

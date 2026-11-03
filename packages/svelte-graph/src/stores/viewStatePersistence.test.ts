import assert from 'node:assert/strict';
import test from 'node:test';
import { decodeViewStateRecord } from './viewStatePersistence.ts';

test('decoder accepts each current view level, nullable identities and string stacks', () => {
  for (const viewLevel of ['orchestration', 'data-graph', 'group']) {
    const record = { viewLevel, orchestrationId: null, dataGraphId: '', groupStack: ['', 'opaque/id'] };
    assert.deepEqual(decodeViewStateRecord(record), record);
  }
});

test('decoder keeps partial legacy fields and ignores unowned extensions', () => {
  assert.deepEqual(decodeViewStateRecord({}), {});
  assert.deepEqual(decodeViewStateRecord({ groupStack: [], extension: { value: true } }), { groupStack: [] });
});

test('decoder adds no new identity-length or nesting-count limit', () => {
  const record = { dataGraphId: 'x'.repeat(10000), groupStack: Array.from({ length: 512 }, (_, i) => String(i)) };
  assert.deepEqual(decodeViewStateRecord(record), record);
});

test('decoder validates the whole record and detaches the returned stack', () => {
  const groupStack = ['group'];
  const decoded = decodeViewStateRecord({ viewLevel: 'group', groupStack });
  groupStack.push('mutated');
  assert.deepEqual(decoded, { viewLevel: 'group', groupStack: ['group'] });
  assert.equal(decodeViewStateRecord({ viewLevel: 'group', groupStack: ['group', {}] }), null);
});

test('decoder rejects malformed roots and every invalid owned field including nonfinite values', () => {
  const invalid: unknown[] = [
    null, [], 'view', 42, true,
    { viewLevel: 'other' }, { viewLevel: null }, { viewLevel: undefined },
    { orchestrationId: 0 }, { orchestrationId: NaN }, { orchestrationId: {} },
    { dataGraphId: Infinity }, { dataGraphId: -Infinity }, { dataGraphId: false },
    { groupStack: null }, { groupStack: 'group' }, { groupStack: {} },
    { groupStack: ['group', null] }, { groupStack: ['group', undefined] },
    { groupStack: ['group', NaN] }, { groupStack: [['group']] },
    { groupStack: Array(1) },
  ];
  for (const record of invalid) assert.equal(decodeViewStateRecord(record), null);
});

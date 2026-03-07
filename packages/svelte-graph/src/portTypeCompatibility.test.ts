import test from 'node:test';
import assert from 'node:assert/strict';

import { isPortTypeCompatible } from './portTypeCompatibility.ts';

test('isPortTypeCompatible accepts wildcard connections', () => {
  assert.equal(isPortTypeCompatible('any', 'number'), true);
  assert.equal(isPortTypeCompatible('string', 'any'), true);
});

test('isPortTypeCompatible keeps prompt and string interchangeable', () => {
  assert.equal(isPortTypeCompatible('string', 'prompt'), true);
  assert.equal(isPortTypeCompatible('prompt', 'string'), true);
});

test('isPortTypeCompatible preserves audio stream and legacy stream compatibility', () => {
  assert.equal(isPortTypeCompatible('audio_stream', 'stream'), true);
  assert.equal(isPortTypeCompatible('stream', 'audio_stream'), true);
});

test('isPortTypeCompatible accepts identical audio ports', () => {
  assert.equal(isPortTypeCompatible('audio', 'audio'), true);
});

test('isPortTypeCompatible allows primitive-to-string coercions', () => {
  assert.equal(isPortTypeCompatible('number', 'string'), true);
  assert.equal(isPortTypeCompatible('boolean', 'string'), true);
  assert.equal(isPortTypeCompatible('json', 'string'), true);
});

test('isPortTypeCompatible rejects unrelated types', () => {
  assert.equal(isPortTypeCompatible('image', 'string'), false);
  assert.equal(isPortTypeCompatible('number', 'boolean'), false);
});

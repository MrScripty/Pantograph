import test from 'node:test';
import assert from 'node:assert/strict';

import { removeNodeDataKeys } from './runtimeData.ts';

test('removeNodeDataKeys removes only targeted runtime fields', () => {
  const result = removeNodeDataKeys(
    {
      label: 'Audio Output',
      audio: 'base64-audio',
      stream: { sequence: 1 },
      audio_mime: 'audio/wav',
    },
    ['audio', 'stream']
  );

  assert.equal(result.changed, true);
  assert.deepEqual(result.data, {
    label: 'Audio Output',
    audio_mime: 'audio/wav',
  });
});

test('removeNodeDataKeys reports unchanged when no requested keys are present', () => {
  const result = removeNodeDataKeys(
    {
      label: 'Audio Output',
      definition: { node_type: 'audio-output' },
    },
    ['audio', 'stream']
  );

  assert.equal(result.changed, false);
  assert.deepEqual(result.data, {
    label: 'Audio Output',
    definition: { node_type: 'audio-output' },
  });
});

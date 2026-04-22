import test from 'node:test';
import assert from 'node:assert/strict';
import type { Node } from '@xyflow/svelte';

import {
  appendNodeStreamContent,
  clearNodeRuntimeDataKeysInNodes,
  clearNodeStreamContent,
  removeNodeDataKeys,
  setNodeStreamContent,
  updateNodeRuntimeDataInNodes,
} from './runtimeData.ts';

function node(id: string, data: Record<string, unknown> = {}): Node {
  return {
    id,
    data,
    position: { x: 0, y: 0 },
  };
}

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

test('updateNodeRuntimeDataInNodes merges runtime data into the targeted node', () => {
  const first = node('first', { label: 'First' });
  const second = node('second', { label: 'Second' });
  const result = updateNodeRuntimeDataInNodes([first, second], 'second', {
    output: 'ready',
  });

  assert.equal(result[0], first);
  assert.notEqual(result[1], second);
  assert.deepEqual(result[1].data, {
    label: 'Second',
    output: 'ready',
  });
});

test('clearNodeRuntimeDataKeysInNodes removes runtime keys while preserving unchanged nodes', () => {
  const first = node('first', { label: 'First', stream: 'chunk' });
  const second = node('second', { label: 'Second' });
  const result = clearNodeRuntimeDataKeysInNodes([first, second], ['stream']);

  assert.notEqual(result[0], first);
  assert.deepEqual(result[0].data, { label: 'First' });
  assert.equal(result[1], second);
});

test('stream content helpers append set and clear node stream content', () => {
  const first = node('first', { label: 'First' });
  const second = node('second', { label: 'Second', streamContent: 'old' });

  const appended = appendNodeStreamContent([first, second], 'first', 'chunk');
  assert.deepEqual(appended[0].data, { label: 'First', streamContent: 'chunk' });
  assert.equal(appended[1], second);

  const set = setNodeStreamContent(appended, 'first', 'final');
  assert.deepEqual(set[0].data, { label: 'First', streamContent: 'final' });
  assert.equal(set[1], second);

  const cleared = clearNodeStreamContent(set);
  assert.deepEqual(cleared[0].data, { label: 'First', streamContent: '' });
  assert.deepEqual(cleared[1].data, { label: 'Second', streamContent: '' });
});

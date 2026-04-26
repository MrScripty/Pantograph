import test from 'node:test';
import assert from 'node:assert/strict';
import type { Node } from '@xyflow/svelte';

import {
  appendNodeStreamContentOverlay,
  clearNodeRuntimeOverlayKeys,
  clearNodeStreamContentOverlay,
  mergeNodeRuntimeOverlays,
  removeNodeDataKeys,
  setNodeStreamContentOverlay,
  updateNodeRuntimeOverlay,
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

test('updateNodeRuntimeOverlay merges runtime data into the targeted overlay', () => {
  const overlays = updateNodeRuntimeOverlay(new Map(), 'second', {
    output: 'ready',
  });

  assert.deepEqual(overlays.get('second'), {
    output: 'ready',
  });

  const updated = updateNodeRuntimeOverlay(overlays, 'second', {
    audio: 'base64',
  });
  assert.deepEqual(updated.get('second'), {
    output: 'ready',
    audio: 'base64',
  });
  assert.equal(overlays.get('second')?.audio, undefined);
});

test('mergeNodeRuntimeOverlays applies overlays without mutating structural nodes', () => {
  const first = node('first', { label: 'First' });
  const second = node('second', { label: 'Second' });
  const overlays = new Map([['second', { output: 'ready' }]]);
  const result = mergeNodeRuntimeOverlays([first, second], overlays);

  assert.equal(result[0], first);
  assert.notEqual(result[1], second);
  assert.deepEqual(result[1].data, {
    label: 'Second',
    output: 'ready',
  });
  assert.deepEqual(second.data, { label: 'Second' });
});

test('clearNodeRuntimeOverlayKeys removes runtime keys while preserving other overlays', () => {
  const overlays = new Map([
    ['first', { stream: 'chunk', audio: 'base64' }],
    ['second', { label: 'Second' }],
  ]);
  const result = clearNodeRuntimeOverlayKeys(overlays, ['stream']);

  assert.deepEqual(result.get('first'), { audio: 'base64' });
  assert.equal(result.get('second'), overlays.get('second'));
});

test('stream content helpers append set and clear node stream content', () => {
  const initial = new Map([['second', { streamContent: 'old' }]]);

  const appended = appendNodeStreamContentOverlay(initial, 'first', 'chunk');
  assert.deepEqual(appended.get('first'), { streamContent: 'chunk' });
  assert.equal(appended.get('second'), initial.get('second'));

  const set = setNodeStreamContentOverlay(appended, 'first', 'final');
  assert.deepEqual(set.get('first'), { streamContent: 'final' });
  assert.equal(set.get('second'), initial.get('second'));

  const cleared = clearNodeStreamContentOverlay(set);
  assert.deepEqual(cleared.get('first'), { streamContent: '' });
  assert.deepEqual(cleared.get('second'), { streamContent: '' });
});

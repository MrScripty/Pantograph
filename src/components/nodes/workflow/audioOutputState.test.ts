import test from 'node:test';
import assert from 'node:assert/strict';

import {
  AUDIO_RUNTIME_DATA_KEYS,
  shouldResetAudioPlaybackState,
} from './audioOutputState.ts';

test('shouldResetAudioPlaybackState returns true for idle rerun cleanup after streamed audio', () => {
  assert.equal(
    shouldResetAudioPlaybackState({
      executionState: 'idle',
      hasFinalAudio: false,
      hasStreamPayload: false,
      hasStreamContext: true,
      hasStreamAudio: true,
    }),
    true
  );
});

test('shouldResetAudioPlaybackState returns false while a new stream chunk is present', () => {
  assert.equal(
    shouldResetAudioPlaybackState({
      executionState: 'idle',
      hasFinalAudio: false,
      hasStreamPayload: true,
      hasStreamContext: true,
      hasStreamAudio: true,
    }),
    false
  );
});

test('shouldResetAudioPlaybackState returns false for final audio rerender with no buffered stream', () => {
  assert.equal(
    shouldResetAudioPlaybackState({
      executionState: 'idle',
      hasFinalAudio: true,
      hasStreamPayload: false,
      hasStreamContext: false,
      hasStreamAudio: false,
    }),
    false
  );
});

test('AUDIO_RUNTIME_DATA_KEYS lists the execution-local audio fields cleared between runs', () => {
  assert.deepEqual([...AUDIO_RUNTIME_DATA_KEYS], [
    'audio',
    'audio_mime',
    'stream',
    'stream_sequence',
    'stream_is_final',
  ]);
});

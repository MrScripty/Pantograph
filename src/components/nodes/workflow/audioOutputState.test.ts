import test from 'node:test';
import assert from 'node:assert/strict';

import {
  AUDIO_RUNTIME_DATA_KEYS,
  buildAudioRuntimeDataFromCompletedOutputs,
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
    'audio_duration_seconds',
    'audio_sample_rate',
    'stream',
    'stream_sequence',
    'stream_is_final',
  ]);
});

test('buildAudioRuntimeDataFromCompletedOutputs maps final audio metadata for audio targets', () => {
  assert.deepEqual(
    buildAudioRuntimeDataFromCompletedOutputs('audio', 'audio', {
      duration_seconds: 12.5,
      sample_rate: 44100,
      mime_type: 'audio/wav',
    }),
    {
      audio_duration_seconds: 12.5,
      audio_sample_rate: 44100,
      audio_mime: 'audio/wav',
    }
  );
});

test('buildAudioRuntimeDataFromCompletedOutputs ignores non-audio connections', () => {
  assert.equal(
    buildAudioRuntimeDataFromCompletedOutputs('response', 'audio', {
      duration_seconds: 12.5,
    }),
    null
  );
});

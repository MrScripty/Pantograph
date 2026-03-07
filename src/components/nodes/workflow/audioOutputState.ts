import type { NodeExecutionState } from '../../../services/workflow/types';

export const AUDIO_RUNTIME_DATA_KEYS = [
  'audio',
  'audio_mime',
  'audio_duration_seconds',
  'audio_sample_rate',
  'stream',
  'stream_sequence',
  'stream_is_final',
] as const;

export interface AudioPlaybackResetState {
  executionState: NodeExecutionState;
  hasFinalAudio: boolean;
  hasStreamPayload: boolean;
  hasStreamContext: boolean;
  hasStreamAudio: boolean;
}

export function shouldResetAudioPlaybackState(state: AudioPlaybackResetState): boolean {
  return (
    state.executionState === 'idle' &&
    !state.hasFinalAudio &&
    !state.hasStreamPayload &&
    (state.hasStreamContext || state.hasStreamAudio)
  );
}

function asPositiveNumber(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : null;
}

export function buildAudioRuntimeDataFromCompletedOutputs(
  sourceHandle: string,
  targetHandle: string,
  outputs: Record<string, unknown>
): Record<string, unknown> | null {
  if (sourceHandle !== 'audio' || targetHandle !== 'audio') {
    return null;
  }

  const runtimeData: Record<string, unknown> = {};
  const durationSeconds =
    asPositiveNumber(outputs.duration_seconds) ?? asPositiveNumber(outputs.audio_duration_seconds);
  if (durationSeconds !== null) {
    runtimeData.audio_duration_seconds = durationSeconds;
  }

  const sampleRate =
    asPositiveNumber(outputs.sample_rate) ?? asPositiveNumber(outputs.audio_sample_rate);
  if (sampleRate !== null) {
    runtimeData.audio_sample_rate = sampleRate;
  }

  const audioMime =
    typeof outputs.audio_mime === 'string' && outputs.audio_mime.length > 0
      ? outputs.audio_mime
      : typeof outputs.mime_type === 'string' && outputs.mime_type.length > 0
        ? outputs.mime_type
        : null;
  if (audioMime) {
    runtimeData.audio_mime = audioMime;
  }

  return Object.keys(runtimeData).length > 0 ? runtimeData : null;
}

import type { NodeExecutionState } from '../../../services/workflow/types';

export const AUDIO_RUNTIME_DATA_KEYS = [
  'audio',
  'audio_mime',
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

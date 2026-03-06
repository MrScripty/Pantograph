<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      audio?: string;
      audio_mime?: string;
      stream?: unknown;
      streamContent?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let audioElement = $state<HTMLAudioElement | null>(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let volume = $state(1);
  let lastAudioSignature = $state('');

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');

  let streamPayload = $derived.by(() => {
    const payload = data.stream;
    if (payload && typeof payload === 'object') {
      const maybeChunk = payload as {
        audio_base64?: unknown;
        content?: unknown;
        mime_type?: unknown;
      };
      if (typeof maybeChunk.audio_base64 === 'string' && maybeChunk.audio_base64.length > 0) {
        return {
          audioBase64: maybeChunk.audio_base64,
          mimeType:
            typeof maybeChunk.mime_type === 'string' && maybeChunk.mime_type.length > 0
              ? maybeChunk.mime_type
              : 'audio/wav',
        };
      }
      if (typeof maybeChunk.content === 'string' && maybeChunk.content.length > 0) {
        return {
          audioBase64: maybeChunk.content,
          mimeType:
            typeof maybeChunk.mime_type === 'string' && maybeChunk.mime_type.length > 0
              ? maybeChunk.mime_type
              : 'audio/wav',
        };
      }
    }
    if (typeof payload === 'string' && payload.length > 0) {
      return { audioBase64: payload, mimeType: 'audio/wav' };
    }
    return null;
  });
  let audioData = $derived(streamPayload?.audioBase64 || data.audio || '');
  let audioMime = $derived(streamPayload?.mimeType || data.audio_mime || 'audio/wav');
  let audioSrc = $derived(audioData ? `data:${audioMime};base64,${audioData}` : '');

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-pink-500 animate-pulse',
      success: 'bg-pink-500',
      error: 'bg-red-500',
    }[executionState]
  );

  function formatTime(totalSeconds: number): string {
    if (!Number.isFinite(totalSeconds) || totalSeconds <= 0) {
      return '0:00';
    }
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = Math.floor(totalSeconds % 60);
    return `${minutes}:${seconds.toString().padStart(2, '0')}`;
  }

  function extensionForMimeType(mimeType: string): string {
    const normalized = mimeType.toLowerCase();
    if (normalized.includes('mpeg') || normalized.includes('mp3')) return 'mp3';
    if (normalized.includes('ogg')) return 'ogg';
    if (normalized.includes('flac')) return 'flac';
    return 'wav';
  }

  function handleLoadedMetadata() {
    if (!audioElement) return;
    const nextDuration = Number.isFinite(audioElement.duration) ? audioElement.duration : 0;
    duration = nextDuration;
    currentTime = audioElement.currentTime || 0;
  }

  function handleTimeUpdate() {
    if (!audioElement) return;
    currentTime = audioElement.currentTime || 0;
  }

  function handleSeek(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    const nextTime = Number(target?.value ?? '0');
    if (!Number.isFinite(nextTime)) return;
    currentTime = nextTime;
    if (audioElement) {
      audioElement.currentTime = nextTime;
    }
  }

  function handleVolumeChange(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    const nextVolume = Number(target?.value ?? '1');
    if (!Number.isFinite(nextVolume)) return;
    volume = Math.min(1, Math.max(0, nextVolume));
    if (audioElement) {
      audioElement.volume = volume;
    }
  }

  function togglePlayback() {
    if (!audioElement) return;
    if (audioElement.paused) {
      void audioElement.play().catch(() => {});
      return;
    }
    audioElement.pause();
  }

  function handlePlay() {
    isPlaying = true;
  }

  function handlePause() {
    isPlaying = false;
  }

  function handleEnded() {
    isPlaying = false;
    currentTime = duration;
  }

  $effect(() => {
    if (audioElement) {
      audioElement.volume = volume;
    }
  });

  $effect(() => {
    if (!audioSrc) {
      lastAudioSignature = '';
      isPlaying = false;
      currentTime = 0;
      duration = 0;
      return;
    }

    if (!audioElement || audioData === lastAudioSignature) {
      return;
    }

    lastAudioSignature = audioData;
    void audioElement.play().catch(() => {});
  });

  function downloadAudio() {
    if (!audioData) return;
    const byteChars = atob(audioData);
    const bytes = new Uint8Array(byteChars.length);
    for (let i = 0; i < byteChars.length; i++) {
      bytes[i] = byteChars.charCodeAt(i);
    }
    const blob = new Blob([bytes], { type: audioMime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `output.${extensionForMimeType(audioMime)}`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="audio-output-wrapper" style="--node-color: #f472b6">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-pink-500 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Audio Output'}</span>
        <span class="w-2 h-2 rounded-full {statusColor} ml-auto"></span>
      </div>
    {/snippet}

    {#snippet children()}
      {#if audioSrc}
        <div class="space-y-1">
          <audio
            bind:this={audioElement}
            src={audioSrc}
            preload="metadata"
            onloadedmetadata={handleLoadedMetadata}
            ontimeupdate={handleTimeUpdate}
            onplay={handlePlay}
            onpause={handlePause}
            onended={handleEnded}
          ></audio>
          <div class="space-y-1">
            <div class="flex items-center gap-2">
              <button
                type="button"
                class="text-[10px] px-2 py-1 rounded bg-neutral-700 hover:bg-neutral-600 text-neutral-200 border border-neutral-600 cursor-pointer"
                onclick={togglePlayback}
              >
                {isPlaying ? 'Pause' : 'Play'}
              </button>
              <span class="text-[10px] text-neutral-400 tabular-nums">
                {formatTime(currentTime)} / {formatTime(duration)}
              </span>
            </div>
            <input
              type="range"
              min="0"
              max={duration > 0 ? duration : 1}
              step="0.01"
              value={currentTime}
              class="w-full h-1.5 accent-pink-500 cursor-pointer"
              oninput={handleSeek}
            />
            <div class="flex items-center gap-2">
              <span class="text-[10px] text-neutral-400">Vol</span>
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={volume}
                class="w-full h-1.5 accent-pink-500 cursor-pointer"
                oninput={handleVolumeChange}
              />
            </div>
          </div>
          <div class="flex justify-end">
            <button type="button"
              class="text-[10px] text-neutral-400 hover:text-neutral-200 bg-transparent border-0 cursor-pointer px-1"
              onclick={downloadAudio}
            >
              Download
            </button>
          </div>
        </div>
      {:else}
        <div class="text-xs text-neutral-500 italic">
          No audio yet
        </div>
      {/if}
    {/snippet}
  </BaseNode>
</div>

<style>
  .audio-output-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .audio-output-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }
</style>

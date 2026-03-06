<script lang="ts">
  import { onDestroy } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';
  import { shouldResetAudioPlaybackState } from './audioOutputState';

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

  interface StreamAudioChunk {
    audioBase64: string;
    mimeType: string;
    sequence: number | null;
    isFinal: boolean;
    mode: 'append' | 'replace';
  }

  let { id, data, selected = false }: Props = $props();

  let audioElement = $state<HTMLAudioElement | null>(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let volume = $state(1);
  let loopEnabled = $state(false);
  let lastAudioSignature = $state('');
  let hasStreamAudio = $state(false);
  let streamBufferedDuration = $state(0);
  let streamQueueEndTime = $state(0);
  let streamPlaybackStartedAt = $state<number | null>(null);
  let lastProcessedSequence = $state<number | null>(null);
  let lastProcessedChunkSignature = $state('');
  let streamProgressTimer = $state<number | null>(null);
  let streamContext = $state<AudioContext | null>(null);
  let streamGainNode = $state<GainNode | null>(null);

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');

  let finalAudioData = $derived(data.audio || '');
  let finalAudioMime = $derived(data.audio_mime || 'audio/wav');
  let finalAudioSrc = $derived(finalAudioData ? `data:${finalAudioMime};base64,${finalAudioData}` : '');

  let streamPayload = $derived.by((): StreamAudioChunk | null => {
    const payload = data.stream;
    if (typeof payload === 'string' && payload.length > 0) {
      return {
        audioBase64: payload,
        mimeType: 'audio/wav',
        sequence: null,
        isFinal: false,
        mode: 'append',
      };
    }
    if (!payload || typeof payload !== 'object') return null;

    const maybeChunk = payload as {
      audio_base64?: unknown;
      content?: unknown;
      mime_type?: unknown;
      sequence?: unknown;
      is_final?: unknown;
      mode?: unknown;
    };

    const audioValue =
      typeof maybeChunk.audio_base64 === 'string' && maybeChunk.audio_base64.length > 0
        ? maybeChunk.audio_base64
        : typeof maybeChunk.content === 'string' && maybeChunk.content.length > 0
          ? maybeChunk.content
          : null;
    if (!audioValue) return null;

    const sequence =
      typeof maybeChunk.sequence === 'number' && Number.isFinite(maybeChunk.sequence)
        ? maybeChunk.sequence
        : null;
    const mimeType =
      typeof maybeChunk.mime_type === 'string' && maybeChunk.mime_type.length > 0
        ? maybeChunk.mime_type
        : 'audio/wav';

    return {
      audioBase64: audioValue,
      mimeType,
      sequence,
      isFinal: maybeChunk.is_final === true,
      mode: maybeChunk.mode === 'replace' ? 'replace' : 'append',
    };
  });

  let displayedDuration = $derived(finalAudioSrc ? duration : streamBufferedDuration);
  let canSeek = $derived(Boolean(finalAudioSrc));
  let hasAnyAudio = $derived(Boolean(finalAudioSrc) || hasStreamAudio);

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

  function base64ToArrayBuffer(base64: string): ArrayBuffer {
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes.buffer;
  }

  function updateStreamingProgress() {
    if (!streamContext || streamPlaybackStartedAt === null) return;
    const elapsed = Math.max(0, streamContext.currentTime - streamPlaybackStartedAt);
    currentTime = Math.min(elapsed, streamBufferedDuration);
  }

  function startStreamingProgressTimer() {
    if (streamProgressTimer !== null) return;
    streamProgressTimer = window.setInterval(() => {
      updateStreamingProgress();
    }, 100);
  }

  function stopStreamingProgressTimer() {
    if (streamProgressTimer === null) return;
    window.clearInterval(streamProgressTimer);
    streamProgressTimer = null;
  }

  async function ensureStreamContext(): Promise<AudioContext | null> {
    if (streamContext) return streamContext;
    if (typeof window === 'undefined') return null;

    const audioWindow = window as Window & { webkitAudioContext?: typeof AudioContext };
    const ContextCtor = audioWindow.AudioContext || audioWindow.webkitAudioContext;
    if (!ContextCtor) return null;

    const context = new ContextCtor();
    const gain = context.createGain();
    gain.gain.value = volume;
    gain.connect(context.destination);

    streamContext = context;
    streamGainNode = gain;
    return context;
  }

  async function stopStreamPlayback(resetTimeline: boolean) {
    const context = streamContext;
    streamContext = null;
    streamGainNode = null;
    stopStreamingProgressTimer();

    if (context) {
      try {
        await context.close();
      } catch {
        // Best-effort teardown.
      }
    }

    if (resetTimeline) {
      hasStreamAudio = false;
      streamBufferedDuration = 0;
      streamQueueEndTime = 0;
      streamPlaybackStartedAt = null;
      lastProcessedSequence = null;
      lastProcessedChunkSignature = '';
      if (!finalAudioSrc) {
        currentTime = 0;
      }
    }
    if (!finalAudioSrc) {
      isPlaying = false;
    }
  }

  async function queueStreamChunk(chunk: StreamAudioChunk) {
    if (chunk.mode === 'replace') {
      await stopStreamPlayback(true);
    }

    if (chunk.sequence !== null) {
      if (lastProcessedSequence !== null && chunk.sequence <= lastProcessedSequence) {
        return;
      }
      lastProcessedSequence = chunk.sequence;
    } else {
      const signature = `${chunk.audioBase64.length}:${chunk.audioBase64.slice(0, 64)}`;
      if (signature === lastProcessedChunkSignature) return;
      lastProcessedChunkSignature = signature;
    }

    const context = await ensureStreamContext();
    if (!context || !streamGainNode) return;

    try {
      const encoded = base64ToArrayBuffer(chunk.audioBase64);
      const decoded = await context.decodeAudioData(encoded.slice(0));

      if (context.state === 'suspended') {
        await context.resume();
      }

      const startAt = Math.max(streamQueueEndTime, context.currentTime + 0.01);
      const source = context.createBufferSource();
      source.buffer = decoded;
      source.connect(streamGainNode);
      source.start(startAt);

      if (streamPlaybackStartedAt === null) {
        streamPlaybackStartedAt = startAt;
      }
      streamQueueEndTime = startAt + decoded.duration;
      streamBufferedDuration = Math.max(
        streamBufferedDuration,
        streamQueueEndTime - streamPlaybackStartedAt
      );
      hasStreamAudio = true;
      isPlaying = true;
      startStreamingProgressTimer();

      if (chunk.isFinal) {
        source.onended = () => {
          if (!finalAudioSrc && streamContext?.state !== 'running') {
            isPlaying = false;
          }
        };
      }
    } catch {
      // Ignore malformed/undecodable stream chunks.
    }
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
    if (!finalAudioSrc) return;
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
    if (streamGainNode) {
      streamGainNode.gain.value = volume;
    }
  }

  async function togglePlayback() {
    if (finalAudioSrc) {
      if (!audioElement) return;
      if (audioElement.paused) {
        await audioElement.play().catch(() => {});
      } else {
        audioElement.pause();
      }
      return;
    }

    if (!streamContext) return;
    if (streamContext.state === 'running') {
      await streamContext.suspend().catch(() => {});
      isPlaying = false;
      return;
    }
    await streamContext.resume().catch(() => {});
    isPlaying = true;
    startStreamingProgressTimer();
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

  function handleReplay() {
    if (!finalAudioSrc || !audioElement) return;
    audioElement.currentTime = 0;
    currentTime = 0;
    void audioElement.play().catch(() => {});
  }

  function handleLoopToggle(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    loopEnabled = target?.checked === true;
    if (audioElement) {
      audioElement.loop = loopEnabled;
    }
  }

  function stopControlEvent(event: Event) {
    event.stopPropagation();
  }

  function downloadAudio() {
    if (!finalAudioData) return;
    const byteChars = atob(finalAudioData);
    const bytes = new Uint8Array(byteChars.length);
    for (let i = 0; i < byteChars.length; i++) {
      bytes[i] = byteChars.charCodeAt(i);
    }
    const blob = new Blob([bytes], { type: finalAudioMime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `output.${extensionForMimeType(finalAudioMime)}`;
    a.click();
    URL.revokeObjectURL(url);
  }

  $effect(() => {
    if (audioElement) {
      audioElement.volume = volume;
      audioElement.loop = loopEnabled;
    }
    if (streamGainNode) {
      streamGainNode.gain.value = volume;
    }
  });

  $effect(() => {
    const chunk = streamPayload;
    if (!chunk || finalAudioSrc) return;
    void queueStreamChunk(chunk);
  });

  $effect(() => {
    if (!finalAudioSrc) return;
    if (streamContext || hasStreamAudio) {
      void stopStreamPlayback(true);
    }

    if (!audioElement || finalAudioData === lastAudioSignature) {
      return;
    }
    lastAudioSignature = finalAudioData;
    void audioElement.play().catch(() => {});
  });

  $effect(() => {
    const chunk = streamPayload;
    if (
      shouldResetAudioPlaybackState({
        executionState,
        hasFinalAudio: Boolean(finalAudioSrc),
        hasStreamPayload: Boolean(chunk),
        hasStreamContext: streamContext !== null,
        hasStreamAudio,
      })
    ) {
      void stopStreamPlayback(true);
      return;
    }
    if (finalAudioSrc || hasStreamAudio || chunk) return;

    lastAudioSignature = '';
    isPlaying = false;
    currentTime = 0;
    duration = 0;
  });

  onDestroy(() => {
    stopStreamingProgressTimer();
    void stopStreamPlayback(true);
  });
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

    {#if hasAnyAudio}
      <div class="space-y-1">
        {#if finalAudioSrc}
          <audio
            bind:this={audioElement}
            src={finalAudioSrc}
            preload="metadata"
            onloadedmetadata={handleLoadedMetadata}
            ontimeupdate={handleTimeUpdate}
            onplay={handlePlay}
            onpause={handlePause}
            onended={handleEnded}
          ></audio>
        {/if}
        <div class="space-y-1">
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="nodrag nopan nowheel text-[10px] px-2 py-1 rounded bg-neutral-700 hover:bg-neutral-600 text-neutral-200 border border-neutral-600 cursor-pointer"
              onclick={togglePlayback}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onclickcapture={stopControlEvent}
            >
              {isPlaying ? 'Pause' : 'Play'}
            </button>
            <button
              type="button"
              class="nodrag nopan nowheel text-[10px] px-2 py-1 rounded bg-neutral-700 hover:bg-neutral-600 text-neutral-200 border border-neutral-600 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
              onclick={handleReplay}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onclickcapture={stopControlEvent}
              disabled={!finalAudioSrc}
            >
              Replay
            </button>
            <span class="text-[10px] text-neutral-400 tabular-nums">
              {formatTime(currentTime)} / {formatTime(displayedDuration)}
            </span>
            {#if !finalAudioSrc}
              <span class="text-[10px] text-pink-300">Streaming</span>
            {/if}
          </div>
          <input
            type="range"
            min="0"
            max={displayedDuration > 0 ? displayedDuration : 1}
            step="0.01"
            value={currentTime}
            class="nodrag nopan nowheel w-full h-1.5 accent-pink-500 cursor-pointer"
            disabled={!canSeek}
            oninput={handleSeek}
            onmousedown={stopControlEvent}
            onmouseup={stopControlEvent}
            onpointerdown={stopControlEvent}
            onpointerup={stopControlEvent}
            onclickcapture={stopControlEvent}
          />
          <div class="flex items-center gap-2">
            <span class="text-[10px] text-neutral-400">Vol</span>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={volume}
              class="nodrag nopan nowheel w-full h-1.5 accent-pink-500 cursor-pointer"
              oninput={handleVolumeChange}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onpointerdown={stopControlEvent}
              onpointerup={stopControlEvent}
              onclickcapture={stopControlEvent}
            />
          </div>
          <label class="nodrag nopan nowheel flex items-center gap-2 text-[10px] text-neutral-400">
            <input
              type="checkbox"
              checked={loopEnabled}
              class="cursor-pointer"
              disabled={!finalAudioSrc}
              onchange={handleLoopToggle}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onclickcapture={stopControlEvent}
            />
            Loop
          </label>
        </div>
        {#if finalAudioSrc}
          <div class="flex justify-end">
            <button
              type="button"
              class="nodrag nopan nowheel text-[10px] text-neutral-400 hover:text-neutral-200 bg-transparent border-0 cursor-pointer px-1"
              onclick={downloadAudio}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onclickcapture={stopControlEvent}
            >
              Download
            </button>
          </div>
        {/if}
      </div>
    {:else}
      <div class="text-xs text-neutral-500 italic">
        No audio yet
      </div>
    {/if}
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

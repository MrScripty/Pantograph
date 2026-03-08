<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      audio_data?: string;
      audio_mime?: string;
      fileName?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  const nodeColor = '#f472b6';

  let fileInputElement = $state<HTMLInputElement | null>(null);
  let audioElement = $state<HTMLAudioElement | null>(null);
  let loadError = $state<string | null>(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let volume = $state(1);

  let audioData = $derived(data.audio_data ?? '');
  let audioMime = $derived(data.audio_mime ?? 'audio/wav');
  let audioSrc = $derived(audioData ? `data:${audioMime};base64,${audioData}` : '');

  function formatTime(totalSeconds: number): string {
    if (!Number.isFinite(totalSeconds) || totalSeconds <= 0) {
      return '0:00';
    }
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = Math.floor(totalSeconds % 60);
    return `${minutes}:${seconds.toString().padStart(2, '0')}`;
  }

  function openFilePicker() {
    fileInputElement?.click();
  }

  function readFileAsDataUrl(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onerror = () => reject(new Error('Failed to read selected audio file'));
      reader.onload = () => {
        const result = reader.result;
        if (typeof result !== 'string') {
          reject(new Error('Unexpected file reader payload'));
          return;
        }
        resolve(result);
      };
      reader.readAsDataURL(file);
    });
  }

  async function handleFileSelected(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    const file = target?.files?.[0];
    if (!file) return;

    try {
      loadError = null;
      const dataUrl = await readFileAsDataUrl(file);
      const [header, base64Payload] = dataUrl.split(',', 2);
      if (!header || !base64Payload) {
        throw new Error('Failed to parse encoded audio file payload');
      }

      const mimeMatch = header.match(/^data:(.*);base64$/);
      const resolvedMime =
        (mimeMatch && mimeMatch[1].trim().length > 0 ? mimeMatch[1].trim() : file.type) ||
        'audio/wav';

      updateNodeData(id, {
        audio_data: base64Payload,
        audio_mime: resolvedMime,
        fileName: file.name,
      });
    } catch (error) {
      loadError = error instanceof Error ? error.message : 'Failed to load audio file';
    } finally {
      if (target) {
        target.value = '';
      }
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
      isPlaying = false;
      currentTime = 0;
      duration = 0;
    }
  });
</script>

<div class="audio-input-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {nodeColor}">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Audio Input'}</span>
      </div>
    {/snippet}

      <div class="space-y-2">
        <input
          bind:this={fileInputElement}
          type="file"
          accept="audio/*,.wav,.mp3,.ogg,.flac"
          class="hidden"
          onchange={handleFileSelected}
        />
        <button type="button"
          class="w-full text-xs px-2 py-1 rounded bg-neutral-700 hover:bg-neutral-600 text-neutral-300 border border-neutral-600 cursor-pointer"
          onclick={openFilePicker}
        >
          Choose File
        </button>
        {#if data.fileName}
          <div class="text-[10px] text-neutral-400 truncate">{data.fileName}</div>
        {/if}
        {#if loadError}
          <div class="text-[10px] text-red-400">{loadError}</div>
        {/if}
        {#if audioSrc}
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
        {/if}
      </div>
  </BaseNode>
</div>

<style>
  .audio-input-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .audio-input-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }
</style>

<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import type {
    NodeDefinition,
    WorkflowAudioArtifactFormatSettings,
    WorkflowMediaFormatOption,
  } from '../../../services/workflow/types';
  import { workflowService } from '../../../services/workflow/WorkflowService';
  import { nodeExecutionStates, updateNodeData } from '../../../stores/workflowStore';
  import { shouldResetAudioPlaybackState } from './audioOutputState';

  interface AudioArtifactFormatOverride {
    container_id: string;
    codec_id: string;
    bitrate_kbps: number;
  }

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      audio?: string;
      audio_mime?: string;
      audio_duration_seconds?: number;
      audio_sample_rate?: number;
      stream?: unknown;
      streamContent?: string;
      artifact_format_override?: AudioArtifactFormatOverride | null;
    };
    selected?: boolean;
  }

  interface StreamAudioChunk {
    audioBase64: string | null;
    artifactId: string | null;
    streamHandle: string | null;
    mimeType: string;
    sequence: number | null;
    byteLength: number | null;
    availableByteLength: number | null;
    byteRangeStart: number | null;
    byteRangeEndExclusive: number | null;
    lifecycleState: string | null;
    isFinal: boolean;
    mode: 'append' | 'replace';
  }

  interface AudioFormatConfig {
    defaults: WorkflowAudioArtifactFormatSettings;
    formats: WorkflowMediaFormatOption[];
  }

  const DEFAULT_SELECTION_VALUE = '__pantograph_default__';
  let audioFormatConfigPromise: Promise<AudioFormatConfig> | null = null;

  function loadAudioFormatConfig(): Promise<AudioFormatConfig> {
    audioFormatConfigPromise ??= Promise.all([
      workflowService.artifactFormatSettings(),
      workflowService.artifactFormatCapabilities(),
    ]).then(([settingsResponse, capabilities]) => ({
      defaults: settingsResponse.settings.audio,
      formats: capabilities.audio_formats,
    }));
    return audioFormatConfigPromise;
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
  let defaultFormat = $state<WorkflowAudioArtifactFormatSettings | null>(null);
  let formatOptions = $state<WorkflowMediaFormatOption[]>([]);
  let formatLoadError = $state<string | null>(null);

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');

  let finalAudioData = $derived(data.audio || '');
  let finalAudioMime = $derived(data.audio_mime || 'audio/wav');
  let finalAudioDurationSeconds = $derived(
    typeof data.audio_duration_seconds === 'number' &&
      Number.isFinite(data.audio_duration_seconds) &&
      data.audio_duration_seconds > 0
      ? data.audio_duration_seconds
      : 0
  );
  let finalAudioSrc = $derived(finalAudioData ? `data:${finalAudioMime};base64,${finalAudioData}` : '');

  let streamPayload = $derived.by((): StreamAudioChunk | null => {
    const payload = data.stream;
    if (typeof payload === 'string' && payload.length > 0) {
      return {
        audioBase64: payload,
        artifactId: null,
        streamHandle: null,
        mimeType: 'audio/wav',
        sequence: null,
        byteLength: null,
        availableByteLength: null,
        byteRangeStart: null,
        byteRangeEndExclusive: null,
        lifecycleState: null,
        isFinal: false,
        mode: 'append',
      };
    }
    if (!payload || typeof payload !== 'object') return null;

    const maybeChunk = payload as {
      audio_base64?: unknown;
      content?: unknown;
      artifact_id?: unknown;
      stream_handle?: unknown;
      media_type?: unknown;
      mime_type?: unknown;
      sequence?: unknown;
      byte_length?: unknown;
      available_byte_length?: unknown;
      byte_range_start?: unknown;
      byte_range_end_exclusive?: unknown;
      lifecycle_state?: unknown;
      is_final?: unknown;
      mode?: unknown;
      descriptor?: unknown;
    };
    const descriptor =
      maybeChunk.descriptor && typeof maybeChunk.descriptor === 'object'
        ? maybeChunk.descriptor as Record<string, unknown>
        : null;
    const descriptorFormat =
      descriptor?.format && typeof descriptor.format === 'object'
        ? descriptor.format as Record<string, unknown>
        : null;

    const audioValue =
      typeof maybeChunk.audio_base64 === 'string' && maybeChunk.audio_base64.length > 0
        ? maybeChunk.audio_base64
        : typeof maybeChunk.content === 'string' && maybeChunk.content.length > 0
          ? maybeChunk.content
          : null;
    const artifactId = nonEmptyStringOrNull(maybeChunk.artifact_id) ?? nonEmptyStringOrNull(descriptor?.artifact_id);
    const streamHandle =
      nonEmptyStringOrNull(maybeChunk.stream_handle) ?? nonEmptyStringOrNull(descriptor?.stream_handle);
    if (!audioValue && !artifactId) return null;

    const sequence =
      typeof maybeChunk.sequence === 'number' && Number.isFinite(maybeChunk.sequence)
        ? maybeChunk.sequence
        : null;
    const mediaType =
      nonEmptyStringOrNull(maybeChunk.media_type) ?? nonEmptyStringOrNull(descriptorFormat?.media_type);
    const mimeType = nonEmptyStringOrNull(maybeChunk.mime_type) ?? mediaType ?? 'audio/wav';

    return {
      audioBase64: audioValue,
      artifactId,
      streamHandle,
      mimeType,
      sequence,
      byteLength: finiteNumberOrNull(maybeChunk.byte_length) ?? finiteNumberOrNull(descriptor?.byte_length),
      availableByteLength: finiteNumberOrNull(maybeChunk.available_byte_length),
      byteRangeStart: finiteNumberOrNull(maybeChunk.byte_range_start),
      byteRangeEndExclusive: finiteNumberOrNull(maybeChunk.byte_range_end_exclusive),
      lifecycleState:
        nonEmptyStringOrNull(maybeChunk.lifecycle_state) ?? nonEmptyStringOrNull(descriptor?.lifecycle_state),
      isFinal: maybeChunk.is_final === true,
      mode: maybeChunk.mode === 'replace' ? 'replace' : 'append',
    };
  });

  let finalDisplayedDuration = $derived(Math.max(duration, finalAudioDurationSeconds));
  let displayedDuration = $derived(finalAudioSrc ? finalDisplayedDuration : streamBufferedDuration);
  let canSeek = $derived(Boolean(finalAudioSrc));
  let hasAnyAudio = $derived(Boolean(finalAudioSrc) || hasStreamAudio);
  let formatOverride = $derived(normalizeFormatOverride(data.artifact_format_override));
  let selectedContainerId = $derived(formatOverride?.container_id ?? defaultFormat?.container_id ?? '');
  let selectedFormat = $derived(findFormatOption(formatOptions, selectedContainerId));
  let selectableFormats = $derived(formatOptionItems(formatOptions, selectedContainerId));
  let codecOptions = $derived(
    optionValuesWithCurrent(selectedFormat?.codec_ids ?? [], effectiveCodecId())
  );
  let bitrateRangeLabel = $derived(
    formatRangeLabel(selectedFormat?.bitrate_min_kbps, selectedFormat?.bitrate_max_kbps, ' kbps')
  );
  let supportsBitrate = $derived(
    selectedFormat?.bitrate_min_kbps !== null &&
      selectedFormat?.bitrate_min_kbps !== undefined &&
      selectedFormat?.bitrate_max_kbps !== null &&
      selectedFormat?.bitrate_max_kbps !== undefined
  );
  let formatSelectId = $derived(`audio-output-${id}-format`);
  let codecSelectId = $derived(`audio-output-${id}-codec`);
  let bitrateInputId = $derived(`audio-output-${id}-bitrate`);
  let isUsingDefaultFormat = $derived(!formatOverride);

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

  function normalizeFormatOverride(value: unknown): AudioArtifactFormatOverride | null {
    if (!value || typeof value !== 'object') return null;
    const record = value as Record<string, unknown>;
    const containerId = typeof record.container_id === 'string' ? record.container_id : '';
    const codecId = typeof record.codec_id === 'string' ? record.codec_id : '';
    const bitrateKbps =
      typeof record.bitrate_kbps === 'number' && Number.isFinite(record.bitrate_kbps)
        ? Math.round(record.bitrate_kbps)
        : null;

    if (!containerId || !codecId || bitrateKbps === null) {
      return null;
    }

    return {
      container_id: containerId,
      codec_id: codecId,
      bitrate_kbps: bitrateKbps,
    };
  }

  function findFormatOption(
    options: WorkflowMediaFormatOption[],
    formatId: string | null | undefined,
  ): WorkflowMediaFormatOption | null {
    return options.find((option) => option.format_id === formatId) ?? null;
  }

  function formatOptionItems(
    options: WorkflowMediaFormatOption[],
    currentValue: string,
  ): WorkflowMediaFormatOption[] {
    if (!currentValue || options.some((option) => option.format_id === currentValue)) {
      return options;
    }
    return [
      {
        format_id: currentValue,
        display_name: `${currentValue} (unsupported)`,
        media_type: 'unknown',
        codec_ids: [],
        quality_min_percent: null,
        quality_max_percent: null,
        bitrate_min_kbps: null,
        bitrate_max_kbps: null,
        crf_min: null,
        crf_max: null,
        bit_depths: [],
        color_profile_ids: [],
        provided_by_dependency_id: 'unknown',
        provided_by_version: null,
      },
      ...options,
    ];
  }

  function optionValuesWithCurrent(values: string[], currentValue: string): string[] {
    if (currentValue && !values.includes(currentValue)) {
      return [currentValue, ...values];
    }
    return values;
  }

  function formatRangeLabel(
    min: number | null | undefined,
    max: number | null | undefined,
    suffix = '',
  ): string {
    if (min === null || min === undefined || max === null || max === undefined) {
      return 'Validated';
    }
    return `${min}${suffix} to ${max}${suffix}`;
  }

  function effectiveCodecId(): string {
    return formatOverride?.codec_id ?? defaultFormat?.codec_id ?? '';
  }

  function effectiveBitrateKbps(): number {
    return formatOverride?.bitrate_kbps ?? defaultFormat?.bitrate_kbps ?? 96;
  }

  function finiteNumberOrNull(value: unknown): number | null {
    return typeof value === 'number' && Number.isFinite(value) ? value : null;
  }

  function nonEmptyStringOrNull(value: unknown): string | null {
    return typeof value === 'string' && value.length > 0 ? value : null;
  }

  function clampToRange(value: number, min: number | null | undefined, max: number | null | undefined): number {
    let nextValue = Number.isFinite(value) ? Math.round(value) : defaultFormat?.bitrate_kbps ?? 96;
    if (min !== null && min !== undefined) {
      nextValue = Math.max(min, nextValue);
    }
    if (max !== null && max !== undefined) {
      nextValue = Math.min(max, nextValue);
    }
    return nextValue;
  }

  function buildOverrideForFormat(containerId: string): AudioArtifactFormatOverride {
    const option = findFormatOption(formatOptions, containerId);
    const codecs = optionValuesWithCurrent(option?.codec_ids ?? [], effectiveCodecId());
    return {
      container_id: containerId,
      codec_id: codecs[0] ?? defaultFormat?.codec_id ?? 'opus',
      bitrate_kbps: clampToRange(
        effectiveBitrateKbps(),
        option?.bitrate_min_kbps,
        option?.bitrate_max_kbps
      ),
    };
  }

  function updateFormatOverride(override: AudioArtifactFormatOverride | null) {
    void updateNodeData(id, { artifact_format_override: override });
  }

  function handleFormatChange(event: Event) {
    const target = event.currentTarget as HTMLSelectElement | null;
    const containerId = target?.value ?? DEFAULT_SELECTION_VALUE;
    if (containerId === DEFAULT_SELECTION_VALUE) {
      updateFormatOverride(null);
      return;
    }
    updateFormatOverride(buildOverrideForFormat(containerId));
  }

  function handleCodecChange(event: Event) {
    const target = event.currentTarget as HTMLSelectElement | null;
    const codecId = target?.value ?? effectiveCodecId();
    updateFormatOverride({
      ...buildOverrideForFormat(selectedContainerId),
      codec_id: codecId,
    });
  }

  function handleBitrateChange(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    const rawValue = Number(target?.value ?? effectiveBitrateKbps());
    updateFormatOverride({
      ...buildOverrideForFormat(selectedContainerId),
      bitrate_kbps: clampToRange(
        rawValue,
        selectedFormat?.bitrate_min_kbps,
        selectedFormat?.bitrate_max_kbps
      ),
    });
  }

  function base64ToArrayBuffer(base64: string): ArrayBuffer {
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes.buffer;
  }

  async function streamChunkToArrayBuffer(chunk: StreamAudioChunk): Promise<ArrayBuffer | null> {
    if (chunk.audioBase64) {
      return base64ToArrayBuffer(chunk.audioBase64);
    }
    if (!chunk.artifactId) {
      return null;
    }

    const request = {
      artifact_id: chunk.artifactId,
      byte_range_start: chunk.byteRangeStart,
      byte_range_end_exclusive: chunk.byteRangeEndExclusive,
    };
    const read =
      chunk.lifecycleState === 'retained' || chunk.isFinal
        ? await workflowService.readArtifactBody(request)
        : await workflowService.readArtifactStream(request);
    return new Uint8Array(read.body).buffer;
  }

  function streamChunkSignature(chunk: StreamAudioChunk): string {
    if (chunk.audioBase64) {
      return `inline:${chunk.audioBase64.length}:${chunk.audioBase64.slice(0, 64)}`;
    }
    return [
      'artifact',
      chunk.artifactId ?? '',
      chunk.streamHandle ?? '',
      chunk.byteRangeStart ?? '',
      chunk.byteRangeEndExclusive ?? '',
      chunk.availableByteLength ?? '',
      chunk.byteLength ?? '',
      chunk.lifecycleState ?? '',
      chunk.isFinal ? 'final' : 'open',
    ].join(':');
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
      const signature = streamChunkSignature(chunk);
      if (signature === lastProcessedChunkSignature) return;
      lastProcessedChunkSignature = signature;
    }

    const context = await ensureStreamContext();
    if (!context || !streamGainNode) return;

    try {
      const encoded = await streamChunkToArrayBuffer(chunk);
      if (!encoded || encoded.byteLength === 0) return;
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
    const nextDuration = Number.isFinite(audioElement.duration) && audioElement.duration > 0
      ? audioElement.duration
      : finalAudioDurationSeconds;
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
    currentTime = finalDisplayedDuration;
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

  onMount(() => {
    let disposed = false;
    loadAudioFormatConfig()
      .then((config) => {
        if (disposed) return;
        defaultFormat = config.defaults;
        formatOptions = config.formats;
        formatLoadError = null;
      })
      .catch((error: unknown) => {
        if (disposed) return;
        formatLoadError = error instanceof Error ? error.message : String(error);
      });

    return () => {
      disposed = true;
    };
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

    if (finalAudioDurationSeconds > 0 && duration <= 0) {
      duration = finalAudioDurationSeconds;
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

    <div class="mt-2 space-y-2 border-t border-neutral-700/70 pt-2">
      <div class="flex items-center justify-between gap-2">
        <label class="text-[10px] text-neutral-400" for={formatSelectId}>Format</label>
        {#if isUsingDefaultFormat && defaultFormat}
          <span class="text-[10px] text-neutral-500">
            Default {defaultFormat.container_id}/{defaultFormat.codec_id}
          </span>
        {:else if formatOverride}
          <span class="text-[10px] text-pink-300">Override</span>
        {/if}
      </div>
      <select
        id={formatSelectId}
        class="nodrag nopan nowheel w-full rounded border border-neutral-600 bg-neutral-900 px-2 py-1 text-xs text-neutral-200 focus:border-pink-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
        value={formatOverride?.container_id ?? DEFAULT_SELECTION_VALUE}
        disabled={!defaultFormat && !formatOverride}
        onchange={handleFormatChange}
        onmousedown={stopControlEvent}
        onmouseup={stopControlEvent}
        onpointerdown={stopControlEvent}
        onpointerup={stopControlEvent}
        onclickcapture={stopControlEvent}
      >
        <option value={DEFAULT_SELECTION_VALUE}>
          Use default{defaultFormat ? ` (${defaultFormat.container_id}/${defaultFormat.codec_id})` : ''}
        </option>
        {#each selectableFormats as option}
          <option value={option.format_id}>
            {option.display_name}
          </option>
        {/each}
      </select>

      {#if formatOverride}
        <div class="grid grid-cols-2 gap-2">
          <div class="flex flex-col gap-1">
            <label class="text-[10px] text-neutral-400" for={codecSelectId}>Codec</label>
            <select
              id={codecSelectId}
              class="nodrag nopan nowheel w-full rounded border border-neutral-600 bg-neutral-900 px-2 py-1 text-xs text-neutral-200 focus:border-pink-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
              value={formatOverride.codec_id}
              disabled={codecOptions.length === 0}
              onchange={handleCodecChange}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onpointerdown={stopControlEvent}
              onpointerup={stopControlEvent}
              onclickcapture={stopControlEvent}
            >
              {#each codecOptions as codecId}
                <option value={codecId}>{codecId}</option>
              {/each}
            </select>
          </div>
          <div class="flex flex-col gap-1">
            <label class="text-[10px] text-neutral-400" for={bitrateInputId}>Bitrate</label>
            <input
              id={bitrateInputId}
              class="nodrag nopan nowheel w-full rounded border border-neutral-600 bg-neutral-900 px-2 py-1 text-xs text-neutral-200 focus:border-pink-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
              type="number"
              min={selectedFormat?.bitrate_min_kbps ?? undefined}
              max={selectedFormat?.bitrate_max_kbps ?? undefined}
              step="1"
              value={formatOverride.bitrate_kbps}
              disabled={!supportsBitrate}
              aria-describedby={`${bitrateInputId}-range`}
              onchange={handleBitrateChange}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onpointerdown={stopControlEvent}
              onpointerup={stopControlEvent}
              onclickcapture={stopControlEvent}
            />
            <span id={`${bitrateInputId}-range`} class="text-[10px] text-neutral-500">
              {bitrateRangeLabel}
            </span>
          </div>
        </div>
      {/if}

      {#if formatLoadError}
        <div class="text-[10px] text-red-300">{formatLoadError}</div>
      {/if}
    </div>
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

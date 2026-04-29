<script lang="ts">
  import { onMount } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import type {
    NodeDefinition,
    WorkflowImageArtifactFormatSettings,
    WorkflowMediaFormatOption,
  } from '../../../services/workflow/types';
  import { workflowService } from '../../../services/workflow/WorkflowService';
  import { nodeExecutionStates, updateNodeData } from '../../../stores/workflowStore';

  interface ImageArtifactFormatOverride {
    format_id: string;
    quality_percent: number;
    color_profile_id: string;
  }

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      image?: string;
      streamContent?: string;
      artifact_format_override?: ImageArtifactFormatOverride | null;
    };
    selected?: boolean;
  }

  interface ImageFormatConfig {
    defaults: WorkflowImageArtifactFormatSettings;
    formats: WorkflowMediaFormatOption[];
  }

  const DEFAULT_SELECTION_VALUE = '__pantograph_default__';
  let imageFormatConfigPromise: Promise<ImageFormatConfig> | null = null;

  function loadImageFormatConfig(): Promise<ImageFormatConfig> {
    imageFormatConfigPromise ??= Promise.all([
      workflowService.artifactFormatSettings(),
      workflowService.artifactFormatCapabilities(),
    ]).then(([settingsResponse, capabilities]) => ({
      defaults: settingsResponse.settings.image,
      formats: capabilities.image_formats,
    }));
    return imageFormatConfigPromise;
  }

  let { id, data, selected = false }: Props = $props();

  let defaultFormat = $state<WorkflowImageArtifactFormatSettings | null>(null);
  let formatOptions = $state<WorkflowMediaFormatOption[]>([]);
  let formatLoadError = $state<string | null>(null);

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let imageData = $derived(data.image || '');
  let imageSrc = $derived(imageData ? `data:image/png;base64,${imageData}` : '');
  let formatOverride = $derived(normalizeFormatOverride(data.artifact_format_override));
  let selectedFormatId = $derived(formatOverride?.format_id ?? defaultFormat?.format_id ?? '');
  let selectedFormat = $derived(findFormatOption(formatOptions, selectedFormatId));
  let selectableFormats = $derived(formatOptionItems(formatOptions, selectedFormatId));
  let colorProfileOptions = $derived(
    optionValuesWithCurrent(selectedFormat?.color_profile_ids ?? [], effectiveColorProfile())
  );
  let qualityRangeLabel = $derived(
    formatRangeLabel(selectedFormat?.quality_min_percent, selectedFormat?.quality_max_percent, '%')
  );
  let supportsQuality = $derived(
    selectedFormat?.quality_min_percent !== null &&
      selectedFormat?.quality_min_percent !== undefined &&
      selectedFormat?.quality_max_percent !== null &&
      selectedFormat?.quality_max_percent !== undefined
  );
  let formatSelectId = $derived(`image-output-${id}-format`);
  let qualityInputId = $derived(`image-output-${id}-quality`);
  let colorProfileSelectId = $derived(`image-output-${id}-color-profile`);
  let isUsingDefaultFormat = $derived(!formatOverride);

  let showModal = $state(false);
  let modalElement = $state<HTMLDialogElement | null>(null);

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-violet-500 animate-pulse',
      success: 'bg-violet-500',
      error: 'bg-red-500',
    }[executionState]
  );

  function stopControlEvent(event: Event) {
    event.stopPropagation();
  }

  function normalizeFormatOverride(value: unknown): ImageArtifactFormatOverride | null {
    if (!value || typeof value !== 'object') return null;
    const record = value as Record<string, unknown>;
    const formatId = typeof record.format_id === 'string' ? record.format_id : '';
    const qualityPercent =
      typeof record.quality_percent === 'number' && Number.isFinite(record.quality_percent)
        ? Math.round(record.quality_percent)
        : null;
    const colorProfileId =
      typeof record.color_profile_id === 'string' ? record.color_profile_id : '';

    if (!formatId || qualityPercent === null || !colorProfileId) {
      return null;
    }

    return {
      format_id: formatId,
      quality_percent: qualityPercent,
      color_profile_id: colorProfileId,
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

  function effectiveQualityPercent(): number {
    return formatOverride?.quality_percent ?? defaultFormat?.quality_percent ?? 75;
  }

  function effectiveColorProfile(): string {
    return formatOverride?.color_profile_id ?? defaultFormat?.color_profile_id ?? '';
  }

  function clampToRange(value: number, min: number | null | undefined, max: number | null | undefined): number {
    let nextValue = Number.isFinite(value) ? Math.round(value) : defaultFormat?.quality_percent ?? 75;
    if (min !== null && min !== undefined) {
      nextValue = Math.max(min, nextValue);
    }
    if (max !== null && max !== undefined) {
      nextValue = Math.min(max, nextValue);
    }
    return nextValue;
  }

  function buildOverrideForFormat(formatId: string): ImageArtifactFormatOverride {
    const option = findFormatOption(formatOptions, formatId);
    const colorProfileValues = optionValuesWithCurrent(
      option?.color_profile_ids ?? [],
      effectiveColorProfile()
    );
    return {
      format_id: formatId,
      quality_percent: clampToRange(
        effectiveQualityPercent(),
        option?.quality_min_percent,
        option?.quality_max_percent
      ),
      color_profile_id: colorProfileValues[0] ?? defaultFormat?.color_profile_id ?? 'srgb',
    };
  }

  function updateFormatOverride(override: ImageArtifactFormatOverride | null) {
    void updateNodeData(id, { artifact_format_override: override });
  }

  function handleFormatChange(event: Event) {
    const target = event.currentTarget as HTMLSelectElement | null;
    const formatId = target?.value ?? DEFAULT_SELECTION_VALUE;
    if (formatId === DEFAULT_SELECTION_VALUE) {
      updateFormatOverride(null);
      return;
    }
    updateFormatOverride(buildOverrideForFormat(formatId));
  }

  function handleQualityChange(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    const rawValue = Number(target?.value ?? effectiveQualityPercent());
    updateFormatOverride({
      ...buildOverrideForFormat(selectedFormatId),
      quality_percent: clampToRange(
        rawValue,
        selectedFormat?.quality_min_percent,
        selectedFormat?.quality_max_percent
      ),
    });
  }

  function handleColorProfileChange(event: Event) {
    const target = event.currentTarget as HTMLSelectElement | null;
    const colorProfileId = target?.value ?? effectiveColorProfile();
    updateFormatOverride({
      ...buildOverrideForFormat(selectedFormatId),
      color_profile_id: colorProfileId,
    });
  }

  function openModal(event?: Event) {
    event?.stopPropagation();
    showModal = true;
  }

  function closeModal() {
    showModal = false;
  }

  function downloadImage(event?: Event) {
    event?.stopPropagation();
    if (!imageData) return;
    const byteChars = atob(imageData);
    const bytes = new Uint8Array(byteChars.length);
    for (let i = 0; i < byteChars.length; i++) {
      bytes[i] = byteChars.charCodeAt(i);
    }
    const blob = new Blob([bytes], { type: 'image/png' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'output.png';
    a.click();
    requestAnimationFrame(() => URL.revokeObjectURL(url));
  }

  onMount(() => {
    let disposed = false;
    loadImageFormatConfig()
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
    if (!modalElement) {
      return;
    }

    if (showModal) {
      if (!modalElement.open) {
        modalElement.showModal();
      }
      return;
    }

    if (modalElement.open) {
      modalElement.close();
    }
  });
</script>

<div class="image-output-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-violet-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Image Output'}</span>
        <span class="w-2 h-2 rounded-full {statusColor} ml-auto"></span>
      </div>
    {/snippet}

      {#if imageSrc}
        <div class="space-y-1">
          <button type="button"
            class="nodrag nopan nowheel w-full cursor-pointer border-0 bg-transparent p-0"
            onclick={openModal}
            aria-label="Open output image preview"
            onmousedown={stopControlEvent}
            onmouseup={stopControlEvent}
            onpointerdown={stopControlEvent}
            onpointerup={stopControlEvent}
            onclickcapture={stopControlEvent}
          >
            <img src={imageSrc} alt="Output" class="max-h-40 w-full object-contain rounded" />
          </button>
          <div class="flex justify-end gap-1">
            <button type="button"
              class="nodrag nopan nowheel text-[10px] text-neutral-400 hover:text-neutral-200 bg-transparent border-0 cursor-pointer px-1"
              onclick={downloadImage}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onpointerdown={stopControlEvent}
              onpointerup={stopControlEvent}
              onclickcapture={stopControlEvent}
            >
              Download
            </button>
            <button type="button"
              class="nodrag nopan nowheel text-[10px] text-neutral-400 hover:text-neutral-200 bg-transparent border-0 cursor-pointer px-1"
              onclick={openModal}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onpointerdown={stopControlEvent}
              onpointerup={stopControlEvent}
              onclickcapture={stopControlEvent}
            >
              Expand
            </button>
          </div>
        </div>
      {:else}
        <div class="text-xs text-neutral-500 italic">
          No image yet
        </div>
      {/if}

      <div class="mt-2 space-y-2 border-t border-neutral-700/70 pt-2">
        <div class="flex items-center justify-between gap-2">
          <label class="text-[10px] text-neutral-400" for={formatSelectId}>Format</label>
          {#if isUsingDefaultFormat && defaultFormat}
            <span class="text-[10px] text-neutral-500">Default {defaultFormat.format_id}</span>
          {:else if formatOverride}
            <span class="text-[10px] text-violet-300">Override</span>
          {/if}
        </div>
        <select
          id={formatSelectId}
          class="nodrag nopan nowheel w-full rounded border border-neutral-600 bg-neutral-900 px-2 py-1 text-xs text-neutral-200 focus:border-violet-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
          value={formatOverride?.format_id ?? DEFAULT_SELECTION_VALUE}
          disabled={!defaultFormat && !formatOverride}
          onchange={handleFormatChange}
          onmousedown={stopControlEvent}
          onmouseup={stopControlEvent}
          onpointerdown={stopControlEvent}
          onpointerup={stopControlEvent}
          onclickcapture={stopControlEvent}
        >
          <option value={DEFAULT_SELECTION_VALUE}>
            Use default{defaultFormat ? ` (${defaultFormat.format_id})` : ''}
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
              <label class="text-[10px] text-neutral-400" for={qualityInputId}>Quality</label>
              <input
                id={qualityInputId}
                class="nodrag nopan nowheel w-full rounded border border-neutral-600 bg-neutral-900 px-2 py-1 text-xs text-neutral-200 focus:border-violet-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
                type="number"
                min={selectedFormat?.quality_min_percent ?? undefined}
                max={selectedFormat?.quality_max_percent ?? undefined}
                step="1"
                value={formatOverride.quality_percent}
                disabled={!supportsQuality}
                aria-describedby={`${qualityInputId}-range`}
                onchange={handleQualityChange}
                onmousedown={stopControlEvent}
                onmouseup={stopControlEvent}
                onpointerdown={stopControlEvent}
                onpointerup={stopControlEvent}
                onclickcapture={stopControlEvent}
              />
              <span id={`${qualityInputId}-range`} class="text-[10px] text-neutral-500">
                {qualityRangeLabel}
              </span>
            </div>
            <div class="flex flex-col gap-1">
              <label class="text-[10px] text-neutral-400" for={colorProfileSelectId}>
                Color
              </label>
              <select
                id={colorProfileSelectId}
                class="nodrag nopan nowheel w-full rounded border border-neutral-600 bg-neutral-900 px-2 py-1 text-xs text-neutral-200 focus:border-violet-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
                value={formatOverride.color_profile_id}
                disabled={colorProfileOptions.length === 0}
                onchange={handleColorProfileChange}
                onmousedown={stopControlEvent}
                onmouseup={stopControlEvent}
                onpointerdown={stopControlEvent}
                onpointerup={stopControlEvent}
                onclickcapture={stopControlEvent}
              >
                {#each colorProfileOptions as profileId}
                  <option value={profileId}>{profileId}</option>
                {/each}
              </select>
            </div>
          </div>
        {/if}

        {#if formatLoadError}
          <div class="text-[10px] text-red-300">{formatLoadError}</div>
        {/if}
      </div>
  </BaseNode>
</div>

<dialog
  bind:this={modalElement}
  class="image-preview-dialog"
  onclick={(event) => event.target === modalElement && closeModal()}
  onclose={closeModal}
>
  {#if imageSrc}
    <div class="dialog-content">
      <button type="button" class="nodrag nopan nowheel dialog-close" onclick={closeModal}>
        Close
      </button>
      <img src={imageSrc} alt="Full resolution output" class="dialog-image" />
    </div>
  {/if}
</dialog>

<style>
  .image-output-wrapper :global(.base-node) {
    border-color: rgba(139, 92, 246, 0.5);
  }

  .image-output-wrapper :global(.node-header) {
    background-color: rgba(139, 92, 246, 0.2);
    border-color: rgba(139, 92, 246, 0.3);
  }

  .image-preview-dialog {
    width: min(96vw, 1600px);
    max-width: 96vw;
    height: min(96vh, 1100px);
    max-height: 96vh;
    border: 0;
    border-radius: 16px;
    padding: 0;
    background: rgba(10, 10, 10, 0.96);
    color: white;
  }

  .image-preview-dialog::backdrop {
    background: rgba(0, 0, 0, 0.82);
  }

  .dialog-content {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    padding: 1rem;
  }

  .dialog-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 12px;
  }

  .dialog-close {
    position: absolute;
    top: 1rem;
    right: 1rem;
    border: 0;
    border-radius: 999px;
    padding: 0.4rem 0.75rem;
    background: rgba(38, 38, 38, 0.9);
    color: rgb(229, 229, 229);
    cursor: pointer;
    font-size: 0.75rem;
  }

  .dialog-close:hover {
    background: rgba(82, 82, 82, 0.95);
  }
</style>

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { ConfigService, type ConfigState, type DeviceConfig, type DeviceInfo, type EmbeddingMemoryMode } from '../services/ConfigService';
  import { LLMService } from '../services/LLMService';
  import { RagService } from '../services/RagService';
  import { expandedSection, toggleSection } from '../stores/accordionStore';
  import { StatusGlyph } from './workbench/ui';
  import { createScopedDeviceRefresh, type DeviceRefreshScope } from './deviceConfigRefreshScope';
  import {
    buildBackendConfirmedDeviceOptions,
    formatDeviceDisplayName,
    isBackendConfirmedDeviceSelection,
    resolveSelectedDeviceName,
  } from './deviceConfigPresenters';

  let state: ConfigState = $state(ConfigService.getState());
  let unsubscribe: (() => void) | null = null;
  let unsubscribeLLM: (() => void) | null = null;
  let unsubscribeRag: (() => void) | null = null;
  let isSaving = $state(false);
  let isLoadingDevices = $state(false);
  let isRefreshingDevices = $state(false);
  let deviceLoadError: string | null = $state(null);
  let oomFlash = $state(false);
  let oomFlashTimer: ReturnType<typeof setTimeout> | null = null;
  let lastOomAt = 0;
  let deviceRefreshScope: DeviceRefreshScope | null = null;

  // Available devices confirmed by the backend.
  let availableDevices: DeviceInfo[] = $state([]);

  // Local form state
  let selectedDevice: string = $state('auto');
  let gpuLayers: number = $state(-1);
  let embeddingMemoryMode: EmbeddingMemoryMode = $state('cpu_parallel');
  let initialEmbeddingMode: EmbeddingMemoryMode = $state('cpu_parallel');

  onMount(async () => {
    unsubscribe = ConfigService.subscribe((nextState) => {
      state = nextState;
      // Sync local form state
      selectedDevice = nextState.config.device.device;
      gpuLayers = nextState.config.device.gpu_layers;
      triggerOomFlash(nextState.error);
    });

    unsubscribeLLM = LLMService.subscribe((nextState) => {
      triggerOomFlash(nextState.error);
    });

    unsubscribeRag = RagService.subscribe((nextState) => {
      triggerOomFlash(nextState.error);
    });

    // Load available devices
    await loadDevices();
    deviceRefreshScope = createScopedDeviceRefresh(
      () => {
        void loadDevices({ silent: true });
      },
      window,
    );
    deviceRefreshScope.update($expandedSection === 'device');

    // Load embedding memory mode
    const mode = await ConfigService.getEmbeddingMemoryMode();
    embeddingMemoryMode = mode;
    initialEmbeddingMode = mode;
  });

  onDestroy(() => {
    unsubscribe?.();
    unsubscribeLLM?.();
    unsubscribeRag?.();
    deviceRefreshScope?.stop();
    deviceRefreshScope = null;
    if (oomFlashTimer) {
      clearTimeout(oomFlashTimer);
      oomFlashTimer = null;
    }
  });

  const isOomError = (value: string | null | undefined): boolean => {
    if (!value) return false;
    const lower = value.toLowerCase();
    return (
      value.includes('OOM') ||
      lower.includes('out of memory') ||
      lower.includes('outofdevicememory') ||
      lower.includes('erroroutofdevicememory') ||
      lower.includes('device memory allocation') ||
      (lower.includes('failed to allocate') &&
        (lower.includes('vulkan') || lower.includes('cuda')))
    );
  };

  const triggerOomFlash = (error: string | null | undefined) => {
    if (!isOomError(error)) return;
    const now = Date.now();
    if (now - lastOomAt < 1000) return;
    lastOomAt = now;
    oomFlash = true;
    if (oomFlashTimer) {
      clearTimeout(oomFlashTimer);
    }
    oomFlashTimer = setTimeout(() => {
      oomFlash = false;
    }, 2200);
  };

  const loadDevices = async (options: { silent?: boolean } = {}) => {
    const silent = options.silent ?? false;
    if (silent) {
      if (isRefreshingDevices) return;
      isRefreshingDevices = true;
    } else {
      if (isLoadingDevices) return;
      isLoadingDevices = true;
      deviceLoadError = null;
    }
    try {
      availableDevices = await ConfigService.listDevices();
      if (deviceLoadError && silent) {
        deviceLoadError = null;
      }
    } catch (error) {
      if (!silent) {
        deviceLoadError = String(error);
        console.error('Failed to load devices:', error);
        availableDevices = [];
      }
    } finally {
      if (silent) {
        isRefreshingDevices = false;
      } else {
        isLoadingDevices = false;
      }
    }
  };

  const saveConfig = async () => {
    isSaving = true;
    try {
      if (deviceConfigChanged) {
        if (!isBackendConfirmedDeviceSelection(selectedDevice, deviceOptions)) {
          deviceLoadError = 'Selected device is not backend-confirmed. Refresh devices before saving.';
          return;
        }
        const device: DeviceConfig = {
          device: selectedDevice,
          gpu_layers: gpuLayers,
        };
        await ConfigService.setDeviceConfig(device);
      }

      // Save embedding memory mode if changed
      if (embeddingMemoryMode !== initialEmbeddingMode) {
        await ConfigService.setEmbeddingMemoryMode(embeddingMemoryMode);
        initialEmbeddingMode = embeddingMemoryMode;
      }

      // Auto-restart server if running in sidecar mode
      if (state.serverMode.mode.startsWith('sidecar')) {
        await ConfigService.startInferenceMode();
      }
    } catch (error) {
      console.error('Failed to save device config:', error);
    } finally {
      isSaving = false;
    }
  };

  const getGpuLayersLabel = (layers: number): string => {
    if (layers === -1) return 'All layers (GPU)';
    if (layers === 0) return 'CPU only';
    return `${layers} layers`;
  };

  const formatVramValue = (mb: number): string => {
    if (mb <= 0) return '0 MB';
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(1)} GB`;
    }
    return `${mb} MB`;
  };

  const getSelectedDeviceName = (): string => {
    return resolveSelectedDeviceName(selectedDevice, availableDevices);
  };

  let deviceConfigChanged = $derived(
    selectedDevice !== state.config.device.device ||
    gpuLayers !== state.config.device.gpu_layers
  );
  let hasChanges = $derived(deviceConfigChanged || embeddingMemoryMode !== initialEmbeddingMode);

  let selectedDeviceInfo = $derived(availableDevices.find(d => d.id === selectedDevice) || null);
  let vramUsage = $derived(selectedDeviceInfo && selectedDeviceInfo.total_vram_mb > 0
    ? {
        used: Math.max(selectedDeviceInfo.total_vram_mb - selectedDeviceInfo.free_vram_mb, 0),
        free: Math.max(selectedDeviceInfo.free_vram_mb, 0),
        total: selectedDeviceInfo.total_vram_mb,
        percent: Math.min(
          100,
          Math.round(
            (Math.max(selectedDeviceInfo.total_vram_mb - selectedDeviceInfo.free_vram_mb, 0) /
              Math.max(selectedDeviceInfo.total_vram_mb, 1)) *
              100
          )
        ),
      }
    : null
  );

  let deviceOptions = $derived(buildBackendConfirmedDeviceOptions(availableDevices));

  // Minimum VRAM needed for embedding model (~800MB with buffer)
  const EMBEDDING_MODEL_VRAM_MB = 800;
  let canFitBothModels = $derived(vramUsage ? vramUsage.free >= EMBEDDING_MODEL_VRAM_MB : false);

  const getEmbeddingModeLabel = (mode: EmbeddingMemoryMode): string => {
    switch (mode) {
      case 'cpu_parallel': return 'CPU + GPU';
      case 'gpu_parallel': return 'Both GPU';
      case 'sequential': return 'Sequential';
    }
  };

  const getEmbeddingModeDescription = (mode: EmbeddingMemoryMode): string => {
    switch (mode) {
      case 'cpu_parallel': return 'Embedding on RAM, LLM on VRAM. Fast searches, uses RAM.';
      case 'gpu_parallel': return 'Both models on GPU. Fastest but needs ~800MB extra VRAM.';
      case 'sequential': return 'One model at a time. Slowest but lowest memory usage.';
    }
  };

  $effect(() => {
    deviceRefreshScope?.update($expandedSection === 'device');
  });
</script>

<section class="pg-card" aria-labelledby="settings-device-title">
  <div class="flex items-center gap-2">
    <h3 class="m-0 min-w-0 flex-1">
      <button
        type="button"
        id="settings-device-title"
        class="flex w-full items-center gap-2 text-left text-[13px] font-bold text-pg-text-bright"
        aria-expanded={$expandedSection === 'device'}
        onclick={() => toggleSection('device')}
      >
        Device configuration
        <span class="pg-muted text-[11px] font-normal" aria-hidden="true">
          {$expandedSection === 'device' ? '▴' : '▾'}
        </span>
      </button>
    </h3>
    <span class="flex items-center gap-1.5 pg-small text-pg-text-soft">
      <StatusGlyph state={isLoadingDevices ? 'running' : deviceLoadError ? 'failed' : 'succeeded'} small />
      {isLoadingDevices
        ? 'Detecting devices'
        : deviceLoadError
          ? 'Detection failed'
          : `${deviceOptions.length} device${deviceOptions.length === 1 ? '' : 's'}`}
    </span>
  </div>

  {#if $expandedSection === 'device'}
    <div class="mt-3 flex flex-col gap-4 border-t border-pg-border-soft pt-3">
      <div class="flex flex-col gap-3">
        <div class="flex items-center justify-between gap-2">
          <h4 class="m-0 text-[12.5px] font-bold text-pg-text-soft">Inference device</h4>
          <button
            type="button"
            onclick={() => loadDevices()}
            disabled={isLoadingDevices}
            class="pg-btn pg-btn-sm"
            aria-label={isLoadingDevices ? 'Loading devices' : 'Refresh devices'}
          >
            {isLoadingDevices ? 'Loading...' : 'Refresh'}
          </button>
        </div>

        {#if deviceLoadError}
          <p class="m-0 pg-muted pg-small" role="alert">Failed to detect devices: {deviceLoadError}</p>
        {/if}

        <div class="flex flex-col gap-1">
          <label for="compute-device-select" class="pg-fl">Compute device</label>
          <select
            id="compute-device-select"
            bind:value={selectedDevice}
            disabled={isLoadingDevices || deviceOptions.length === 0}
            class="pg-field w-full"
          >
            {#each deviceOptions as device (device.id)}
              <option value={device.id}>{formatDeviceDisplayName(device)}</option>
            {/each}
          </select>
          <p class="m-0 pg-muted pg-small">
            {deviceOptions.length === 0
              ? 'No backend-confirmed devices available.'
              : 'Select a backend-confirmed device for inference.'}
          </p>
        </div>

        <div class="flex flex-col gap-1">
          <label for="gpu-layers-range" class="pg-fl">GPU layers</label>
          <div class="flex items-center gap-3">
            <input
              id="gpu-layers-range"
              type="range"
              bind:value={gpuLayers}
              min="-1"
              max="100"
              class="min-w-0 flex-1 cursor-pointer accent-pg-text"
            />
            <span class="pg-small w-28 text-right">{getGpuLayersLabel(gpuLayers)}</span>
          </div>
          <p class="m-0 pg-muted pg-small">-1 = all layers on GPU, 0 = CPU only</p>
        </div>

        <div class="flex flex-col gap-1">
          <span class="pg-fl">VRAM usage</span>
          {#if vramUsage}
            <div
              class="flex flex-col gap-1.5 rounded border bg-pg-panel-sunken px-3 py-2 pg-small {oomFlash
                ? 'border-pg-danger'
                : 'border-pg-border'}"
            >
              <div class="flex items-baseline justify-between gap-2">
                <span class="pg-muted">{getSelectedDeviceName()}</span>
                <span class="pg-mono">
                  {formatVramValue(vramUsage.used)} / {formatVramValue(vramUsage.total)} used
                </span>
              </div>
              <div class="pg-meter" aria-hidden="true">
                <i class:pg-meter-accent={vramUsage.percent >= 75} style="width: {vramUsage.percent}%"></i>
              </div>
              <div class="flex justify-between pg-muted">
                <span>{formatVramValue(vramUsage.free)} free</span>
                <span class="pg-mono">{vramUsage.percent}%</span>
              </div>
              {#if oomFlash}
                <p class="m-0 flex items-center gap-1.5">
                  <StatusGlyph state="failed" small />
                  Out of memory reported
                </p>
              {/if}
            </div>
          {:else}
            <p class="m-0 pg-muted pg-small">Select a GPU device to see VRAM usage.</p>
          {/if}
        </div>

        <div class="flex flex-col gap-1">
          <label for="embedding-memory-mode-select" class="pg-fl">Embedding memory mode</label>
          <select
            id="embedding-memory-mode-select"
            bind:value={embeddingMemoryMode}
            class="pg-field w-full"
          >
            <option value="cpu_parallel">CPU + GPU (Recommended)</option>
            <option value="gpu_parallel" disabled={!canFitBothModels}>
              Both on GPU {canFitBothModels ? '' : '(Insufficient VRAM)'}
            </option>
            <option value="sequential">Sequential (Low Memory)</option>
          </select>
          <p class="m-0 pg-muted pg-small">{getEmbeddingModeDescription(embeddingMemoryMode)}</p>
        </div>
      </div>

      {#if hasChanges}
        <div>
          <button type="button" onclick={saveConfig} disabled={isSaving} class="pg-btn pg-btn-primary">
            {isSaving ? 'Saving...' : 'Save device configuration'}
          </button>
        </div>
      {/if}

      <p class="m-0 pg-muted pg-small">Server will restart automatically when you save changes.</p>
    </div>
  {:else}
    <p class="m-0 mt-1 pg-muted pg-small truncate">
      {getSelectedDeviceName()}, {getGpuLayersLabel(gpuLayers)}, {getEmbeddingModeLabel(embeddingMemoryMode)}
    </p>
  {/if}
</section>

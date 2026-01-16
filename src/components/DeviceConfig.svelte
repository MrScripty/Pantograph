<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { ConfigService, type ConfigState, type DeviceConfig, type DeviceInfo, type EmbeddingMemoryMode } from '../services/ConfigService';
  import { LLMService } from '../services/LLMService';
  import { RagService } from '../services/RagService';
  import { expandedSection, toggleSection } from '../stores/accordionStore';

  let state: ConfigState = ConfigService.getState();
  let unsubscribe: (() => void) | null = null;
  let unsubscribeLLM: (() => void) | null = null;
  let unsubscribeRag: (() => void) | null = null;
  let isSaving = false;
  let isLoadingDevices = false;
  let isRefreshingDevices = false;
  let deviceLoadError: string | null = null;
  let oomFlash = false;
  let oomFlashTimer: ReturnType<typeof setTimeout> | null = null;
  let lastOomAt = 0;
  let refreshTimer: ReturnType<typeof setInterval> | null = null;

  // Available devices from llama-server
  let availableDevices: DeviceInfo[] = [];

  // Local form state
  let selectedDevice: string = 'auto';
  let gpuLayers: number = -1;
  let embeddingMemoryMode: EmbeddingMemoryMode = 'cpu_parallel';
  let initialEmbeddingMode: EmbeddingMemoryMode = 'cpu_parallel';

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

    // Load embedding memory mode
    const mode = await ConfigService.getEmbeddingMemoryMode();
    embeddingMemoryMode = mode;
    initialEmbeddingMode = mode;
  });

  onDestroy(() => {
    unsubscribe?.();
    unsubscribeLLM?.();
    unsubscribeRag?.();
    if (refreshTimer) {
      clearInterval(refreshTimer);
      refreshTimer = null;
    }
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
        // Provide fallback options
        availableDevices = [
          { id: 'none', name: 'CPU Only', total_vram_mb: 0, free_vram_mb: 0 },
        ];
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
      const device: DeviceConfig = {
        device: selectedDevice,
        gpu_layers: gpuLayers,
      };
      await ConfigService.setDeviceConfig(device);

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

  const formatVram = (mb: number): string => {
    if (mb === 0) return '';
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(1)} GB`;
    }
    return `${mb} MB`;
  };

  const formatVramValue = (mb: number): string => {
    if (mb <= 0) return '0 MB';
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(1)} GB`;
    }
    return `${mb} MB`;
  };

  const getDeviceDisplayName = (device: DeviceInfo): string => {
    if (device.id === 'none') return device.name;
    if (device.id === 'auto') return 'Auto (let llama-server choose)';
    const vram = formatVram(device.total_vram_mb);
    return vram ? `${device.name} (${vram})` : device.name;
  };

  const getSelectedDeviceName = (): string => {
    if (selectedDevice === 'auto') return 'Auto';
    const device = availableDevices.find(d => d.id === selectedDevice);
    return device?.name || selectedDevice;
  };

  $: hasChanges =
    selectedDevice !== state.config.device.device ||
    gpuLayers !== state.config.device.gpu_layers ||
    embeddingMemoryMode !== initialEmbeddingMode;

  $: selectedDeviceInfo = availableDevices.find(d => d.id === selectedDevice) || null;
  $: vramUsage = selectedDeviceInfo && selectedDeviceInfo.total_vram_mb > 0
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
    : null;

  // Add auto option to device list if not present
  $: deviceOptions = [
    { id: 'auto', name: 'Auto (let llama-server choose)', total_vram_mb: 0, free_vram_mb: 0 },
    ...availableDevices.filter(d => d.id !== 'auto'),
  ];

  // Minimum VRAM needed for embedding model (~800MB with buffer)
  const EMBEDDING_MODEL_VRAM_MB = 800;
  $: canFitBothModels = vramUsage ? vramUsage.free >= EMBEDDING_MODEL_VRAM_MB : false;

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

  $: {
    if ($expandedSection === 'device' && !refreshTimer) {
      void loadDevices({ silent: true });
      refreshTimer = setInterval(() => {
        void loadDevices({ silent: true });
      }, 3000);
    } else if ($expandedSection !== 'device' && refreshTimer) {
      clearInterval(refreshTimer);
      refreshTimer = null;
    }
  }
</script>

<div class="space-y-3">
  <!-- Header with toggle -->
  <button
    class="w-full flex items-center justify-between text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-400 transition-colors"
    on:click={() => toggleSection('device')}
  >
    <div class="flex items-center gap-2">
      <span>Device Configuration</span>
      {#if isLoadingDevices}
        <span class="w-1.5 h-1.5 rounded-full bg-yellow-500 animate-pulse"></span>
      {:else if deviceLoadError}
        <span class="w-1.5 h-1.5 rounded-full bg-red-500"></span>
      {:else}
        <span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
      {/if}
    </div>
    <svg
      class="w-3 h-3 transform transition-transform {$expandedSection === 'device' ? 'rotate-180' : ''}"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  {#if $expandedSection === 'device'}
    <div class="space-y-4 p-3 bg-neutral-800/30 rounded-lg">
      <!-- Device Selection -->
      <div class="space-y-2">
        <div class="flex items-center justify-between">
          <div class="text-[10px] text-neutral-600 uppercase tracking-wider">
            Inference Device
          </div>
          <button
            on:click={() => loadDevices()}
            disabled={isLoadingDevices}
            class="text-[10px] text-neutral-500 hover:text-neutral-400 disabled:opacity-50"
          >
            {isLoadingDevices ? 'Loading...' : 'Refresh'}
          </button>
        </div>

        {#if deviceLoadError}
          <div class="text-[10px] text-red-400 mb-2">
            Failed to detect devices: {deviceLoadError}
          </div>
        {/if}

        <!-- Device Selection -->
        <div class="space-y-1">
          <label class="text-xs text-neutral-400">Compute Device</label>
          <select
            bind:value={selectedDevice}
            disabled={isLoadingDevices}
            class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500 disabled:opacity-50"
            style="color-scheme: dark;"
          >
            {#each deviceOptions as device}
              <option value={device.id} class="bg-neutral-900 text-neutral-200">{getDeviceDisplayName(device)}</option>
            {/each}
          </select>
          <div class="text-[10px] text-neutral-600">
            Select your GPU for inference. Use dGPU for better performance.
          </div>
        </div>

        <!-- GPU Layers -->
        <div class="space-y-1">
          <label class="text-xs text-neutral-400">GPU Layers</label>
          <div class="flex items-center gap-3">
            <input
              type="range"
              bind:value={gpuLayers}
              min="-1"
              max="100"
              class="flex-1 h-1.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer"
            />
            <span class="text-xs text-neutral-300 w-24 text-right">{getGpuLayersLabel(gpuLayers)}</span>
          </div>
          <div class="text-[10px] text-neutral-600">
            -1 = all layers on GPU, 0 = CPU only
          </div>
        </div>

        <!-- VRAM Usage -->
        <div class="space-y-1">
          <label class="text-xs text-neutral-400">VRAM Usage</label>
          {#if vramUsage}
            <div class={`space-y-1 rounded border p-2 ${oomFlash ? 'border-red-500/70 bg-red-900/20 animate-pulse' : 'border-neutral-700/60 bg-neutral-900/40'}`}>
              <div class="flex items-center justify-between text-[10px] text-neutral-500">
                <span>{getSelectedDeviceName()}</span>
                <span class="text-neutral-300">
                  {formatVramValue(vramUsage.used)} / {formatVramValue(vramUsage.total)} used
                </span>
              </div>
              <div class="h-1.5 bg-neutral-700 rounded-full overflow-hidden">
                <div
                  class={vramUsage.percent >= 90 ? 'bg-red-500' : vramUsage.percent >= 75 ? 'bg-amber-500' : 'bg-blue-500'}
                  style="width: {vramUsage.percent}%"
                ></div>
              </div>
              <div class="flex justify-between text-[10px] text-neutral-600">
                <span>{formatVramValue(vramUsage.free)} free</span>
                <span>{vramUsage.percent}%</span>
              </div>
            </div>
          {:else}
            <div class="text-[10px] text-neutral-600">
              Select a GPU device to see VRAM usage.
            </div>
          {/if}
        </div>

        <!-- Embedding Memory Mode -->
        <div class="space-y-1">
          <label class="text-xs text-neutral-400">Embedding Memory Mode</label>
          <select
            bind:value={embeddingMemoryMode}
            class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500"
            style="color-scheme: dark;"
          >
            <option value="cpu_parallel" class="bg-neutral-900 text-neutral-200">
              CPU + GPU (Recommended)
            </option>
            <option
              value="gpu_parallel"
              class="bg-neutral-900 text-neutral-200"
              disabled={!canFitBothModels}
            >
              Both on GPU {canFitBothModels ? '' : '(Insufficient VRAM)'}
            </option>
            <option value="sequential" class="bg-neutral-900 text-neutral-200">
              Sequential (Low Memory)
            </option>
          </select>
          <div class="text-[10px] text-neutral-600">
            {getEmbeddingModeDescription(embeddingMemoryMode)}
          </div>
        </div>
      </div>

      <!-- Save Button -->
      {#if hasChanges}
        <button
          on:click={saveConfig}
          disabled={isSaving}
          class="w-full py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500 rounded text-xs transition-colors"
        >
          {isSaving ? 'Saving...' : 'Save Device Configuration'}
        </button>
      {/if}

      <!-- Help text -->
      <div class="text-[10px] text-neutral-600 leading-relaxed">
        Server will restart automatically when you save changes.
      </div>
    </div>
  {:else}
    <!-- Collapsed summary -->
    <div class="text-xs text-neutral-500">
      <span class="text-neutral-400">{getSelectedDeviceName()}</span>
      <span class="mx-1">|</span>
      <span class="text-neutral-400">{getGpuLayersLabel(gpuLayers)}</span>
      <span class="mx-1">|</span>
      <span class="text-neutral-400">{getEmbeddingModeLabel(embeddingMemoryMode)}</span>
    </div>
  {/if}
</div>

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke, Channel } from '@tauri-apps/api/core';
  import Modal from '../templates/Modal.svelte';
  import { ConfigService } from '../services/ConfigService';
  import { LLMService } from '../services/LLMService';

  interface BackendCapabilities {
    vision: boolean;
    embeddings: boolean;
    gpu: boolean;
    device_selection: boolean;
    streaming: boolean;
    tool_calling: boolean;
  }

  interface BackendInfo {
    name: string;
    description: string;
    capabilities: BackendCapabilities;
    active: boolean;
    available: boolean;
    unavailable_reason: string | null;
    can_install: boolean;
  }

  interface DownloadProgress {
    status: string;
    current: number;
    total: number;
    done: boolean;
    error: string | null;
  }

  // Download sizes for confirmation dialog
  const DOWNLOAD_SIZES: Record<string, string> = {
    'llama.cpp': '~60 MB',
    Ollama: '~1.6 GB',
  };

  let backends: BackendInfo[] = [];
  let currentBackend: string = '';
  let isLoading = false;
  let isSwitching = false;
  let error: string | null = null;
  let showDetails = false;

  // Download state
  let downloadingBackend: string | null = null;
  let downloadProgress: DownloadProgress | null = null;

  // Confirmation dialog state
  let confirmDownload: BackendInfo | null = null;

  const loadBackends = async () => {
    isLoading = true;
    error = null;
    try {
      backends = await invoke<BackendInfo[]>('list_backends');
      currentBackend = await invoke<string>('get_current_backend');
    } catch (e) {
      error = String(e);
      console.error('Failed to load backends:', e);
    } finally {
      isLoading = false;
    }
  };

  const switchBackend = async (name: string) => {
    if (name === currentBackend || isSwitching) return;

    isSwitching = true;
    error = null;
    try {
      await invoke('switch_backend', { backendName: name });
      currentBackend = name;

      // Auto-start the LLM after switching backends
      try {
        await ConfigService.startInferenceMode();
        await LLMService.refreshStatus();
      } catch (e) {
        // Show error to user instead of silently swallowing it
        error = `Auto-start failed: ${String(e)}`;
        console.warn('Auto-start failed:', e);
      }

      // Reload backends to update active status
      await loadBackends();
    } catch (e) {
      error = String(e);
      console.error('Failed to switch backend:', e);
    } finally {
      isSwitching = false;
    }
  };

  const promptDownload = (backend: BackendInfo) => {
    confirmDownload = backend;
  };

  const cancelDownload = () => {
    confirmDownload = null;
  };

  const startDownload = async () => {
    if (!confirmDownload) return;
    const name = confirmDownload.name;
    confirmDownload = null;

    downloadingBackend = name;
    downloadProgress = { status: 'Starting...', current: 0, total: 0, done: false, error: null };
    error = null;

    try {
      const channel = new Channel<DownloadProgress>();
      channel.onmessage = (event: DownloadProgress) => {
        downloadProgress = event;
        if (event.error) {
          error = event.error;
          downloadingBackend = null;
          downloadProgress = null;
        }
        if (event.done && !event.error) {
          downloadingBackend = null;
          downloadProgress = null;
          // Reload backends to update availability
          loadBackends();
        }
      };

      // Call backend-specific download command
      if (name === 'llama.cpp') {
        await invoke('download_llama_binaries', { channel });
      } else if (name === 'Ollama') {
        await invoke('download_ollama_binary', { channel });
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      downloadingBackend = null;
      downloadProgress = null;
    }
  };

  const handleBackendClick = (backend: BackendInfo) => {
    if (backend.available) {
      switchBackend(backend.name);
    } else if (backend.can_install) {
      promptDownload(backend);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  onMount(() => {
    loadBackends();
  });

  // Get the active backend info
  $: activeBackend = backends.find((b) => b.name === currentBackend);
</script>

<!-- Confirmation Dialog -->
<Modal open={confirmDownload !== null} title="Download Backend" size="sm" onclose={cancelDownload}>
  {#if confirmDownload}
    <p class="text-neutral-300">
      Download <strong>{confirmDownload.name}</strong> backend?
    </p>
    <p class="text-sm text-neutral-500 mt-2">Size: {DOWNLOAD_SIZES[confirmDownload.name] || 'Unknown'}</p>
  {/if}
  {#snippet footer()}
    <button onclick={cancelDownload} class="px-4 py-2 text-sm text-neutral-400 hover:text-white">
      Cancel
    </button>
    <button
      onclick={startDownload}
      class="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 text-white rounded"
    >
      Download
    </button>
  {/snippet}
</Modal>

<div class="space-y-3">
  <!-- Header with toggle -->
  <button
    class="w-full flex items-center justify-between text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-400 transition-colors"
    onclick={() => (showDetails = !showDetails)}
  >
    <div class="flex items-center gap-2">
      <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
        />
      </svg>
      <span>Backend</span>
      {#if currentBackend}
        <span class="text-blue-400 normal-case">({currentBackend})</span>
      {/if}
    </div>
    <svg
      class="w-3 h-3 transform transition-transform {showDetails ? 'rotate-180' : ''}"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  {#if showDetails}
    <div class="space-y-3 p-3 bg-neutral-800/30 rounded-lg">
      {#if isLoading}
        <div class="flex items-center gap-2 text-xs text-neutral-500">
          <div
            class="w-3 h-3 border border-neutral-500 border-t-transparent rounded-full animate-spin"
          ></div>
          <span>Loading backends...</span>
        </div>
      {:else if backends.length === 0}
        <div class="text-xs text-neutral-500">No backends available</div>
      {:else}
        <!-- Backend selection buttons -->
        <div class="flex flex-wrap gap-2">
          {#each backends as backend}
            <div class="flex flex-col">
              <button
                class="px-3 py-1.5 text-xs rounded transition-colors flex items-center gap-1.5 {backend.name ===
                currentBackend
                  ? 'bg-blue-600 text-white'
                  : backend.available
                    ? 'bg-neutral-700 text-neutral-300 hover:bg-neutral-600'
                    : backend.can_install
                      ? 'bg-neutral-700/50 text-neutral-400 hover:bg-neutral-600/50 cursor-pointer'
                      : 'bg-neutral-800 text-neutral-600 cursor-not-allowed'}"
                onclick={() => handleBackendClick(backend)}
                disabled={(!backend.available && !backend.can_install) ||
                  isSwitching ||
                  downloadingBackend !== null}
                title={backend.available
                  ? backend.description
                  : backend.can_install
                    ? `Click to download (${DOWNLOAD_SIZES[backend.name] || 'Unknown size'})`
                    : backend.unavailable_reason || 'Not available'}
              >
                {#if isSwitching && backend.name === currentBackend}
                  <span
                    class="inline-block w-3 h-3 border border-white border-t-transparent rounded-full animate-spin"
                  ></span>
                {:else if downloadingBackend === backend.name}
                  <span
                    class="inline-block w-3 h-3 border border-current border-t-transparent rounded-full animate-spin"
                  ></span>
                {:else if !backend.available && backend.can_install}
                  <!-- Lucide Download Icon -->
                  <svg
                    class="w-3 h-3"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="7 10 12 15 17 10" />
                    <line x1="12" y1="15" x2="12" y2="3" />
                  </svg>
                {/if}
                {backend.name}
              </button>

              {#if downloadingBackend === backend.name && downloadProgress}
                <div class="mt-1 space-y-1">
                  <div class="text-[9px] text-blue-400">{downloadProgress.status}</div>
                  {#if downloadProgress.total > 0}
                    <div class="bg-neutral-700 rounded-full h-1 overflow-hidden">
                      <div
                        class="bg-blue-500 h-1 transition-all duration-300"
                        style="width: {(downloadProgress.current / downloadProgress.total) * 100}%"
                      ></div>
                    </div>
                    <div class="text-[8px] text-neutral-500">
                      {formatBytes(downloadProgress.current)} / {formatBytes(downloadProgress.total)}
                    </div>
                  {/if}
                </div>
              {:else if !backend.available && backend.unavailable_reason}
                <span class="text-[9px] text-amber-400 mt-0.5 max-w-[120px] leading-tight">
                  {#if backend.can_install}
                    Click to install
                  {:else}
                    {backend.unavailable_reason}
                  {/if}
                </span>
              {/if}
            </div>
          {/each}
        </div>

        <!-- Active backend description -->
        {#if activeBackend}
          <div class="text-[10px] text-neutral-500">
            {activeBackend.description}
          </div>

          <!-- Capability badges -->
          <div class="flex flex-wrap gap-1.5">
            {#if activeBackend.capabilities.vision}
              <span
                class="px-1.5 py-0.5 text-[10px] bg-green-900/30 text-green-400 rounded"
                title="Supports vision/multimodal models"
              >
                Vision
              </span>
            {/if}
            {#if activeBackend.capabilities.embeddings}
              <span
                class="px-1.5 py-0.5 text-[10px] bg-purple-900/30 text-purple-400 rounded"
                title="Supports embedding generation"
              >
                Embeddings
              </span>
            {/if}
            {#if activeBackend.capabilities.gpu}
              <span
                class="px-1.5 py-0.5 text-[10px] bg-amber-900/30 text-amber-400 rounded"
                title="Has GPU acceleration"
              >
                GPU
              </span>
            {/if}
            {#if activeBackend.capabilities.device_selection}
              <span
                class="px-1.5 py-0.5 text-[10px] bg-cyan-900/30 text-cyan-400 rounded"
                title="Allows manual GPU device selection"
              >
                Device Select
              </span>
            {/if}
            {#if activeBackend.capabilities.streaming}
              <span
                class="px-1.5 py-0.5 text-[10px] bg-blue-900/30 text-blue-400 rounded"
                title="Supports streaming token output"
              >
                Streaming
              </span>
            {/if}
            {#if activeBackend.capabilities.tool_calling}
              <span
                class="px-1.5 py-0.5 text-[10px] bg-pink-900/30 text-pink-400 rounded"
                title="Supports tool/function calling"
              >
                Tools
              </span>
            {/if}
          </div>
        {/if}
      {/if}

      <!-- Error display -->
      {#if error}
        <div class="text-xs text-red-400 bg-red-900/20 border border-red-800/50 rounded p-2">
          {error}
        </div>
      {/if}

      <!-- Refresh button -->
      <button
        onclick={loadBackends}
        disabled={isLoading}
        class="text-[10px] text-neutral-600 hover:text-neutral-400 transition-colors disabled:opacity-50"
      >
        Refresh
      </button>
    </div>
  {:else}
    <!-- Collapsed summary -->
    <div class="flex items-center gap-2 text-xs text-neutral-500">
      {#if activeBackend}
        <span class="text-blue-400">{currentBackend}</span>
        <!-- Show key capabilities as small indicators -->
        <div class="flex gap-1">
          {#if activeBackend.capabilities.vision}
            <span class="w-1.5 h-1.5 rounded-full bg-green-500" title="Vision"></span>
          {/if}
          {#if activeBackend.capabilities.gpu}
            <span class="w-1.5 h-1.5 rounded-full bg-amber-500" title="GPU"></span>
          {/if}
        </div>
      {:else if isLoading}
        <span>Loading...</span>
      {:else}
        <span>No backend selected</span>
      {/if}
    </div>
  {/if}
</div>

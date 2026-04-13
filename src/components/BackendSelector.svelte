<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke, Channel } from '@tauri-apps/api/core';
  import Modal from '../templates/Modal.svelte';
  import { ConfigService, type ServerModeInfo } from '../services/ConfigService';
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
    backend_key: string;
    description: string;
    capabilities: BackendCapabilities;
    active: boolean;
    available: boolean;
    unavailable_reason: string | null;
    can_install: boolean;
    runtime_binary_id: 'llama_cpp' | 'ollama' | null;
  }

  interface ManagedRuntimeCapability {
    id: 'llama_cpp' | 'ollama';
    display_name: string;
    install_state: 'installed' | 'system_provided' | 'missing' | 'unsupported';
    available: boolean;
    can_install: boolean;
    can_remove: boolean;
    missing_files: string[];
    unavailable_reason: string | null;
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
    llama_cpp: '~60 MB',
    ollama: '~1.6 GB',
  };

  let backends: BackendInfo[] = $state([]);
  let runtimes: ManagedRuntimeCapability[] = $state([]);
  let currentBackendKey: string = $state('');
  let isLoading = $state(false);
  let isSwitching = $state(false);
  let error: string | null = $state(null);
  let serverRunning = $state(false);

  // Download state
  let downloadingBackend: string | null = $state(null);
  let downloadProgress: DownloadProgress | null = $state(null);

  // LLM status subscription
  let unsubscribe: (() => void) | null = null;

  // Confirmation dialog state
  let confirmDownload: BackendInfo | null = $state(null);

  const loadBackends = async () => {
    isLoading = true;
    error = null;
    try {
      backends = await invoke<BackendInfo[]>('list_backends');
      runtimes = await invoke<ManagedRuntimeCapability[]>('list_managed_runtimes');
      const status = await LLMService.refreshStatus();
      currentBackendKey = status.backend_key || '';
    } catch (e) {
      error = String(e);
      console.error('Failed to load backends:', e);
    } finally {
      isLoading = false;
    }
  };

  const switchBackend = async (backendKey: string) => {
    if (backendKey === currentBackendKey || isSwitching) return;

    isSwitching = true;
    error = null;
    try {
      const status = await invoke<ServerModeInfo>('switch_backend', { backendName: backendKey });
      currentBackendKey = status.backend_key || backendKey;

      // Auto-start the LLM after switching backends
      try {
        const started = await ConfigService.startInferenceMode();
        currentBackendKey = started.backend_key || currentBackendKey;
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

  const stopServer = async () => {
    isSwitching = true;
    error = null;
    try {
      const status = await LLMService.stop();
      currentBackendKey = status.backend_key || '';
    } catch (e) {
      error = String(e);
      console.error('Failed to stop server:', e);
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
    const runtime = runtimeForBackend(confirmDownload);
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

      if (!runtime) {
        throw new Error(`No managed runtime is associated with ${name}`);
      }

      await invoke('install_managed_runtime', {
        binaryId: runtime.id,
        channel
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      downloadingBackend = null;
      downloadProgress = null;
    }
  };

  const removeRuntime = async (backend: BackendInfo) => {
    const runtime = runtimeForBackend(backend);
    if (!runtime?.can_remove || isSwitching || downloadingBackend !== null) return;

    isSwitching = true;
    error = null;
    try {
      await invoke('remove_managed_runtime', { binaryId: runtime.id });
      await loadBackends();
    } catch (e) {
      error = String(e);
    } finally {
      isSwitching = false;
    }
  };

  const handleBackendClick = (backend: BackendInfo) => {
    if (backend.backend_key === currentBackendKey && serverRunning) {
      // Toggle off - stop the server
      stopServer();
    } else if (backend.available) {
      switchBackend(backend.backend_key);
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
    // Subscribe to LLM status to track when server is actually running
    unsubscribe = LLMService.subscribe((state) => {
      serverRunning = state.status.ready;
    });
  });

  onDestroy(() => {
    if (unsubscribe) unsubscribe();
  });

  const runtimeForBackend = (backend: BackendInfo): ManagedRuntimeCapability | undefined => {
    if (!backend.runtime_binary_id) return undefined;
    return runtimes.find((runtime) => runtime.id === backend.runtime_binary_id);
  };

  // Get the active backend info
  // Only show backend as active if server is actually running
  let activeBackend = $derived(serverRunning ? backends.find((b) => b.backend_key === currentBackendKey) : null);
</script>

<!-- Confirmation Dialog -->
<Modal open={confirmDownload !== null} title="Download Backend" size="sm" onclose={cancelDownload}>
  {#if confirmDownload}
    <p class="text-neutral-300">
      Download <strong>{confirmDownload.name}</strong> backend?
    </p>
    <p class="text-sm text-neutral-500 mt-2">Size: {DOWNLOAD_SIZES[confirmDownload.backend_key] || 'Unknown'}</p>
  {/if}
  {#snippet footer()}
    <button type="button" onclick={cancelDownload} class="px-4 py-2 text-sm text-neutral-400 hover:text-white">
      Cancel
    </button>
    <button type="button"
      onclick={startDownload}
      class="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 text-white rounded"
    >
      Download
    </button>
  {/snippet}
</Modal>

<div class="space-y-2">
  <div class="text-[10px] text-neutral-600 uppercase tracking-wider">
    Built-in inference engine
  </div>

  <div class="space-y-2">
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
          {#each backends as backend (backend.name)}
            {@const runtime = runtimeForBackend(backend)}
            <div class="flex flex-col">
              <div class="flex items-center gap-1.5">
                <button type="button"
                  class="px-3 py-1.5 text-xs rounded transition-colors flex items-center gap-1.5 {backend.backend_key ===
                    currentBackendKey && serverRunning
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
                      ? `Click to download (${DOWNLOAD_SIZES[backend.backend_key] || 'Unknown size'})`
                      : backend.unavailable_reason || 'Not available'}
                >
                  {#if isSwitching && backend.backend_key === currentBackendKey}
                    <span
                      class="inline-block w-3 h-3 border border-white border-t-transparent rounded-full animate-spin"
                    ></span>
                  {:else if downloadingBackend === backend.name}
                    <span
                      class="inline-block w-3 h-3 border border-current border-t-transparent rounded-full animate-spin"
                    ></span>
                  {:else if !backend.available && backend.can_install}
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

                {#if runtime?.can_remove}
                  <button
                    type="button"
                    class="px-2 py-1 text-[10px] rounded bg-neutral-800 text-neutral-400 hover:bg-neutral-700 hover:text-white transition-colors"
                    onclick={(event) => {
                      event.stopPropagation();
                      removeRuntime(backend);
                    }}
                    disabled={isSwitching || downloadingBackend !== null}
                    title={`Remove ${runtime.display_name}`}
                  >
                    Remove
                  </button>
                {/if}
              </div>

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
              {:else if runtime?.install_state === 'system_provided'}
                <span class="text-[9px] text-neutral-500 mt-0.5 max-w-[140px] leading-tight">
                  Using system runtime
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
      <button type="button"
        onclick={loadBackends}
        disabled={isLoading}
        class="text-[10px] text-neutral-600 hover:text-neutral-400 transition-colors disabled:opacity-50"
      >
        Refresh
      </button>
  </div>
</div>

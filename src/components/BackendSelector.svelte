<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

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
  }

  let backends: BackendInfo[] = [];
  let currentBackend: string = '';
  let isLoading = false;
  let isSwitching = false;
  let error: string | null = null;
  let showDetails = false;

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
      // Reload backends to update active status
      await loadBackends();
    } catch (e) {
      error = String(e);
      console.error('Failed to switch backend:', e);
    } finally {
      isSwitching = false;
    }
  };

  onMount(() => {
    loadBackends();
  });

  // Get the active backend info
  $: activeBackend = backends.find((b) => b.name === currentBackend);
</script>

<div class="space-y-3">
  <!-- Header with toggle -->
  <button
    class="w-full flex items-center justify-between text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-400 transition-colors"
    on:click={() => (showDetails = !showDetails)}
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
          <div class="w-3 h-3 border border-neutral-500 border-t-transparent rounded-full animate-spin"
          ></div>
          <span>Loading backends...</span>
        </div>
      {:else if backends.length === 0}
        <div class="text-xs text-neutral-500">No backends available</div>
      {:else}
        <!-- Backend selection buttons -->
        <div class="flex flex-wrap gap-2">
          {#each backends as backend}
            <button
              class="px-3 py-1.5 text-xs rounded transition-colors {backend.name === currentBackend
                ? 'bg-blue-600 text-white'
                : backend.available
                  ? 'bg-neutral-700 text-neutral-300 hover:bg-neutral-600'
                  : 'bg-neutral-800 text-neutral-600 cursor-not-allowed'}"
              on:click={() => switchBackend(backend.name)}
              disabled={!backend.available || isSwitching}
              title={backend.description}
            >
              {#if isSwitching && backend.name === currentBackend}
                <span class="inline-block w-3 h-3 border border-white border-t-transparent rounded-full animate-spin mr-1"></span>
              {/if}
              {backend.name}
            </button>
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
        on:click={loadBackends}
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

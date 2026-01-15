<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { LLMService, type LLMState } from '../services/LLMService';
  import {
    ConfigService,
    type ConfigState,
    type ServerModeInfo,
  } from '../services/ConfigService';

  let llmState: LLMState = LLMService.getState();
  let configState: ConfigState = ConfigService.getState();
  let unsubscribeLLM: (() => void) | null = null;
  let unsubscribeConfig: (() => void) | null = null;

  // UI state
  let connectionType: 'external' | 'sidecar' = 'external';
  let externalUrl = 'http://localhost:1234';
  let isConnecting = false;
  let isStarting = false;
  let showSettings = false;

  onMount(async () => {
    unsubscribeLLM = LLMService.subscribe((state) => {
      llmState = state;
      // Determine connection type from current mode
      if (state.status.mode.startsWith('sidecar')) {
        connectionType = 'sidecar';
      } else if (state.status.mode === 'external') {
        connectionType = 'external';
      }
    });

    unsubscribeConfig = ConfigService.subscribe((state) => {
      configState = state;
      if (state.config.external_url) {
        externalUrl = state.config.external_url;
      }
    });

    // Load config and server mode
    try {
      await ConfigService.loadConfig();
      await ConfigService.refreshServerMode();
    } catch {
      // Errors handled by service
    }
  });

  onDestroy(() => {
    unsubscribeLLM?.();
    unsubscribeConfig?.();
  });

  const connectExternal = async () => {
    if (!externalUrl.trim()) return;
    isConnecting = true;
    try {
      await LLMService.connectToServer(externalUrl);
      // Save the URL to config
      await ConfigService.saveConfig({
        ...configState.config,
        external_url: externalUrl,
        connection_mode: { type: 'External', url: externalUrl },
      });
    } catch (error) {
      console.error('Failed to connect:', error);
    } finally {
      isConnecting = false;
    }
  };

  const startSidecarVLM = async () => {
    isStarting = true;
    try {
      await ConfigService.startInferenceMode();
      await LLMService.refreshStatus();
    } catch (error) {
      console.error('Failed to start sidecar:', error);
    } finally {
      isStarting = false;
    }
  };

  const stopServer = async () => {
    await LLMService.stop();
    await ConfigService.refreshServerMode();
  };

  $: statusColor = llmState.status.ready
    ? 'bg-green-500'
    : llmState.status.mode !== 'none'
      ? 'bg-yellow-500 animate-pulse'
      : 'bg-neutral-500';

  $: statusText = llmState.status.ready
    ? 'Ready'
    : llmState.status.mode !== 'none'
      ? 'Starting...'
      : 'Not connected';

  $: modeText = (() => {
    switch (configState.serverMode.mode) {
      case 'external':
        return 'External';
      case 'sidecar_inference':
        return 'Sidecar (VLM)';
      case 'sidecar_embedding':
        return 'Sidecar (Embedding)';
      default:
        return 'None';
    }
  })();

  $: canStartSidecar =
    configState.config.models.vlm_model_path &&
    configState.config.models.vlm_mmproj_path &&
    !llmState.status.ready &&
    !isStarting;
</script>

<div class="space-y-3">
  <!-- Header with toggle -->
  <button
    class="w-full flex items-center justify-between text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-400 transition-colors"
    on:click={() => (showSettings = !showSettings)}
  >
    <div class="flex items-center gap-2">
      <span class="w-2 h-2 rounded-full {statusColor}"></span>
      <span>LLM Server</span>
      {#if llmState.status.ready}
        <span class="text-green-400 normal-case">({modeText})</span>
      {/if}
    </div>
    <svg
      class="w-3 h-3 transform transition-transform {showSettings ? 'rotate-180' : ''}"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  {#if showSettings}
    <div class="space-y-3 p-3 bg-neutral-800/30 rounded-lg">
      <!-- Connection Type Tabs -->
      <div class="flex gap-1 p-1 bg-neutral-900 rounded">
        <button
          class="flex-1 px-2 py-1 text-xs rounded transition-colors {connectionType === 'external'
            ? 'bg-neutral-700 text-neutral-200'
            : 'text-neutral-500 hover:text-neutral-400'}"
          on:click={() => (connectionType = 'external')}
        >
          External
        </button>
        <button
          class="flex-1 px-2 py-1 text-xs rounded transition-colors {connectionType === 'sidecar'
            ? 'bg-neutral-700 text-neutral-200'
            : 'text-neutral-500 hover:text-neutral-400'}"
          on:click={() => (connectionType = 'sidecar')}
        >
          Sidecar
        </button>
      </div>

      {#if connectionType === 'external'}
        <!-- External Connection -->
        <div class="space-y-2">
          <div class="text-[10px] text-neutral-600 uppercase tracking-wider">
            Connect to external server (LM Studio, API, etc.)
          </div>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={externalUrl}
              placeholder="http://localhost:1234"
              class="flex-1 bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500"
              disabled={isConnecting}
            />
            <button
              on:click={connectExternal}
              disabled={isConnecting || !externalUrl.trim()}
              class="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500 rounded text-xs transition-colors"
            >
              {isConnecting ? '...' : 'Connect'}
            </button>
          </div>
        </div>
      {:else}
        <!-- Sidecar Mode -->
        <div class="space-y-2">
          <div class="text-[10px] text-neutral-600 uppercase tracking-wider">
            Built-in llama.cpp server
          </div>

          {#if !ConfigService.hasVlmModels}
            <div class="text-xs text-amber-400 bg-amber-900/20 border border-amber-800/50 rounded p-2">
              Configure model paths below to use sidecar mode
            </div>
          {:else if configState.serverMode.mode === 'sidecar_inference'}
            <div class="flex items-center gap-2 text-xs text-green-400">
              <span class="w-2 h-2 rounded-full bg-green-500"></span>
              Running in VLM mode
            </div>
            {#if configState.serverMode.model_path}
              <div class="text-[10px] text-neutral-500 truncate">
                {configState.serverMode.model_path.split('/').pop()}
              </div>
            {/if}
          {:else if configState.serverMode.mode === 'sidecar_embedding'}
            <div class="flex items-center gap-2 text-xs text-blue-400">
              <span class="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></span>
              Running in Embedding mode
            </div>
          {:else}
            <div class="text-xs text-neutral-500">
              Server not running
            </div>
          {/if}

          <div class="flex gap-2">
            <button
              on:click={startSidecarVLM}
              disabled={!canStartSidecar}
              class="flex-1 px-2 py-1.5 bg-green-600 hover:bg-green-500 disabled:bg-neutral-700 disabled:text-neutral-500 rounded text-xs transition-colors"
            >
              {isStarting ? 'Starting...' : 'Start VLM'}
            </button>
            {#if llmState.status.ready}
              <button
                on:click={stopServer}
                class="px-3 py-1.5 bg-red-600 hover:bg-red-500 rounded text-xs transition-colors"
              >
                Stop
              </button>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Status -->
      {#if llmState.status.ready}
        <div class="flex items-center justify-between text-xs border-t border-neutral-700 pt-2">
          <span class="text-neutral-500">Status: {statusText}</span>
          {#if llmState.status.url}
            <span class="text-neutral-600 font-mono text-[10px]">{llmState.status.url}</span>
          {/if}
        </div>
      {/if}

      <!-- Error -->
      {#if llmState.error || configState.error}
        <div class="text-xs text-red-400 bg-red-900/20 border border-red-800/50 rounded p-2">
          {llmState.error || configState.error}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Collapsed summary -->
    <div class="flex items-center gap-2 text-xs text-neutral-500">
      {#if llmState.status.ready}
        <span class="text-green-400">{modeText}</span>
        {#if llmState.status.url}
          <span class="text-neutral-600 font-mono text-[10px] truncate">{llmState.status.url}</span>
        {/if}
      {:else}
        <span>Not connected</span>
      {/if}
    </div>
  {/if}
</div>

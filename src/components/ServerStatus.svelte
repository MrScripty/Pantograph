<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { LLMService, type LLMState } from '../services/LLMService';
  import {
    ConfigService,
    type ConfigState,
    type ServerModeInfo,
  } from '../services/ConfigService';
  import BackendSelector from './BackendSelector.svelte';
  import { expandedSection, toggleSection } from '../stores/accordionStore';

  let llmState: LLMState = LLMService.getState();
  let configState: ConfigState = ConfigService.getState();
  let unsubscribeLLM: (() => void) | null = null;
  let unsubscribeConfig: (() => void) | null = null;

  // UI state
  let connectionType: 'external' | 'sidecar' = 'sidecar';
  let externalUrl = 'http://localhost:1234';
  let apiKey = '';
  let isConnecting = false;

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
      if (state.config.api_key) {
        apiKey = state.config.api_key;
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
      // Save the URL and API key to config
      await ConfigService.saveConfig({
        ...configState.config,
        external_url: externalUrl,
        api_key: apiKey || null,
        connection_mode: { type: 'External', url: externalUrl },
      });
    } catch (error) {
      console.error('Failed to connect:', error);
    } finally {
      isConnecting = false;
    }
  };

  const disconnectExternal = async () => {
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
</script>

<div class="space-y-3">
  <!-- Header with toggle -->
  <button
    class="w-full flex items-center justify-between text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-400 transition-colors"
    on:click={() => toggleSection('server')}
  >
    <div class="flex items-center gap-2">
      <span class="w-2 h-2 rounded-full {statusColor}"></span>
      <span>LLM Server</span>
      {#if llmState.status.ready}
        <span class="text-green-400 normal-case">({modeText})</span>
      {/if}
    </div>
    <svg
      class="w-3 h-3 transform transition-transform {$expandedSection === 'server' ? 'rotate-180' : ''}"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  {#if $expandedSection === 'server'}
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
            Connect to external server (LM Studio, OpenAI, etc.)
          </div>
          <input
            type="text"
            bind:value={externalUrl}
            placeholder="http://localhost:1234 or https://api.openai.com/v1"
            class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500"
            disabled={isConnecting || (llmState.status.ready && llmState.status.mode === 'external')}
          />
          <input
            type="password"
            bind:value={apiKey}
            placeholder="API Key (optional)"
            class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500"
            disabled={isConnecting || (llmState.status.ready && llmState.status.mode === 'external')}
          />
          <div class="flex gap-2">
            {#if llmState.status.ready && llmState.status.mode === 'external'}
              <button
                on:click={disconnectExternal}
                class="flex-1 px-3 py-1.5 bg-red-600 hover:bg-red-500 rounded text-xs transition-colors"
              >
                Disconnect
              </button>
            {:else}
              <button
                on:click={connectExternal}
                disabled={isConnecting || !externalUrl.trim()}
                class="flex-1 px-3 py-1.5 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500 rounded text-xs transition-colors"
              >
                {isConnecting ? 'Connecting...' : 'Connect'}
              </button>
            {/if}
          </div>
        </div>
      {:else}
        <!-- Sidecar Mode - Backend Selector -->
        <BackendSelector />
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

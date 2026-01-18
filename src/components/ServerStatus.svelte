<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { LLMService, type LLMState } from '../services/LLMService';
  import {
    ConfigService,
    type ConfigState,
  } from '../services/ConfigService';
  import {
    HealthMonitorService,
    type HealthMonitorState,
    type ServerEvent,
  } from '../services/HealthMonitorService';
  import BackendSelector from './BackendSelector.svelte';
  import { expandedSection, toggleSection } from '../stores/accordionStore';

  let llmState: LLMState = $state(LLMService.getState());
  let configState: ConfigState = $state(ConfigService.getState());
  let healthState: HealthMonitorState = $state(HealthMonitorService.getState());
  let unsubscribeLLM: (() => void) | null = null;
  let unsubscribeConfig: (() => void) | null = null;
  let unsubscribeHealth: (() => void) | null = null;
  let unsubscribeEvents: (() => void) | null = null;

  // UI state
  let connectionType: 'external' | 'sidecar' = $state('sidecar');
  let externalUrl = $state('http://localhost:1234');
  let apiKey = $state('');
  let isConnecting = $state(false);
  let showHealthDetails = $state(false);

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

    unsubscribeHealth = HealthMonitorService.subscribeState((state) => {
      healthState = state;
    });

    unsubscribeEvents = HealthMonitorService.subscribeEvents((event: ServerEvent) => {
      // Handle server events (could show toast notifications, etc.)
      if (event.type === 'server_crashed') {
        console.warn('[ServerStatus] Server crashed:', event.error);
      }
    });

    // Load config and server mode
    try {
      await ConfigService.loadConfig();
      await ConfigService.refreshServerMode();
    } catch {
      // Errors handled by service
    }

    // Start health monitoring if server is ready
    if (llmState.status.ready) {
      try {
        await HealthMonitorService.start();
      } catch {
        // Non-critical
      }
    }
  });

  onDestroy(() => {
    unsubscribeLLM?.();
    unsubscribeConfig?.();
    unsubscribeHealth?.();
    unsubscribeEvents?.();
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
      // Start health monitoring
      await HealthMonitorService.start();
    } catch (error) {
      console.error('Failed to connect:', error);
    } finally {
      isConnecting = false;
    }
  };

  const disconnectExternal = async () => {
    await HealthMonitorService.stop();
    await LLMService.stop();
    await ConfigService.refreshServerMode();
  };

  const triggerRecovery = async () => {
    try {
      await HealthMonitorService.triggerRecovery();
    } catch (error) {
      console.error('Recovery failed:', error);
    }
  };

  const checkHealthNow = async () => {
    await HealthMonitorService.checkNow();
  };

  let statusColor = $derived(
    llmState.status.ready
      ? healthState.lastResult?.healthy === false
        ? 'bg-red-500'
        : 'bg-green-500'
      : llmState.status.mode !== 'none'
        ? 'bg-yellow-500 animate-pulse'
        : 'bg-neutral-500'
  );

  let statusText = $derived(
    llmState.status.ready
      ? healthState.lastResult?.healthy === false
        ? 'Unhealthy'
        : 'Ready'
      : llmState.status.mode !== 'none'
        ? 'Starting...'
        : 'Not connected'
  );

  let modeText = $derived((() => {
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
  })());

  let healthStatusColor = $derived(
    healthState.lastResult
      ? HealthMonitorService.getStatusColor(healthState.lastResult.status)
      : 'text-neutral-500'
  );

  let healthStatusLabel = $derived(
    healthState.lastResult
      ? HealthMonitorService.getStatusLabel(healthState.lastResult.status)
      : 'Unknown'
  );
</script>

<div class="space-y-3">
  <!-- Header with toggle -->
  <button
    class="w-full flex items-center justify-between text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-400 transition-colors"
    onclick={() => toggleSection('server')}
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
          onclick={() => (connectionType = 'external')}
        >
          External
        </button>
        <button
          class="flex-1 px-2 py-1 text-xs rounded transition-colors {connectionType === 'sidecar'
            ? 'bg-neutral-700 text-neutral-200'
            : 'text-neutral-500 hover:text-neutral-400'}"
          onclick={() => (connectionType = 'sidecar')}
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
                onclick={disconnectExternal}
                class="flex-1 px-3 py-1.5 bg-red-600 hover:bg-red-500 rounded text-xs transition-colors"
              >
                Disconnect
              </button>
            {:else}
              <button
                onclick={connectExternal}
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

        <!-- Health Status -->
        <button
          class="w-full flex items-center justify-between text-xs py-1 hover:bg-neutral-800/50 rounded px-1 transition-colors"
          onclick={() => (showHealthDetails = !showHealthDetails)}
        >
          <div class="flex items-center gap-2">
            <span class="text-neutral-500">Health:</span>
            <span class={healthStatusColor}>{healthStatusLabel}</span>
            {#if healthState.lastResult?.response_time_ms}
              <span class="text-neutral-600 text-[10px]">
                ({healthState.lastResult.response_time_ms}ms)
              </span>
            {/if}
          </div>
          <svg
            class="w-3 h-3 text-neutral-500 transform transition-transform {showHealthDetails ? 'rotate-180' : ''}"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </button>

        {#if showHealthDetails}
          <div class="space-y-2 pl-2 border-l-2 border-neutral-700">
            <!-- Health details -->
            {#if healthState.lastResult}
              <div class="text-[10px] text-neutral-500 space-y-1">
                <div>Consecutive failures: {healthState.lastResult.consecutive_failures}</div>
                <div>Last check: {new Date(healthState.lastResult.timestamp).toLocaleTimeString()}</div>
                {#if healthState.lastResult.error}
                  <div class="text-red-400">Error: {healthState.lastResult.error}</div>
                {/if}
              </div>
            {/if}

            <!-- Health actions -->
            <div class="flex gap-2">
              <button
                onclick={checkHealthNow}
                class="flex-1 px-2 py-1 text-[10px] bg-neutral-700 hover:bg-neutral-600 rounded transition-colors"
              >
                Check Now
              </button>
              {#if healthState.lastResult?.healthy === false}
                <button
                  onclick={triggerRecovery}
                  disabled={healthState.isRecovering}
                  class="flex-1 px-2 py-1 text-[10px] bg-yellow-700 hover:bg-yellow-600 disabled:opacity-50 rounded transition-colors"
                >
                  {healthState.isRecovering ? 'Recovering...' : 'Recover'}
                </button>
              {/if}
            </div>

            <!-- Monitoring status -->
            <div class="text-[10px] text-neutral-600">
              {#if healthState.isRunning}
                <span class="text-green-500">● Monitoring active</span>
              {:else}
                <span class="text-neutral-500">○ Monitoring inactive</span>
              {/if}
            </div>
          </div>
        {/if}
      {/if}

      <!-- Error -->
      {#if llmState.error || configState.error || healthState.error}
        <div class="text-xs text-red-400 bg-red-900/20 border border-red-800/50 rounded p-2">
          {llmState.error || configState.error || healthState.error}
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

<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
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
  import ManagedRuntimePanel from './runtime-manager/ManagedRuntimePanel.svelte';
  import ExternalConnectionPanel from './server-status/ExternalConnectionPanel.svelte';
  import HealthStatusPanel from './server-status/HealthStatusPanel.svelte';
  import RuntimeSnapshotGrid from './server-status/RuntimeSnapshotGrid.svelte';
  import { expandedSection, toggleSection } from '../stores/accordionStore';

  let llmState: LLMState = $state(LLMService.getState());
  let configState: ConfigState = $state(ConfigService.getState());
  let healthState: HealthMonitorState = $state(HealthMonitorService.getState());
  let unsubscribeLLM: (() => void) | null = null;
  let unsubscribeConfig: (() => void) | null = null;
  let unsubscribeHealth: (() => void) | null = null;
  let unsubscribeEvents: (() => void) | null = null;

  let connectionType: 'external' | 'sidecar' = $state('sidecar');
  let externalUrl = $state('http://localhost:1234');
  let apiKey = $state('');
  let isConnecting = $state(false);
  let showHealthDetails = $state(false);

  onMount(async () => {
    unsubscribeLLM = LLMService.subscribe((state) => {
      llmState = state;
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
      if (event.type === 'server_crashed') {
        console.warn('[ServerStatus] Server crashed:', event.error);
      }
    });

    try {
      await ConfigService.loadConfig();
      await ConfigService.refreshServerMode();
    } catch {
      // Errors are surfaced by the services themselves.
    }

    if (llmState.status.ready) {
      try {
        await HealthMonitorService.start();
      } catch {
        // Monitoring start failures are non-critical here.
      }
    }
  });

  onDestroy(() => {
    unsubscribeLLM?.();
    unsubscribeConfig?.();
    unsubscribeHealth?.();
    unsubscribeEvents?.();
  });

  async function connectExternal() {
    if (!externalUrl.trim()) {
      return;
    }

    isConnecting = true;
    try {
      await LLMService.connectToServer(externalUrl);
      await ConfigService.saveConfig({
        ...configState.config,
        external_url: externalUrl,
        api_key: apiKey || null,
        connection_mode: { type: 'External', url: externalUrl },
      });
      await HealthMonitorService.start();
    } catch (cause) {
      console.error('Failed to connect:', cause);
    } finally {
      isConnecting = false;
    }
  }

  async function disconnectExternal() {
    await HealthMonitorService.stop();
    await LLMService.stop();
    await ConfigService.refreshServerMode();
  }

  async function triggerRecovery() {
    try {
      await HealthMonitorService.triggerRecovery();
    } catch (cause) {
      console.error('Recovery failed:', cause);
    }
  }

  async function checkHealthNow() {
    await HealthMonitorService.checkNow();
  }

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

  let modeText = $derived.by(() => {
    switch (configState.serverMode.mode) {
      case 'external':
        return 'External';
      case 'sidecar_inference':
        return 'Sidecar (VLM)';
      case 'sidecar_embedding':
        return 'Sidecar (Embedding)';
      case 'sidecar_reranking':
        return 'Sidecar (Reranker)';
      default:
        return 'None';
    }
  });
</script>

<div class="space-y-3">
  <button
    type="button"
    class="w-full flex items-center justify-between text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-400 transition-colors"
    onclick={() => toggleSection('server')}
  >
    <div class="flex items-center gap-2">
      <span class={`h-2 w-2 rounded-full ${statusColor}`}></span>
      <span>LLM Server</span>
      {#if llmState.status.ready}
        <span class="normal-case text-green-400">({modeText})</span>
      {/if}
    </div>
    <svg
      class={`h-3 w-3 transform transition-transform ${$expandedSection === 'server' ? 'rotate-180' : ''}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  {#if $expandedSection === 'server'}
    <div class="space-y-3 rounded-lg bg-neutral-800/30 p-3">
      <div class="flex gap-1 rounded bg-neutral-900 p-1">
        <button
          type="button"
          class={`flex-1 rounded px-2 py-1 text-xs transition-colors ${
            connectionType === 'external'
              ? 'bg-neutral-700 text-neutral-200'
              : 'text-neutral-500 hover:text-neutral-400'
          }`}
          onclick={() => (connectionType = 'external')}
        >
          External
        </button>
        <button
          type="button"
          class={`flex-1 rounded px-2 py-1 text-xs transition-colors ${
            connectionType === 'sidecar'
              ? 'bg-neutral-700 text-neutral-200'
              : 'text-neutral-500 hover:text-neutral-400'
          }`}
          onclick={() => (connectionType = 'sidecar')}
        >
          Sidecar
        </button>
      </div>

      {#if connectionType === 'external'}
        <ExternalConnectionPanel
          bind:externalUrl
          bind:apiKey
          {isConnecting}
          isConnected={llmState.status.ready && llmState.status.mode === 'external'}
          onConnect={connectExternal}
          onDisconnect={disconnectExternal}
        />
      {:else}
        <div class="space-y-3">
          <BackendSelector />
          <ManagedRuntimePanel />
        </div>
      {/if}

      {#if llmState.status.ready}
        <div class="flex items-center justify-between border-t border-neutral-700 pt-2 text-xs">
          <span class="text-neutral-500">Status: {statusText}</span>
          {#if llmState.status.url}
            <span class="font-mono text-[10px] text-neutral-600">{llmState.status.url}</span>
          {/if}
        </div>

        <RuntimeSnapshotGrid
          activeRuntime={llmState.status.active_runtime}
          activeModelTarget={llmState.status.active_model_target}
          embeddingRuntime={llmState.status.embedding_runtime}
          embeddingModelTarget={llmState.status.embedding_model_target}
          fallbackActiveRuntimeId={llmState.status.backend_name}
        />

        <HealthStatusPanel
          {healthState}
          bind:showHealthDetails
          onCheckNow={checkHealthNow}
          onTriggerRecovery={triggerRecovery}
        />
      {/if}

      {#if llmState.error || configState.error || healthState.error}
        <div class="rounded border border-red-800/50 bg-red-900/20 p-2 text-xs text-red-400">
          {llmState.error || configState.error || healthState.error}
        </div>
      {/if}
    </div>
  {:else}
    <div class="flex items-center gap-2 text-xs text-neutral-500">
      {#if llmState.status.ready}
        <span class="text-green-400">{modeText}</span>
        {#if llmState.status.url}
          <span class="truncate font-mono text-[10px] text-neutral-600">{llmState.status.url}</span>
        {/if}
      {:else}
        <span>Not connected</span>
      {/if}
    </div>
  {/if}
</div>

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
  import ConnectionModeTabs from './server-status/ConnectionModeTabs.svelte';
  import ExternalConnectionPanel from './server-status/ExternalConnectionPanel.svelte';
  import HealthStatusPanel from './server-status/HealthStatusPanel.svelte';
  import RuntimeSnapshotGrid from './server-status/RuntimeSnapshotGrid.svelte';
  import { expandedSection, toggleSection } from '../stores/accordionStore';
  import { StatusGlyph } from './workbench/ui';
  import { serverConnectionGlyph } from './server-status/serverStatusPresenters';

  let { showManagedRuntimePanel = true }: { showManagedRuntimePanel?: boolean } = $props();

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
  let configReady = $state(false);
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
      configReady = ConfigService.getState().error === null;
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
    if (!configReady || configState.isLoading || !externalUrl.trim()) {
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

  let statusGlyph = $derived(
    serverConnectionGlyph(
      llmState.status.ready,
      healthState.lastResult?.healthy,
      llmState.status.mode,
    )
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

<section class="pg-card" aria-labelledby="settings-server-title">
  <div class="flex items-center gap-2">
    <h3 class="m-0 min-w-0 flex-1">
      <button
        type="button"
        id="settings-server-title"
        class="flex w-full items-center gap-2 text-left text-[13px] font-bold text-pg-text-bright"
        aria-expanded={$expandedSection === 'server'}
        onclick={() => toggleSection('server')}
      >
        LLM server
        <span class="pg-muted text-[11px] font-normal" aria-hidden="true">
          {$expandedSection === 'server' ? '▴' : '▾'}
        </span>
      </button>
    </h3>
    <span class="flex items-center gap-1.5 pg-small text-pg-text-soft">
      <StatusGlyph state={statusGlyph} small />
      {statusText}
    </span>
  </div>

  {#if $expandedSection === 'server'}
    <div class="mt-3 flex flex-col gap-3 border-t border-pg-border-soft pt-3">
      <ConnectionModeTabs bind:connectionType />

      {#if connectionType === 'external'}
        <ExternalConnectionPanel
          bind:externalUrl
          bind:apiKey
          {isConnecting}
          canConnect={configReady && !configState.isLoading}
          isConnected={llmState.status.ready && llmState.status.mode === 'external'}
          onConnect={connectExternal}
          onDisconnect={disconnectExternal}
        />
      {:else}
        <div class="flex flex-col gap-3">
          <BackendSelector />
          {#if showManagedRuntimePanel}
            <ManagedRuntimePanel />
          {/if}
        </div>
      {/if}

      {#if llmState.status.ready}
        <p class="m-0 flex flex-wrap items-baseline justify-between gap-2 pg-small">
          <span class="pg-muted">Mode {modeText}</span>
          {#if llmState.status.url}
            <span class="pg-mono pg-muted break-all">{llmState.status.url}</span>
          {/if}
        </p>

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
        <p class="m-0 pg-muted pg-small" role="alert">
          {llmState.error || configState.error || healthState.error}
        </p>
      {/if}
    </div>
  {:else}
    <p class="m-0 mt-1 pg-muted pg-small truncate">
      {#if llmState.status.ready}
        {modeText}
        {#if llmState.status.url}
          <span class="pg-mono">{llmState.status.url}</span>
        {/if}
      {:else}
        Not connected
      {/if}
    </p>
  {/if}
</section>

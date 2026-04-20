<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { ConfigService, type ServerModeInfo } from '../services/ConfigService';
  import { LLMService } from '../services/LLMService';
  import {
    managedRuntimeService,
    type ManagedRuntimeManagerRuntimeView,
  } from '../services/managedRuntime';

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
    default_start_mode: 'inference' | 'embedding';
    description: string;
    capabilities: BackendCapabilities;
    active: boolean;
    available: boolean;
    unavailable_reason: string | null;
    can_install: boolean;
    runtime_binary_id: 'llama_cpp' | 'ollama' | null;
  }

  let backends: BackendInfo[] = $state([]);
  let runtimes: ManagedRuntimeManagerRuntimeView[] = $state([]);
  let currentBackendKey = $state('');
  let isLoading = $state(false);
  let isSwitching = $state(false);
  let error: string | null = $state(null);
  let serverRunning = $state(false);
  let unsubscribe: (() => void) | null = null;
  let managedRuntimeUnsubscribe: (() => void) | null = null;

  const projectManagedRuntimeBackends = (
    nextBackends: BackendInfo[],
    runtimeViews: ManagedRuntimeManagerRuntimeView[],
  ): BackendInfo[] =>
    nextBackends.map((backend) => {
      if (!backend.runtime_binary_id) {
        return backend;
      }

      const runtime = runtimeViews.find(
        (candidate) => candidate.id === backend.runtime_binary_id
      );
      if (!runtime) {
        return backend;
      }

      return {
        ...backend,
        available: runtime.available,
        can_install: runtime.can_install,
        unavailable_reason: runtime.unavailable_reason,
      };
    });

  async function loadBackends() {
    isLoading = true;
    error = null;

    try {
      const runtimeViews = await managedRuntimeService.listRuntimes();
      const backendViews = await invoke<BackendInfo[]>('list_backends');
      runtimes = runtimeViews;
      backends = projectManagedRuntimeBackends(backendViews, runtimeViews);

      const status = await LLMService.refreshStatus();
      currentBackendKey = status.backend_key || '';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
      console.error('Failed to load backends:', cause);
    } finally {
      isLoading = false;
    }
  }

  async function switchBackend(backend: BackendInfo) {
    if (backend.backend_key === currentBackendKey || isSwitching || !backend.available) {
      return;
    }

    isSwitching = true;
    error = null;

    try {
      const status = await invoke<ServerModeInfo>('switch_backend', {
        backendName: backend.backend_key,
      });
      currentBackendKey = status.backend_key || backend.backend_key;

      try {
        const started =
          backend.default_start_mode === 'embedding'
            ? await ConfigService.startEmbeddingMode()
            : await ConfigService.startInferenceMode();
        currentBackendKey = started.backend_key || currentBackendKey;
        await LLMService.refreshStatus();
      } catch (cause) {
        error = `Auto-start failed: ${String(cause)}`;
        console.warn('Auto-start failed:', cause);
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
      console.error('Failed to switch backend:', cause);
    } finally {
      isSwitching = false;
    }
  }

  async function stopServer() {
    isSwitching = true;
    error = null;

    try {
      const status = await LLMService.stop();
      currentBackendKey = status.backend_key || '';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
      console.error('Failed to stop server:', cause);
    } finally {
      isSwitching = false;
    }
  }

  function runtimeForBackend(
    backend: BackendInfo
  ): ManagedRuntimeManagerRuntimeView | undefined {
    if (!backend.runtime_binary_id) {
      return undefined;
    }

    return runtimes.find((runtime) => runtime.id === backend.runtime_binary_id);
  }

  function runtimeSummary(
    backend: BackendInfo,
    runtime: ManagedRuntimeManagerRuntimeView | undefined
  ): string | null {
    if (!runtime) {
      return backend.unavailable_reason;
    }

    if (runtime.available && runtime.selection.selected_version) {
      return `Selected ${runtime.selection.selected_version}`;
    }

    if (runtime.available && runtime.selection.active_version) {
      return `Active ${runtime.selection.active_version}`;
    }

    if (runtime.active_job) {
      return runtime.active_job.status;
    }

    if (backend.can_install) {
      return 'Install or update it in Runtime Manager below.';
    }

    return runtime.unavailable_reason ?? backend.unavailable_reason;
  }

  function handleBackendClick(backend: BackendInfo) {
    if (backend.backend_key === currentBackendKey && serverRunning) {
      void stopServer();
      return;
    }

    if (backend.available) {
      void switchBackend(backend);
    }
  }

  onMount(() => {
    managedRuntimeUnsubscribe = managedRuntimeService.subscribe((nextRuntimes) => {
      runtimes = nextRuntimes;
      backends = projectManagedRuntimeBackends(backends, nextRuntimes);
    });

    unsubscribe = LLMService.subscribe((state) => {
      serverRunning = state.status.ready;
    });

    void loadBackends();
  });

  onDestroy(() => {
    unsubscribe?.();
    managedRuntimeUnsubscribe?.();
  });

  let activeBackend = $derived(
    serverRunning ? backends.find((backend) => backend.backend_key === currentBackendKey) : null
  );
</script>

<div class="space-y-2">
  <div class="text-[10px] uppercase tracking-wider text-neutral-600">
    Built-in inference engine
  </div>

  {#if isLoading}
    <div class="flex items-center gap-2 text-xs text-neutral-500">
      <div class="h-3 w-3 animate-spin rounded-full border border-neutral-500 border-t-transparent"></div>
      <span>Loading backends...</span>
    </div>
  {:else if backends.length === 0}
    <div class="text-xs text-neutral-500">No backends available</div>
  {:else}
    <div class="flex flex-wrap gap-2">
      {#each backends as backend (backend.name)}
        {@const runtime = runtimeForBackend(backend)}
        {@const summary = runtimeSummary(backend, runtime)}
        <div class="flex max-w-[190px] flex-col">
          <button
            type="button"
            class={`flex items-center gap-1.5 rounded px-3 py-1.5 text-xs transition-colors ${
              backend.backend_key === currentBackendKey && serverRunning
                ? 'bg-blue-600 text-white'
                : backend.available
                  ? 'bg-neutral-700 text-neutral-300 hover:bg-neutral-600'
                  : 'bg-neutral-800 text-neutral-500'
            }`}
            onclick={() => handleBackendClick(backend)}
            disabled={!backend.available || isSwitching}
            title={backend.description}
          >
            {#if isSwitching && backend.backend_key === currentBackendKey}
              <span class="inline-block h-3 w-3 animate-spin rounded-full border border-white border-t-transparent"></span>
            {/if}
            {backend.name}
          </button>

          {#if summary}
            <span class={`mt-1 text-[10px] leading-tight ${
              backend.available ? 'text-neutral-500' : 'text-amber-400'
            }`}>
              {summary}
            </span>
          {/if}
        </div>
      {/each}
    </div>

    {#if activeBackend}
      <div class="text-[10px] text-neutral-500">{activeBackend.description}</div>

      <div class="flex flex-wrap gap-1.5">
        {#if activeBackend.capabilities.vision}
          <span class="rounded bg-green-900/30 px-1.5 py-0.5 text-[10px] text-green-400">
            Vision
          </span>
        {/if}
        {#if activeBackend.capabilities.embeddings}
          <span class="rounded bg-purple-900/30 px-1.5 py-0.5 text-[10px] text-purple-400">
            Embeddings
          </span>
        {/if}
        {#if activeBackend.capabilities.gpu}
          <span class="rounded bg-amber-900/30 px-1.5 py-0.5 text-[10px] text-amber-400">
            GPU
          </span>
        {/if}
        {#if activeBackend.capabilities.device_selection}
          <span class="rounded bg-cyan-900/30 px-1.5 py-0.5 text-[10px] text-cyan-400">
            Device Select
          </span>
        {/if}
        {#if activeBackend.capabilities.streaming}
          <span class="rounded bg-blue-900/30 px-1.5 py-0.5 text-[10px] text-blue-400">
            Streaming
          </span>
        {/if}
        {#if activeBackend.capabilities.tool_calling}
          <span class="rounded bg-pink-900/30 px-1.5 py-0.5 text-[10px] text-pink-400">
            Tools
          </span>
        {/if}
      </div>
    {/if}
  {/if}

  {#if error}
    <div class="rounded border border-red-800/50 bg-red-900/20 p-2 text-xs text-red-400">
      {error}
    </div>
  {/if}

  <button
    type="button"
    class="text-[10px] text-neutral-600 transition-colors hover:text-neutral-400 disabled:opacity-50"
    onclick={loadBackends}
    disabled={isLoading}
  >
    Refresh backends
  </button>
</div>

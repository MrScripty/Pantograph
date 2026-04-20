<script lang="ts">
  type BackendCapabilities = {
    vision: boolean;
    embeddings: boolean;
    gpu: boolean;
    device_selection: boolean;
    streaming: boolean;
    tool_calling: boolean;
  };

  type BackendInfo = {
    name: string;
    backend_key: string;
    description: string;
    capabilities: BackendCapabilities;
    available: boolean;
  };

  type Props = {
    backends: BackendInfo[];
    currentBackendKey: string;
    serverRunning: boolean;
    isSwitching: boolean;
    onSelectBackend: (backend: BackendInfo) => void;
    summaryForBackend: (backend: BackendInfo) => string | null;
  };

  let {
    backends,
    currentBackendKey,
    serverRunning,
    isSwitching,
    onSelectBackend,
    summaryForBackend,
  }: Props = $props();
</script>

<div class="flex flex-wrap gap-2">
  {#each backends as backend (backend.name)}
    {@const summary = summaryForBackend(backend)}
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
        onclick={() => onSelectBackend(backend)}
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

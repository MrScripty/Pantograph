<script lang="ts">
  import { StatusGlyph } from '../workbench/ui';

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
    {@const current = backend.backend_key === currentBackendKey && serverRunning}
    <div class="flex max-w-[190px] flex-col">
      <button
        type="button"
        class="pg-tgl"
        aria-pressed={current}
        onclick={() => onSelectBackend(backend)}
        disabled={!backend.available || isSwitching}
        title={backend.description}
      >
        {backend.name}
        {#if isSwitching && backend.backend_key === currentBackendKey}
          <span class="pg-small">switching</span>
        {/if}
      </button>

      {#if summary}
        <span class="mt-1 flex items-start gap-1.5 text-[11.5px] leading-tight text-pg-text-muted">
          {#if !backend.available}<StatusGlyph state="warn" small class="mt-0.5" />{/if}
          {summary}
        </span>
      {/if}
    </div>
  {/each}
</div>

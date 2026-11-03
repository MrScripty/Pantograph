<script lang="ts">
  import type {
    ManagedRuntimeInstallHistoryEntry,
    ManagedRuntimeManagerRuntimeView,
  } from '../../services/managedRuntime';

  type Props = {
    runtime: ManagedRuntimeManagerRuntimeView;
    recentHistory: ManagedRuntimeInstallHistoryEntry[];
    installActionLabel: string;
    canInstallRuntime: boolean;
    installRequested: boolean;
    removeRequested: boolean;
    onInstallRuntime: () => Promise<void>;
    onRemoveRuntime: () => Promise<void>;
    formatHistoryEvent: (event: string) => string;
    formatHistoryTime: (atMs: number) => string;
  };

  let {
    runtime,
    recentHistory,
    installActionLabel,
    canInstallRuntime,
    installRequested,
    removeRequested,
    onInstallRuntime,
    onRemoveRuntime,
    formatHistoryEvent,
    formatHistoryTime,
  }: Props = $props();
</script>

<div class="flex min-w-0 flex-col gap-3">
  <div>
    <h5 class="m-0 text-[12.5px] font-bold text-pg-text-soft">Install history</h5>
    {#if recentHistory.length > 0}
      <ul class="m-0 mt-2 flex list-none flex-col gap-2 p-0">
        {#each recentHistory as entry (`${entry.event}-${entry.at_ms}`)}
          <li class="rounded border border-pg-border-soft bg-pg-panel-sunken px-3 py-2 pg-small">
            <div class="flex items-baseline justify-between gap-2">
              <span>{formatHistoryEvent(entry.event)}</span>
              <span class="pg-mono pg-muted">{formatHistoryTime(entry.at_ms)}</span>
            </div>
            {#if entry.version}
              <div class="mt-1 pg-muted">Version <span class="pg-mono">{entry.version}</span></div>
            {/if}
            {#if entry.detail}
              <div class="mt-1 pg-muted">{entry.detail}</div>
            {/if}
          </li>
        {/each}
      </ul>
    {:else}
      <p class="m-0 mt-2 pg-muted pg-small">No install history recorded yet.</p>
    {/if}
  </div>

  {#if runtime.missing_files.length > 0}
    <details class="rounded border border-pg-border-soft bg-pg-panel-sunken px-3 py-2 pg-small">
      <summary class="cursor-pointer text-pg-text-soft hover:text-pg-text-bright">
        {runtime.missing_files.length} missing file{runtime.missing_files.length === 1 ? '' : 's'}
      </summary>
      <ul class="m-0 mt-2 list-disc pl-4">
        {#each runtime.missing_files as file (file)}
          <li class="break-all pg-mono pg-muted">{file}</li>
        {/each}
      </ul>
    </details>
  {/if}

  <div class="flex flex-wrap gap-2">
    {#if canInstallRuntime && !runtime.active_job}
      <button
        type="button"
        class="pg-btn pg-btn-primary"
        onclick={() => onInstallRuntime()}
        disabled={installRequested || removeRequested}
      >
        {installRequested ? 'Starting...' : installActionLabel}
      </button>
    {/if}

    {#if runtime.can_remove && !runtime.active_job}
      <button
        type="button"
        class="pg-btn pg-btn-danger"
        onclick={onRemoveRuntime}
        disabled={removeRequested || installRequested}
      >
        {removeRequested ? 'Removing...' : 'Remove runtime'}
      </button>
    {/if}
  </div>
</div>

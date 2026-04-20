<script lang="ts">
  import type {
    ManagedRuntimeInstallHistoryEntry,
    ManagedRuntimeManagerRuntimeView,
  } from '../../services/managedRuntime';

  type Props = {
    runtime: ManagedRuntimeManagerRuntimeView;
    recentHistory: ManagedRuntimeInstallHistoryEntry[];
    installActionLabel: string;
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
    installRequested,
    removeRequested,
    onInstallRuntime,
    onRemoveRuntime,
    formatHistoryEvent,
    formatHistoryTime,
  }: Props = $props();
</script>

<div class="space-y-3">
  <div>
    <h5 class="text-xs uppercase tracking-wider text-neutral-500">Install History</h5>
    {#if recentHistory.length > 0}
      <ul class="mt-2 space-y-2">
        {#each recentHistory as entry (`${entry.event}-${entry.at_ms}`)}
          <li class="rounded border border-neutral-800 bg-neutral-950/60 p-2 text-xs text-neutral-300">
            <div class="flex items-center justify-between gap-2">
              <span>{formatHistoryEvent(entry.event)}</span>
              <span class="text-[11px] text-neutral-500">{formatHistoryTime(entry.at_ms)}</span>
            </div>
            {#if entry.version}
              <div class="mt-1 text-[11px] text-neutral-500">Version {entry.version}</div>
            {/if}
            {#if entry.detail}
              <div class="mt-1 text-[11px] text-neutral-600">{entry.detail}</div>
            {/if}
          </li>
        {/each}
      </ul>
    {:else}
      <p class="mt-2 text-xs text-neutral-500">No install history recorded yet.</p>
    {/if}
  </div>

  {#if runtime.missing_files.length > 0}
    <details class="rounded border border-neutral-800 bg-neutral-950/60 p-2 text-xs text-neutral-400">
      <summary class="cursor-pointer hover:text-neutral-300">
        {runtime.missing_files.length} missing file{runtime.missing_files.length === 1 ? '' : 's'}
      </summary>
      <ul class="mt-2 list-disc pl-4">
        {#each runtime.missing_files as file (file)}
          <li class="break-all font-mono text-[11px] text-neutral-500">{file}</li>
        {/each}
      </ul>
    </details>
  {/if}

  <div class="flex flex-wrap gap-2">
    {#if runtime.can_install && !runtime.active_job}
      <button
        type="button"
        class="rounded bg-blue-600 px-3 py-1.5 text-xs text-white transition-colors hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500"
        onclick={onInstallRuntime}
        disabled={installRequested || removeRequested}
      >
        {installRequested ? 'Starting...' : installActionLabel}
      </button>
    {/if}

    {#if runtime.can_remove && !runtime.active_job}
      <button
        type="button"
        class="rounded border border-neutral-700 px-3 py-1.5 text-xs text-neutral-300 transition-colors hover:bg-neutral-800 disabled:border-neutral-800 disabled:text-neutral-600"
        onclick={onRemoveRuntime}
        disabled={removeRequested || installRequested}
      >
        {removeRequested ? 'Removing...' : 'Remove runtime'}
      </button>
    {/if}
  </div>
</div>

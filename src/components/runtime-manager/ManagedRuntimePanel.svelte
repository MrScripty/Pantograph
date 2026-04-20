<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    managedRuntimeService,
    type ManagedRuntimeManagerRuntimeView,
  } from '../../services/managedRuntime';
  import ManagedRuntimeCard from './ManagedRuntimeCard.svelte';

  let runtimes: ManagedRuntimeManagerRuntimeView[] = $state([]);
  let isLoading = $state(false);
  let error: string | null = $state(null);
  let unsubscribe: (() => void) | null = null;

  async function loadRuntimes() {
    isLoading = true;
    error = null;

    try {
      runtimes = await managedRuntimeService.listRuntimes();
      runtimes = await managedRuntimeService.refreshCatalogs();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    unsubscribe = managedRuntimeService.subscribe((nextRuntimes) => {
      runtimes = nextRuntimes;
    });
    void loadRuntimes();
  });

  onDestroy(() => {
    unsubscribe?.();
  });
</script>

<div class="space-y-3">
  <div class="flex items-center justify-between gap-3">
    <div>
      <div class="text-[10px] uppercase tracking-wider text-neutral-600">
        Runtime Manager
      </div>
      <p class="mt-1 text-xs text-neutral-500">
        Install, select, and inspect redistributable sidecar runtimes without moving version policy out of the backend.
      </p>
    </div>
    <button
      type="button"
      class="rounded border border-neutral-700 px-2 py-1 text-[10px] text-neutral-400 transition-colors hover:bg-neutral-800 hover:text-neutral-200 disabled:border-neutral-800 disabled:text-neutral-600"
      onclick={loadRuntimes}
      disabled={isLoading}
    >
      {isLoading ? 'Refreshing...' : 'Refresh runtimes'}
    </button>
  </div>

  {#if error}
    <div class="rounded border border-red-800/50 bg-red-950/20 p-2 text-xs text-red-300">
      {error}
    </div>
  {/if}

  {#if isLoading && runtimes.length === 0}
    <div class="flex items-center gap-2 text-xs text-neutral-500">
      <div class="h-3 w-3 animate-spin rounded-full border border-neutral-500 border-t-transparent"></div>
      <span>Loading managed runtimes...</span>
    </div>
  {:else if runtimes.length === 0}
    <div class="rounded border border-neutral-800 bg-neutral-900/40 p-3 text-xs text-neutral-500">
      No managed runtimes are registered for this build.
    </div>
  {:else}
    <div class="space-y-3">
      {#each runtimes as runtime (runtime.id)}
        <ManagedRuntimeCard {runtime} />
      {/each}
    </div>
  {/if}
</div>

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

<section class="flex flex-col gap-3" aria-labelledby="managed-runtime-manager-title">
  <div class="flex flex-wrap items-start justify-between gap-3">
    <div class="min-w-0 flex-1">
      <h3 id="managed-runtime-manager-title" class="m-0 text-[13px] font-bold text-pg-text-bright">
        Runtime manager
      </h3>
      <p class="m-0 mt-0.5 pg-muted pg-small">
        Install, select, and inspect redistributable sidecar runtimes without moving version policy out of the backend.
      </p>
    </div>
    <button type="button" class="pg-btn" onclick={loadRuntimes} disabled={isLoading}>
      {isLoading ? 'Refreshing...' : 'Refresh runtimes'}
    </button>
  </div>

  {#if error}
    <p class="m-0 pg-muted pg-small" role="alert">{error}</p>
  {/if}

  {#if isLoading && runtimes.length === 0}
    <p class="m-0 pg-muted pg-small" role="status">Loading managed runtimes...</p>
  {:else if runtimes.length === 0}
    <p class="m-0 pg-muted pg-small">No managed runtimes are registered for this build.</p>
  {:else}
    <div class="flex flex-col gap-3">
      {#each runtimes as runtime (runtime.id)}
        <ManagedRuntimeCard {runtime} />
      {/each}
    </div>
  {/if}
</section>

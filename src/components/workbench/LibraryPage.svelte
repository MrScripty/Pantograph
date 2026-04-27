<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw } from 'lucide-svelte';
  import type {
    LibraryUsageProjectionRecord,
    ProjectionStateRecord,
  } from '../../services/diagnostics/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
  import {
    formatLibraryAssetCategory,
    formatLibraryBytes,
    formatLibraryProjectionFreshness,
    isLibraryAssetLastUsedByRun,
  } from './libraryUsagePresenters';

  let assets = $state<LibraryUsageProjectionRecord[]>([]);
  let projectionState = $state<ProjectionStateRecord | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let requestSerial = 0;

  function formatTimestamp(value: number): string {
    return new Date(value).toLocaleString();
  }

  async function refreshLibraryUsage(): Promise<void> {
    const serial = ++requestSerial;
    loading = assets.length === 0;
    error = null;
    try {
      const response = await workflowService.queryLibraryUsage({ limit: 250 });
      if (serial !== requestSerial) {
        return;
      }
      assets = response.assets;
      projectionState = response.projection_state;
    } catch (usageError) {
      if (serial !== requestSerial) {
        return;
      }
      error = usageError instanceof Error ? usageError.message : String(usageError);
    } finally {
      if (serial === requestSerial) {
        loading = false;
      }
    }
  }

  onMount(() => {
    void refreshLibraryUsage();
  });
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  <div class="flex shrink-0 items-center justify-between border-b border-neutral-800 px-4 py-3">
    <div class="min-w-0">
      <h1 class="text-base font-semibold text-neutral-100">Library</h1>
      <div class="mt-1 truncate text-xs text-neutral-500">
        {#if $activeWorkflowRun}
          {$activeWorkflowRun.workflow_run_id}
        {:else}
          No active run selected
        {/if}
      </div>
    </div>
    <button
      type="button"
      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-sm text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
      onclick={() => refreshLibraryUsage()}
      disabled={loading}
    >
      <RefreshCw size={14} aria-hidden="true" class={loading ? 'animate-spin' : ''} />
      Refresh
    </button>
  </div>

  <div class="border-b border-neutral-900 px-4 py-3 text-xs text-neutral-500">
    {formatLibraryProjectionFreshness(projectionState)}
  </div>

  {#if error}
    <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{error}</div>
  {/if}

  <div class="min-h-0 flex-1 overflow-auto">
    <table class="w-full min-w-[72rem] border-collapse text-left text-sm">
      <thead class="sticky top-0 z-10 bg-neutral-950 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
        <tr class="border-b border-neutral-800">
          <th class="px-4 py-3 font-medium">Asset</th>
          <th class="px-3 py-3 font-medium">Category</th>
          <th class="px-3 py-3 font-medium">Operation</th>
          <th class="px-3 py-3 font-medium">Accesses</th>
          <th class="px-3 py-3 font-medium">Runs</th>
          <th class="px-3 py-3 font-medium">Network</th>
          <th class="px-3 py-3 font-medium">Last Workflow</th>
          <th class="px-3 py-3 font-medium">Last Run</th>
          <th class="px-4 py-3 font-medium">Last Accessed</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-neutral-900">
        {#if loading}
          <tr>
            <td colspan="9" class="px-4 py-8 text-center text-neutral-500">Loading Library usage</td>
          </tr>
        {:else if assets.length === 0}
          <tr>
            <td colspan="9" class="px-4 py-8 text-center text-neutral-500">No Library usage recorded</td>
          </tr>
        {:else}
          {#each assets as asset (asset.asset_id)}
            {@const activeRunMatch = isLibraryAssetLastUsedByRun(asset, $activeWorkflowRun?.workflow_run_id)}
            <tr
              class:bg-cyan-950={activeRunMatch}
              class:bg-opacity-30={activeRunMatch}
              class="hover:bg-neutral-900/70"
            >
              <td class="max-w-[20rem] px-4 py-2">
                <div class="truncate font-mono text-xs text-neutral-100" title={asset.asset_id}>
                  {asset.asset_id}
                </div>
                {#if activeRunMatch}
                  <div class="mt-1 text-[11px] text-cyan-200">Last used by active run</div>
                {/if}
              </td>
              <td class="px-3 py-2 text-xs text-neutral-300">
                {formatLibraryAssetCategory(asset.asset_id)}
              </td>
              <td class="px-3 py-2 text-xs text-neutral-300">{asset.last_operation}</td>
              <td class="px-3 py-2 text-xs text-neutral-300">{asset.total_access_count}</td>
              <td class="px-3 py-2 text-xs text-neutral-300">{asset.run_access_count}</td>
              <td class="px-3 py-2 text-xs text-neutral-300">{formatLibraryBytes(asset.total_network_bytes)}</td>
              <td class="max-w-[14rem] truncate px-3 py-2 text-xs text-neutral-400" title={asset.last_workflow_id ?? ''}>
                {asset.last_workflow_id ?? 'Unavailable'}
              </td>
              <td class="max-w-[14rem] truncate px-3 py-2 text-xs text-neutral-400" title={asset.last_workflow_run_id ?? ''}>
                {asset.last_workflow_run_id ?? 'Unavailable'}
              </td>
              <td class="px-4 py-2 text-xs text-neutral-400">{formatTimestamp(asset.last_accessed_at_ms)}</td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</section>

<script lang="ts">
  import { onMount } from 'svelte';
  import { Download, RefreshCw, Search, Trash2 } from 'lucide-svelte';
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
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

  let assets = $state<LibraryUsageProjectionRecord[]>([]);
  let projectionState = $state<ProjectionStateRecord | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let actionMessage = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let actionBusy = $state<string | null>(null);
  let hfSearchQuery = $state('');
  let hfSearchKind = $state('');
  let hfSearchLimit = $state(25);
  let hfSearchResults = $state<unknown[]>([]);
  let downloadRepoId = $state('');
  let downloadFamily = $state('model');
  let downloadOfficialName = $state('');
  let downloadModelType = $state('');
  let deleteModelId = $state('');
  let requestSerial = 0;

  function formatTimestamp(value: number): string {
    return new Date(value).toLocaleString();
  }

  function modelLabel(model: unknown): string {
    if (typeof model === 'object' && model !== null && 'id' in model) {
      const id = (model as { id?: unknown }).id;
      if (typeof id === 'string' && id.trim().length > 0) {
        return id;
      }
    }
    return 'Unknown model';
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
      error = formatWorkflowCommandError(usageError);
    } finally {
      if (serial === requestSerial) {
        loading = false;
      }
    }
  }

  async function searchHfModels(): Promise<void> {
    actionBusy = 'search';
    actionError = null;
    actionMessage = null;
    try {
      const response = await workflowService.searchHfModelsWithAudit({
        query: hfSearchQuery,
        kind: hfSearchKind.trim().length > 0 ? hfSearchKind : null,
        limit: hfSearchLimit,
        hydrateLimit: Math.min(hfSearchLimit, 10),
      });
      hfSearchResults = response.models;
      actionMessage = `Search recorded${response.auditEventSeq ? ` at event ${response.auditEventSeq}` : ''}`;
      await refreshLibraryUsage();
    } catch (searchError) {
      actionError = formatWorkflowCommandError(searchError);
    } finally {
      actionBusy = null;
    }
  }

  async function startHfDownload(): Promise<void> {
    actionBusy = 'download';
    actionError = null;
    actionMessage = null;
    try {
      const repoId = downloadRepoId.trim();
      const response = await workflowService.startHfDownloadWithAudit({
        repo_id: repoId,
        family: downloadFamily.trim() || 'model',
        official_name: downloadOfficialName.trim() || repoId,
        model_type: downloadModelType.trim().length > 0 ? downloadModelType.trim() : null,
      });
      actionMessage = `Download ${response.downloadId}${response.auditEventSeq ? ` recorded at event ${response.auditEventSeq}` : ''}`;
      await refreshLibraryUsage();
    } catch (downloadError) {
      actionError = formatWorkflowCommandError(downloadError);
    } finally {
      actionBusy = null;
    }
  }

  async function deletePumasModel(): Promise<void> {
    actionBusy = 'delete';
    actionError = null;
    actionMessage = null;
    try {
      const response = await workflowService.deletePumasModelWithAudit(deleteModelId);
      actionMessage = response.success
        ? `Delete recorded${response.auditEventSeq ? ` at event ${response.auditEventSeq}` : ''}`
        : (response.error ?? 'Delete did not complete');
      await refreshLibraryUsage();
    } catch (deleteError) {
      actionError = formatWorkflowCommandError(deleteError);
    } finally {
      actionBusy = null;
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

  <div class="grid shrink-0 gap-3 border-b border-neutral-900 px-4 py-3 lg:grid-cols-[1.25fr_1fr_1fr]">
    <div class="grid gap-2">
      <div class="grid grid-cols-[minmax(0,1fr)_2.5rem] gap-2 sm:grid-cols-[minmax(0,1fr)_9rem_5rem_2.5rem]">
        <input
          class="h-9 min-w-0 rounded border border-neutral-800 bg-neutral-900 px-3 text-sm text-neutral-100 outline-none focus:border-cyan-500"
          placeholder="HuggingFace query"
          aria-label="HuggingFace search query"
          bind:value={hfSearchQuery}
        />
        <input
          class="h-9 min-w-0 rounded border border-neutral-800 bg-neutral-900 px-3 text-sm text-neutral-100 outline-none focus:border-cyan-500"
          placeholder="kind"
          aria-label="HuggingFace search kind"
          bind:value={hfSearchKind}
        />
        <input
          class="h-9 min-w-0 rounded border border-neutral-800 bg-neutral-900 px-2 text-sm text-neutral-100 outline-none focus:border-cyan-500"
          type="number"
          min="1"
          max="100"
          aria-label="HuggingFace search result limit"
          bind:value={hfSearchLimit}
        />
        <button
          type="button"
          title="Search HuggingFace"
          aria-label="Search HuggingFace"
          class="inline-flex h-9 items-center justify-center rounded border border-neutral-700 text-neutral-300 transition-colors hover:border-cyan-500 hover:text-cyan-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
          onclick={() => void searchHfModels()}
          disabled={actionBusy !== null}
        >
          <Search size={16} aria-hidden="true" />
        </button>
      </div>
      {#if hfSearchResults.length > 0}
        <div class="flex min-h-8 gap-2 overflow-x-auto">
          {#each hfSearchResults.slice(0, 8) as model}
            <span class="shrink-0 rounded border border-neutral-800 px-2 py-1 font-mono text-[11px] text-neutral-300">
              {modelLabel(model)}
            </span>
          {/each}
        </div>
      {/if}
    </div>

    <div class="grid grid-cols-[minmax(0,1fr)_2.5rem] gap-2 sm:grid-cols-[minmax(0,1fr)_7rem_2.5rem]">
      <input
        class="h-9 min-w-0 rounded border border-neutral-800 bg-neutral-900 px-3 text-sm text-neutral-100 outline-none focus:border-cyan-500"
        placeholder="repo id"
        aria-label="HuggingFace repository id"
        bind:value={downloadRepoId}
      />
      <input
        class="h-9 min-w-0 rounded border border-neutral-800 bg-neutral-900 px-3 text-sm text-neutral-100 outline-none focus:border-cyan-500"
        placeholder="type"
        aria-label="HuggingFace model type"
        bind:value={downloadModelType}
      />
      <button
        type="button"
        title="Start download"
        aria-label="Start HuggingFace download"
        class="inline-flex h-9 items-center justify-center rounded border border-neutral-700 text-neutral-300 transition-colors hover:border-cyan-500 hover:text-cyan-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
        onclick={() => void startHfDownload()}
        disabled={actionBusy !== null || downloadRepoId.trim().length === 0}
      >
        <Download size={16} aria-hidden="true" />
      </button>
      <input
        class="col-span-2 h-9 min-w-0 rounded border border-neutral-800 bg-neutral-900 px-3 text-sm text-neutral-100 outline-none focus:border-cyan-500"
        placeholder="official name"
        aria-label="Official model name"
        bind:value={downloadOfficialName}
      />
      <input
        class="h-9 min-w-0 rounded border border-neutral-800 bg-neutral-900 px-3 text-sm text-neutral-100 outline-none focus:border-cyan-500"
        placeholder="family"
        aria-label="Model family"
        bind:value={downloadFamily}
      />
    </div>

    <div class="grid grid-cols-[minmax(0,1fr)_2.5rem] content-start gap-2">
      <input
        class="h-9 min-w-0 rounded border border-neutral-800 bg-neutral-900 px-3 text-sm text-neutral-100 outline-none focus:border-cyan-500"
        placeholder="Pumas model id"
        aria-label="Pumas model id"
        bind:value={deleteModelId}
      />
      <button
        type="button"
        title="Delete model"
        aria-label="Delete Pumas model"
        class="inline-flex h-9 items-center justify-center rounded border border-neutral-700 text-neutral-300 transition-colors hover:border-red-500 hover:text-red-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-red-400 disabled:opacity-50"
        onclick={() => void deletePumasModel()}
        disabled={actionBusy !== null || deleteModelId.trim().length === 0}
      >
        <Trash2 size={16} aria-hidden="true" />
      </button>
      {#if actionMessage || actionError}
        <div
          class="col-span-2 truncate text-xs"
          class:text-red-200={Boolean(actionError)}
          class:text-neutral-400={!actionError}
          title={actionError ?? actionMessage ?? ''}
        >
          {actionError ?? actionMessage}
        </div>
      {/if}
    </div>
  </div>

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

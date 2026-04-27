<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw } from 'lucide-svelte';
  import type { RunListProjectionRecord } from '../../services/diagnostics/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import {
    activeWorkflowRun,
    selectActiveWorkflowRun,
    setWorkbenchPage,
  } from '../../stores/workbenchStore';

  let runs = $state<RunListProjectionRecord[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let projectionUpdatedAtMs = $state<number | null>(null);
  let refreshInFlight = false;
  let refreshAgain = false;
  let eventUnsubscribe: (() => void) | null = null;

  function formatTimestamp(value: number | null | undefined): string {
    if (!value) {
      return 'Unscheduled';
    }
    return new Date(value).toLocaleString();
  }

  function formatDuration(
    value: number | null | undefined,
    status: RunListProjectionRecord['status'],
  ): string {
    if (value === null || value === undefined) {
      if (status === 'running') {
        return 'Running';
      }
      if (status === 'queued' || status === 'accepted') {
        return 'Pending';
      }
      return 'Unavailable';
    }
    if (value < 1_000) {
      return `${Math.round(value)} ms`;
    }
    return `${(value / 1_000).toFixed(1)} s`;
  }

  function statusClass(status: RunListProjectionRecord['status']): string {
    switch (status) {
      case 'completed':
        return 'border-emerald-700 bg-emerald-950/60 text-emerald-200';
      case 'running':
        return 'border-cyan-700 bg-cyan-950/60 text-cyan-200';
      case 'queued':
      case 'accepted':
        return 'border-amber-700 bg-amber-950/60 text-amber-200';
      case 'failed':
        return 'border-red-700 bg-red-950/60 text-red-200';
      case 'cancelled':
        return 'border-neutral-700 bg-neutral-900 text-neutral-300';
    }
  }

  async function refreshRuns(): Promise<void> {
    if (refreshInFlight) {
      refreshAgain = true;
      return;
    }

    refreshInFlight = true;
    loading = runs.length === 0;
    error = null;
    try {
      const response = await workflowService.queryRunList({ limit: 250 });
      runs = response.runs;
      projectionUpdatedAtMs = response.projection_state.updated_at_ms;
    } catch (refreshError) {
      error = refreshError instanceof Error ? refreshError.message : String(refreshError);
    } finally {
      loading = false;
      refreshInFlight = false;
      if (refreshAgain) {
        refreshAgain = false;
        void refreshRuns();
      }
    }
  }

  function selectRun(run: RunListProjectionRecord): void {
    selectActiveWorkflowRun({
      workflow_run_id: run.workflow_run_id,
      workflow_id: run.workflow_id,
      workflow_version_id: run.workflow_version_id ?? null,
      workflow_semantic_version: run.workflow_semantic_version ?? null,
      status: run.status,
    });
  }

  function openRun(run: RunListProjectionRecord, pageId: 'diagnostics' | 'graph' | 'io_inspector'): void {
    selectRun(run);
    setWorkbenchPage(pageId);
  }

  onMount(() => {
    void refreshRuns();
    eventUnsubscribe = workflowService.subscribeEvents(() => {
      void refreshRuns();
    });

    return () => {
      eventUnsubscribe?.();
      eventUnsubscribe = null;
    };
  });
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  <div class="flex shrink-0 items-center justify-between border-b border-neutral-800 px-4 py-3">
    <div>
      <h1 class="text-base font-semibold text-neutral-100">Scheduler</h1>
      <div class="mt-1 text-xs text-neutral-500">
        Projection updated {projectionUpdatedAtMs ? formatTimestamp(projectionUpdatedAtMs) : 'when runs are available'}
      </div>
    </div>
    <button
      type="button"
      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-sm text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
      onclick={() => refreshRuns()}
      disabled={loading}
    >
      <RefreshCw size={14} aria-hidden="true" class={loading ? 'animate-spin' : ''} />
      Refresh
    </button>
  </div>

  {#if error}
    <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{error}</div>
  {/if}

  <div class="min-h-0 flex-1 overflow-auto">
    <table class="w-full min-w-[72rem] border-collapse text-left text-sm">
      <thead class="sticky top-0 z-10 bg-neutral-950 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
        <tr class="border-b border-neutral-800">
          <th class="px-4 py-3 font-medium">Run</th>
          <th class="px-3 py-3 font-medium">Workflow</th>
          <th class="px-3 py-3 font-medium">Version</th>
          <th class="px-3 py-3 font-medium">Status</th>
          <th class="px-3 py-3 font-medium">Queued</th>
          <th class="px-3 py-3 font-medium">Started</th>
          <th class="px-3 py-3 font-medium">Duration</th>
          <th class="px-3 py-3 font-medium">Updated</th>
          <th class="px-4 py-3 font-medium">Open</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-neutral-900">
        {#if loading}
          <tr>
            <td colspan="9" class="px-4 py-8 text-center text-neutral-500">Loading runs</td>
          </tr>
        {:else if runs.length === 0}
          <tr>
            <td colspan="9" class="px-4 py-8 text-center text-neutral-500">No workflow runs recorded</td>
          </tr>
        {:else}
          {#each runs as run (run.workflow_run_id)}
            <tr
              class:bg-cyan-950={$activeWorkflowRun?.workflow_run_id === run.workflow_run_id}
              class:bg-opacity-30={$activeWorkflowRun?.workflow_run_id === run.workflow_run_id}
              class="hover:bg-neutral-900/70"
            >
              <td class="max-w-[18rem] px-4 py-2">
                <button
                  type="button"
                  class="max-w-full truncate text-left font-mono text-xs text-neutral-100 hover:text-cyan-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  title={run.workflow_run_id}
                  onclick={() => selectRun(run)}
                >
                  {run.workflow_run_id}
                </button>
              </td>
              <td class="max-w-[14rem] truncate px-3 py-2 text-neutral-300" title={run.workflow_id}>
                {run.workflow_id}
              </td>
              <td class="max-w-[10rem] truncate px-3 py-2 text-neutral-400" title={run.workflow_semantic_version ?? run.workflow_version_id ?? ''}>
                {run.workflow_semantic_version ?? run.workflow_version_id ?? 'Unversioned'}
              </td>
              <td class="px-3 py-2">
                <span class={`inline-flex rounded border px-2 py-0.5 text-xs ${statusClass(run.status)}`}>
                  {run.status}
                </span>
              </td>
              <td class="px-3 py-2 text-xs text-neutral-400">{formatTimestamp(run.enqueued_at_ms ?? run.accepted_at_ms)}</td>
              <td class="px-3 py-2 text-xs text-neutral-400">{formatTimestamp(run.started_at_ms)}</td>
              <td class="px-3 py-2 text-xs text-neutral-400">{formatDuration(run.duration_ms, run.status)}</td>
              <td class="px-3 py-2 text-xs text-neutral-400">{formatTimestamp(run.last_updated_at_ms)}</td>
              <td class="px-4 py-2">
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    class="rounded border border-neutral-800 px-2 py-1 text-xs text-neutral-300 hover:border-neutral-600 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    onclick={() => openRun(run, 'diagnostics')}
                  >
                    Diagnostics
                  </button>
                  <button
                    type="button"
                    class="rounded border border-neutral-800 px-2 py-1 text-xs text-neutral-300 hover:border-neutral-600 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    onclick={() => openRun(run, 'graph')}
                  >
                    Graph
                  </button>
                  <button
                    type="button"
                    class="rounded border border-neutral-800 px-2 py-1 text-xs text-neutral-300 hover:border-neutral-600 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    onclick={() => openRun(run, 'io_inspector')}
                  >
                    I/O
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</section>

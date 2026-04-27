<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw } from 'lucide-svelte';
  import type {
    ProjectionStateRecord,
    RunListProjectionRecord,
    SchedulerTimelineProjectionRecord,
  } from '../../services/diagnostics/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import {
    activeWorkflowRun,
    selectActiveWorkflowRun,
    setWorkbenchPage,
  } from '../../stores/workbenchStore';
  import {
    filterAndSortSchedulerRuns,
    formatSchedulerProjectionFreshness,
    formatSchedulerTimelineKind,
    formatSchedulerTimelineSource,
    formatSchedulerDuration,
    formatSchedulerTimestamp,
    schedulerStatusClass,
    schedulerTimelinePayloadLabel,
    SCHEDULER_SORT_OPTIONS,
    SCHEDULER_STATUS_FILTERS,
    type SchedulerSortKey,
    type SchedulerStatusFilter,
  } from './schedulerPagePresenters';

  let runs = $state<RunListProjectionRecord[]>([]);
  let searchQuery = $state('');
  let statusFilter = $state<SchedulerStatusFilter>('all');
  let sortKey = $state<SchedulerSortKey>('last_updated_desc');
  let loading = $state(false);
  let error = $state<string | null>(null);
  let projectionUpdatedAtMs = $state<number | null>(null);
  let timelineEvents = $state<SchedulerTimelineProjectionRecord[]>([]);
  let timelineProjectionState = $state<ProjectionStateRecord | null>(null);
  let timelineLoading = $state(false);
  let timelineError = $state<string | null>(null);
  let timelineRequestSerial = 0;
  let activeTimelineRunId = $state<string | null>(null);
  let refreshInFlight = false;
  let refreshAgain = false;
  let eventUnsubscribe: (() => void) | null = null;
  let displayedRuns = $derived(
    filterAndSortSchedulerRuns(runs, {
      search: searchQuery,
      status: statusFilter,
      sort: sortKey,
    }),
  );

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
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

  async function refreshTimeline(runId = activeRunId()): Promise<void> {
    const requestSerial = ++timelineRequestSerial;
    timelineError = null;

    if (!runId) {
      timelineEvents = [];
      timelineProjectionState = null;
      timelineLoading = false;
      return;
    }

    timelineLoading = true;
    try {
      const response = await workflowService.querySchedulerTimeline({
        workflow_run_id: runId,
        limit: 100,
      });
      if (requestSerial !== timelineRequestSerial) {
        return;
      }
      timelineEvents = response.events;
      timelineProjectionState = response.projection_state;
    } catch (refreshError) {
      if (requestSerial !== timelineRequestSerial) {
        return;
      }
      timelineError = refreshError instanceof Error ? refreshError.message : String(refreshError);
      timelineEvents = [];
    } finally {
      if (requestSerial === timelineRequestSerial) {
        timelineLoading = false;
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
      void refreshTimeline();
    });

    return () => {
      eventUnsubscribe?.();
      eventUnsubscribe = null;
    };
  });

  $effect(() => {
    const runId = activeRunId();
    if (runId === activeTimelineRunId) {
      return;
    }

    activeTimelineRunId = runId;
    void refreshTimeline(runId);
  });
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  <div class="flex shrink-0 items-center justify-between border-b border-neutral-800 px-4 py-3">
    <div>
      <h1 class="text-base font-semibold text-neutral-100">Scheduler</h1>
      <div class="mt-1 text-xs text-neutral-500">
        Projection updated {projectionUpdatedAtMs ? formatSchedulerTimestamp(projectionUpdatedAtMs) : 'when runs are available'}
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

  <div class="grid shrink-0 gap-3 border-b border-neutral-900 px-4 py-3 md:grid-cols-[minmax(12rem,1fr)_12rem_12rem]">
    <div>
      <label for="scheduler-run-search" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Search
      </label>
      <input
        id="scheduler-run-search"
        type="search"
        bind:value={searchQuery}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      />
    </div>
    <div>
      <label for="scheduler-status-filter" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Status
      </label>
      <select
        id="scheduler-status-filter"
        bind:value={statusFilter}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      >
        {#each SCHEDULER_STATUS_FILTERS as status}
          <option value={status}>{status}</option>
        {/each}
      </select>
    </div>
    <div>
      <label for="scheduler-sort" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Sort
      </label>
      <select
        id="scheduler-sort"
        bind:value={sortKey}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      >
        {#each SCHEDULER_SORT_OPTIONS as option}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="grid min-h-0 flex-1 grid-cols-1 overflow-hidden xl:grid-cols-[minmax(0,1fr)_24rem]">
    <div class="min-h-0 overflow-auto">
      <table class="w-full min-w-[78rem] border-collapse text-left text-sm">
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
          {:else if displayedRuns.length === 0}
            <tr>
              <td colspan="9" class="px-4 py-8 text-center text-neutral-500">No matching workflow runs</td>
            </tr>
          {:else}
            {#each displayedRuns as run (run.workflow_run_id)}
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
                  <span class={`inline-flex rounded border px-2 py-0.5 text-xs ${schedulerStatusClass(run.status)}`}>
                    {run.status}
                  </span>
                </td>
                <td class="px-3 py-2 text-xs text-neutral-400">{formatSchedulerTimestamp(run.enqueued_at_ms ?? run.accepted_at_ms)}</td>
                <td class="px-3 py-2 text-xs text-neutral-400">{formatSchedulerTimestamp(run.started_at_ms)}</td>
                <td class="px-3 py-2 text-xs text-neutral-400">{formatSchedulerDuration(run.duration_ms, run.status)}</td>
                <td class="px-3 py-2 text-xs text-neutral-400">{formatSchedulerTimestamp(run.last_updated_at_ms)}</td>
                <td class="px-4 py-2">
                  <div class="flex items-center gap-2">
                    <button
                      type="button"
                      class="rounded border border-neutral-800 px-2 py-1 text-xs text-neutral-300 hover:border-neutral-600 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                      onclick={() => selectRun(run)}
                    >
                      Timeline
                    </button>
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

    <aside class="min-h-0 overflow-auto border-l border-neutral-800 bg-neutral-950/80">
      <div class="border-b border-neutral-800 px-4 py-3">
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <h2 class="text-sm font-semibold text-neutral-100">Timeline</h2>
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
            class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
            onclick={() => refreshTimeline()}
            disabled={timelineLoading || !$activeWorkflowRun}
          >
            <RefreshCw size={12} aria-hidden="true" class={timelineLoading ? 'animate-spin' : ''} />
            Refresh
          </button>
        </div>
        <div class="mt-3 text-xs text-neutral-500">
          {formatSchedulerProjectionFreshness(timelineProjectionState)}
        </div>
      </div>

      {#if timelineError}
        <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{timelineError}</div>
      {/if}

      {#if !$activeWorkflowRun}
        <div class="px-4 py-8 text-sm text-neutral-500">Select a run to inspect scheduler events</div>
      {:else if timelineLoading && timelineEvents.length === 0}
        <div class="px-4 py-8 text-sm text-neutral-500">Loading timeline</div>
      {:else if timelineEvents.length === 0}
        <div class="px-4 py-8 text-sm text-neutral-500">No scheduler timeline events projected</div>
      {:else}
        <div class="divide-y divide-neutral-900">
          {#each timelineEvents as event (event.event_id)}
            <article class="px-4 py-3">
              <div class="flex items-center justify-between gap-3">
                <div class="font-mono text-[11px] text-neutral-500">seq {event.event_seq}</div>
                <div class="text-[11px] text-neutral-600">{schedulerTimelinePayloadLabel(event)}</div>
              </div>
              <div class="mt-2 text-xs font-semibold text-neutral-200">
                {formatSchedulerTimelineKind(event)}
              </div>
              <div class="mt-1 text-xs text-neutral-500">
                {formatSchedulerTimelineSource(event)} · {formatSchedulerTimestamp(event.occurred_at_ms)}
              </div>
              <div class="mt-2 text-sm text-neutral-300">{event.summary}</div>
              {#if event.detail}
                <div class="mt-1 text-xs text-neutral-500">{event.detail}</div>
              {/if}
            </article>
          {/each}
        </div>
      {/if}
    </aside>
  </div>
</section>

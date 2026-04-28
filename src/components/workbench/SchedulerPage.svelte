<script lang="ts">
  import { onMount } from 'svelte';
  import { ChevronsUp, RefreshCw, ShieldAlert, SlidersHorizontal, XCircle } from 'lucide-svelte';
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
    schedulerRunFilters,
    setSchedulerRunFilters,
    SCHEDULER_SORT_OPTIONS,
    SCHEDULER_STATUS_FILTERS,
    type SchedulerSortKey,
    type SchedulerStatusFilter,
  } from '../../stores/schedulerRunListStore';
  import {
    buildSchedulerRunListQuery,
    filterSchedulerTimelineEvents,
    filterAndSortSchedulerRuns,
    formatSchedulerProjectionFreshness,
    formatSchedulerPolicyLabel,
    formatSchedulerRetentionLabel,
    formatSchedulerTimelineKind,
    formatSchedulerTimelineSource,
    formatSchedulerDuration,
    formatSchedulerEstimateLabel,
    formatSchedulerTimestamp,
    formatSchedulerPriority,
    formatSchedulerQueuePosition,
    formatSchedulerReasonLabel,
    schedulerStatusClass,
    schedulerAcceptedDateFilterOptions,
    schedulerBucketFilterOptions,
    schedulerClientFilterOptions,
    schedulerClientSessionFilterOptions,
    schedulerPolicyFilterOptions,
    schedulerRetentionFilterOptions,
    formatSchedulerScopeLabel,
    schedulerRunSupportsAdminQueueControls,
    schedulerRunSupportsQueueControls,
    schedulerTimelinePayloadLabel,
    schedulerTimelineKindFilterOptions,
    schedulerTimelineSourceFilterOptions,
  } from './schedulerPagePresenters';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

  let runs = $state<RunListProjectionRecord[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let projectionUpdatedAtMs = $state<number | null>(null);
  let timelineEvents = $state<SchedulerTimelineProjectionRecord[]>([]);
  let timelineProjectionState = $state<ProjectionStateRecord | null>(null);
  let timelineLoading = $state(false);
  let timelineError = $state<string | null>(null);
  let actionBusy = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let actionMessage = $state<string | null>(null);
  let adminPriorityInput = $state('0');
  let timelineKindFilter = $state('all');
  let timelineSourceFilter = $state('all');
  let adminPriorityRunId = '';
  let timelineRequestSerial = 0;
  let activeTimelineRunId = $state<string | null>(null);
  let refreshInFlight = false;
  let refreshAgain = false;
  let eventUnsubscribe: (() => void) | null = null;
  let activeRunFilterKey = '';
  let displayedRuns = $derived(filterAndSortSchedulerRuns(runs, $schedulerRunFilters));
  let displayedTimelineEvents = $derived(
    filterSchedulerTimelineEvents(timelineEvents, {
      eventKind: timelineKindFilter,
      sourceComponent: timelineSourceFilter,
    }),
  );
  let timelineKindOptions = $derived(schedulerTimelineKindFilterOptions(timelineEvents));
  let timelineSourceOptions = $derived(schedulerTimelineSourceFilterOptions(timelineEvents));
  let schedulerPolicyOptions = $derived(schedulerPolicyFilterOptions(runs));
  let retentionPolicyOptions = $derived(schedulerRetentionFilterOptions(runs));
  let clientOptions = $derived(schedulerClientFilterOptions(runs));
  let clientSessionOptions = $derived(schedulerClientSessionFilterOptions(runs));
  let bucketOptions = $derived(schedulerBucketFilterOptions(runs));
  let acceptedDateOptions = $derived(schedulerAcceptedDateFilterOptions(runs));
  let selectedRunRecord = $derived(
    runs.find((run) => run.workflow_run_id === $activeWorkflowRun?.workflow_run_id) ?? null,
  );
  let selectedRunHasQueueControls = $derived(schedulerRunSupportsQueueControls(selectedRunRecord));
  let selectedRunHasAdminQueueControls = $derived(schedulerRunSupportsAdminQueueControls(selectedRunRecord));

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
  }

  function eventValue(event: Event): string {
    return (event.currentTarget as HTMLInputElement | HTMLSelectElement).value;
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
      const response = await workflowService.queryRunList(
        buildSchedulerRunListQuery($schedulerRunFilters, 250),
      );
      runs = response.runs;
      projectionUpdatedAtMs = response.projection_state.updated_at_ms;
    } catch (refreshError) {
      error = formatWorkflowCommandError(refreshError);
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
      timelineError = formatWorkflowCommandError(refreshError);
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

  async function cancelSelectedRun(): Promise<void> {
    const run = selectedRunRecord;
    if (!schedulerRunSupportsQueueControls(run)) {
      return;
    }
    actionBusy = 'cancel';
    actionError = null;
    actionMessage = null;
    try {
      await workflowService.cancelSessionQueueItem({
        session_id: run.workflow_execution_session_id as string,
        workflow_run_id: run.workflow_run_id,
      });
      actionMessage = 'Cancel accepted by scheduler';
      await refreshRuns();
      await refreshTimeline(run.workflow_run_id);
    } catch (actionFailure) {
      actionError = formatWorkflowCommandError(actionFailure);
    } finally {
      actionBusy = null;
    }
  }

  async function pushSelectedRunToFront(): Promise<void> {
    const run = selectedRunRecord;
    if (!schedulerRunSupportsQueueControls(run)) {
      return;
    }
    actionBusy = 'front';
    actionError = null;
    actionMessage = null;
    try {
      await workflowService.pushSessionQueueItemToFront({
        session_id: run.workflow_execution_session_id as string,
        workflow_run_id: run.workflow_run_id,
      });
      actionMessage = 'Push-front accepted by scheduler';
      await refreshRuns();
      await refreshTimeline(run.workflow_run_id);
    } catch (actionFailure) {
      actionError = formatWorkflowCommandError(actionFailure);
    } finally {
      actionBusy = null;
    }
  }

  async function adminCancelSelectedRun(): Promise<void> {
    const run = selectedRunRecord;
    if (!schedulerRunSupportsAdminQueueControls(run)) {
      return;
    }
    actionBusy = 'admin-cancel';
    actionError = null;
    actionMessage = null;
    try {
      await workflowService.adminCancelQueueItem({
        workflow_run_id: run.workflow_run_id,
      });
      actionMessage = 'Admin cancel accepted by scheduler';
      await refreshRuns();
      await refreshTimeline(run.workflow_run_id);
    } catch (actionFailure) {
      actionError = formatWorkflowCommandError(actionFailure);
    } finally {
      actionBusy = null;
    }
  }

  async function adminPushSelectedRunToFront(): Promise<void> {
    const run = selectedRunRecord;
    if (!schedulerRunSupportsAdminQueueControls(run)) {
      return;
    }
    actionBusy = 'admin-front';
    actionError = null;
    actionMessage = null;
    try {
      await workflowService.adminPushQueueItemToFront({
        workflow_run_id: run.workflow_run_id,
      });
      actionMessage = 'Admin push-front accepted by scheduler';
      await refreshRuns();
      await refreshTimeline(run.workflow_run_id);
    } catch (actionFailure) {
      actionError = formatWorkflowCommandError(actionFailure);
    } finally {
      actionBusy = null;
    }
  }

  async function adminReprioritizeSelectedRun(): Promise<void> {
    const run = selectedRunRecord;
    if (!schedulerRunSupportsAdminQueueControls(run)) {
      return;
    }
    const priority = Number(adminPriorityInput);
    if (!Number.isInteger(priority)) {
      actionError = 'Priority must be an integer';
      actionMessage = null;
      return;
    }
    actionBusy = 'admin-priority';
    actionError = null;
    actionMessage = null;
    try {
      await workflowService.adminReprioritizeQueueItem({
        workflow_run_id: run.workflow_run_id,
        priority,
      });
      actionMessage = 'Admin priority accepted by scheduler';
      await refreshRuns();
      await refreshTimeline(run.workflow_run_id);
    } catch (actionFailure) {
      actionError = formatWorkflowCommandError(actionFailure);
    } finally {
      actionBusy = null;
    }
  }

  onMount(() => {
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

  $effect(() => {
    const filterKey = JSON.stringify($schedulerRunFilters);
    if (filterKey === activeRunFilterKey) {
      return;
    }

    activeRunFilterKey = filterKey;
    void refreshRuns();
  });

  $effect(() => {
    const run = selectedRunRecord;
    const runId = run?.workflow_run_id ?? '';
    if (runId === adminPriorityRunId) {
      return;
    }
    adminPriorityRunId = runId;
    adminPriorityInput = String(run?.scheduler_priority ?? 0);
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

  <div class="grid shrink-0 gap-3 border-b border-neutral-900 px-4 py-3 md:grid-cols-3 xl:grid-cols-6 2xl:grid-cols-[minmax(12rem,1.5fr)_repeat(8,minmax(8rem,1fr))]">
    <div class="md:col-span-3 xl:col-span-2 2xl:col-span-1">
      <label for="scheduler-run-search" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Search
      </label>
      <input
        id="scheduler-run-search"
        type="search"
        value={$schedulerRunFilters.search}
        oninput={(event) => setSchedulerRunFilters({ search: eventValue(event) })}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      />
    </div>
    <div>
      <label for="scheduler-status-filter" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Status
      </label>
      <select
        id="scheduler-status-filter"
        value={$schedulerRunFilters.status}
        onchange={(event) =>
          setSchedulerRunFilters({ status: eventValue(event) as SchedulerStatusFilter })}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      >
        {#each SCHEDULER_STATUS_FILTERS as status (status)}
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
        value={$schedulerRunFilters.sort}
        onchange={(event) => setSchedulerRunFilters({ sort: eventValue(event) as SchedulerSortKey })}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      >
        {#each SCHEDULER_SORT_OPTIONS as option (option.value)}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
    </div>
    <div>
      <label for="scheduler-policy-filter" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Policy
      </label>
      <select
        id="scheduler-policy-filter"
        value={$schedulerRunFilters.schedulerPolicy}
        onchange={(event) => setSchedulerRunFilters({ schedulerPolicy: eventValue(event) })}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      >
        <option value="all">all</option>
        {#each schedulerPolicyOptions as policy (policy)}
          <option value={policy}>{policy}</option>
        {/each}
      </select>
    </div>
    <div>
      <label for="scheduler-retention-filter" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Retention
      </label>
      <select
        id="scheduler-retention-filter"
        value={$schedulerRunFilters.retentionPolicy}
        onchange={(event) => setSchedulerRunFilters({ retentionPolicy: eventValue(event) })}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      >
        <option value="all">all</option>
        {#each retentionPolicyOptions as retention (retention)}
          <option value={retention}>{retention}</option>
        {/each}
      </select>
    </div>
    <div>
      <label for="scheduler-client-filter" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Client
      </label>
      <select
        id="scheduler-client-filter"
        value={$schedulerRunFilters.client}
        onchange={(event) => setSchedulerRunFilters({ client: eventValue(event) })}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      >
        <option value="all">all</option>
        {#each clientOptions as client (client)}
          <option value={client}>{client}</option>
        {/each}
      </select>
    </div>
    <div>
      <label for="scheduler-session-filter" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Session
      </label>
      <select
        id="scheduler-session-filter"
        value={$schedulerRunFilters.clientSession}
        onchange={(event) => setSchedulerRunFilters({ clientSession: eventValue(event) })}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      >
        <option value="all">all</option>
        {#each clientSessionOptions as clientSession (clientSession)}
          <option value={clientSession}>{clientSession}</option>
        {/each}
      </select>
    </div>
    <div>
      <label for="scheduler-bucket-filter" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Bucket
      </label>
      <select
        id="scheduler-bucket-filter"
        value={$schedulerRunFilters.bucket}
        onchange={(event) => setSchedulerRunFilters({ bucket: eventValue(event) })}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      >
        <option value="all">all</option>
        {#each bucketOptions as bucket (bucket)}
          <option value={bucket}>{bucket}</option>
        {/each}
      </select>
    </div>
    <div>
      <label for="scheduler-accepted-filter" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
        Accepted
      </label>
      <select
        id="scheduler-accepted-filter"
        value={$schedulerRunFilters.acceptedDate}
        onchange={(event) => setSchedulerRunFilters({ acceptedDate: eventValue(event) })}
        class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
      >
        <option value="all">all</option>
        {#each acceptedDateOptions as acceptedDate (acceptedDate)}
          <option value={acceptedDate}>{acceptedDate}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="grid min-h-0 flex-1 grid-cols-1 overflow-hidden xl:grid-cols-[minmax(0,1fr)_24rem]">
    <div class="min-h-0 overflow-auto">
      <table class="w-full min-w-[146rem] border-collapse text-left text-sm">
        <thead class="sticky top-0 z-10 bg-neutral-950 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
          <tr class="border-b border-neutral-800">
            <th class="px-4 py-3 font-medium">Run</th>
            <th class="px-3 py-3 font-medium">Workflow</th>
            <th class="px-3 py-3 font-medium">Version</th>
            <th class="px-3 py-3 font-medium">Status</th>
            <th class="px-3 py-3 font-medium">Scheduler Policy</th>
            <th class="px-3 py-3 font-medium">Retention</th>
            <th class="px-3 py-3 font-medium">Client</th>
            <th class="px-3 py-3 font-medium">Session</th>
            <th class="px-3 py-3 font-medium">Bucket</th>
            <th class="px-3 py-3 font-medium">Queue</th>
            <th class="px-3 py-3 font-medium">Priority</th>
            <th class="px-3 py-3 font-medium">Estimate</th>
            <th class="px-3 py-3 font-medium">Reason</th>
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
              <td colspan="18" class="px-4 py-8 text-center text-neutral-500">Loading runs</td>
            </tr>
          {:else if runs.length === 0}
            <tr>
              <td colspan="18" class="px-4 py-8 text-center text-neutral-500">No workflow runs recorded</td>
            </tr>
          {:else if displayedRuns.length === 0}
            <tr>
              <td colspan="18" class="px-4 py-8 text-center text-neutral-500">No matching workflow runs</td>
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
                <td class="max-w-[10rem] truncate px-3 py-2 text-xs text-neutral-400" title={formatSchedulerPolicyLabel(run.scheduler_policy_id)}>
                  {formatSchedulerPolicyLabel(run.scheduler_policy_id)}
                </td>
                <td class="max-w-[10rem] truncate px-3 py-2 text-xs text-neutral-400" title={formatSchedulerRetentionLabel(run.retention_policy_id)}>
                  {formatSchedulerRetentionLabel(run.retention_policy_id)}
                </td>
                <td class="max-w-[10rem] truncate px-3 py-2 font-mono text-xs text-neutral-400" title={formatSchedulerScopeLabel(run.client_id)}>
                  {formatSchedulerScopeLabel(run.client_id)}
                </td>
                <td class="max-w-[12rem] truncate px-3 py-2 font-mono text-xs text-neutral-400" title={formatSchedulerScopeLabel(run.client_session_id)}>
                  {formatSchedulerScopeLabel(run.client_session_id)}
                </td>
                <td class="max-w-[10rem] truncate px-3 py-2 font-mono text-xs text-neutral-400" title={formatSchedulerScopeLabel(run.bucket_id)}>
                  {formatSchedulerScopeLabel(run.bucket_id)}
                </td>
                <td class="px-3 py-2 text-xs text-neutral-400">{formatSchedulerQueuePosition(run.scheduler_queue_position)}</td>
                <td class="px-3 py-2 text-xs text-neutral-400">{formatSchedulerPriority(run.scheduler_priority)}</td>
                <td class="max-w-[16rem] truncate px-3 py-2 text-xs text-neutral-400" title={formatSchedulerEstimateLabel(run)}>
                  {formatSchedulerEstimateLabel(run)}
                </td>
                <td class="max-w-[16rem] truncate px-3 py-2 text-xs text-neutral-400" title={formatSchedulerReasonLabel(run.scheduler_reason)}>
                  {formatSchedulerReasonLabel(run.scheduler_reason)}
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

      <div class="border-b border-neutral-900 px-4 py-3">
        <div class="mb-3 grid grid-cols-2 gap-2">
          <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-600">
            Kind
            <select
              aria-label="Scheduler timeline kind filter"
              class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
              value={timelineKindFilter}
              onchange={(event) => {
                timelineKindFilter = eventValue(event);
              }}
              disabled={timelineEvents.length === 0}
            >
              <option value="all">All</option>
              {#each timelineKindOptions as eventKind (eventKind)}
                <option value={eventKind}>{formatSchedulerTimelineKind({ event_kind: eventKind })}</option>
              {/each}
            </select>
          </label>
          <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-600">
            Source
            <select
              aria-label="Scheduler timeline source filter"
              class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
              value={timelineSourceFilter}
              onchange={(event) => {
                timelineSourceFilter = eventValue(event);
              }}
              disabled={timelineEvents.length === 0}
            >
              <option value="all">All</option>
              {#each timelineSourceOptions as sourceComponent (sourceComponent)}
                <option value={sourceComponent}>{formatSchedulerTimelineSource({ source_component: sourceComponent })}</option>
              {/each}
            </select>
          </label>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            title="Cancel selected queued run"
            class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-300 transition-colors hover:border-red-500 hover:text-red-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-red-400 disabled:opacity-50"
            onclick={() => void cancelSelectedRun()}
            disabled={!selectedRunHasQueueControls || actionBusy !== null}
          >
            <XCircle size={12} aria-hidden="true" />
            Cancel
          </button>
          <button
            type="button"
            title="Push selected run to the front of its session queue"
            class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-300 transition-colors hover:border-cyan-500 hover:text-cyan-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
            onclick={() => void pushSelectedRunToFront()}
            disabled={!selectedRunHasQueueControls || actionBusy !== null}
          >
            <ChevronsUp size={12} aria-hidden="true" />
            Front
          </button>
        </div>
        <div class="mt-3 border-t border-neutral-900 pt-3">
          <div class="mb-2 text-[11px] uppercase tracking-[0.18em] text-neutral-600">GUI Admin</div>
          <div class="flex flex-wrap items-center gap-2">
            <button
              type="button"
              title="Admin cancel selected queued run by run id"
              class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-300 transition-colors hover:border-red-500 hover:text-red-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-red-400 disabled:opacity-50"
              onclick={() => void adminCancelSelectedRun()}
              disabled={!selectedRunHasAdminQueueControls || actionBusy !== null}
            >
              <ShieldAlert size={12} aria-hidden="true" />
              Cancel
            </button>
            <button
              type="button"
              title="Admin push selected queued run to the front by run id"
              class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-300 transition-colors hover:border-cyan-500 hover:text-cyan-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
              onclick={() => void adminPushSelectedRunToFront()}
              disabled={!selectedRunHasAdminQueueControls || actionBusy !== null}
            >
              <ChevronsUp size={12} aria-hidden="true" />
              Front
            </button>
            <label class="inline-flex items-center gap-2 text-xs text-neutral-500">
              Priority
              <input
                type="number"
                step="1"
                value={adminPriorityInput}
                oninput={(event) => {
                  adminPriorityInput = eventValue(event);
                }}
                class="h-7 w-20 rounded border border-neutral-700 bg-neutral-900 px-2 text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none disabled:opacity-50"
                disabled={!selectedRunHasAdminQueueControls || actionBusy !== null}
              />
            </label>
            <button
              type="button"
              title="Admin set selected queued run priority by run id"
              class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-300 transition-colors hover:border-cyan-500 hover:text-cyan-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
              onclick={() => void adminReprioritizeSelectedRun()}
              disabled={!selectedRunHasAdminQueueControls || actionBusy !== null}
            >
              <SlidersHorizontal size={12} aria-hidden="true" />
              Set
            </button>
          </div>
        </div>
        {#if actionMessage || actionError}
          <div
            class="mt-2 truncate text-xs"
            class:text-red-200={Boolean(actionError)}
            class:text-neutral-500={!actionError}
            title={actionError ?? actionMessage ?? ''}
          >
            {actionError ?? actionMessage}
          </div>
        {/if}
      </div>

      {#if !$activeWorkflowRun}
        <div class="px-4 py-8 text-sm text-neutral-500">Select a run to inspect scheduler events</div>
      {:else if timelineLoading && timelineEvents.length === 0}
        <div class="px-4 py-8 text-sm text-neutral-500">Loading timeline</div>
      {:else if timelineEvents.length === 0}
        <div class="px-4 py-8 text-sm text-neutral-500">No scheduler timeline events projected</div>
      {:else if displayedTimelineEvents.length === 0}
        <div class="px-4 py-8 text-sm text-neutral-500">No matching scheduler timeline events</div>
      {:else}
        <div class="divide-y divide-neutral-900">
          {#each displayedTimelineEvents as event (event.event_id)}
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

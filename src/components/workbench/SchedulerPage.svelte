<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw } from 'lucide-svelte';
  import type {
    IoArtifactRetentionSummaryRecord,
    ProjectionStateRecord,
    RunListProjectionRecord,
    SchedulerTimelineProjectionRecord,
    WorkflowSchedulerEstimateRecord,
  } from '../../services/diagnostics/types';
  import { subscribeDiagnosticsProjectionInvalidations } from '../../services/workflow/WorkflowProjectionSubscriptionService';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun, setWorkbenchPage, type WorkbenchPageId } from '../../stores/workbenchStore';
  import {
    DEFAULT_SCHEDULER_RUN_FILTERS,
    clearSchedulerSecondaryFilters,
    resetSchedulerRunFilters,
    schedulerRunFilters,
    schedulerViewPreferences,
    setSchedulerRunFilters,
    type SchedulerRunFilters,
    type SchedulerSecondaryFilterKey,
  } from '../../stores/schedulerRunListStore';
  import {
    buildSchedulerFilterFields,
    buildSchedulerQueueSummary,
    buildSchedulerStatusSegments,
    filterAndSortSchedulerRuns,
    filterSchedulerRunsBySegment,
    filterSchedulerTimelineEvents,
    formatSchedulerClockTime,
    formatSchedulerTimelineKind,
    formatSchedulerTimelineSource,
    formatSchedulerTimestamp,
    groupSchedulerRuns,
    schedulerAdminControlsUnavailableReason,
    schedulerAppliedFilterChips,
    schedulerDetailFreshnessText,
    schedulerEstimateSentence,
    schedulerFooterState,
    schedulerQueueControlsUnavailableReason,
    schedulerResumeUnavailableReason,
    schedulerRetentionPolicyText,
    schedulerRetentionText,
    schedulerRunDetailChips,
    schedulerRunPlacementText,
    schedulerRunPolicyText,
    schedulerRunProgressText,
    schedulerRunSupportsAdminQueueControls,
    schedulerRunSupportsDependencyReadinessResume,
    schedulerRunSupportsQueueControls,
    schedulerTimelineAttemptText,
    schedulerTimelineKindFilterOptions,
    schedulerTimelineSourceFilterOptions,
  } from './schedulerPagePresenters';
  import { formatClockTime, formatRunVersion, runContextStatusLabel } from './shell/runContextPicker';
  import {
    refreshRunContextRunList,
    runContextLiveUpdates,
    runContextRunList,
    selectRunContext,
  } from './shell/runContextStore';
  import {
    Chip,
    DimensionLine,
    ErrorCard,
    FilterChip,
    GroupHeader,
    ListRow,
    Panel,
    SegmentedControl,
    TwoStepButton,
    glyphStateForStatus,
  } from './ui';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

  // Device memory per lane is not in the local network status projection
  // (WR-006); the strip says so instead of drawing invented lanes.
  const DEVICE_MEMORY_UNAVAILABLE = 'Device memory is not reported by this engine build.';
  const NO_SELECTION_FOOTER = 'Select a run to act on it. The other tabs follow this selection.';

  type SchedulerActionId =
    | 'cancel'
    | 'front'
    | 'priority'
    | 'dependency-readiness-resume'
    | 'admin-cancel'
    | 'admin-front'
    | 'admin-priority';

  type LiveUpdates = 'connecting' | 'live' | 'unavailable';

  let timelineEvents = $state<SchedulerTimelineProjectionRecord[]>([]);
  let timelineProjectionState = $state<ProjectionStateRecord | null>(null);
  let selectedRunEstimate = $state<WorkflowSchedulerEstimateRecord | null>(null);
  let estimateProjectionState = $state<ProjectionStateRecord | null>(null);
  let selectedRunRetentionSummary = $state<IoArtifactRetentionSummaryRecord[]>([]);
  let retentionProjectionState = $state<ProjectionStateRecord | null>(null);
  let timelineLoading = $state(false);
  let estimateLoading = $state(false);
  let retentionLoading = $state(false);
  let timelineError = $state<string | null>(null);
  let estimateError = $state<string | null>(null);
  let retentionError = $state<string | null>(null);
  let detailLiveUpdates = $state<LiveUpdates>('connecting');
  let actionBusy = $state<SchedulerActionId | null>(null);
  let actionError = $state<string | null>(null);
  let actionMessage = $state<string | null>(null);
  let sessionPriorityInput = $state('0');
  let adminPriorityInput = $state('0');
  let timelineKindFilter = $state('all');
  let timelineSourceFilter = $state('all');
  let filtersOpen = $state(false);
  let moreActionsOpen = $state(false);
  let collapsedGroups = $state<Record<string, boolean>>({});
  let activeTimelineRunId = $state<string | null>(null);
  let selectionSyncRunId = '';
  let timelineRequestSerial = 0;
  let estimateRequestSerial = 0;
  let retentionRequestSerial = 0;

  // The one run-list projection, owned and refreshed by the shell (WR-005).
  let runs = $derived($runContextRunList.runs);
  let listLoaded = $derived($runContextRunList.loadedAtMs !== null);
  let listLoading = $derived($runContextRunList.status === 'loading');
  let listUnreachable = $derived($runContextRunList.availability.kind === 'unreachable');
  let activeRunIdValue = $derived($activeWorkflowRun?.workflow_run_id ?? null);

  let filteredRuns = $derived(filterAndSortSchedulerRuns(runs, $schedulerRunFilters));
  let statusSegments = $derived(buildSchedulerStatusSegments(filteredRuns));
  let visibleRuns = $derived(filterSchedulerRunsBySegment(filteredRuns, $schedulerRunFilters.statusSegment));
  let runGroups = $derived(
    groupSchedulerRuns(visibleRuns, {
      selectedRunId: activeRunIdValue,
      nowMs: $runContextRunList.loadedAtMs ?? 0,
    }),
  );
  let filterFields = $derived(buildSchedulerFilterFields(runs, $schedulerRunFilters));
  let filterChips = $derived(schedulerAppliedFilterChips($schedulerRunFilters));
  let queueSummary = $derived(
    buildSchedulerQueueSummary(runs, { loaded: listLoaded, loading: listLoading, unreachable: listUnreachable }),
  );
  let liveUpdatesUnavailable = $derived(
    $runContextLiveUpdates === 'unavailable' || detailLiveUpdates === 'unavailable',
  );
  let listRefreshTitle = $derived(
    $runContextRunList.loadedAtMs === null
      ? 'Refresh runs'
      : `Refresh runs (loaded ${formatClockTime($runContextRunList.loadedAtMs)})`,
  );

  let selectedRunRecord = $derived(runs.find((run) => run.workflow_run_id === activeRunIdValue) ?? null);
  let footerState = $derived(schedulerFooterState(activeRunIdValue, selectedRunRecord));
  let queueControlsReason = $derived(schedulerQueueControlsUnavailableReason(selectedRunRecord));
  let adminControlsReason = $derived(schedulerAdminControlsUnavailableReason(selectedRunRecord));
  let resumeReason = $derived(schedulerResumeUnavailableReason(selectedRunRecord));
  let queueControlsDisabled = $derived(queueControlsReason !== null || actionBusy !== null);
  let adminControlsDisabled = $derived(adminControlsReason !== null || actionBusy !== null);
  let actionFeedback = $derived(actionBusy ? 'Sending to the scheduler…' : (actionError ?? actionMessage));

  let detailStatus = $derived(selectedRunRecord?.status ?? $activeWorkflowRun?.status ?? null);
  let detailVersion = $derived(
    formatRunVersion(selectedRunRecord?.workflow_semantic_version ?? $activeWorkflowRun?.workflow_semantic_version),
  );
  let detailChips = $derived(selectedRunRecord ? schedulerRunDetailChips(selectedRunRecord) : []);
  let displayedTimelineEvents = $derived(
    filterSchedulerTimelineEvents(timelineEvents, {
      eventKind: timelineKindFilter,
      sourceComponent: timelineSourceFilter,
    }),
  );
  let timelineKindOptions = $derived(schedulerTimelineKindFilterOptions(timelineEvents));
  let timelineSourceOptions = $derived(schedulerTimelineSourceFilterOptions(timelineEvents));
  let detailLoading = $derived(timelineLoading || estimateLoading || retentionLoading);
  let detailFreshness = $derived(
    schedulerDetailFreshnessText({
      timeline: timelineProjectionState,
      estimate: estimateProjectionState,
      retention: retentionProjectionState,
    }),
  );

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
  }

  function eventValue(event: Event): string {
    return (event.currentTarget as HTMLInputElement | HTMLSelectElement).value;
  }

  function setFilter(key: SchedulerSecondaryFilterKey, value: string): void {
    // The store normalises unknown status and sort values back to their defaults.
    setSchedulerRunFilters({ [key]: value } as Partial<SchedulerRunFilters>);
  }

  function toggleGroup(groupId: string): void {
    collapsedGroups = { ...collapsedGroups, [groupId]: !collapsedGroups[groupId] };
  }

  function openPage(pageId: WorkbenchPageId): void {
    setWorkbenchPage(pageId);
  }

  async function refreshFromProjectionInvalidation(projectionKind: string): Promise<void> {
    if (projectionKind === 'scheduler_timeline') {
      await refreshTimeline();
      return;
    }
    if (projectionKind === 'run_detail') {
      await refreshEstimate();
      return;
    }
    if (projectionKind === 'io_artifact') {
      await refreshRetentionSummary();
    }
  }

  function refreshDetail(): void {
    void refreshEstimate();
    void refreshRetentionSummary();
    void refreshTimeline();
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

  async function refreshEstimate(runId = activeRunId()): Promise<void> {
    const requestSerial = ++estimateRequestSerial;
    estimateError = null;

    if (!runId) {
      selectedRunEstimate = null;
      estimateProjectionState = null;
      estimateLoading = false;
      return;
    }

    estimateLoading = true;
    try {
      const response = await workflowService.querySchedulerEstimate({
        workflow_run_id: runId,
      });
      if (requestSerial !== estimateRequestSerial) {
        return;
      }
      selectedRunEstimate = response.estimate ?? null;
      estimateProjectionState = response.projection_state;
    } catch (refreshError) {
      if (requestSerial !== estimateRequestSerial) {
        return;
      }
      estimateError = formatWorkflowCommandError(refreshError);
      selectedRunEstimate = null;
      estimateProjectionState = null;
    } finally {
      if (requestSerial === estimateRequestSerial) {
        estimateLoading = false;
      }
    }
  }

  async function refreshRetentionSummary(runId = activeRunId()): Promise<void> {
    const requestSerial = ++retentionRequestSerial;
    retentionError = null;

    if (!runId) {
      selectedRunRetentionSummary = [];
      retentionProjectionState = null;
      retentionLoading = false;
      return;
    }

    retentionLoading = true;
    try {
      const response = await workflowService.queryIoArtifacts({
        workflow_run_id: runId,
        limit: 1,
      });
      if (requestSerial !== retentionRequestSerial) {
        return;
      }
      selectedRunRetentionSummary = response.retention_summary;
      retentionProjectionState = response.projection_state;
    } catch (refreshError) {
      if (requestSerial !== retentionRequestSerial) {
        return;
      }
      retentionError = formatWorkflowCommandError(refreshError);
      selectedRunRetentionSummary = [];
      retentionProjectionState = null;
    } finally {
      if (requestSerial === retentionRequestSerial) {
        retentionLoading = false;
      }
    }
  }

  /**
   * Run one scheduler command for `run`, then refresh the shared run list and
   * this run's detail projections. The scheduler stays the authority: nothing
   * is changed locally until the projections say so.
   */
  async function performAction(
    action: SchedulerActionId,
    run: RunListProjectionRecord,
    acceptedMessage: string,
    command: () => Promise<unknown>,
  ): Promise<void> {
    actionBusy = action;
    actionError = null;
    actionMessage = null;
    try {
      await command();
      actionMessage = acceptedMessage;
      await refreshRunContextRunList();
      await refreshEstimate(run.workflow_run_id);
      await refreshRetentionSummary(run.workflow_run_id);
      await refreshTimeline(run.workflow_run_id);
    } catch (actionFailure) {
      actionError = formatWorkflowCommandError(actionFailure);
    } finally {
      actionBusy = null;
    }
  }

  function parsePriority(input: string): number | null {
    const priority = Number(input);
    if (!Number.isInteger(priority)) {
      actionError = 'Priority must be an integer';
      actionMessage = null;
      return null;
    }
    return priority;
  }

  function cancelSelectedRun(): void {
    const run = selectedRunRecord;
    if (!run || !schedulerRunSupportsQueueControls(run)) return;
    void performAction('cancel', run, 'Cancel accepted by scheduler', () =>
      workflowService.cancelSessionQueueItem({
        session_id: run.workflow_execution_session_id as string,
        workflow_run_id: run.workflow_run_id,
      }),
    );
  }

  function pushSelectedRunToFront(): void {
    const run = selectedRunRecord;
    if (!run || !schedulerRunSupportsQueueControls(run)) return;
    void performAction('front', run, 'Push-front accepted by scheduler', () =>
      workflowService.pushSessionQueueItemToFront({
        session_id: run.workflow_execution_session_id as string,
        workflow_run_id: run.workflow_run_id,
      }),
    );
  }

  function reprioritizeSelectedRun(): void {
    const run = selectedRunRecord;
    if (!run || !schedulerRunSupportsQueueControls(run)) return;
    const priority = parsePriority(sessionPriorityInput);
    if (priority === null) return;
    void performAction('priority', run, 'Priority accepted by scheduler', () =>
      workflowService.reprioritizeSessionQueueItem({
        session_id: run.workflow_execution_session_id as string,
        workflow_run_id: run.workflow_run_id,
        priority,
      }),
    );
  }

  function resumeSelectedRunDependencyReadiness(): void {
    const run = selectedRunRecord;
    if (!run || !schedulerRunSupportsDependencyReadinessResume(run)) return;
    void performAction('dependency-readiness-resume', run, 'Dependency readiness resume accepted by scheduler', () =>
      workflowService.resumeWorkflowExecutionSessionRuntimeDependencyReadiness({
        session_id: run.workflow_execution_session_id as string,
        workflow_run_id: run.workflow_run_id,
      }),
    );
  }

  function adminCancelSelectedRun(): void {
    const run = selectedRunRecord;
    if (!run || !schedulerRunSupportsAdminQueueControls(run)) return;
    void performAction('admin-cancel', run, 'Admin cancel accepted by scheduler', () =>
      workflowService.adminCancelQueueItem({ workflow_run_id: run.workflow_run_id }),
    );
  }

  function adminPushSelectedRunToFront(): void {
    const run = selectedRunRecord;
    if (!run || !schedulerRunSupportsAdminQueueControls(run)) return;
    void performAction('admin-front', run, 'Admin push-front accepted by scheduler', () =>
      workflowService.adminPushQueueItemToFront({ workflow_run_id: run.workflow_run_id }),
    );
  }

  function adminReprioritizeSelectedRun(): void {
    const run = selectedRunRecord;
    if (!run || !schedulerRunSupportsAdminQueueControls(run)) return;
    const priority = parsePriority(adminPriorityInput);
    if (priority === null) return;
    void performAction('admin-priority', run, 'Admin priority accepted by scheduler', () =>
      workflowService.adminReprioritizeQueueItem({ workflow_run_id: run.workflow_run_id, priority }),
    );
  }

  // The run list is kept current by the shell (startRunContextSync). This page
  // owns only the selected run's detail projections: one workflow-event
  // listener and one invalidation subscription, both released on destroy.
  onMount(() => {
    let disposed = false;
    let unlistenProjections: (() => void) | null = null;
    const unsubscribeEvents = workflowService.subscribeEvents(() => {
      refreshDetail();
    });
    subscribeDiagnosticsProjectionInvalidations({
      projections: ['scheduler_timeline', 'run_detail', 'io_artifact'],
      getActiveRunId: activeRunId,
      refresh: (event) => refreshFromProjectionInvalidation(event.projection_kind),
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        unlistenProjections = nextUnlisten;
        detailLiveUpdates = 'live';
      })
      .catch(() => {
        if (!disposed) detailLiveUpdates = 'unavailable';
      });

    return () => {
      disposed = true;
      unsubscribeEvents();
      unlistenProjections?.();
      unlistenProjections = null;
    };
  });

  // A new active run (from a row here, the header picker or a paste) reloads
  // the detail projections; older responses are dropped by the request serials.
  $effect(() => {
    const runId = activeRunIdValue;
    if (runId === activeTimelineRunId) {
      return;
    }

    activeTimelineRunId = runId;
    timelineKindFilter = 'all';
    timelineSourceFilter = 'all';
    void refreshTimeline(runId);
    void refreshEstimate(runId);
    void refreshRetentionSummary(runId);
  });

  // Priority inputs start from the selected run's projected priority; action
  // feedback belongs to the run it was about.
  $effect(() => {
    const run = selectedRunRecord;
    const runId = run?.workflow_run_id ?? activeRunIdValue ?? '';
    if (runId === selectionSyncRunId) {
      return;
    }
    selectionSyncRunId = runId;
    sessionPriorityInput = String(run?.scheduler_priority ?? 0);
    adminPriorityInput = String(run?.scheduler_priority ?? 0);
    actionError = null;
    actionMessage = null;
    moreActionsOpen = false;
  });
</script>

<div class="pg-main scheduler">
  <DimensionLine label={queueSummary} />

  {#if $schedulerViewPreferences.showLanes}
    <section class="pg-lanes" aria-label="Device memory">
      <span class="pg-mono pg-muted lanes-scope">local</span>
      <span class="pg-muted pg-small">{DEVICE_MEMORY_UNAVAILABLE}</span>
    </section>
  {/if}

  <div class="inbox">
    <Panel label="Runs" width="380px">
      {#snippet top()}
        <div class="search-row">
          <label for="scheduler-run-search" class="pg-sr-only">Search runs</label>
          <input
            id="scheduler-run-search"
            class="pg-field search"
            type="search"
            placeholder="Search runs"
            value={$schedulerRunFilters.search}
            oninput={(event) => setSchedulerRunFilters({ search: eventValue(event) })}
          />
          <button
            type="button"
            class="pg-btn"
            aria-expanded={filtersOpen}
            aria-controls={filtersOpen ? 'scheduler-filters' : undefined}
            onclick={() => (filtersOpen = !filtersOpen)}
          >
            Filters{#if filterChips.length > 0}<span class="pg-n">{filterChips.length}</span>{/if}
          </button>
          <button
            type="button"
            class="pg-btn icon-btn"
            aria-label="Refresh runs"
            title={listRefreshTitle}
            aria-disabled={listLoading ? 'true' : undefined}
            onclick={() => {
              if (!listLoading) void refreshRunContextRunList();
            }}
          >
            <RefreshCw size={14} aria-hidden="true" />
          </button>
        </div>
        <SegmentedControl
          label="Status"
          options={statusSegments}
          value={$schedulerRunFilters.statusSegment}
          onchange={(segment) => setSchedulerRunFilters({ statusSegment: segment })}
        />
        {#if filterChips.length > 0}
          <div class="chips" role="group" aria-label="Applied filters">
            {#each filterChips as chip (chip.key)}
              <FilterChip label={chip.label} onclear={() => setFilter(chip.key, DEFAULT_SCHEDULER_RUN_FILTERS[chip.key])} />
            {/each}
            {#if filterChips.length > 1}
              <button type="button" class="pg-tgl clear-all" onclick={clearSchedulerSecondaryFilters}>Clear all</button>
            {/if}
          </div>
        {/if}
        {#if filtersOpen}
          <div id="scheduler-filters" class="filters">
            {#each filterFields as field (field.key)}
              <label class="pg-fl" for={field.id}>
                {field.label}
                <select
                  id={field.id}
                  class="pg-field"
                  value={field.value}
                  onchange={(event) => setFilter(field.key, eventValue(event))}
                >
                  {#each field.options as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
            {/each}
          </div>
        {/if}
        {#if liveUpdatesUnavailable}
          <div class="pg-muted pg-small">Live updates are unavailable here; use Refresh to update.</div>
        {/if}
      {/snippet}

      {#if !listLoaded}
        {#if $runContextRunList.errorMessage}
          <div class="list-note">
            <ErrorCard
              title="The run list did not load"
              explanation={$runContextRunList.errorMessage}
              suggestion="Retry; if it keeps failing, check the engine log."
              actionLabel="Retry"
              onaction={() => void refreshRunContextRunList()}
            />
          </div>
        {:else if listUnreachable}
          <div class="pg-empty">
            <p class="empty-text">The run list is unavailable because the engine did not answer.</p>
            <button type="button" class="pg-btn pg-btn-sm" onclick={() => void refreshRunContextRunList()}>Retry</button>
          </div>
        {:else}
          <div class="pg-empty">Loading runs…</div>
        {/if}
      {:else}
        {#if $runContextRunList.errorMessage}
          <div class="list-note">
            <ErrorCard
              title="The last run list refresh failed"
              explanation={$runContextRunList.errorMessage}
              suggestion="The list shows the previous projection. Retry, or check the engine log."
              actionLabel="Retry"
              onaction={() => void refreshRunContextRunList()}
            />
          </div>
        {/if}
        {#if runs.length === 0}
          <div class="pg-empty">No runs yet. Runs appear here once a workflow is submitted.</div>
        {:else if runGroups.length === 0}
          <div class="pg-empty">
            <p class="empty-text">No runs match the search and filters.</p>
            <button type="button" class="pg-btn pg-btn-sm" onclick={resetSchedulerRunFilters}>Clear search and filters</button>
          </div>
        {:else}
          {#each runGroups as group (group.id)}
            {@const expanded = !collapsedGroups[group.id]}
            <GroupHeader
              label={group.title}
              count={group.rows.length}
              {expanded}
              controls={expanded ? `scheduler-group-${group.id}` : undefined}
              ontoggle={() => toggleGroup(group.id)}
            />
            {#if expanded}
              <div id={`scheduler-group-${group.id}`}>
                {#each group.rows as row (row.runId)}
                  <ListRow
                    glyph={row.glyph}
                    name={row.workflowId}
                    version={row.versionLabel}
                    statusLabel={row.statusLabel}
                    statusDanger={row.statusIsFailure}
                    context={row.contextSuffix}
                    detail={row.detail ?? undefined}
                    time={row.time}
                    selected={row.selected}
                    runId={row.runId}
                    onselect={() => selectRunContext(row.record)}
                  />
                {/each}
              </div>
            {/if}
          {/each}
        {/if}
      {/if}

      {#snippet footer()}
        <div class="foot">
          {#if footerState.kind === 'none'}
            <span class="pg-muted pg-small">{NO_SELECTION_FOOTER}</span>
          {:else if footerState.kind === 'missing'}
            <span class="pg-muted pg-small">This run is not in the loaded run list, so no queue actions are offered.</span>
          {:else if footerState.kind === 'finished'}
            <span class="pg-muted pg-small">{footerState.workflowId} has finished. No actions available.</span>
          {:else}
            <div class="foot-row">
              <button
                type="button"
                class="pg-btn pg-btn-sm"
                disabled={queueControlsDisabled}
                title={queueControlsReason ?? 'Push this run to the front of its session queue'}
                onclick={pushSelectedRunToFront}
              >
                Move to front
              </button>
              <label class="pg-muted pg-small priority">
                Priority
                <input
                  class="pg-field priority-input"
                  type="number"
                  step="1"
                  value={sessionPriorityInput}
                  oninput={(event) => {
                    sessionPriorityInput = eventValue(event);
                  }}
                  disabled={queueControlsDisabled}
                  title={queueControlsReason ?? undefined}
                />
              </label>
              <button
                type="button"
                class="pg-btn pg-btn-sm"
                disabled={queueControlsDisabled}
                title={queueControlsReason ?? 'Set this run’s priority within its session queue'}
                onclick={reprioritizeSelectedRun}
              >
                Set
              </button>
            </div>
            <div class="foot-row">
              <button
                type="button"
                class="pg-tgl"
                aria-expanded={moreActionsOpen}
                aria-controls={moreActionsOpen ? 'scheduler-more-actions' : undefined}
                onclick={() => (moreActionsOpen = !moreActionsOpen)}
              >
                More actions <span aria-hidden="true">{moreActionsOpen ? '▾' : '▸'}</span>
              </button>
              <TwoStepButton
                class="push-right"
                label="Cancel run"
                confirmLabel="Confirm cancel"
                onconfirm={cancelSelectedRun}
                disabled={queueControlsDisabled}
                disabledReason={queueControlsReason ?? 'Another scheduler action is in progress'}
              />
            </div>
            <!-- Stays mounted while a live run is selected so each outcome is announced. -->
            <div
              class="pg-small action-feedback"
              class:pg-sr-only={!actionFeedback}
              class:action-error={Boolean(actionError) && !actionBusy}
              role="status"
              title={actionFeedback ?? undefined}
            >
              {actionFeedback ?? ''}
            </div>
            {#if moreActionsOpen}
              <div id="scheduler-more-actions" class="more">
                <div class="foot-row">
                  <button
                    type="button"
                    class="pg-btn pg-btn-sm"
                    disabled={resumeReason !== null || actionBusy !== null}
                    title={resumeReason ?? 'Resume this run once its runtime dependencies are ready'}
                    onclick={resumeSelectedRunDependencyReadiness}
                  >
                    Resume
                  </button>
                  <span class="pg-muted pg-small">Dependency readiness</span>
                </div>
                <div class="pg-muted pg-small">GUI admin, acts by run id across sessions</div>
                <div class="foot-row">
                  <TwoStepButton
                    label="Admin cancel"
                    confirmLabel="Confirm admin cancel"
                    onconfirm={adminCancelSelectedRun}
                    disabled={adminControlsDisabled}
                    disabledReason={adminControlsReason ?? 'Another scheduler action is in progress'}
                  />
                  <button
                    type="button"
                    class="pg-btn pg-btn-sm"
                    disabled={adminControlsDisabled}
                    title={adminControlsReason ?? 'Admin: push this run to the front by run id'}
                    onclick={adminPushSelectedRunToFront}
                  >
                    Front
                  </button>
                  <label class="pg-muted pg-small priority">
                    Priority
                    <input
                      class="pg-field priority-input"
                      type="number"
                      step="1"
                      value={adminPriorityInput}
                      oninput={(event) => {
                        adminPriorityInput = eventValue(event);
                      }}
                      disabled={adminControlsDisabled}
                      title={adminControlsReason ?? undefined}
                    />
                  </label>
                  <button
                    type="button"
                    class="pg-btn pg-btn-sm"
                    disabled={adminControlsDisabled}
                    title={adminControlsReason ?? 'Admin: set this run’s priority by run id'}
                    onclick={adminReprioritizeSelectedRun}
                  >
                    Set
                  </button>
                </div>
              </div>
            {/if}
          {/if}
        </div>
      {/snippet}
    </Panel>

    <Panel label="Run detail">
      {#if $activeWorkflowRun}
        {@const runId = $activeWorkflowRun.workflow_run_id}
        <div class="head">
          <div class="head-main">
            <div class="pg-title">
              {selectedRunRecord?.workflow_id ?? $activeWorkflowRun.workflow_id ?? runId}
              {#if detailVersion}<span class="pg-muted">{detailVersion}</span>{/if}
            </div>
            <div class="pg-mono pg-muted run-id" data-run-id={runId} title="Ctrl+C copies the run id">{runId}</div>
            <div class="chips">
              <Chip glyph={glyphStateForStatus(detailStatus)}>{runContextStatusLabel(detailStatus)}</Chip>
              {#each detailChips as chip (chip)}
                <Chip>{chip}</Chip>
              {/each}
            </div>
          </div>
          <div class="head-actions">
            <button type="button" class="pg-btn" onclick={() => openPage('diagnostics')}>Diagnostics</button>
            <button type="button" class="pg-btn" onclick={() => openPage('graph')}>Graph</button>
            <button type="button" class="pg-btn" onclick={() => openPage('io_inspector')}>I/O</button>
          </div>
        </div>

        {#if selectedRunRecord}
          <div class="cells">
            <div class="pg-sec cell">
              <h3>Progress</h3>
              <div>{schedulerRunProgressText(selectedRunRecord)}</div>
              <div class="pg-muted pg-small">
                {schedulerEstimateSentence(selectedRunEstimate, { loading: estimateLoading, error: estimateError })}
              </div>
            </div>
            <div class="pg-sec cell">
              <h3>Placement</h3>
              <div>{schedulerRunPlacementText(selectedRunRecord)}</div>
              <div class="pg-muted pg-small">{schedulerRunPolicyText(selectedRunRecord)}</div>
            </div>
            <div class="pg-sec cell">
              <h3>Retention</h3>
              <div>
                {schedulerRetentionText(selectedRunRetentionSummary, { loading: retentionLoading, error: retentionError })}
              </div>
              <div class="pg-muted pg-small">{schedulerRetentionPolicyText(selectedRunRecord)}</div>
            </div>
          </div>
        {:else}
          <div class="pg-sec pg-muted pg-small">
            This run is not in the loaded run list, so its queue and placement facts are not shown. Its scheduler
            timeline is below.
          </div>
        {/if}

        <section class="pg-sec" aria-labelledby="scheduler-timeline-heading">
          <div class="timeline-head">
            <h3 id="scheduler-timeline-heading">Scheduler timeline</h3>
            <label class="pg-sr-only" for="scheduler-timeline-kind">Event kind</label>
            <select
              id="scheduler-timeline-kind"
              class="pg-field timeline-filter"
              value={timelineKindFilter}
              onchange={(event) => {
                timelineKindFilter = eventValue(event);
              }}
              disabled={timelineEvents.length === 0}
            >
              <option value="all">All kinds</option>
              {#each timelineKindOptions as eventKind (eventKind)}
                <option value={eventKind}>{formatSchedulerTimelineKind({ event_kind: eventKind })}</option>
              {/each}
            </select>
            <label class="pg-sr-only" for="scheduler-timeline-source">Event source</label>
            <select
              id="scheduler-timeline-source"
              class="pg-field timeline-filter"
              value={timelineSourceFilter}
              onchange={(event) => {
                timelineSourceFilter = eventValue(event);
              }}
              disabled={timelineEvents.length === 0}
            >
              <option value="all">All sources</option>
              {#each timelineSourceOptions as sourceComponent (sourceComponent)}
                <option value={sourceComponent}>{formatSchedulerTimelineSource({ source_component: sourceComponent })}</option>
              {/each}
            </select>
            <button
              type="button"
              class="pg-tgl"
              aria-disabled={detailLoading ? 'true' : undefined}
              onclick={() => {
                if (!detailLoading) refreshDetail();
              }}
            >
              {detailLoading ? 'Refreshing' : 'Refresh'}
            </button>
          </div>

          {#if timelineError}
            <ErrorCard
              title="The scheduler timeline did not load"
              explanation={timelineError}
              suggestion="Refresh; if it keeps failing, open Diagnostics for this run."
              actionLabel="Refresh"
              onaction={refreshDetail}
            />
          {:else if timelineLoading && timelineEvents.length === 0}
            <div class="pg-muted pg-small">Loading the scheduler timeline…</div>
          {:else if timelineEvents.length === 0}
            <div class="pg-muted pg-small">No scheduler events are projected for this run yet.</div>
          {:else if displayedTimelineEvents.length === 0}
            <div class="pg-muted pg-small">No events match the kind and source filters.</div>
          {:else}
            <div class="ev">
              {#each displayedTimelineEvents as event (event.event_id)}
                {@const attempt = schedulerTimelineAttemptText(event)}
                <span class="ev-t" title={formatSchedulerTimestamp(event.occurred_at_ms)}>
                  {formatSchedulerClockTime(event.occurred_at_ms)}
                </span>
                <span class="ev-m">
                  {event.summary}
                  <span class="pg-muted ev-src">
                    {formatSchedulerTimelineKind(event)}, {formatSchedulerTimelineSource(event)}
                  </span>
                  {#if event.detail}<span class="pg-muted pg-small ev-line">{event.detail}</span>{/if}
                  {#if attempt}<span class="pg-mono pg-muted ev-line ev-attempt">{attempt}</span>{/if}
                </span>
              {/each}
            </div>
          {/if}
          <div class="pg-muted pg-small timeline-foot">
            Node-level events for this run are in
            <button type="button" class="linkish" onclick={() => openPage('diagnostics')}>Diagnostics</button>.
          </div>
          <div class="pg-muted pg-small">{detailFreshness}</div>
        </section>
      {:else}
        <div class="pg-empty">Select a run to see its progress, placement and timeline.</div>
      {/if}
    </Panel>
  </div>
</div>

<style>
  .scheduler {
    height: 100%;
    box-sizing: border-box;
  }

  .lanes-scope {
    font-size: 11px;
    flex: none;
  }

  .inbox {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
    align-items: stretch;
  }

  .search-row {
    display: flex;
    gap: 8px;
  }

  .search {
    flex: 1;
    min-width: 0;
  }

  .icon-btn {
    width: 30px;
    padding: 0;
    justify-content: center;
    flex: none;
  }

  .chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }

  .clear-all {
    height: 24px;
  }

  .filters {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px 10px;
    padding: 10px;
    border: 1px dashed var(--pg-border);
    border-radius: 4px;
    max-height: 32vh;
    overflow: auto;
  }

  .list-note {
    padding: 10px 12px;
  }

  .empty-text {
    margin: 0 0 10px;
  }

  .foot {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    min-width: 0;
  }

  .foot-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 8px;
    align-items: center;
    min-width: 0;
  }

  .foot-row :global(.push-right) {
    margin-left: auto;
  }

  .priority {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .priority-input {
    width: 54px;
    height: 28px;
    padding: 0 6px;
  }

  .action-feedback {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--pg-text-muted);
  }

  .action-feedback.action-error {
    color: var(--pg-danger);
  }

  .more {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--pg-border-soft);
  }

  .head {
    padding: 16px 18px 14px;
    display: flex;
    gap: 16px;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .head-main {
    flex: 1 1 260px;
    min-width: 0;
  }

  .run-id {
    margin-top: 2px;
    overflow-wrap: anywhere;
  }

  .head-main .chips {
    margin-top: 10px;
  }

  .head-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }

  .cells {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(220px, 100%), 1fr));
    border-top: 1px solid var(--pg-border);
  }

  .cell {
    border-top: 0;
  }

  .cell + .cell {
    border-left: 1px solid var(--pg-border);
  }

  .timeline-head {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    margin-bottom: 8px;
  }

  .timeline-head h3 {
    margin: 0 auto 0 0;
  }

  .timeline-filter {
    height: 26px;
    font-size: 12.5px;
    max-width: 180px;
  }

  .ev {
    display: grid;
    grid-template-columns: 66px 1fr;
    gap: 5px 14px;
    font-size: 13.5px;
  }

  .ev-t {
    font-family: var(--pg-font-mono);
    font-size: 12px;
    color: var(--pg-text-muted);
    padding-top: 1px;
  }

  .ev-m {
    min-width: 0;
  }

  .ev-src {
    font-size: 12px;
    margin-left: 4px;
  }

  .ev-line {
    display: block;
  }

  .ev-attempt {
    font-size: 11.5px;
    overflow-wrap: anywhere;
  }

  .timeline-foot {
    margin-top: 10px;
  }

  .linkish {
    border: 0;
    padding: 0;
    background: none;
    color: var(--pg-text);
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
    font: inherit;
  }
</style>

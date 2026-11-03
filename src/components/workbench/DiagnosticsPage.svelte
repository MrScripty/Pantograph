<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    IoArtifactRetentionSummaryRecord,
    NodeStatusProjectionRecord,
    ProjectionStateRecord,
    RunDetailProjectionRecord,
    RunListFacetRecord,
    RunListProjectionRecord,
    SchedulerTimelineProjectionRecord,
  } from '../../services/diagnostics/types';
  import { subscribeDiagnosticsProjectionInvalidations } from '../../services/workflow/WorkflowProjectionSubscriptionService';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun, diagnosticsFocus, setWorkbenchPage } from '../../stores/workbenchStore';
  import { openRunContextPicker } from './shell/runContextStore';
  import {
    Chip,
    DimensionLine,
    EmptyState,
    ErrorCard,
    FilterChip,
    GroupHeader,
    KeyValue,
    LogLine,
    Panel,
    SegmentedControl,
    glyphStateForStatus,
  } from './ui';
  import type {
    DiagnosticsComparisonFilters,
    DiagnosticsExecutionFilters,
    DiagnosticsLevelFilter,
    DiagnosticsSourceFilter,
  } from './diagnosticsPagePresenters';
  import {
    DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
    DEFAULT_DIAGNOSTICS_EXECUTION_FILTERS,
    DIAGNOSTICS_COMPARISON_FILTER_FIELDS,
    DIAGNOSTICS_EXECUTION_FILTER_FIELDS,
    DIAGNOSTICS_FILTER_ALL,
    DIAGNOSTICS_SOURCE_OPTIONS,
    EMPTY_DIAGNOSTICS_COMPARISON_FILTER_OPTIONS,
    EMPTY_DIAGNOSTICS_EXECUTION_FILTER_OPTIONS,
    buildDiagnosticsClientLabel,
    buildDiagnosticsComparisonFilterChips,
    buildDiagnosticsComparisonFilterOptions,
    buildDiagnosticsComparisonShortcuts,
    buildDiagnosticsComparisonSummary,
    buildDiagnosticsDimensionLine,
    buildDiagnosticsErrorCard,
    buildDiagnosticsExecutionFacetRows,
    buildDiagnosticsExecutionFilterChips,
    buildDiagnosticsExecutionFilterOptions,
    buildDiagnosticsFacetSummary,
    buildDiagnosticsFactSections,
    buildDiagnosticsLevelOptions,
    buildDiagnosticsLogLines,
    buildDiagnosticsNodeErrorsOutsideTimeline,
    buildDiagnosticsNodeStatusBars,
    buildDiagnosticsPlacementLabel,
    buildDiagnosticsProjectionRows,
    buildDiagnosticsRetentionRows,
    createDiagnosticsRefreshGate,
    defaultDiagnosticsComparisonFilterValue,
    diagnosticsFocusKey,
    diagnosticsNodeErrorEventLabel,
    diagnosticsSentenceCase,
    filterDiagnosticsComparisonRuns,
    filterDiagnosticsExecutionNodes,
    filterDiagnosticsLogLines,
    formatDiagnosticErrorPhase,
    formatDiagnosticErrorSeverity,
    formatDiagnosticsComparisonOption,
    formatDiagnosticsStatusLabel,
    groupDiagnosticsExecutionFacetRows,
    hasActiveDiagnosticsComparisonFilters,
    resolveDiagnosticsFocus,
  } from './diagnosticsPagePresenters';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

  // Projection data, replaced together by each successful refresh.
  let runDetail = $state<RunDetailProjectionRecord | null>(null);
  let runList = $state<RunListProjectionRecord[]>([]);
  let runListFacets = $state<RunListFacetRecord[]>([]);
  let nodeStatuses = $state<NodeStatusProjectionRecord[]>([]);
  let retentionSummary = $state<IoArtifactRetentionSummaryRecord[]>([]);
  let timelineEvents = $state<SchedulerTimelineProjectionRecord[]>([]);
  let runDetailProjectionState = $state<ProjectionStateRecord | null>(null);
  let runListProjectionState = $state<ProjectionStateRecord | null>(null);
  let nodeStatusProjectionState = $state<ProjectionStateRecord | null>(null);
  let ioProjectionState = $state<ProjectionStateRecord | null>(null);
  let timelineProjectionState = $state<ProjectionStateRecord | null>(null);
  /** Run id the projection data above belongs to, or null after a clear. */
  let loadedRunId = $state<string | null>(null);
  /** Wall clock of the last applied refresh; formats "today" times without a timer. */
  let loadedAtMs = $state(Date.now());
  let loading = $state(false);
  let refreshError = $state<string | null>(null);
  let liveUpdatesError = $state<string | null>(null);

  // Presentation state.
  let comparisonFilters = $state<DiagnosticsComparisonFilters>({ ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS });
  let executionFilters = $state<DiagnosticsExecutionFilters>({ ...DEFAULT_DIAGNOSTICS_EXECUTION_FILTERS });
  let levelFilter = $state<DiagnosticsLevelFilter>('all');
  let sourceFilter = $state<DiagnosticsSourceFilter>('all');
  let searchQuery = $state('');
  let follow = $state(true);
  let nodesOpen = $state(true);
  let nodeFiltersOpen = $state(false);
  let breakdownOpen = $state(false);
  let comparisonOpen = $state(true);
  let comparisonFiltersOpen = $state(false);
  let moreFactsOpen = $state(false);
  /** Error line selected by "Show in log", scoped to the run it was chosen on. */
  let shownError = $state<{ runId: string; eventId: string } | null>(null);
  /** Line to scroll into view once it has rendered. */
  let pendingScrollDomId = $state<string | null>(null);
  let logElement = $state<HTMLDivElement | null>(null);
  /** Focus request already applied; each `diagnosticsFocus` request switches filters once. */
  let appliedFocusKey: string | null = null;

  const refreshGate = createDiagnosticsRefreshGate();

  let activeRunId = $derived($activeWorkflowRun?.workflow_run_id ?? null);
  /** The loaded run detail, only while it belongs to the selected run. */
  let run = $derived(runDetail && loadedRunId === activeRunId ? runDetail : null);

  let factSections = $derived(run ? buildDiagnosticsFactSections(run, loadedAtMs) : null);
  let retentionRows = $derived(buildDiagnosticsRetentionRows(retentionSummary));
  let projectionRows = $derived(
    buildDiagnosticsProjectionRows({
      runDetail: runDetailProjectionState,
      timeline: timelineProjectionState,
      runList: runListProjectionState,
      nodeStatus: nodeStatusProjectionState,
      io: ioProjectionState,
    }),
  );
  let placementLabel = $derived(run ? buildDiagnosticsPlacementLabel(run) : '');
  let clientLabel = $derived(run ? buildDiagnosticsClientLabel(run) : null);
  let dimensionLabel = $derived(
    run ? buildDiagnosticsDimensionLine(run, nodeStatuses, loadedAtMs) : (activeRunId ?? 'no run selected'),
  );

  let executionFilterOptions = $derived(
    nodeStatuses.length > 0 ? buildDiagnosticsExecutionFilterOptions(nodeStatuses) : EMPTY_DIAGNOSTICS_EXECUTION_FILTER_OPTIONS,
  );
  let filteredExecutionNodes = $derived(filterDiagnosticsExecutionNodes(nodeStatuses, executionFilters));
  let nodeStatusBars = $derived(buildDiagnosticsNodeStatusBars(filteredExecutionNodes));
  let executionBreakdown = $derived(
    groupDiagnosticsExecutionFacetRows(buildDiagnosticsExecutionFacetRows(filteredExecutionNodes)),
  );
  let executionChips = $derived(buildDiagnosticsExecutionFilterChips(executionFilters));

  let comparisonFilterOptions = $derived(
    run ? buildDiagnosticsComparisonFilterOptions(run, runList) : EMPTY_DIAGNOSTICS_COMPARISON_FILTER_OPTIONS,
  );
  let filteredComparisonRuns = $derived(run ? filterDiagnosticsComparisonRuns(run, runList, comparisonFilters) : []);
  let hasComparisonFilters = $derived(hasActiveDiagnosticsComparisonFilters(comparisonFilters));
  let facetSummary = $derived(
    run ? buildDiagnosticsFacetSummary(run, filteredComparisonRuns, hasComparisonFilters ? [] : runListFacets) : null,
  );
  let comparisonSummary = $derived(run ? buildDiagnosticsComparisonSummary(run, filteredComparisonRuns) : null);
  let comparisonChips = $derived(buildDiagnosticsComparisonFilterChips(comparisonFilters));
  let comparisonShortcuts = $derived(run ? buildDiagnosticsComparisonShortcuts(run, comparisonFilters, loadedAtMs) : []);

  let logLines = $derived(buildDiagnosticsLogLines(timelineEvents, nodeStatuses));
  let levelOptions = $derived(buildDiagnosticsLevelOptions(logLines));
  let visibleLines = $derived(
    filterDiagnosticsLogLines(logLines, { level: levelFilter, source: sourceFilter, query: searchQuery }),
  );
  let errorCard = $derived(run ? buildDiagnosticsErrorCard(run, nodeStatuses) : null);
  let nodeErrorsOutsideTimeline = $derived(buildDiagnosticsNodeErrorsOutsideTimeline(filteredExecutionNodes, timelineEvents));

  let focusResolution = $derived(
    run ? resolveDiagnosticsFocus($diagnosticsFocus, run.workflow_run_id, logLines, nodeStatuses) : null,
  );
  let selectedEventId = $derived(
    shownError && shownError.runId === activeRunId ? shownError.eventId : (focusResolution?.eventId ?? null),
  );
  let focusedNodeId = $derived(focusResolution?.nodeId ?? null);

  function clearProjection(): void {
    runDetail = null;
    runList = [];
    runListFacets = [];
    nodeStatuses = [];
    retentionSummary = [];
    timelineEvents = [];
    runDetailProjectionState = null;
    runListProjectionState = null;
    nodeStatusProjectionState = null;
    ioProjectionState = null;
    timelineProjectionState = null;
    loadedRunId = null;
  }

  async function refreshDiagnostics(runId = activeRunId): Promise<void> {
    const token = refreshGate.begin();
    if (!runId) {
      clearProjection();
      refreshError = null;
      loading = false;
      return;
    }

    loading = true;
    try {
      const [runResponse, timelineResponse] = await Promise.all([
        workflowService.queryRunDetail({ workflow_run_id: runId }),
        workflowService.querySchedulerTimeline({ workflow_run_id: runId, limit: 250 }),
      ]);
      if (!refreshGate.isCurrent(token)) return;
      const selectedRun = runResponse.run ?? null;
      const selectedRunResponses = selectedRun
        ? await Promise.all([
            workflowService.queryRunList({ workflow_id: selectedRun.workflow_id, limit: 250 }),
            workflowService.queryNodeStatus({ workflow_run_id: selectedRun.workflow_run_id, limit: 250 }),
            workflowService.queryIoArtifacts({ workflow_run_id: selectedRun.workflow_run_id, limit: 1 }),
          ])
        : null;
      if (!refreshGate.isCurrent(token)) return;
      runDetail = selectedRun;
      runDetailProjectionState = runResponse.projection_state;
      runList = selectedRunResponses?.[0].runs ?? [];
      runListFacets = selectedRunResponses?.[0].facets ?? [];
      runListProjectionState = selectedRunResponses?.[0].projection_state ?? null;
      nodeStatuses = selectedRunResponses?.[1].nodes ?? [];
      nodeStatusProjectionState = selectedRunResponses?.[1].projection_state ?? null;
      retentionSummary = selectedRunResponses?.[2].retention_summary ?? [];
      ioProjectionState = selectedRunResponses?.[2].projection_state ?? null;
      timelineEvents = timelineResponse.events;
      timelineProjectionState = timelineResponse.projection_state;
      loadedRunId = runId;
      loadedAtMs = Date.now();
      refreshError = null;
    } catch (failure) {
      if (!refreshGate.isCurrent(token)) return;
      refreshError = formatWorkflowCommandError(failure);
      // Keep the last projection of this run on screen (the shell dims the page
      // when the engine is unreachable); never show another run's projection.
      if (loadedRunId !== runId) clearProjection();
    } finally {
      if (refreshGate.isCurrent(token)) loading = false;
    }
  }

  function updateComparisonFilter(field: keyof DiagnosticsComparisonFilters, value: string): void {
    comparisonFilters = { ...comparisonFilters, [field]: value };
  }

  function updateExecutionFilter(field: keyof DiagnosticsExecutionFilters, value: string): void {
    executionFilters = { ...executionFilters, [field]: value };
  }

  function controlValue(event: Event): string {
    return (event.currentTarget as HTMLSelectElement | HTMLInputElement).value;
  }

  function showErrorInLog(): void {
    levelFilter = 'error';
    sourceFilter = 'all';
    searchQuery = '';
    follow = false;
    const eventId = errorCard?.eventId ?? null;
    const line = eventId ? logLines.find((candidate) => candidate.eventId === eventId) : undefined;
    shownError = line && activeRunId ? { runId: activeRunId, eventId: line.eventId } : null;
    pendingScrollDomId = line?.domId ?? null;
  }

  $effect(() => {
    void refreshDiagnostics(activeRunId);
  });

  // Apply each diagnostics focus request once, after its run's projection has
  // loaded: show the focused line's level, stop following, then scroll to it.
  // The store clears the focus when another run is selected.
  $effect(() => {
    const focus = $diagnosticsFocus;
    const resolution = focusResolution;
    if (!focus || !resolution || loading) return;
    const key = diagnosticsFocusKey(focus);
    if (key === appliedFocusKey) return;
    appliedFocusKey = key;
    shownError = null;
    levelFilter = resolution.level;
    sourceFilter = 'all';
    searchQuery = '';
    follow = false;
    pendingScrollDomId = resolution.lineDomId;
  });

  $effect(() => {
    const domId = pendingScrollDomId;
    const container = logElement;
    void visibleLines;
    if (!domId || !container) return;
    const target = container.querySelector(`#${domId}`);
    if (!target) return;
    target.scrollIntoView({ block: 'center' });
    pendingScrollDomId = null;
  });

  // Follow keeps the newest event in view as the timeline grows.
  $effect(() => {
    const container = logElement;
    void visibleLines;
    if (!follow || !container || pendingScrollDomId) return;
    container.scrollTop = container.scrollHeight;
  });

  onMount(() => {
    let unlisten: (() => void) | null = null;
    let disposed = false;
    const recordLiveUpdatesError = (failure: unknown) => {
      if (!disposed) liveUpdatesError = formatWorkflowCommandError(failure);
    };
    void subscribeDiagnosticsProjectionInvalidations({
      projections: ['run_detail', 'run_list', 'node_status', 'io_artifact', 'scheduler_timeline'],
      getActiveRunId: () => activeRunId,
      refresh: () => refreshDiagnostics(),
      onRefreshError: recordLiveUpdatesError,
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        unlisten = nextUnlisten;
        liveUpdatesError = null;
      })
      .catch(recordLiveUpdatesError);

    return () => {
      disposed = true;
      refreshGate.close();
      unlisten?.();
    };
  });
</script>

{#snippet filterChipRow(chips: { field: string; label: string }[], onclear: (field: string) => void)}
  {#if chips.length > 0}
    <div class="chips">
      {#each chips as chip (chip.field)}
        <FilterChip label={chip.label} onclear={() => onclear(chip.field)} />
      {/each}
    </div>
  {/if}
{/snippet}

<div class="pg-main diagnostics">
  <DimensionLine label={dimensionLabel} />

  {#if !activeRunId}
    <EmptyState
      question="Which run do you want to look into?"
      sentence="Diagnostics shows the events, errors and placement of one run. Pick one here, select it in the Scheduler, or paste a run id."
      actionLabel="Pick a run"
      onaction={() => openRunContextPicker()}
    />
  {:else if !run}
    <section class="pg-panel pending" aria-label="Diagnostics">
      <div class="pending-body">
        {#if loading}
          <p class="pg-muted">Loading diagnostics for <span class="pg-mono" data-run-id={activeRunId}>{activeRunId}</span>.</p>
        {:else if refreshError}
          <p class="pg-muted">
            Diagnostics for <span class="pg-mono" data-run-id={activeRunId}>{activeRunId}</span> could not be loaded. {refreshError}
          </p>
          <button class="pg-btn" type="button" onclick={() => refreshDiagnostics()}>Try again</button>
        {:else}
          <p class="pg-muted">
            The run detail projection has no entry for <span class="pg-mono" data-run-id={activeRunId}>{activeRunId}</span> yet.
          </p>
          <button class="pg-btn" type="button" onclick={() => refreshDiagnostics()}>Refresh</button>
        {/if}
      </div>
    </section>
  {:else}
    <div class="layout">
      <Panel label="Run facts" width="300px">
        <div class="head">
          <div class="pg-subtitle">
            {run.workflow_id}{#if run.workflow_semantic_version}<span class="pg-muted version">v{run.workflow_semantic_version}</span>{/if}
          </div>
          <div class="pg-mono pg-muted runid" data-run-id={run.workflow_run_id} title="Ctrl+C copies the run id">
            {run.workflow_run_id}
          </div>
          <div class="chips">
            <Chip glyph={glyphStateForStatus(run.status)}>{formatDiagnosticsStatusLabel(run.status)}</Chip>
            <Chip>{placementLabel}</Chip>
            {#if clientLabel}<Chip>{clientLabel}</Chip>{/if}
          </div>
        </div>

        {#if factSections}
          <section class="pg-sec sec" aria-labelledby="diag-timing">
            <h3 id="diag-timing">Timing</h3>
            <KeyValue rows={factSections.timing} />
          </section>
          <section class="pg-sec sec" aria-labelledby="diag-placement">
            <h3 id="diag-placement">Placement</h3>
            <KeyValue rows={factSections.placement} />
          </section>
          <section class="pg-sec sec" aria-labelledby="diag-retention">
            <h3 id="diag-retention">Retention</h3>
            <KeyValue rows={[...factSections.retention, ...retentionRows]} />
            {#if retentionRows.length === 0}
              <p class="pg-muted pg-small note">No retained artifact summary for this run.</p>
            {/if}
          </section>
        {/if}

        <GroupHeader
          label="Nodes by status"
          count={filteredExecutionNodes.length}
          expanded={nodesOpen}
          ontoggle={() => (nodesOpen = !nodesOpen)}
          controls="diag-nodes"
        />
        {#if nodesOpen}
          <div id="diag-nodes" class="group">
            {#if nodeStatuses.length === 0}
              <p class="pg-muted pg-small">The node status projection has no rows for this run.</p>
            {:else}
              {#if nodeStatusBars.length === 0}
                <p class="pg-muted pg-small">No node matches these filters.</p>
              {:else}
                <div class="facet">
                  {#each nodeStatusBars as bar (bar.status)}
                    <span class="k">{bar.label}</span><span class="n">{bar.count}</span>
                    <span class="bar" aria-hidden="true"><i style:width="{bar.percent}%"></i></span>
                  {/each}
                </div>
              {/if}
              {@render filterChipRow(executionChips, (field) =>
                updateExecutionFilter(field as keyof DiagnosticsExecutionFilters, DIAGNOSTICS_FILTER_ALL),
              )}
              <div class="toggles">
                <button
                  class="pg-tgl"
                  type="button"
                  aria-expanded={nodeFiltersOpen}
                  aria-controls="diag-node-filters"
                  onclick={() => (nodeFiltersOpen = !nodeFiltersOpen)}
                >
                  Filters{#if executionChips.length > 0}<span class="pg-n">{executionChips.length}</span>{/if}
                </button>
                <button
                  class="pg-tgl"
                  type="button"
                  aria-expanded={breakdownOpen}
                  aria-controls="diag-node-breakdown"
                  onclick={() => (breakdownOpen = !breakdownOpen)}
                >
                  Breakdown
                </button>
              </div>
              {#if nodeFiltersOpen}
                <div id="diag-node-filters" class="filters">
                  {#each DIAGNOSTICS_EXECUTION_FILTER_FIELDS as descriptor (descriptor.field)}
                    <label class="pg-fl">
                      {descriptor.label}
                      <select
                        class="pg-field"
                        value={executionFilters[descriptor.field]}
                        onchange={(event) => updateExecutionFilter(descriptor.field, controlValue(event))}
                      >
                        <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                        {#each executionFilterOptions[descriptor.options] as option (option)}
                          <option value={option}>{option}</option>
                        {/each}
                      </select>
                    </label>
                  {/each}
                </div>
              {/if}
              {#if breakdownOpen}
                <div id="diag-node-breakdown" class="breakdown">
                  {#each executionBreakdown as group (group.label)}
                    <div class="pg-small breakdown-label">{group.label}</div>
                    <dl class="facet-list">
                      {#each group.rows as row (row.value)}
                        <dt class="pg-mono" title={row.value}>{row.value}</dt>
                        <dd class="pg-n">{row.count}</dd>
                      {/each}
                    </dl>
                  {/each}
                </div>
              {/if}
            {/if}
          </div>
        {/if}

        <GroupHeader
          label="Compared with earlier runs"
          expanded={comparisonOpen}
          ontoggle={() => (comparisonOpen = !comparisonOpen)}
          controls="diag-compare"
        />
        {#if comparisonOpen && comparisonSummary}
          <div id="diag-compare" class="group">
            <div class="cmp">
              <div>
                <div class="big">{comparisonSummary.failed}</div>
                <div class="lbl">{comparisonSummary.failedLabel}</div>
              </div>
              <div>
                <div class="big">{comparisonSummary.median}</div>
                <div class="lbl">{comparisonSummary.medianLabel}</div>
              </div>
            </div>
            <p class="pg-muted pg-small note">{comparisonSummary.note}</p>
            {#if facetSummary?.mixedVersionWarning}
              <p class="pg-muted pg-small note">{facetSummary.mixedVersionWarning}</p>
            {/if}
            <div class="chips">
              {#each comparisonShortcuts as shortcut (shortcut.id)}
                <button class="pg-tgl" type="button" aria-pressed={shortcut.pressed} onclick={() => (comparisonFilters = shortcut.toggled)}>
                  {shortcut.label}
                </button>
              {/each}
            </div>
            {@render filterChipRow(comparisonChips, (field) => {
              const key = field as keyof DiagnosticsComparisonFilters;
              updateComparisonFilter(key, defaultDiagnosticsComparisonFilterValue(key));
            })}
            <div class="toggles">
              <button
                class="pg-tgl"
                type="button"
                aria-expanded={comparisonFiltersOpen}
                aria-controls="diag-compare-filters"
                onclick={() => (comparisonFiltersOpen = !comparisonFiltersOpen)}
              >
                Filters{#if comparisonChips.length > 0}<span class="pg-n">{comparisonChips.length}</span>{/if}
              </button>
            </div>
            {#if comparisonFiltersOpen}
              <div id="diag-compare-filters" class="filters">
                {#each DIAGNOSTICS_COMPARISON_FILTER_FIELDS as descriptor (descriptor.field)}
                  <label class="pg-fl">
                    {descriptor.label}
                    {#if descriptor.input === 'date'}
                      <input
                        class="pg-field"
                        type="date"
                        value={comparisonFilters[descriptor.field]}
                        onchange={(event) => updateComparisonFilter(descriptor.field, controlValue(event))}
                      />
                    {:else}
                      <select
                        class="pg-field"
                        value={comparisonFilters[descriptor.field]}
                        onchange={(event) => updateComparisonFilter(descriptor.field, controlValue(event))}
                      >
                        <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                        {#each comparisonFilterOptions[descriptor.options] as option (option)}
                          <option value={option}>{formatDiagnosticsComparisonOption(descriptor.field, option)}</option>
                        {/each}
                      </select>
                    {/if}
                  </label>
                {/each}
              </div>
            {/if}
            {#if facetSummary}
              <dl class="facet-list compare-facets" aria-label="Runs sharing this run's values">
                {#each facetSummary.rows as row (row.label)}
                  <dt>{diagnosticsSentenceCase(row.label)} <span class="pg-mono" title={row.value}>{row.value}</span></dt>
                  <dd class="pg-n">{row.count}/{row.total}</dd>
                {/each}
              </dl>
            {/if}
          </div>
        {/if}

        {#if factSections}
          <GroupHeader
            label="More facts"
            count={factSections.more.length}
            expanded={moreFactsOpen}
            ontoggle={() => (moreFactsOpen = !moreFactsOpen)}
            controls="diag-more"
          />
          {#if moreFactsOpen}
            <div id="diag-more" class="group">
              <KeyValue rows={factSections.more} label="More facts" />
            </div>
          {/if}
        {/if}

        <section class="pg-sec sec" aria-labelledby="diag-projection">
          <h3 id="diag-projection">Projection</h3>
          <KeyValue rows={projectionRows} />
        </section>
      </Panel>

      <section class="pg-panel pg-col events" aria-label="Events">
        <div class="pg-ptop bar">
          <SegmentedControl label="Show" options={levelOptions} value={levelFilter} onchange={(next) => (levelFilter = next)} compact />
          <SegmentedControl
            label="Source"
            options={DIAGNOSTICS_SOURCE_OPTIONS}
            value={sourceFilter}
            onchange={(next) => (sourceFilter = next)}
            compact
          />
          <label class="pg-sr-only" for="diag-search">Search events</label>
          <input id="diag-search" class="pg-field search" type="search" placeholder="Search events" bind:value={searchQuery} />
          <button class="pg-tgl" type="button" aria-pressed={follow} onclick={() => (follow = !follow)} title="Keep the newest event in view">
            Follow
          </button>
          <button class="pg-tgl" type="button" disabled={loading} onclick={() => refreshDiagnostics()}>
            {loading ? 'Refreshing' : 'Refresh'}
          </button>
        </div>

        {#if refreshError || liveUpdatesError}
          <div class="notices pg-small pg-muted" role="status">
            {#if refreshError}<p>Could not refresh; showing the last projection. {refreshError}</p>{/if}
            {#if liveUpdatesError}<p>Live updates are off, so new events appear on Refresh. {liveUpdatesError}</p>{/if}
          </div>
        {/if}

        {#if errorCard?.kind === 'error'}
          <div class="errwrap">
            <ErrorCard
              title={errorCard.title}
              code={errorCard.codeLine}
              explanation={errorCard.explanation}
              suggestion={errorCard.suggestion}
              actionLabel="Show in log"
              onaction={showErrorInLog}
            />
          </div>
        {:else if errorCard}
          <p class="notices pg-small pg-muted">
            Latest warning: {errorCard.title}, {errorCard.codeLine}. {errorCard.explanation}
          </p>
        {/if}

        {#if nodeErrorsOutsideTimeline.length > 0}
          <div class="nodeerrs">
            <table class="pg-tbl">
              <caption class="pg-small pg-muted">Node errors not in the timeline</caption>
              <thead>
                <tr><th scope="col">Node</th><th scope="col">Status</th><th scope="col">Severity</th><th scope="col">Phase</th><th scope="col">Code</th><th scope="col">Event</th></tr>
              </thead>
              <tbody>
                {#each nodeErrorsOutsideTimeline as node (node.node_id)}
                  <tr aria-current={node.node_id === focusedNodeId ? 'true' : undefined}>
                    <td class="pg-mono">{node.node_id}</td>
                    <td>{node.status}</td>
                    <td>{formatDiagnosticErrorSeverity(node.error_severity)}</td>
                    <td>{formatDiagnosticErrorPhase(node.error_phase)}</td>
                    <td class="pg-mono">{node.error_code ?? 'unknown_error'}</td>
                    <td class="pg-mono">{diagnosticsNodeErrorEventLabel(node)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}

        <div class="pg-scroll pg-log log" role="log" aria-label="Run events" bind:this={logElement}>
          {#if timelineEvents.length === 0}
            <p class="pg-empty">The scheduler timeline has no events for this run yet.</p>
          {:else if visibleLines.length === 0}
            <p class="pg-empty">No events match these filters.</p>
          {/if}
          {#each visibleLines as line (line.eventId)}
            <LogLine
              id={line.domId}
              time={line.time}
              level={line.level}
              kind={line.kind}
              message={line.message}
              source={line.source}
              selected={line.eventId === selectedEventId}
            />
          {/each}
          <p class="pg-muted pg-small foot">
            {visibleLines.length} of {logLines.length} events shown. Payload bodies for this run are in
            <button class="link" type="button" onclick={() => setWorkbenchPage('io_inspector')}>I/O Inspector</button>.
          </p>
        </div>
      </section>
    </div>
  {/if}
</div>

<style>
  .diagnostics {
    height: 100%;
  }

  .layout {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
    align-items: stretch;
  }

  .pending {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .pending-body {
    max-width: 460px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .pending-body p {
    margin: 0;
  }

  .head {
    padding: 14px 14px 12px;
  }

  .version {
    margin-left: 6px;
    font-size: 16px;
  }

  .runid {
    margin-top: 2px;
    overflow-wrap: anywhere;
  }

  .chips {
    margin-top: 10px;
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .sec {
    padding: 12px 14px;
  }

  .note {
    margin: 8px 0 0;
  }

  .group {
    padding: 10px 14px 14px;
  }

  .group > p {
    margin: 0;
  }

  .facet {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 3px 12px;
    font-size: 12.5px;
  }

  .facet .k {
    color: var(--pg-text-soft);
  }

  .facet .n {
    font-family: var(--pg-font-mono);
    font-size: 11.5px;
    color: var(--pg-text-muted);
  }

  .facet .bar {
    grid-column: 1 / -1;
    height: 3px;
    background: var(--pg-panel-sunken);
    border-radius: 2px;
    margin-bottom: 4px;
  }

  .facet .bar i {
    display: block;
    height: 100%;
    background: var(--pg-border-strong);
    border-radius: 2px;
  }

  .toggles {
    margin-top: 10px;
    display: flex;
    gap: 6px;
  }

  .toggles .pg-n {
    margin-left: 2px;
    color: inherit;
  }

  .filters {
    margin-top: 10px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px 10px;
  }

  .filters .pg-field {
    min-width: 0;
    font-size: 12px;
    padding: 0 6px;
  }

  .breakdown {
    margin-top: 10px;
  }

  .breakdown-label {
    margin-top: 8px;
    color: var(--pg-text-soft);
    font-weight: 700;
  }

  .facet-list {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 2px 12px;
    margin: 4px 0 0;
    font-size: 12.5px;
  }

  .facet-list dt {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--pg-text-soft);
  }

  .facet-list dt .pg-mono {
    font-size: 11.5px;
    color: var(--pg-text);
  }

  .facet-list dd {
    margin: 0;
    text-align: right;
  }

  .compare-facets {
    margin-top: 12px;
  }

  .cmp {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(120px, 100%), 1fr));
    gap: 12px 18px;
  }

  .cmp .big {
    font-family: var(--pg-font-serif);
    font-size: 22px;
    line-height: 1.1;
    color: var(--pg-text-bright);
  }

  .cmp .lbl {
    margin-top: 2px;
    font-size: 12px;
    color: var(--pg-text-muted);
  }

  .events {
    flex: 1 1 0%;
    min-height: 0;
  }

  .bar {
    flex-direction: row;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .search {
    flex: 1;
    min-width: 140px;
  }

  .notices {
    margin: 10px 12px 0;
  }

  .notices p {
    margin: 0;
  }

  .errwrap {
    margin: 10px 12px 0;
    flex: none;
  }

  .nodeerrs {
    margin: 10px 12px 0;
    max-height: 160px;
    overflow: auto;
    flex: none;
    border: 1px solid var(--pg-border-soft);
    border-radius: 4px;
  }

  .nodeerrs caption {
    text-align: left;
    padding: 6px 12px;
  }

  .nodeerrs tr[aria-current='true'] td {
    background: var(--pg-panel-selected);
  }

  .log {
    margin-top: 10px;
    padding-bottom: 12px;
  }

  .foot {
    margin: 0;
    padding: 10px 12px;
    font-family: var(--pg-font-sans);
  }

  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--pg-text);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
  }
</style>

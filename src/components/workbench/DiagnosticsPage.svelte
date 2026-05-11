<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw } from 'lucide-svelte';
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
  import { activeWorkflowRun, diagnosticsFocus, focusWorkflowDiagnostics } from '../../stores/workbenchStore';
  import type { DiagnosticsComparisonFilters, DiagnosticsExecutionFilters } from './diagnosticsPagePresenters';
  import {
    DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
    DEFAULT_DIAGNOSTICS_EXECUTION_FILTERS,
    DIAGNOSTICS_FILTER_ALL,
    EMPTY_DIAGNOSTICS_COMPARISON_FILTER_OPTIONS,
    EMPTY_DIAGNOSTICS_EXECUTION_FILTER_OPTIONS,
    buildDiagnosticsFacetSummary,
    buildDiagnosticsFactRows,
    buildDiagnosticsRetentionSummaryRows,
    buildDiagnosticsComparisonFilterOptions,
    buildDiagnosticsExecutionFacetRows,
    buildDiagnosticsExecutionFilterOptions,
    buildDiagnosticsRunErrorSummary,
    diagnosticsErrorSeverityClass,
    diagnosticsStatusClass,
    diagnosticsTimelineRowClass,
    filterDiagnosticsExecutionNodes,
    filterDiagnosticsComparisonRuns,
    filterDiagnosticsTimelineEvents,
    formatDiagnosticErrorPhase,
    formatDiagnosticErrorSeverity,
    formatDiagnosticEventKind,
    formatDiagnosticSourceComponent,
    formatDiagnosticsDuration,
    formatDiagnosticsProjectionFreshness,
    formatDiagnosticsStatusLabel,
    formatDiagnosticsTimestamp,
    hasActiveDiagnosticsComparisonFilters,
    hasTimelinePayload,
  } from './diagnosticsPagePresenters';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

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
  let loading = $state(false);
  let error = $state<string | null>(null);
  let comparisonFilters = $state<DiagnosticsComparisonFilters>({
    ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
  });
  let timelineEventFilter = $state<'all' | 'errors'>('all');
  let executionFilters = $state<DiagnosticsExecutionFilters>({
    ...DEFAULT_DIAGNOSTICS_EXECUTION_FILTERS,
  });
  let requestSerial = 0;

  let factRows = $derived(runDetail ? buildDiagnosticsFactRows(runDetail) : []);
  let runErrorSummary = $derived(runDetail ? buildDiagnosticsRunErrorSummary(runDetail) : null);
  let executionFilterOptions = $derived(
    nodeStatuses.length > 0
      ? buildDiagnosticsExecutionFilterOptions(nodeStatuses)
      : EMPTY_DIAGNOSTICS_EXECUTION_FILTER_OPTIONS,
  );
  let filteredExecutionNodes = $derived(filterDiagnosticsExecutionNodes(nodeStatuses, executionFilters));
  let filteredExecutionErrorNodes = $derived(
    filteredExecutionNodes.filter((node) => nodeHasErrorFocus(node)),
  );
  let executionFacetRows = $derived(buildDiagnosticsExecutionFacetRows(filteredExecutionNodes));
  let retentionSummaryRows = $derived(buildDiagnosticsRetentionSummaryRows(retentionSummary));
  let comparisonFilterOptions = $derived(
    runDetail ? buildDiagnosticsComparisonFilterOptions(runDetail, runList) : EMPTY_DIAGNOSTICS_COMPARISON_FILTER_OPTIONS,
  );
  let filteredComparisonRuns = $derived(
    runDetail ? filterDiagnosticsComparisonRuns(runDetail, runList, comparisonFilters) : [],
  );
  let filteredTimelineEvents = $derived(filterDiagnosticsTimelineEvents(timelineEvents, timelineEventFilter));
  let errorTimelineEvents = $derived(filterDiagnosticsTimelineEvents(timelineEvents, 'errors'));
  let focusedDiagnosticEventId = $derived(
    $diagnosticsFocus?.workflow_run_id === runDetail?.workflow_run_id
      ? ($diagnosticsFocus?.diagnostic_event_id ?? null)
      : null,
  );
  let focusedNodeId = $derived(
    $diagnosticsFocus?.workflow_run_id === runDetail?.workflow_run_id ? ($diagnosticsFocus?.node_id ?? null) : null,
  );
  let hasComparisonFilters = $derived(hasActiveDiagnosticsComparisonFilters(comparisonFilters));
  let facetSummary = $derived(
    runDetail
      ? buildDiagnosticsFacetSummary(
          runDetail,
          filteredComparisonRuns,
          hasComparisonFilters ? [] : runListFacets,
        )
      : null,
  );

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
  }

  function recordSubscriptionError(subscriptionError: unknown): void {
    error = formatWorkflowCommandError(subscriptionError);
  }

  function nodeDiagnosticErrorEventId(node: NodeStatusProjectionRecord): string | null {
    return node.canonical_error_event_id ?? node.error_event_id ?? null;
  }

  function nodeHasErrorFocus(node: NodeStatusProjectionRecord): boolean {
    return Boolean(nodeDiagnosticErrorEventId(node) || node.error || node.error_severity);
  }

  function nodeMatchesFocusedDiagnostic(node: NodeStatusProjectionRecord): boolean {
    return nodeDiagnosticErrorEventId(node) === focusedDiagnosticEventId || node.node_id === focusedNodeId;
  }

  async function refreshDiagnostics(runId = activeRunId()): Promise<void> {
    const currentRequest = ++requestSerial;
    error = null;

    if (!runId) {
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
      loading = false;
      return;
    }

    loading = true;
    try {
      const [runResponse, timelineResponse] = await Promise.all([
        workflowService.queryRunDetail({ workflow_run_id: runId }),
        workflowService.querySchedulerTimeline({
          workflow_run_id: runId,
          limit: 250,
        }),
      ]);
      if (currentRequest !== requestSerial) {
        return;
      }
      const selectedRun = runResponse.run ?? null;
      const selectedRunResponses = selectedRun
        ? await Promise.all([
            workflowService.queryRunList({ workflow_id: selectedRun.workflow_id, limit: 250 }),
            workflowService.queryNodeStatus({ workflow_run_id: selectedRun.workflow_run_id, limit: 250 }),
            workflowService.queryIoArtifacts({ workflow_run_id: selectedRun.workflow_run_id, limit: 1 }),
          ])
        : null;
      if (currentRequest !== requestSerial) {
        return;
      }
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
    } catch (refreshError) {
      if (currentRequest !== requestSerial) {
        return;
      }
      error = formatWorkflowCommandError(refreshError);
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
    } finally {
      if (currentRequest === requestSerial) {
        loading = false;
      }
    }
  }

  function updateComparisonFilter(field: keyof DiagnosticsComparisonFilters, value: string): void {
    comparisonFilters = {
      ...comparisonFilters,
      [field]: value,
    };
  }

  function updateExecutionFilter(field: keyof DiagnosticsExecutionFilters, value: string): void {
    executionFilters = {
      ...executionFilters,
      [field]: value,
    };
  }

  function focusTimelineError(event: SchedulerTimelineProjectionRecord): void {
    if (!runDetail) return;
    timelineEventFilter = 'errors';
    focusWorkflowDiagnostics(
      {
        workflow_run_id: runDetail.workflow_run_id,
        workflow_id: runDetail.workflow_id,
        workflow_version_id: runDetail.workflow_version_id,
        workflow_semantic_version: runDetail.workflow_semantic_version,
        status: runDetail.status,
      },
      {
        diagnostic_event_id: event.event_id,
        node_id: event.node_id ?? null,
      },
    );
  }

  function selectValue(event: Event): string {
    return (event.currentTarget as HTMLSelectElement).value;
  }

  $effect(() => {
    const runId = activeRunId();
    void refreshDiagnostics(runId);
  });

  onMount(() => {
    let unlisten: (() => void) | null = null;
    let disposed = false;
    void subscribeDiagnosticsProjectionInvalidations({
      projections: ['run_detail', 'run_list', 'node_status', 'io_artifact', 'scheduler_timeline'],
      getActiveRunId: activeRunId,
      refresh: () => refreshDiagnostics(),
      onRefreshError: recordSubscriptionError,
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        unlisten = nextUnlisten;
      })
      .catch(recordSubscriptionError);

    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  <div class="flex shrink-0 items-center justify-between gap-4 border-b border-neutral-800 px-4 py-3">
    <div class="min-w-0">
      <h1 class="text-base font-semibold text-neutral-100">Diagnostics</h1>
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
      onclick={() => refreshDiagnostics()}
      disabled={loading || !$activeWorkflowRun}
    >
      <RefreshCw size={14} aria-hidden="true" class={loading ? 'animate-spin' : ''} />
      Refresh
    </button>
  </div>

  {#if error}
    <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{error}</div>
  {/if}

  {#if !$activeWorkflowRun}
    <div class="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
      No active run selected
    </div>
  {:else if loading && !runDetail}
    <div class="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
      Loading diagnostics
    </div>
  {:else if !runDetail}
    <div class="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
      No run detail projection available
    </div>
  {:else}
    <div class="min-h-0 flex-1 overflow-auto">
      <div class="grid gap-4 p-4 xl:grid-cols-[24rem_minmax(0,1fr)]">
        <aside class="space-y-4">
          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-4">
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <h2 class="text-sm font-semibold text-neutral-100">Run Detail</h2>
                <div class="mt-1 truncate font-mono text-xs text-neutral-500" title={runDetail.workflow_run_id}>
                  {runDetail.workflow_run_id}
                </div>
              </div>
              <span class={`shrink-0 rounded border px-2 py-0.5 text-xs ${diagnosticsStatusClass(runDetail.status)}`}>
                {formatDiagnosticsStatusLabel(runDetail.status)}
              </span>
            </div>

            {#if runErrorSummary}
              <div class={`mt-4 rounded border px-3 py-2 text-xs ${diagnosticsErrorSeverityClass(runErrorSummary.severity)}`}>
                <div class="flex items-center justify-between gap-3">
                  <span class="font-semibold">{formatDiagnosticErrorSeverity(runErrorSummary.severity)}</span>
                  <span class="font-mono text-[11px]" title={runErrorSummary.eventId}>{runErrorSummary.eventId}</span>
                </div>
                <div class="mt-1 font-mono text-[11px]">{runErrorSummary.code}</div>
                <div class="mt-1 text-[11px]">{formatDiagnosticErrorPhase(runErrorSummary.phase)}</div>
              </div>
            {/if}

            <dl class="mt-4 grid grid-cols-2 gap-3 text-xs">
              <div>
                <dt class="text-neutral-500">Accepted</dt>
                <dd class="mt-1 text-neutral-200">{formatDiagnosticsTimestamp(runDetail.accepted_at_ms)}</dd>
              </div>
              <div>
                <dt class="text-neutral-500">Queued</dt>
                <dd class="mt-1 text-neutral-200">{formatDiagnosticsTimestamp(runDetail.enqueued_at_ms)}</dd>
              </div>
              <div>
                <dt class="text-neutral-500">Started</dt>
                <dd class="mt-1 text-neutral-200">{formatDiagnosticsTimestamp(runDetail.started_at_ms)}</dd>
              </div>
              <div>
                <dt class="text-neutral-500">Duration</dt>
                <dd class="mt-1 text-neutral-200">{formatDiagnosticsDuration(runDetail.duration_ms, runDetail.status)}</dd>
              </div>
            </dl>
          </section>

          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-4">
            <h2 class="text-sm font-semibold text-neutral-100">Projection State</h2>
            <dl class="mt-4 space-y-3 text-xs">
              <div>
                <dt class="text-neutral-500">Run Detail</dt>
                <dd class="mt-1 text-neutral-200">{formatDiagnosticsProjectionFreshness(runDetailProjectionState)}</dd>
              </div>
              <div>
                <dt class="text-neutral-500">Scheduler Timeline</dt>
                <dd class="mt-1 text-neutral-200">{formatDiagnosticsProjectionFreshness(timelineProjectionState)}</dd>
              </div>
              <div>
                <dt class="text-neutral-500">Run List</dt>
                <dd class="mt-1 text-neutral-200">{formatDiagnosticsProjectionFreshness(runListProjectionState)}</dd>
              </div>
              <div>
                <dt class="text-neutral-500">Node Status</dt>
                <dd class="mt-1 text-neutral-200">{formatDiagnosticsProjectionFreshness(nodeStatusProjectionState)}</dd>
              </div>
              <div>
                <dt class="text-neutral-500">I/O Retention</dt>
                <dd class="mt-1 text-neutral-200">{formatDiagnosticsProjectionFreshness(ioProjectionState)}</dd>
              </div>
            </dl>
          </section>

          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-4">
            <h2 class="text-sm font-semibold text-neutral-100">Facts</h2>
            <dl class="mt-4 space-y-3 text-xs">
              {#each factRows as row (row.label)}
                <div>
                  <dt class="text-neutral-500">{row.label}</dt>
                  <dd
                    class={`mt-1 truncate text-neutral-200 ${row.mono ? 'font-mono' : ''}`}
                    title={row.value}
                  >
                    {row.value}
                  </dd>
                </div>
              {/each}
            </dl>
          </section>

          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-4">
            <h2 class="text-sm font-semibold text-neutral-100">Execution Facets</h2>
            {#if nodeStatuses.length > 0}
              <div class="mt-4 grid grid-cols-2 gap-2">
                <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                  Node Status
                  <select
                    aria-label="Diagnostics node status filter"
                    class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    value={executionFilters.status}
                    onchange={(event) => updateExecutionFilter('status', selectValue(event))}
                  >
                    <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                    {#each executionFilterOptions.statuses as status (status)}
                      <option value={status}>{status}</option>
                    {/each}
                  </select>
                </label>
                <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                  Node Version
                  <select
                    aria-label="Diagnostics node version filter"
                    class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    value={executionFilters.nodeVersion}
                    onchange={(event) => updateExecutionFilter('nodeVersion', selectValue(event))}
                  >
                    <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                    {#each executionFilterOptions.nodeVersions as nodeVersion (nodeVersion)}
                      <option value={nodeVersion}>{nodeVersion}</option>
                    {/each}
                  </select>
                </label>
                <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                  Runtime
                  <select
                    aria-label="Diagnostics runtime filter"
                    class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    value={executionFilters.runtime}
                    onchange={(event) => updateExecutionFilter('runtime', selectValue(event))}
                  >
                    <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                    {#each executionFilterOptions.runtimes as runtime (runtime)}
                      <option value={runtime}>{runtime}</option>
                    {/each}
                  </select>
                </label>
                <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                  Runtime Ver
                  <select
                    aria-label="Diagnostics runtime version filter"
                    class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    value={executionFilters.runtimeVersion}
                    onchange={(event) => updateExecutionFilter('runtimeVersion', selectValue(event))}
                  >
                    <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                    {#each executionFilterOptions.runtimeVersions as runtimeVersion (runtimeVersion)}
                      <option value={runtimeVersion}>{runtimeVersion}</option>
                    {/each}
                  </select>
                </label>
                <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                  Model
                  <select
                    aria-label="Diagnostics model filter"
                    class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    value={executionFilters.model}
                    onchange={(event) => updateExecutionFilter('model', selectValue(event))}
                  >
                    <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                    {#each executionFilterOptions.models as model (model)}
                      <option value={model}>{model}</option>
                    {/each}
                  </select>
                </label>
                <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                  Model Ver
                  <select
                    aria-label="Diagnostics model version filter"
                    class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    value={executionFilters.modelVersion}
                    onchange={(event) => updateExecutionFilter('modelVersion', selectValue(event))}
                  >
                    <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                    {#each executionFilterOptions.modelVersions as modelVersion (modelVersion)}
                      <option value={modelVersion}>{modelVersion}</option>
                    {/each}
                  </select>
                </label>
              </div>
            {/if}
            {#if executionFacetRows.length === 0}
              <div class="mt-4 text-xs text-neutral-500">
                {nodeStatuses.length === 0 ? 'No node status projection rows' : 'No matching node status projection rows'}
              </div>
            {:else}
              <dl class="mt-4 space-y-3 text-xs">
                {#each executionFacetRows as row (`${row.label}:${row.value}`)}
                  <div>
                    <dt class="text-neutral-500">{row.label}</dt>
                    <dd class="mt-1 flex items-center justify-between gap-3 text-neutral-200">
                      <span class="min-w-0 truncate font-mono" title={row.value}>{row.value}</span>
                      <span class="shrink-0 text-neutral-500">{row.count}</span>
                    </dd>
                  </div>
                {/each}
              </dl>
            {/if}
          </section>

          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-4">
            <h2 class="text-sm font-semibold text-neutral-100">Retention Completeness</h2>
            {#if retentionSummaryRows.length === 0}
              <div class="mt-4 text-xs text-neutral-500">No retained artifact summary</div>
            {:else}
              <dl class="mt-4 space-y-3 text-xs">
                {#each retentionSummaryRows as row (row.label)}
                  <div class="flex items-center justify-between gap-3">
                    <dt class="min-w-0 truncate text-neutral-500">{row.label}</dt>
                    <dd class="shrink-0 font-mono text-neutral-200">{row.count}</dd>
                  </div>
                {/each}
              </dl>
            {/if}
          </section>

          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-4">
            <h2 class="text-sm font-semibold text-neutral-100">Comparison Facets</h2>
            <div class="mt-4 grid grid-cols-2 gap-2">
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Version
                <select
                  aria-label="Diagnostics workflow version filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.workflowVersion}
                  onchange={(event) => updateComparisonFilter('workflowVersion', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.workflowVersions as workflowVersion (workflowVersion)}
                    <option value={workflowVersion}>{workflowVersion}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Status
                <select
                  aria-label="Diagnostics status filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.status}
                  onchange={(event) => updateComparisonFilter('status', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.statuses as status (status)}
                    <option value={status}>{formatDiagnosticsStatusLabel(status)}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Scheduler
                <select
                  aria-label="Diagnostics scheduler policy filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.schedulerPolicy}
                  onchange={(event) => updateComparisonFilter('schedulerPolicy', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.schedulerPolicies as schedulerPolicy (schedulerPolicy)}
                    <option value={schedulerPolicy}>{schedulerPolicy}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Retention
                <select
                  aria-label="Diagnostics retention policy filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.retentionPolicy}
                  onchange={(event) => updateComparisonFilter('retentionPolicy', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.retentionPolicies as retentionPolicy (retentionPolicy)}
                    <option value={retentionPolicy}>{retentionPolicy}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Client
                <select
                  aria-label="Diagnostics client filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.client}
                  onchange={(event) => updateComparisonFilter('client', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.clients as client (client)}
                    <option value={client}>{client}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Runtime
                <select
                  aria-label="Diagnostics selected runtime filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.selectedRuntime}
                  onchange={(event) => updateComparisonFilter('selectedRuntime', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.selectedRuntimes as selectedRuntime (selectedRuntime)}
                    <option value={selectedRuntime}>{selectedRuntime}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Device Class
                <select
                  aria-label="Diagnostics selected device class filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.selectedDeviceClass}
                  onchange={(event) => updateComparisonFilter('selectedDeviceClass', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.selectedDeviceClasses as selectedDeviceClass (selectedDeviceClass)}
                    <option value={selectedDeviceClass}>{selectedDeviceClass}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Device
                <select
                  aria-label="Diagnostics selected device filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.selectedDevice}
                  onchange={(event) => updateComparisonFilter('selectedDevice', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.selectedDevices as selectedDevice (selectedDevice)}
                    <option value={selectedDevice}>{selectedDevice}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Network Node
                <select
                  aria-label="Diagnostics selected network node filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.selectedNetworkNode}
                  onchange={(event) => updateComparisonFilter('selectedNetworkNode', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.selectedNetworkNodes as selectedNetworkNode (selectedNetworkNode)}
                    <option value={selectedNetworkNode}>{selectedNetworkNode}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Session
                <select
                  aria-label="Diagnostics client session filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.clientSession}
                  onchange={(event) => updateComparisonFilter('clientSession', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.clientSessions as clientSession (clientSession)}
                    <option value={clientSession}>{clientSession}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Bucket
                <select
                  aria-label="Diagnostics bucket filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.bucket}
                  onchange={(event) => updateComparisonFilter('bucket', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.buckets as bucket (bucket)}
                    <option value={bucket}>{bucket}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                Accepted
                <select
                  aria-label="Diagnostics accepted date filter"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.acceptedDate}
                  onchange={(event) => updateComparisonFilter('acceptedDate', selectValue(event))}
                >
                  <option value={DIAGNOSTICS_FILTER_ALL}>All</option>
                  {#each comparisonFilterOptions.acceptedDates as acceptedDate (acceptedDate)}
                    <option value={acceptedDate}>{acceptedDate}</option>
                  {/each}
                </select>
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                From
                <input
                  aria-label="Diagnostics accepted from date"
                  type="date"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.acceptedFromDate}
                  onchange={(event) => updateComparisonFilter('acceptedFromDate', selectValue(event))}
                />
              </label>
              <label class="min-w-0 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                To
                <input
                  aria-label="Diagnostics accepted to date"
                  type="date"
                  class="mt-1 w-full rounded border border-neutral-800 bg-neutral-950 px-2 py-1.5 text-xs normal-case tracking-normal text-neutral-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                  value={comparisonFilters.acceptedToDate}
                  onchange={(event) => updateComparisonFilter('acceptedToDate', selectValue(event))}
                />
              </label>
            </div>
            {#if facetSummary?.mixedVersionWarning}
              <div class="mt-3 rounded border border-amber-900 bg-amber-950/40 px-3 py-2 text-xs text-amber-100">
                {facetSummary.mixedVersionWarning}
              </div>
            {/if}
            <dl class="mt-4 space-y-3 text-xs">
              {#each facetSummary?.rows ?? [] as row (row.label)}
                <div>
                  <dt class="text-neutral-500">{row.label}</dt>
                  <dd class="mt-1 flex items-center justify-between gap-3 text-neutral-200">
                    <span class="min-w-0 truncate font-mono" title={row.value}>{row.value}</span>
                    <span class="shrink-0 text-neutral-500">{row.count}/{row.total}</span>
                  </dd>
                </div>
              {/each}
            </dl>
          </section>
        </aside>

        <main class="min-w-0 space-y-4">
          {#if runDetail.terminal_error}
            <section class="rounded border border-red-900 bg-red-950/40 p-4">
              <h2 class="text-sm font-semibold text-red-100">Terminal Error</h2>
              <p class="mt-2 whitespace-pre-wrap text-sm text-red-200">{runDetail.terminal_error}</p>
            </section>
          {/if}

          {#if runErrorSummary}
            <section class={`rounded border p-4 ${diagnosticsErrorSeverityClass(runErrorSummary.severity)}`}>
              <div class="flex flex-wrap items-start justify-between gap-3">
                <div class="min-w-0">
                  <h2 class="text-sm font-semibold">{formatDiagnosticErrorSeverity(runErrorSummary.severity)} Diagnostic</h2>
                  <div class="mt-1 font-mono text-xs">{runErrorSummary.code}</div>
                </div>
                <div class="shrink-0 text-right text-xs">
                  <div>{formatDiagnosticErrorPhase(runErrorSummary.phase)}</div>
                  <div class="mt-1 font-mono" title={runErrorSummary.eventId}>{runErrorSummary.eventId}</div>
                </div>
              </div>
              <p class="mt-3 whitespace-pre-wrap text-sm">{runErrorSummary.message}</p>
            </section>
          {/if}

          {#if filteredExecutionErrorNodes.length > 0}
            <section class="rounded border border-neutral-800 bg-neutral-900/50">
              <div class="border-b border-neutral-800 px-4 py-3">
                <h2 class="text-sm font-semibold text-neutral-100">Node Errors</h2>
                <div class="mt-1 text-xs text-neutral-500">Node-scoped diagnostic failures</div>
              </div>
              <div class="overflow-auto">
                <table class="w-full min-w-[48rem] text-left text-sm">
                  <thead class="sticky top-0 bg-neutral-950 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                    <tr class="border-b border-neutral-800">
                      <th class="px-4 py-3 font-medium">Node</th>
                      <th class="px-3 py-3 font-medium">Status</th>
                      <th class="px-3 py-3 font-medium">Severity</th>
                      <th class="px-3 py-3 font-medium">Phase</th>
                      <th class="px-3 py-3 font-medium">Code</th>
                      <th class="px-4 py-3 font-medium">Event</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-neutral-900">
                    {#each filteredExecutionErrorNodes as node (node.node_id)}
                      <tr class:border-l-2={nodeMatchesFocusedDiagnostic(node)} class:border-cyan-400={nodeMatchesFocusedDiagnostic(node)}>
                        <td class="px-4 py-2 font-mono text-xs text-neutral-300" title={node.node_id}>{node.node_id}</td>
                        <td class="px-3 py-2 text-xs text-neutral-300">{node.status}</td>
                        <td class="px-3 py-2 text-xs text-neutral-300">
                          {formatDiagnosticErrorSeverity(node.error_severity)}
                        </td>
                        <td class="px-3 py-2 text-xs text-neutral-400">
                          {formatDiagnosticErrorPhase(node.error_phase)}
                        </td>
                        <td class="px-3 py-2 font-mono text-xs text-neutral-400">{node.error_code ?? 'unknown_error'}</td>
                        <td class="px-4 py-2 font-mono text-xs text-neutral-500" title={nodeDiagnosticErrorEventId(node) ?? ''}>
                          {nodeDiagnosticErrorEventId(node) ?? 'Unassigned'}
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </section>
          {/if}

          <section class="rounded border border-neutral-800 bg-neutral-900/50">
            <div class="border-b border-neutral-800 px-4 py-3">
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <h2 class="text-sm font-semibold text-neutral-100">Scheduler Timeline</h2>
                  <div class="mt-1 text-xs text-neutral-500">
                    {filteredTimelineEvents.length} of {timelineEvents.length} projected events
                  </div>
                </div>
                <div class="inline-flex rounded border border-neutral-800 bg-neutral-950 p-0.5 text-xs">
                  <button
                    type="button"
                    class="rounded px-2 py-1 text-neutral-300 transition-colors hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    class:bg-neutral-800={timelineEventFilter === 'all'}
                    aria-pressed={timelineEventFilter === 'all'}
                    onclick={() => (timelineEventFilter = 'all')}
                  >
                    All
                  </button>
                  <button
                    type="button"
                    class="rounded px-2 py-1 text-neutral-300 transition-colors hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                    class:bg-neutral-800={timelineEventFilter === 'errors'}
                    aria-pressed={timelineEventFilter === 'errors'}
                    onclick={() => (timelineEventFilter = 'errors')}
                  >
                    Errors {errorTimelineEvents.length}
                  </button>
                </div>
              </div>
              {#if errorTimelineEvents.length > 0}
                <div class="mt-3 space-y-2">
                  {#each errorTimelineEvents.slice(0, 3) as event (event.event_id)}
                    <button
                      type="button"
                      class="block w-full rounded border border-red-900 bg-red-950/30 px-3 py-2 text-left text-xs text-red-100 transition-colors hover:border-red-700 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
                      onclick={() => focusTimelineError(event)}
                      aria-label={`Focus diagnostic error event ${event.event_id}`}
                    >
                      <span class="font-semibold">{formatDiagnosticErrorSeverity(event.error_severity)}</span>
                      <span class="ml-2 font-mono">{event.event_id}</span>
                      <span class="ml-2 text-red-200">{formatDiagnosticErrorPhase(event.error_phase)}</span>
                      <span class="mt-1 block truncate text-red-100" title={event.summary}>{event.summary}</span>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>

            {#if filteredTimelineEvents.length === 0}
              <div class="px-4 py-8 text-sm text-neutral-500">No scheduler timeline events projected</div>
            {:else}
              <div class="overflow-auto">
                <table class="w-full min-w-[56rem] text-left text-sm">
                  <thead class="sticky top-0 bg-neutral-950 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                    <tr class="border-b border-neutral-800">
                      <th class="px-4 py-3 font-medium">Seq</th>
                      <th class="px-3 py-3 font-medium">Time</th>
                      <th class="px-3 py-3 font-medium">Kind</th>
                      <th class="px-3 py-3 font-medium">Source</th>
                      <th class="px-3 py-3 font-medium">Summary</th>
                      <th class="px-4 py-3 font-medium">Payload</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-neutral-900">
                    {#each filteredTimelineEvents as event (event.event_id)}
                      <tr
                        class={diagnosticsTimelineRowClass(event)}
                        class:outline={event.event_id === focusedDiagnosticEventId}
                        class:outline-2={event.event_id === focusedDiagnosticEventId}
                        class:outline-cyan-400={event.event_id === focusedDiagnosticEventId}
                        class:outline-offset-[-2px]={event.event_id === focusedDiagnosticEventId}
                      >
                        <td class="px-4 py-2 font-mono text-xs text-neutral-400">{event.event_seq}</td>
                        <td class="px-3 py-2 text-xs text-neutral-400">
                          {formatDiagnosticsTimestamp(event.occurred_at_ms)}
                        </td>
                        <td class="px-3 py-2 text-xs text-neutral-300">
                          {formatDiagnosticEventKind(event.event_kind)}
                        </td>
                        <td class="px-3 py-2 text-xs text-neutral-400">
                          {formatDiagnosticSourceComponent(event.source_component)}
                        </td>
                        <td class="max-w-[28rem] px-3 py-2">
                          <div class="flex items-center gap-2">
                            {#if event.error_severity}
                              <span class={`shrink-0 rounded border px-1.5 py-0.5 text-[11px] ${diagnosticsErrorSeverityClass(event.error_severity)}`}>
                                {formatDiagnosticErrorSeverity(event.error_severity)}
                              </span>
                            {/if}
                            <div class="min-w-0 truncate text-neutral-200" title={event.summary}>{event.summary}</div>
                          </div>
                          {#if event.detail}
                            <div class="mt-1 truncate text-xs text-neutral-500" title={event.detail}>{event.detail}</div>
                          {/if}
                          {#if event.error_phase}
                            <div class="mt-1 text-xs text-neutral-500">{formatDiagnosticErrorPhase(event.error_phase)}</div>
                          {/if}
                        </td>
                        <td class="px-4 py-2 text-xs text-neutral-500">
                          {hasTimelinePayload(event) ? 'Captured' : 'Metadata only'}
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {/if}
          </section>
        </main>
      </div>
    </div>
  {/if}
</section>

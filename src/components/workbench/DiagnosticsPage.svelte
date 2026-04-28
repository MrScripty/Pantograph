<script lang="ts">
  import { RefreshCw } from 'lucide-svelte';
  import type {
    ProjectionStateRecord,
    RunDetailProjectionRecord,
    RunListFacetRecord,
    RunListProjectionRecord,
    SchedulerTimelineProjectionRecord,
  } from '../../services/diagnostics/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
  import type { DiagnosticsComparisonFilters } from './diagnosticsPagePresenters';
  import {
    DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
    DIAGNOSTICS_FILTER_ALL,
    EMPTY_DIAGNOSTICS_COMPARISON_FILTER_OPTIONS,
    buildDiagnosticsFacetSummary,
    buildDiagnosticsFactRows,
    buildDiagnosticsComparisonFilterOptions,
    diagnosticsStatusClass,
    filterDiagnosticsComparisonRuns,
    formatDiagnosticEventKind,
    formatDiagnosticSourceComponent,
    formatDiagnosticsDuration,
    formatDiagnosticsProjectionFreshness,
    formatDiagnosticsTimestamp,
    hasActiveDiagnosticsComparisonFilters,
    hasTimelinePayload,
  } from './diagnosticsPagePresenters';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

  let runDetail = $state<RunDetailProjectionRecord | null>(null);
  let runList = $state<RunListProjectionRecord[]>([]);
  let runListFacets = $state<RunListFacetRecord[]>([]);
  let timelineEvents = $state<SchedulerTimelineProjectionRecord[]>([]);
  let runDetailProjectionState = $state<ProjectionStateRecord | null>(null);
  let runListProjectionState = $state<ProjectionStateRecord | null>(null);
  let timelineProjectionState = $state<ProjectionStateRecord | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let comparisonFilters = $state<DiagnosticsComparisonFilters>({
    ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
  });
  let requestSerial = 0;

  let factRows = $derived(runDetail ? buildDiagnosticsFactRows(runDetail) : []);
  let comparisonFilterOptions = $derived(
    runDetail ? buildDiagnosticsComparisonFilterOptions(runDetail, runList) : EMPTY_DIAGNOSTICS_COMPARISON_FILTER_OPTIONS,
  );
  let filteredComparisonRuns = $derived(
    runDetail ? filterDiagnosticsComparisonRuns(runDetail, runList, comparisonFilters) : [],
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

  async function refreshDiagnostics(runId = activeRunId()): Promise<void> {
    const currentRequest = ++requestSerial;
    error = null;

    if (!runId) {
      runDetail = null;
      runList = [];
      runListFacets = [];
      timelineEvents = [];
      runDetailProjectionState = null;
      runListProjectionState = null;
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
      const runListResponse = selectedRun
        ? await workflowService.queryRunList({ workflow_id: selectedRun.workflow_id, limit: 250 })
        : null;
      if (currentRequest !== requestSerial) {
        return;
      }
      runDetail = selectedRun;
      runDetailProjectionState = runResponse.projection_state;
      runList = runListResponse?.runs ?? [];
      runListFacets = runListResponse?.facets ?? [];
      runListProjectionState = runListResponse?.projection_state ?? null;
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
      timelineEvents = [];
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

  function selectValue(event: Event): string {
    return (event.currentTarget as HTMLSelectElement).value;
  }

  $effect(() => {
    const runId = activeRunId();
    void refreshDiagnostics(runId);
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
                {runDetail.status}
              </span>
            </div>

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
            <h2 class="text-sm font-semibold text-neutral-100">Comparison Facets</h2>
            <div class="mt-4 grid grid-cols-2 gap-2">
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
                    <option value={status}>{status}</option>
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

          <section class="rounded border border-neutral-800 bg-neutral-900/50">
            <div class="border-b border-neutral-800 px-4 py-3">
              <h2 class="text-sm font-semibold text-neutral-100">Scheduler Timeline</h2>
              <div class="mt-1 text-xs text-neutral-500">{timelineEvents.length} projected events</div>
            </div>

            {#if timelineEvents.length === 0}
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
                    {#each timelineEvents as event (event.event_id)}
                      <tr>
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
                          <div class="truncate text-neutral-200" title={event.summary}>{event.summary}</div>
                          {#if event.detail}
                            <div class="mt-1 truncate text-xs text-neutral-500" title={event.detail}>{event.detail}</div>
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

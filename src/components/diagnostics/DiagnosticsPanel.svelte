<script lang="ts">
  import type { DiagnosticsTab } from '../../services/diagnostics/types';
  import { diagnosticsSnapshot, clearDiagnosticsHistory, selectDiagnosticsNode, selectDiagnosticsRun, setDiagnosticsPanelOpen, setDiagnosticsTab } from '../../stores/diagnosticsStore';
  import DiagnosticsEvents from './DiagnosticsEvents.svelte';
  import DiagnosticsOverview from './DiagnosticsOverview.svelte';
  import DiagnosticsTimeline from './DiagnosticsTimeline.svelte';
  import {
    formatDiagnosticsDuration,
    formatDiagnosticsTimestamp,
    getDiagnosticsStatusClasses,
  } from './presenters';

  const tabDefinitions: Array<{ id: DiagnosticsTab; label: string; available: boolean }> = [
    { id: 'overview', label: 'Overview', available: true },
    { id: 'timeline', label: 'Timeline', available: true },
    { id: 'events', label: 'Events', available: true },
    { id: 'scheduler', label: 'Scheduler', available: false },
    { id: 'runtime', label: 'Runtime', available: false },
    { id: 'graph', label: 'Graph', available: false },
  ];

  let snapshot = $derived($diagnosticsSnapshot);
  let runs = $derived.by(() => {
    return snapshot.state.runOrder
      .map((runId) => snapshot.state.runsById[runId])
      .filter((run): run is NonNullable<typeof run> => Boolean(run));
  });

  function handleSelectRun(runId: string): void {
    selectDiagnosticsRun(runId);
    selectDiagnosticsNode(null);
  }

  function handleSelectNode(nodeId: string | null): void {
    selectDiagnosticsNode(nodeId);
  }

  function renderPlannedTabCopy(tab: DiagnosticsTab): string {
    switch (tab) {
      case 'scheduler':
        return 'Scheduler V2 traces will land here once queue ownership, fairness, and admission decisions are emitted as first-class diagnostics records.';
      case 'runtime':
        return 'Runtime adapter unification and KV-cache lifecycle metrics will surface here after the runtime roadmap items start publishing stable host-facing state.';
      case 'graph':
        return 'Incremental graph execution and graph invalidation traces will expand this tab beyond the current dirty-task summary cards.';
      default:
        return 'This tab is not part of the initial diagnostics slice.';
    }
  }
</script>

{#if snapshot.state.panelOpen}
  <section class="h-[26rem] min-h-0 border-t border-neutral-800 bg-neutral-950/95 backdrop-blur-sm">
    <div class="flex h-full min-h-0">
      <aside class="flex w-80 min-w-[18rem] flex-col border-r border-neutral-800 bg-neutral-950/80">
        <div class="border-b border-neutral-800 px-4 py-4">
          <div class="flex items-center justify-between">
            <div>
              <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Workflow Diagnostics</div>
              <div class="mt-2 text-sm font-medium text-neutral-100">
                {snapshot.state.currentWorkflowName ?? snapshot.state.currentWorkflowId ?? 'Active Workflow'}
              </div>
            </div>
            <button
              type="button"
              class="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100"
              onclick={() => setDiagnosticsPanelOpen(false)}
            >
              Hide
            </button>
          </div>

          <div class="mt-3 space-y-1 text-xs text-neutral-500">
            <div>Workflow ID: {snapshot.state.currentWorkflowId ?? 'Unavailable'}</div>
            <div>Graph Fingerprint: {snapshot.state.currentGraphFingerprint ?? 'Unavailable'}</div>
            <div>Retained Runs: {runs.length}</div>
          </div>

          <div class="mt-4 flex items-center gap-2">
            <button
              type="button"
              class="rounded border border-neutral-700 px-3 py-1.5 text-xs text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 disabled:opacity-50 disabled:cursor-not-allowed"
              onclick={() => clearDiagnosticsHistory()}
              disabled={runs.length === 0}
            >
              Clear History
            </button>
          </div>
        </div>

        <div class="min-h-0 flex-1 overflow-auto">
          {#if runs.length === 0}
            <div class="px-4 py-6 text-sm text-neutral-500">
              Run a workflow to populate the diagnostics panel with traces, timing, and event history.
            </div>
          {:else}
            <div class="divide-y divide-neutral-900">
              {#each runs as run (run.executionId)}
                <button
                  type="button"
                  class:selected-run={snapshot.state.selectedRunId === run.executionId}
                  class="w-full px-4 py-3 text-left transition-colors hover:bg-neutral-900/80"
                  onclick={() => handleSelectRun(run.executionId)}
                >
                  <div class="flex items-center justify-between gap-3">
                    <div class="truncate text-sm font-medium text-neutral-100">{run.executionId}</div>
                    <span class={`inline-flex rounded-full border px-2 py-0.5 text-[11px] font-medium ${getDiagnosticsStatusClasses(run.status)}`}>
                      {run.status}
                    </span>
                  </div>
                  <div class="mt-1 truncate text-xs text-neutral-500">
                    {run.workflowName ?? run.workflowId ?? 'Unlabeled workflow'}
                  </div>
                  <div class="mt-2 flex items-center justify-between text-[11px] text-neutral-500">
                    <span>{formatDiagnosticsTimestamp(run.startedAtMs)}</span>
                    <span>{formatDiagnosticsDuration(run.durationMs)}</span>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </aside>

      <div class="flex min-h-0 flex-1 flex-col">
        <div class="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
          <div class="flex flex-wrap items-center gap-2">
            {#each tabDefinitions as tab (tab.id)}
              <button
                type="button"
                class:active-tab={snapshot.state.activeTab === tab.id}
                class:opacity-70={!tab.available}
                class="rounded-full border border-neutral-700 px-3 py-1.5 text-xs uppercase tracking-[0.24em] text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100"
                onclick={() => setDiagnosticsTab(tab.id)}
              >
                {tab.label}
              </button>
            {/each}
          </div>

          {#if snapshot.selectedRun}
            <div class="text-xs text-neutral-500">
              {Object.keys(snapshot.selectedRun.nodes).length} nodes • {snapshot.selectedRun.eventCount} events
            </div>
          {/if}
        </div>

        <div class="min-h-0 flex-1">
          {#if !snapshot.selectedRun}
            <div class="flex h-full items-center justify-center px-6 text-center text-sm text-neutral-500">
              Select a retained run to inspect its overview, timeline, and raw event stream.
            </div>
          {:else if snapshot.state.activeTab === 'overview'}
            <DiagnosticsOverview
              run={snapshot.selectedRun}
              selectedNode={snapshot.selectedNode}
              selectedNodeId={snapshot.state.selectedNodeId}
              onSelectNode={handleSelectNode}
            />
          {:else if snapshot.state.activeTab === 'timeline'}
            <DiagnosticsTimeline
              run={snapshot.selectedRun}
              selectedNodeId={snapshot.state.selectedNodeId}
              onSelectNode={handleSelectNode}
            />
          {:else if snapshot.state.activeTab === 'events'}
            <DiagnosticsEvents
              run={snapshot.selectedRun}
              selectedNodeId={snapshot.state.selectedNodeId}
              onSelectNode={handleSelectNode}
            />
          {:else}
            <div class="flex h-full items-center justify-center px-6">
              <div class="max-w-2xl rounded-2xl border border-dashed border-neutral-800 bg-neutral-950/70 px-6 py-8 text-center">
                <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">
                  Planned Surface
                </div>
                <div class="mt-3 text-lg font-medium text-neutral-100">
                  {tabDefinitions.find((tab) => tab.id === snapshot.state.activeTab)?.label}
                </div>
                <div class="mt-3 text-sm leading-6 text-neutral-400">
                  {renderPlannedTabCopy(snapshot.state.activeTab)}
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </section>
{/if}

<style>
  .selected-run {
    background: rgba(14, 165, 233, 0.12);
  }

  .active-tab {
    border-color: rgb(8 145 178);
    color: rgb(165 243 252);
    background: rgba(8, 145, 178, 0.15);
  }
</style>

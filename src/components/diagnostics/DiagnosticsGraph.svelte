<script lang="ts">
  import type { DiagnosticsRunTrace, WorkflowDiagnosticsState } from '../../services/diagnostics/types';
  import {
    formatCheckpointSummary,
    formatNodeMemoryCompatibilityLabel,
    formatNodeMemoryStatusLabel,
    formatSessionResidencyLabel,
    formatDiagnosticsTimestamp,
    getGraphMemoryImpactCounts,
    getNodeMemoryStatusCounts,
  } from './presenters';

  export let state: WorkflowDiagnosticsState;
  export let selectedRun: DiagnosticsRunTrace | null = null;

  let graphEvents = $derived.by(() => {
    if (!selectedRun) {
      return [];
    }

    return selectedRun.events
      .filter((event) => event.type === 'GraphModified' || event.type === 'IncrementalExecutionStarted')
      .slice()
      .reverse();
  });

  let graphMemoryImpact = $derived(selectedRun?.lastGraphMemoryImpact ?? null);
  let graphMemoryImpactCounts = $derived(getGraphMemoryImpactCounts(graphMemoryImpact));
  let currentSessionState = $derived(state.currentSessionState ?? null);
  let nodeMemoryStatusCounts = $derived(
    getNodeMemoryStatusCounts(currentSessionState?.node_memory ?? null),
  );
</script>

<div class="h-full overflow-auto px-4 py-4">
  <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Current Graph</div>
      <div class="mt-3 text-2xl font-semibold text-neutral-100">{state.currentGraphNodeCount}</div>
      <div class="mt-2 text-xs text-neutral-500">nodes • {state.currentGraphEdgeCount} edges</div>
    </article>

    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Fingerprint</div>
      <div class="mt-3 break-all text-sm text-neutral-200">
        {state.currentGraphFingerprint ?? 'Unavailable'}
      </div>
    </article>

    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Dirty Tasks</div>
      <div class="mt-3 text-2xl font-semibold text-neutral-100">
        {selectedRun?.lastDirtyTasks.length ?? 0}
      </div>
      <div class="mt-2 text-xs text-neutral-500">
        Latest observed invalidation set for the selected run.
      </div>
    </article>

    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Incremental Tasks</div>
      <div class="mt-3 text-2xl font-semibold text-neutral-100">
        {selectedRun?.lastIncrementalTaskIds.length ?? 0}
      </div>
      <div class="mt-2 text-xs text-neutral-500">
        Latest observed incremental execution subset.
      </div>
    </article>

    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Memory Impact</div>
      <div class="mt-3 text-2xl font-semibold text-neutral-100">
        {graphMemoryImpact?.node_decisions?.length ?? 0}
      </div>
      <div class="mt-2 text-xs text-neutral-500">
        Latest backend-owned compatibility decisions for the selected run.
      </div>
    </article>

    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Session Residency</div>
      <div class="mt-3 text-2xl font-semibold text-neutral-100">
        {formatSessionResidencyLabel(currentSessionState?.residency)}
      </div>
      <div class="mt-2 text-xs text-neutral-500">
        Current backend-owned workflow-session residency for this diagnostics scope.
      </div>
    </article>

    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Checkpoint</div>
      <div class="mt-3 text-lg font-semibold text-neutral-100">
        {formatCheckpointSummary(currentSessionState?.checkpoint)}
      </div>
      <div class="mt-2 text-xs text-neutral-500">
        {#if currentSessionState?.checkpoint?.checkpointed_at_ms}
          Updated {formatDiagnosticsTimestamp(currentSessionState.checkpoint.checkpointed_at_ms)}
        {:else}
          Latest checkpoint summary forwarded from backend diagnostics inspection.
        {/if}
      </div>
    </article>

    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Node Memory</div>
      <div class="mt-3 text-2xl font-semibold text-neutral-100">
        {currentSessionState?.node_memory?.length ?? 0}
      </div>
      <div class="mt-2 text-xs text-neutral-500">
        Ready {nodeMemoryStatusCounts.ready} • Empty {nodeMemoryStatusCounts.empty} • Invalidated {nodeMemoryStatusCounts.invalidated}
      </div>
    </article>
  </div>

  <div class="mt-4 grid gap-4 xl:grid-cols-[minmax(0,2fr)_minmax(20rem,1fr)]">
    <section class="rounded-xl border border-neutral-800 bg-neutral-950/80">
      <header class="border-b border-neutral-800 px-4 py-3">
        <div class="text-sm font-medium text-neutral-100">Graph Trace Events</div>
        <div class="text-xs text-neutral-500">
          Graph modifications and incremental execution transitions observed in the selected run.
        </div>
      </header>

      {#if !selectedRun}
        <div class="px-4 py-6 text-sm text-neutral-500">
          Select a run to inspect graph-modification events.
        </div>
      {:else if graphEvents.length === 0}
        <div class="px-4 py-6 text-sm text-neutral-500">
          The selected run did not emit graph-specific diagnostics events.
        </div>
      {:else}
        <div class="divide-y divide-neutral-900">
          {#each graphEvents as event (event.id)}
            <div class="px-4 py-3">
              <div class="flex items-center justify-between gap-3">
                <div class="text-sm font-medium text-neutral-100">{event.summary}</div>
                <div class="text-xs text-neutral-500">{formatDiagnosticsTimestamp(event.timestampMs)}</div>
              </div>
              <pre class="mt-3 overflow-x-auto rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-3 text-[11px] text-neutral-300">{JSON.stringify(event.payload, null, 2)}</pre>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <section class="space-y-4">
      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-sm font-medium text-neutral-100">Current Session Memory</div>
        {#if currentSessionState?.node_memory?.length}
          <div class="mt-3 space-y-2">
            {#each currentSessionState.node_memory as snapshot (`${snapshot.identity.node_id}:${snapshot.status}`)}
              <div class="rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-3">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="text-sm font-medium text-neutral-100">{snapshot.identity.node_id}</div>
                    <div class="mt-1 text-xs text-neutral-500">{snapshot.identity.node_type}</div>
                  </div>
                  <div class="text-[11px] uppercase tracking-[0.18em] text-neutral-400">
                    {formatNodeMemoryStatusLabel(snapshot.status)}
                  </div>
                </div>
                {#if snapshot.input_fingerprint}
                  <div class="mt-2 text-xs text-neutral-500">
                    Input fingerprint: {snapshot.input_fingerprint}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="mt-3 text-sm text-neutral-500">
            No backend-owned node-memory snapshot is currently available for this session.
          </div>
        {/if}
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-sm font-medium text-neutral-100">Latest Dirty Tasks</div>
        {#if selectedRun?.lastDirtyTasks.length}
          <div class="mt-3 flex flex-wrap gap-2">
            {#each selectedRun.lastDirtyTasks as taskId (taskId)}
              <span class="rounded-full border border-amber-800 bg-amber-950/40 px-2 py-1 text-xs text-amber-200">{taskId}</span>
            {/each}
          </div>
        {:else}
          <div class="mt-3 text-sm text-neutral-500">No dirty task set has been observed yet.</div>
        {/if}
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-sm font-medium text-neutral-100">Latest Incremental Execution</div>
        {#if selectedRun?.lastIncrementalTaskIds.length}
          <div class="mt-3 flex flex-wrap gap-2">
            {#each selectedRun.lastIncrementalTaskIds as taskId (taskId)}
              <span class="rounded-full border border-cyan-800 bg-cyan-950/40 px-2 py-1 text-xs text-cyan-200">{taskId}</span>
            {/each}
          </div>
        {:else}
          <div class="mt-3 text-sm text-neutral-500">No incremental task set has been observed yet.</div>
        {/if}
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="flex items-center justify-between gap-3">
          <div class="text-sm font-medium text-neutral-100">Latest Memory Impact</div>
          {#if graphMemoryImpact?.fallback_to_full_invalidation}
            <span class="rounded-full border border-rose-800 bg-rose-950/40 px-2 py-1 text-[11px] uppercase tracking-[0.18em] text-rose-200">
              Fallback
            </span>
          {/if}
        </div>

        {#if graphMemoryImpact}
          <div class="mt-3 grid gap-2 text-sm text-neutral-300 sm:grid-cols-2">
            <div class="rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-2">
              <div class="text-[11px] uppercase tracking-[0.18em] text-neutral-500">Preserved</div>
              <div class="mt-1 text-lg font-medium text-emerald-200">{graphMemoryImpactCounts.preserved}</div>
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-2">
              <div class="text-[11px] uppercase tracking-[0.18em] text-neutral-500">Refresh Inputs</div>
              <div class="mt-1 text-lg font-medium text-cyan-200">{graphMemoryImpactCounts.refreshed}</div>
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-2">
              <div class="text-[11px] uppercase tracking-[0.18em] text-neutral-500">Dropped</div>
              <div class="mt-1 text-lg font-medium text-amber-200">{graphMemoryImpactCounts.dropped}</div>
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-2">
              <div class="text-[11px] uppercase tracking-[0.18em] text-neutral-500">Fallback</div>
              <div class="mt-1 text-lg font-medium text-rose-200">{graphMemoryImpactCounts.fallback}</div>
            </div>
          </div>

          {#if graphMemoryImpact.node_decisions?.length}
            <div class="mt-4 space-y-2">
              {#each graphMemoryImpact.node_decisions ?? [] as decision (`${decision.node_id}:${decision.compatibility}:${decision.reason ?? ''}`)}
                <div class="rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-3">
                  <div class="flex items-center justify-between gap-3">
                    <div class="text-sm font-medium text-neutral-100">{decision.node_id}</div>
                    <div class="text-[11px] uppercase tracking-[0.18em] text-neutral-400">
                      {formatNodeMemoryCompatibilityLabel(decision.compatibility)}
                    </div>
                  </div>
                  {#if decision.reason}
                    <div class="mt-2 text-xs text-neutral-500">{decision.reason}</div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        {:else}
          <div class="mt-3 text-sm text-neutral-500">
            No backend memory-impact summary has been observed yet for the selected run.
          </div>
        {/if}
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-sm font-medium text-neutral-100">Run Graph Context</div>
        {#if selectedRun}
          <div class="mt-3 space-y-2 text-sm text-neutral-300">
            <div class="flex items-center justify-between gap-3">
              <span>Run Fingerprint</span>
              <span class="break-all text-right text-xs text-neutral-400">{selectedRun.graphFingerprintAtStart ?? 'Unavailable'}</span>
            </div>
            <div class="flex items-center justify-between gap-3">
              <span>Workflow</span>
              <span>{selectedRun.workflowName ?? selectedRun.workflowId ?? 'Unknown'}</span>
            </div>
            <div class="flex items-center justify-between gap-3">
              <span>Observed Events</span>
              <span>{selectedRun.eventCount}</span>
            </div>
          </div>
        {:else}
          <div class="mt-3 text-sm text-neutral-500">Select a run to inspect graph context for that execution.</div>
        {/if}
      </article>
    </section>
  </div>
</div>

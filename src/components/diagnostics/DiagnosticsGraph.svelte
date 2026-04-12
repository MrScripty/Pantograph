<script lang="ts">
  import type { DiagnosticsRunTrace, WorkflowDiagnosticsState } from '../../services/diagnostics/types';
  import {
    formatDiagnosticsTimestamp,
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

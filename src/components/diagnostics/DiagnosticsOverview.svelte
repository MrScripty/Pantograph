<script lang="ts">
  import type {
    DiagnosticsNodeTrace,
    DiagnosticsRunTrace,
  } from '../../services/diagnostics/types';
  import {
    formatDiagnosticsDuration,
    formatDiagnosticsPercent,
    formatDiagnosticsTimestamp,
    getDiagnosticsStatusClasses,
    getRunNodeStatusCounts,
  } from './presenters';

  type Props = {
    run: DiagnosticsRunTrace;
    selectedNode?: DiagnosticsNodeTrace | null;
    selectedNodeId?: string | null;
    onSelectNode: (nodeId: string | null) => void;
  };

  let {
    run,
    selectedNode = null,
    selectedNodeId = null,
    onSelectNode,
  }: Props = $props();

  let nodeRows = $derived.by(() => {
    return Object.values(run.nodes).sort((left, right) => {
      const leftStart = left.startedAtMs ?? Number.MAX_SAFE_INTEGER;
      const rightStart = right.startedAtMs ?? Number.MAX_SAFE_INTEGER;
      if (leftStart !== rightStart) {
        return leftStart - rightStart;
      }
      return left.nodeId.localeCompare(right.nodeId);
    });
  });

  let nodeCounts = $derived.by(() => getRunNodeStatusCounts(run));
</script>

<div class="h-full overflow-auto px-4 py-4">
  <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Run Status</div>
      <div class="mt-3 flex items-center gap-3">
        <span class={`inline-flex rounded-full border px-2 py-1 text-xs font-medium ${getDiagnosticsStatusClasses(run.status)}`}>
          {run.status}
        </span>
        <span class="text-sm text-neutral-300">{formatDiagnosticsDuration(run.durationMs)}</span>
      </div>
      <div class="mt-3 text-xs text-neutral-500">
        Started {formatDiagnosticsTimestamp(run.startedAtMs)}
      </div>
      <div class="text-xs text-neutral-500">
        Last update {formatDiagnosticsTimestamp(run.lastUpdatedAtMs)}
      </div>
    </article>

    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Node Health</div>
      <div class="mt-3 grid grid-cols-2 gap-2 text-sm">
        <div class="rounded-lg border border-emerald-900/80 bg-emerald-950/40 px-3 py-2 text-emerald-200">
          {nodeCounts.completed} completed
        </div>
        <div class="rounded-lg border border-cyan-900/80 bg-cyan-950/40 px-3 py-2 text-cyan-200">
          {nodeCounts.running} running
        </div>
        <div class="rounded-lg border border-amber-900/80 bg-amber-950/40 px-3 py-2 text-amber-200">
          {nodeCounts.waiting} waiting
        </div>
        <div class="rounded-lg border border-red-900/80 bg-red-950/40 px-3 py-2 text-red-200">
          {nodeCounts.failed} failed
        </div>
      </div>
      <div class="mt-3 text-xs text-neutral-500">
        {Object.keys(run.nodes).length} observed of {run.nodeCountAtStart} expected nodes
      </div>
    </article>

    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Runtime Signals</div>
      <div class="mt-3 space-y-2 text-sm text-neutral-300">
        <div class="flex items-center justify-between gap-3">
          <span>Runtime</span>
          <span class="truncate text-neutral-400">{run.runtime.runtimeId ?? 'unreported'}</span>
        </div>
        <div class="flex items-center justify-between gap-3">
          <span>Instance</span>
          <span class="truncate text-neutral-400">{run.runtime.runtimeInstanceId ?? 'unreported'}</span>
        </div>
        <div class="flex items-center justify-between">
          <span>Events</span>
          <span>{run.eventCount}</span>
        </div>
        <div class="flex items-center justify-between">
          <span>Streams</span>
          <span>{run.streamEventCount}</span>
        </div>
        <div class="flex items-center justify-between">
          <span>Waiting</span>
          <span>{run.waitingForInput ? 'Yes' : 'No'}</span>
        </div>
      </div>
      {#if run.runtime.modelTarget}
        <div class="mt-3 break-all text-xs text-neutral-500">
          Target {run.runtime.modelTarget}
        </div>
      {/if}
      <div class="mt-2 text-xs text-neutral-500">
        Warmup {formatDiagnosticsDuration(run.runtime.warmupDurationMs)}
        • Reused {run.runtime.runtimeReused === null ? 'unknown' : run.runtime.runtimeReused ? 'yes' : 'no'}
      </div>
      {#if run.runtime.lifecycleDecisionReason}
        <div class="mt-1 text-xs text-neutral-600">
          {run.runtime.lifecycleDecisionReason}
        </div>
      {/if}
      {#if run.error}
        <div class="mt-3 rounded-lg border border-red-900/80 bg-red-950/40 px-3 py-2 text-xs text-red-200">
          {run.error}
        </div>
      {/if}
    </article>

    <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Graph Signals</div>
      <div class="mt-3 space-y-2 text-sm text-neutral-300">
        <div class="flex items-center justify-between gap-3">
          <span>Fingerprint</span>
          <span class="truncate text-neutral-400">{run.graphFingerprintAtStart ?? 'Unavailable'}</span>
        </div>
        <div class="flex items-center justify-between gap-3">
          <span>Dirty Tasks</span>
          <span>{run.lastDirtyTasks.length}</span>
        </div>
        <div class="flex items-center justify-between gap-3">
          <span>Incremental Tasks</span>
          <span>{run.lastIncrementalTaskIds.length}</span>
        </div>
      </div>
      {#if run.lastDirtyTasks.length > 0}
        <div class="mt-3 text-xs text-neutral-500">{run.lastDirtyTasks.join(', ')}</div>
      {/if}
    </article>
  </div>

  <div class="mt-4 grid gap-4 xl:grid-cols-[minmax(0,2fr)_minmax(18rem,1fr)]">
    <section class="rounded-xl border border-neutral-800 bg-neutral-950/80">
      <header class="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
        <div>
          <div class="text-sm font-medium text-neutral-100">Observed Nodes</div>
          <div class="text-xs text-neutral-500">Select a node to inspect its latest execution state.</div>
        </div>
        {#if selectedNodeId}
          <button
            type="button"
            class="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100"
            onclick={() => onSelectNode(null)}
          >
            Clear Selection
          </button>
        {/if}
      </header>

      <div class="overflow-auto">
        <table class="min-w-full divide-y divide-neutral-800 text-sm">
          <thead class="bg-neutral-950/90 text-left text-xs uppercase tracking-[0.24em] text-neutral-500">
            <tr>
              <th class="px-4 py-3 font-medium">Node</th>
              <th class="px-4 py-3 font-medium">Status</th>
              <th class="px-4 py-3 font-medium">Duration</th>
              <th class="px-4 py-3 font-medium">Progress</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-neutral-900">
            {#each nodeRows as node (node.nodeId)}
              <tr class:selected-row={selectedNodeId === node.nodeId}>
                <td class="px-4 py-3">
                  <button
                    type="button"
                    class="w-full text-left"
                    onclick={() => onSelectNode(node.nodeId)}
                  >
                    <div class="font-medium text-neutral-100">{node.nodeId}</div>
                    <div class="text-xs text-neutral-500">{node.nodeType ?? 'unknown-node-type'}</div>
                  </button>
                </td>
                <td class="px-4 py-3">
                  <span class={`inline-flex rounded-full border px-2 py-1 text-xs font-medium ${getDiagnosticsStatusClasses(node.status)}`}>
                    {node.status}
                  </span>
                </td>
                <td class="px-4 py-3 text-neutral-300">{formatDiagnosticsDuration(node.durationMs)}</td>
                <td class="px-4 py-3 text-neutral-300">{formatDiagnosticsPercent(node.lastProgress)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>

    <section class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
      <div class="text-sm font-medium text-neutral-100">
        {selectedNode ? `Node ${selectedNode.nodeId}` : 'Node Detail'}
      </div>
      {#if selectedNode}
        <div class="mt-3 space-y-3 text-sm text-neutral-300">
          <div class="flex items-center justify-between gap-3">
            <span>Status</span>
            <span class={`inline-flex rounded-full border px-2 py-1 text-xs font-medium ${getDiagnosticsStatusClasses(selectedNode.status)}`}>
              {selectedNode.status}
            </span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span>Started</span>
            <span>{formatDiagnosticsTimestamp(selectedNode.startedAtMs)}</span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span>Ended</span>
            <span>{formatDiagnosticsTimestamp(selectedNode.endedAtMs)}</span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span>Duration</span>
            <span>{formatDiagnosticsDuration(selectedNode.durationMs)}</span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span>Progress</span>
            <span>{formatDiagnosticsPercent(selectedNode.lastProgress)}</span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span>Stream Events</span>
            <span>{selectedNode.streamEventCount}</span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span>Total Events</span>
            <span>{selectedNode.eventCount}</span>
          </div>
          {#if selectedNode.lastMessage}
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/80 px-3 py-2 text-xs text-neutral-300">
              {selectedNode.lastMessage}
            </div>
          {/if}
          {#if selectedNode.error}
            <div class="rounded-lg border border-red-900/80 bg-red-950/40 px-3 py-2 text-xs text-red-200">
              {selectedNode.error}
            </div>
          {/if}
        </div>
      {:else}
        <div class="mt-3 rounded-xl border border-dashed border-neutral-800 bg-neutral-950/60 px-4 py-6 text-sm text-neutral-500">
          Select a node row to inspect its duration, progress, messages, and error state.
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .selected-row {
    background: rgba(14, 165, 233, 0.1);
  }
</style>

<script lang="ts">
  import type { DiagnosticsNodeTrace } from '../../services/diagnostics/types';
  import DiagnosticsTimingExpectation from './DiagnosticsTimingExpectation.svelte';
  import {
    formatDiagnosticsDuration,
    formatDiagnosticsPercent,
    formatDiagnosticsTimestamp,
    getDiagnosticsStatusClasses,
  } from './presenters';

  type Props = {
    node?: DiagnosticsNodeTrace | null;
  };

  let { node = null }: Props = $props();
</script>

<section class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
  <div class="text-sm font-medium text-neutral-100">
    {node ? `Node ${node.nodeId}` : 'Node Detail'}
  </div>
  {#if node}
    <div class="mt-3 space-y-3 text-sm text-neutral-300">
      <div class="flex items-center justify-between gap-3">
        <span>Status</span>
        <span class={`inline-flex rounded-full border px-2 py-1 text-xs font-medium ${getDiagnosticsStatusClasses(node.status)}`}>
          {node.status}
        </span>
      </div>
      <div class="flex items-center justify-between gap-3">
        <span>Started</span>
        <span>{formatDiagnosticsTimestamp(node.startedAtMs)}</span>
      </div>
      <div class="flex items-center justify-between gap-3">
        <span>Ended</span>
        <span>{formatDiagnosticsTimestamp(node.endedAtMs)}</span>
      </div>
      <div class="flex items-center justify-between gap-3">
        <span>Duration</span>
        <span>{formatDiagnosticsDuration(node.durationMs)}</span>
      </div>
      <div class="flex items-start justify-between gap-3">
        <span>Expected</span>
        <DiagnosticsTimingExpectation expectation={node.timingExpectation ?? null} align="right" />
      </div>
      {#if node.lastProgress !== null}
        <div class="flex items-center justify-between gap-3">
          <span>Reported Progress</span>
          <span>{formatDiagnosticsPercent(node.lastProgress)}</span>
        </div>
      {/if}
      <div class="flex items-center justify-between gap-3">
        <span>Stream Events</span>
        <span>{node.streamEventCount}</span>
      </div>
      <div class="flex items-center justify-between gap-3">
        <span>Total Events</span>
        <span>{node.eventCount}</span>
      </div>
      {#if node.lastMessage}
        <div class="rounded-lg border border-neutral-800 bg-neutral-900/80 px-3 py-2 text-xs text-neutral-300">
          {node.lastMessage}
        </div>
      {/if}
      {#if node.error}
        <div class="rounded-lg border border-red-900/80 bg-red-950/40 px-3 py-2 text-xs text-red-200">
          {node.error}
        </div>
      {/if}
    </div>
  {:else}
    <div class="mt-3 rounded-xl border border-dashed border-neutral-800 bg-neutral-950/60 px-4 py-6 text-sm text-neutral-500">
      Select a node row to inspect its duration, expected timing, messages, and error state.
    </div>
  {/if}
</section>

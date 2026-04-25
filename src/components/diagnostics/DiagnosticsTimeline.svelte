<script lang="ts">
  import type { DiagnosticsRunTrace } from '../../services/diagnostics/types';
  import DiagnosticsTimingExpectation from './DiagnosticsTimingExpectation.svelte';
  import {
    formatDiagnosticsDuration,
    formatDiagnosticsTimestamp,
    getDiagnosticsStatusClasses,
  } from './presenters';

  type Props = {
    run: DiagnosticsRunTrace;
    selectedNodeId?: string | null;
    onSelectNode: (nodeId: string | null) => void;
  };

  let {
    run,
    selectedNodeId = null,
    onSelectNode,
  }: Props = $props();

  type TimelineRow = {
    nodeId: string;
    nodeType: string | null;
    status: DiagnosticsRunTrace['nodes'][string]['status'];
    startedAtMs: number | null;
    endedAtMs: number | null;
    durationMs: number | null;
    lastMessage: string | null;
    timingExpectation: DiagnosticsRunTrace['nodes'][string]['timingExpectation'];
    barLeft: number;
    barWidth: number;
  };

  let timelineRows = $derived.by<TimelineRow[]>(() => {
    const fallbackEnd = run.endedAtMs ?? run.lastUpdatedAtMs;
    const totalSpanMs = Math.max(fallbackEnd - run.startedAtMs, 1);

    return Object.values(run.nodes)
      .sort((left, right) => {
        const leftStart = left.startedAtMs ?? Number.MAX_SAFE_INTEGER;
        const rightStart = right.startedAtMs ?? Number.MAX_SAFE_INTEGER;
        if (leftStart !== rightStart) {
          return leftStart - rightStart;
        }
        return left.nodeId.localeCompare(right.nodeId);
      })
      .map((node) => {
        const startOffsetMs = node.startedAtMs === null ? 0 : Math.max(node.startedAtMs - run.startedAtMs, 0);
        const endTimestamp = node.endedAtMs ?? fallbackEnd;
        const widthMs = node.startedAtMs === null
          ? 0
          : Math.max(endTimestamp - node.startedAtMs, 0);

        return {
          ...node,
          barLeft: (startOffsetMs / totalSpanMs) * 100,
          barWidth: node.startedAtMs === null ? 0 : Math.max((widthMs / totalSpanMs) * 100, 2),
        };
      });
  });
</script>

<div class="h-full overflow-auto px-4 py-4">
  <div class="rounded-xl border border-neutral-800 bg-neutral-950/80">
    <header class="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
      <div>
        <div class="text-sm font-medium text-neutral-100">Node Timeline</div>
        <div class="text-xs text-neutral-500">
          Relative node spans within run {run.executionId}
        </div>
      </div>
      <div class="text-xs text-neutral-500">
        Started {formatDiagnosticsTimestamp(run.startedAtMs)} • Last update {formatDiagnosticsTimestamp(run.lastUpdatedAtMs)}
      </div>
    </header>

    <div class="divide-y divide-neutral-900">
      {#each timelineRows as node (node.nodeId)}
        <button
          type="button"
          class:selected-row={selectedNodeId === node.nodeId}
          class="grid w-full grid-cols-[15rem_minmax(0,1fr)_12rem] items-center gap-4 px-4 py-3 text-left transition-colors hover:bg-neutral-900/80"
          onclick={() => onSelectNode(node.nodeId)}
        >
          <div class="min-w-0">
            <div class="flex items-center gap-2">
              <span class="truncate text-sm font-medium text-neutral-100">{node.nodeId}</span>
              <span class={`inline-flex rounded-full border px-2 py-0.5 text-[11px] font-medium ${getDiagnosticsStatusClasses(node.status)}`}>
                {node.status}
              </span>
            </div>
            <div class="truncate text-xs text-neutral-500">{node.nodeType ?? 'unknown-node-type'}</div>
            {#if node.lastMessage}
              <div class="mt-1 truncate text-xs text-neutral-400">{node.lastMessage}</div>
            {/if}
          </div>

          <div>
            <div class="relative h-3 rounded-full bg-neutral-900">
              {#if node.barWidth > 0}
                <div
                  class="absolute inset-y-0 rounded-full bg-cyan-500/80"
                  style={`left: ${node.barLeft}%; width: ${node.barWidth}%;`}
                ></div>
              {/if}
            </div>
            <div class="mt-2 flex items-center justify-between text-[11px] text-neutral-500">
              <span>{formatDiagnosticsTimestamp(node.startedAtMs)}</span>
              <span>{formatDiagnosticsTimestamp(node.endedAtMs)}</span>
            </div>
          </div>

          <div class="text-right text-xs text-neutral-400">
            <div>{formatDiagnosticsDuration(node.durationMs)}</div>
            <DiagnosticsTimingExpectation expectation={node.timingExpectation ?? null} align="right" />
          </div>
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .selected-row {
    background: rgba(14, 165, 233, 0.1);
  }
</style>

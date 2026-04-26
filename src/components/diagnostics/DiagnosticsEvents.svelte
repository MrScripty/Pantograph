<script lang="ts">
  import type { DiagnosticsRunTrace } from '../../services/diagnostics/types';
  import {
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

  let eventRows = $derived.by(() => [...run.events].reverse());
</script>

<div class="h-full overflow-auto px-4 py-4">
  <div class="rounded-xl border border-neutral-800 bg-neutral-950/80">
    <header class="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
      <div>
        <div class="text-sm font-medium text-neutral-100">Event Stream</div>
        <div class="text-xs text-neutral-500">
          Most recent {run.events.length} retained events for execution {run.workflowRunId}
        </div>
      </div>
      <div class="text-xs text-neutral-500">
        Node selection {selectedNodeId ?? 'none'}
      </div>
    </header>

    <div class="divide-y divide-neutral-900">
      {#each eventRows as event (event.id)}
        <details class="group">
          <summary class="list-none">
            <div class:selected-row={selectedNodeId !== null && selectedNodeId === event.nodeId}
              class="grid cursor-pointer grid-cols-[9rem_8rem_minmax(0,1fr)_10rem] items-center gap-3 px-4 py-3 transition-colors group-open:bg-neutral-900/70 hover:bg-neutral-900/70"
            >
              <div class="text-xs text-neutral-500">{formatDiagnosticsTimestamp(event.timestampMs)}</div>
              <div>
                <span class={`inline-flex rounded-full border px-2 py-1 text-[11px] font-medium ${getDiagnosticsStatusClasses(
                  event.type === 'Failed'
                    ? 'failed'
                    : event.type === 'Completed'
                      ? 'completed'
                      : event.type === 'WaitingForInput'
                        ? 'waiting'
                        : 'running'
                )}`}>
                  {event.type}
                </span>
              </div>
              <div class="min-w-0">
                <div class="truncate text-sm text-neutral-100">{event.summary}</div>
                <div class="truncate text-xs text-neutral-500">{event.workflowRunId}</div>
              </div>
              <div class="flex items-center justify-end gap-2">
                {#if event.nodeId}
                  <button
                    type="button"
                    class="rounded border border-neutral-700 px-2 py-1 text-[11px] text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100"
                    onclick={(clickEvent) => {
                      clickEvent.preventDefault();
                      clickEvent.stopPropagation();
                      onSelectNode(event.nodeId);
                    }}
                  >
                    {event.nodeId}
                  </button>
                {:else}
                  <span class="text-[11px] text-neutral-600">workflow</span>
                {/if}
              </div>
            </div>
          </summary>
          <div class="border-t border-neutral-900 bg-neutral-950/60 px-4 py-3">
            <pre class="overflow-x-auto rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-3 text-[11px] text-neutral-300">{JSON.stringify(event.payload, null, 2)}</pre>
          </div>
        </details>
      {/each}
    </div>
  </div>
</div>

<style>
  .selected-row {
    background: rgba(14, 165, 233, 0.1);
  }
</style>

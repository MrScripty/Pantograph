<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      vector?: unknown;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  function parseVector(value: unknown): number[] | null {
    if (Array.isArray(value)) {
      const out: number[] = [];
      for (const item of value) {
        if (typeof item !== 'number' || !Number.isFinite(item)) return null;
        out.push(item);
      }
      return out;
    }

    if (typeof value === 'string') {
      try {
        return parseVector(JSON.parse(value));
      } catch {
        return null;
      }
    }

    return null;
  }

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let vector = $derived(parseVector(data.vector));
  let preview = $derived(vector ? vector.slice(0, 8).map((v) => v.toFixed(4)).join(', ') : '');
  let hasMore = $derived(vector ? vector.length > 8 : false);
  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-cyan-500 animate-pulse',
      success: 'bg-cyan-500',
      error: 'bg-red-500',
    }[executionState]
  );
</script>

<div class="vector-output-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-cyan-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 8h10M7 12h10M7 16h6M4 6h16v12H4z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Vector Output'}</span>
        <span class="w-2 h-2 rounded-full {statusColor} ml-auto"></span>
      </div>
    {/snippet}

      {#if vector}
        <div class="text-xs text-cyan-300 mb-1">{vector.length} dimensions</div>
        <div class="copyable-output nodrag nopan nowheel p-2 bg-neutral-900 rounded text-[10px] text-neutral-300 max-h-28 overflow-y-auto break-all">
          [{preview}{#if hasMore}, ...{/if}]
        </div>
      {:else}
        <div class="text-xs text-neutral-500 italic">
          No vector yet
        </div>
      {/if}
  </BaseNode>
</div>

<style>
  .vector-output-node-wrapper :global(.base-node) {
    border-color: rgba(8, 145, 178, 0.5);
  }

  .vector-output-node-wrapper :global(.node-header) {
    background-color: rgba(8, 145, 178, 0.2);
    border-color: rgba(8, 145, 178, 0.3);
  }

  .copyable-output {
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }
</style>

<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';

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

  let vector = $derived(parseVector(data.vector));
  let preview = $derived(vector ? vector.slice(0, 8).map((v) => v.toFixed(4)).join(', ') : '');
  let hasMore = $derived(vector ? vector.length > 8 : false);
</script>

<div class="vector-output-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'Vector Output'} />
    {/snippet}

      {#if vector}
        <div class="text-xs text-pg-text-soft mb-1">{vector.length} dimensions</div>
        <div class="copyable-output nodrag nopan nowheel p-2 bg-pg-panel-sunken rounded text-[10px] text-pg-text-soft max-h-28 overflow-y-auto break-all">
          [{preview}{#if hasMore}, ...{/if}]
        </div>
      {:else}
        <div class="text-xs text-pg-text-muted italic">
          No vector yet
        </div>
      {/if}
  </BaseNode>
</div>

<style>
  .copyable-output {
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }
</style>

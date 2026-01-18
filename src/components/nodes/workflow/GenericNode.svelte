<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, NodeExecutionState } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionState = $derived($nodeExecutionStates.get(id) || 'idle');

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-blue-500 animate-pulse',
      success: 'bg-green-500',
      error: 'bg-red-500',
    }[executionState]
  );

  let statusText = $derived(
    {
      idle: 'Idle',
      running: 'Running...',
      success: 'Complete',
      error: 'Error',
    }[executionState]
  );

  let categoryColor = $derived(
    {
      Input: 'border-blue-600/50',
      Processing: 'border-green-600/50',
      Tool: 'border-amber-600/50',
      Output: 'border-cyan-600/50',
      Control: 'border-purple-600/50',
    }[data.definition?.category || 'Processing']
  );
</script>

<div class="generic-node-wrapper {categoryColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet children()}
      <div class="flex items-center gap-2">
        <span class="w-2 h-2 rounded-full {statusColor}"></span>
        <span class="text-xs text-neutral-400">{statusText}</span>
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .generic-node-wrapper :global(.base-node) {
    border-color: inherit;
  }
</style>

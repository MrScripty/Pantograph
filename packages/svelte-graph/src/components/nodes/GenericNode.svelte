<script lang="ts">
  import BaseNode from './BaseNode.svelte';
  import type { NodeDefinition } from '../../types/workflow.js';
  import { useGraphContext } from '../../context/useGraphContext.js';

  const { stores } = useGraphContext();
  const nodeExecutionStates = stores.workflow.nodeExecutionStates;

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-blue-500 animate-pulse',
      success: 'bg-green-500',
      error: 'bg-red-500',
    }[executionState as string]
  );

  let statusText = $derived(
    {
      idle: 'Idle',
      running: 'Running...',
      success: 'Complete',
      error: 'Error',
    }[executionState as string]
  );

  let categoryColor = $derived(
    {
      input: 'border-blue-600/50',
      processing: 'border-green-600/50',
      tool: 'border-amber-600/50',
      output: 'border-cyan-600/50',
      control: 'border-purple-600/50',
    }[data.definition?.category || 'processing']
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

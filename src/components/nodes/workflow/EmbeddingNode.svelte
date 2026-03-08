<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { nodeExecutionStates, edges } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let errorMessage = $derived(executionInfo?.errorMessage || '');

  let hasTextInput = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'text')
  );

  const nodeColor = '#0ea5e9';
  let statusText = $derived(
    {
      idle: 'Ready',
      running: 'Embedding...',
      success: 'Vector Ready',
      error: 'Error',
    }[executionState]
  );
</script>

<div class="embedding-node-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {nodeColor}">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7h16M4 12h10M4 17h7" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'LlamaCpp Embedding'}</span>
      </div>
    {/snippet}

      <div class="space-y-2">
        <div class="text-xs text-neutral-400">{statusText}</div>
        {#if !hasTextInput}
          <div class="text-[10px] text-amber-400">Connect text input</div>
        {/if}
        {#if executionState === 'error' && errorMessage}
          <div class="p-2 bg-red-950/40 border border-red-600/30 rounded text-[10px] text-red-200">
            {errorMessage}
          </div>
        {/if}
      </div>
  </BaseNode>
</div>

<style>
  .embedding-node-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .embedding-node-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }
</style>

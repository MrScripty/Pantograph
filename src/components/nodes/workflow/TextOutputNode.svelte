<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, NodeExecutionState } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      text?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionState = $derived($nodeExecutionStates.get(id) || 'idle');
  let text = $derived(data.text || '');

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-cyan-500 animate-pulse',
      success: 'bg-cyan-500',
      error: 'bg-red-500',
    }[executionState]
  );
</script>

<div class="output-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-cyan-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Text Output'}</span>
        <span class="w-2 h-2 rounded-full {statusColor} ml-auto"></span>
      </div>
    {/snippet}

    {#snippet children()}
      {#if text}
        <div class="p-2 bg-neutral-900 rounded text-xs text-neutral-300 max-h-32 overflow-y-auto whitespace-pre-wrap">
          {text}
        </div>
      {:else}
        <div class="text-xs text-neutral-500 italic">
          No output yet
        </div>
      {/if}
    {/snippet}
  </BaseNode>
</div>

<style>
  .output-node-wrapper :global(.base-node) {
    border-color: rgba(8, 145, 178, 0.5);
  }

  .output-node-wrapper :global(.node-header) {
    background-color: rgba(8, 145, 178, 0.2);
    border-color: rgba(8, 145, 178, 0.3);
  }
</style>

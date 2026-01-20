<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, NodeExecutionState } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      modelName?: string;
      streamContent?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionState = $derived($nodeExecutionStates.get(id) || 'idle');
  let modelName = $derived(data.modelName || 'Local LLM');
  let streamContent = $derived(data.streamContent || '');

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-green-500 animate-pulse',
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
</script>

<div class="llm-node-wrapper border-green-600/50">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-green-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
          </svg>
        </div>
        <div class="flex-1 min-w-0">
          <span class="text-sm font-medium text-neutral-200">{data.label || 'LLM Inference'}</span>
        </div>
      </div>
    {/snippet}

    {#snippet children()}
      <div class="space-y-2">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full {statusColor}"></span>
          <span class="text-xs text-neutral-400">{statusText}</span>
        </div>
        <div class="flex justify-between items-center text-xs">
          <span class="text-neutral-400">Model:</span>
          <span class="text-neutral-200 font-mono text-[10px]">{modelName}</span>
        </div>
        {#if streamContent}
          <div class="p-2 bg-neutral-900 rounded text-xs text-neutral-300 max-h-20 overflow-y-auto">
            {streamContent}
          </div>
        {/if}
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .llm-node-wrapper :global(.base-node) {
    border-color: rgba(22, 163, 74, 0.5);
  }

  .llm-node-wrapper :global(.node-header) {
    background-color: rgba(22, 163, 74, 0.2);
    border-color: rgba(22, 163, 74, 0.3);
  }
</style>

<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { nodeExecutionStates, edges } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      streamContent?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  // Get execution info (new format with state and errorMessage)
  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let streamContent = $derived(data.streamContent || '');

  // Check if model_path input is connected
  let isModelConnected = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'model_path')
  );

  // Processing node color (purple for llama.cpp to distinguish from Ollama's green)
  const nodeColor = '#9333ea';

  let statusText = $derived(
    {
      idle: 'Ready',
      running: 'Generating...',
      success: 'Complete',
      error: 'Error',
    }[executionState]
  );
</script>

<div class="llamacpp-node-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {nodeColor}">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'LlamaCpp Inference'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      <div class="space-y-2">
        <div class="flex items-center gap-2 text-xs text-neutral-400">
          <span>{statusText}</span>
        </div>
        {#if !isModelConnected}
          <div class="text-[10px] text-amber-400">
            Connect a Puma-Lib node
          </div>
        {/if}
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
  .llamacpp-node-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .llamacpp-node-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }
</style>

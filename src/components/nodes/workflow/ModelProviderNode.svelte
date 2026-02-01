<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData, edges } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      model_name?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let modelName = $state(data.model_name || 'llama2');

  // Input category color (blue)
  const nodeColor = '#2563eb';

  // Check if model_name input is connected
  let isModelConnected = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'model_name')
  );

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    modelName = target.value;
    updateNodeData(id, { model_name: modelName });
  }
</script>

<div class="model-provider-node-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {nodeColor}">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Model Provider'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      {#if isModelConnected}
        <div class="text-xs text-neutral-400 italic py-1">
          Model from connection
        </div>
      {:else}
        <div class="flex flex-col gap-1">
          <label class="text-[10px] text-neutral-400">Model Name</label>
          <input
            type="text"
            class="w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-sm text-neutral-200 focus:outline-none focus:border-blue-500"
            placeholder="e.g., llama2, codellama:7b"
            value={modelName}
            oninput={handleInput}
          />
        </div>
      {/if}
    {/snippet}
  </BaseNode>
</div>

<style>
  .model-provider-node-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .model-provider-node-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }
</style>

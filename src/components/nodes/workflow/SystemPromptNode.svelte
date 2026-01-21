<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData, edges } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      prompt?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let prompt = $state(data.prompt || '');

  // Check if the 'prompt' input is connected
  let isPromptConnected = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'prompt')
  );

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    prompt = target.value;
    updateNodeData(id, { prompt });
  }
</script>

<div class="system-prompt-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-blue-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'System Prompt'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      {#if isPromptConnected}
        <div class="text-xs text-neutral-400 italic py-1">
          Connected to external input
        </div>
      {:else}
        <textarea
          class="w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-sm text-neutral-200 resize-none focus:outline-none focus:border-blue-500"
          rows="4"
          placeholder="Enter system prompt..."
          value={prompt}
          oninput={handleInput}
        ></textarea>
      {/if}
    {/snippet}
  </BaseNode>
</div>

<style>
  .system-prompt-node-wrapper :global(.base-node) {
    border-color: rgba(37, 99, 235, 0.5);
  }

  .system-prompt-node-wrapper :global(.node-header) {
    background-color: rgba(37, 99, 235, 0.2);
    border-color: rgba(37, 99, 235, 0.3);
  }
</style>

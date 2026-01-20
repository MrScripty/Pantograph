<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';

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

  let text = $state(data.text || '');

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    text = target.value;
    updateNodeData(id, { text });
  }
</script>

<div class="text-input-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-blue-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Text Input'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      <textarea
        class="w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-sm text-neutral-200 resize-none focus:outline-none focus:border-blue-500"
        rows="3"
        placeholder="Enter text..."
        value={text}
        oninput={handleInput}
      ></textarea>
    {/snippet}
  </BaseNode>
</div>

<style>
  .text-input-node-wrapper :global(.base-node) {
    border-color: rgba(37, 99, 235, 0.5);
  }

  .text-input-node-wrapper :global(.node-header) {
    background-color: rgba(37, 99, 235, 0.2);
    border-color: rgba(37, 99, 235, 0.3);
  }
</style>

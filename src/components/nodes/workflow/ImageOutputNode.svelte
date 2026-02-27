<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, NodeExecutionState } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      image?: string;
      streamContent?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let imageData = $derived(data.image || '');
  let imageSrc = $derived(imageData ? `data:image/png;base64,${imageData}` : '');

  let showModal = $state(false);

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-violet-500 animate-pulse',
      success: 'bg-violet-500',
      error: 'bg-red-500',
    }[executionState]
  );

  function downloadImage() {
    if (!imageData) return;
    const byteChars = atob(imageData);
    const bytes = new Uint8Array(byteChars.length);
    for (let i = 0; i < byteChars.length; i++) {
      bytes[i] = byteChars.charCodeAt(i);
    }
    const blob = new Blob([bytes], { type: 'image/png' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'output.png';
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="image-output-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-violet-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Image Output'}</span>
        <span class="w-2 h-2 rounded-full {statusColor} ml-auto"></span>
      </div>
    {/snippet}

    {#snippet children()}
      {#if imageSrc}
        <div class="space-y-1">
          <button
            class="w-full cursor-pointer border-0 bg-transparent p-0"
            onclick={() => (showModal = true)}
          >
            <img src={imageSrc} alt="Output" class="max-h-40 w-full object-contain rounded" />
          </button>
          <div class="flex justify-end gap-1">
            <button
              class="text-[10px] text-neutral-400 hover:text-neutral-200 bg-transparent border-0 cursor-pointer px-1"
              onclick={downloadImage}
            >
              Download
            </button>
            <button
              class="text-[10px] text-neutral-400 hover:text-neutral-200 bg-transparent border-0 cursor-pointer px-1"
              onclick={() => (showModal = true)}
            >
              Expand
            </button>
          </div>
        </div>
      {:else}
        <div class="text-xs text-neutral-500 italic">
          No image yet
        </div>
      {/if}
    {/snippet}
  </BaseNode>
</div>

{#if showModal && imageSrc}
  <button
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 border-0 cursor-default p-4"
    onclick={() => (showModal = false)}
    onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
  >
    <img src={imageSrc} alt="Full resolution output" class="max-w-full max-h-full object-contain" />
  </button>
{/if}

<style>
  .image-output-wrapper :global(.base-node) {
    border-color: rgba(139, 92, 246, 0.5);
  }

  .image-output-wrapper :global(.node-header) {
    background-color: rgba(139, 92, 246, 0.2);
    border-color: rgba(139, 92, 246, 0.3);
  }
</style>

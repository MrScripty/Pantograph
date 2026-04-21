<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
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
  let modalElement = $state<HTMLDialogElement | null>(null);

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-violet-500 animate-pulse',
      success: 'bg-violet-500',
      error: 'bg-red-500',
    }[executionState]
  );

  function stopControlEvent(event: Event) {
    event.stopPropagation();
  }

  function openModal(event?: Event) {
    event?.stopPropagation();
    showModal = true;
  }

  function closeModal() {
    showModal = false;
  }

  function downloadImage(event?: Event) {
    event?.stopPropagation();
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
    requestAnimationFrame(() => URL.revokeObjectURL(url));
  }

  $effect(() => {
    if (!modalElement) {
      return;
    }

    if (showModal) {
      if (!modalElement.open) {
        modalElement.showModal();
      }
      return;
    }

    if (modalElement.open) {
      modalElement.close();
    }
  });
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

      {#if imageSrc}
        <div class="space-y-1">
          <button type="button"
            class="nodrag nopan nowheel w-full cursor-pointer border-0 bg-transparent p-0"
            onclick={openModal}
            aria-label="Open output image preview"
            onmousedown={stopControlEvent}
            onmouseup={stopControlEvent}
            onpointerdown={stopControlEvent}
            onpointerup={stopControlEvent}
            onclickcapture={stopControlEvent}
          >
            <img src={imageSrc} alt="Output" class="max-h-40 w-full object-contain rounded" />
          </button>
          <div class="flex justify-end gap-1">
            <button type="button"
              class="nodrag nopan nowheel text-[10px] text-neutral-400 hover:text-neutral-200 bg-transparent border-0 cursor-pointer px-1"
              onclick={downloadImage}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onpointerdown={stopControlEvent}
              onpointerup={stopControlEvent}
              onclickcapture={stopControlEvent}
            >
              Download
            </button>
            <button type="button"
              class="nodrag nopan nowheel text-[10px] text-neutral-400 hover:text-neutral-200 bg-transparent border-0 cursor-pointer px-1"
              onclick={openModal}
              onmousedown={stopControlEvent}
              onmouseup={stopControlEvent}
              onpointerdown={stopControlEvent}
              onpointerup={stopControlEvent}
              onclickcapture={stopControlEvent}
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
  </BaseNode>
</div>

<dialog
  bind:this={modalElement}
  class="image-preview-dialog"
  onclick={(event) => event.target === modalElement && closeModal()}
  onclose={closeModal}
>
  {#if imageSrc}
    <div class="dialog-content" onclick={stopControlEvent}>
      <button type="button" class="nodrag nopan nowheel dialog-close" onclick={closeModal}>
        Close
      </button>
      <img src={imageSrc} alt="Full resolution output" class="dialog-image" />
    </div>
  {/if}
</dialog>

<style>
  .image-output-wrapper :global(.base-node) {
    border-color: rgba(139, 92, 246, 0.5);
  }

  .image-output-wrapper :global(.node-header) {
    background-color: rgba(139, 92, 246, 0.2);
    border-color: rgba(139, 92, 246, 0.3);
  }

  .image-preview-dialog {
    width: min(96vw, 1600px);
    max-width: 96vw;
    height: min(96vh, 1100px);
    max-height: 96vh;
    border: 0;
    border-radius: 16px;
    padding: 0;
    background: rgba(10, 10, 10, 0.96);
    color: white;
  }

  .image-preview-dialog::backdrop {
    background: rgba(0, 0, 0, 0.82);
  }

  .dialog-content {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    padding: 1rem;
  }

  .dialog-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 12px;
  }

  .dialog-close {
    position: absolute;
    top: 1rem;
    right: 1rem;
    border: 0;
    border-radius: 999px;
    padding: 0.4rem 0.75rem;
    background: rgba(38, 38, 38, 0.9);
    color: rgb(229, 229, 229);
    cursor: pointer;
    font-size: 0.75rem;
  }

  .dialog-close:hover {
    background: rgba(82, 82, 82, 0.95);
  }
</style>

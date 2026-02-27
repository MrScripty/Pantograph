<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, NodeExecutionState } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      audio?: string;
      streamContent?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let audioData = $derived(data.audio || '');
  let audioSrc = $derived(audioData ? `data:audio/wav;base64,${audioData}` : '');

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-pink-500 animate-pulse',
      success: 'bg-pink-500',
      error: 'bg-red-500',
    }[executionState]
  );

  function downloadAudio() {
    if (!audioData) return;
    const byteChars = atob(audioData);
    const bytes = new Uint8Array(byteChars.length);
    for (let i = 0; i < byteChars.length; i++) {
      bytes[i] = byteChars.charCodeAt(i);
    }
    const blob = new Blob([bytes], { type: 'audio/wav' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'output.wav';
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="audio-output-wrapper" style="--node-color: #f472b6">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-pink-500 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Audio Output'}</span>
        <span class="w-2 h-2 rounded-full {statusColor} ml-auto"></span>
      </div>
    {/snippet}

    {#snippet children()}
      {#if audioSrc}
        <div class="space-y-1">
          <audio controls src={audioSrc} class="w-full h-8"></audio>
          <div class="flex justify-end">
            <button
              class="text-[10px] text-neutral-400 hover:text-neutral-200 bg-transparent border-0 cursor-pointer px-1"
              onclick={downloadAudio}
            >
              Download
            </button>
          </div>
        </div>
      {:else}
        <div class="text-xs text-neutral-500 italic">
          No audio yet
        </div>
      {/if}
    {/snippet}
  </BaseNode>
</div>

<style>
  .audio-output-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .audio-output-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }
</style>

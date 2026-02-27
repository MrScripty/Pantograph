<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      audio_data?: string;
      fileName?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  const nodeColor = '#f472b6';

  async function pickFile() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');

      const result = await open({
        filters: [{ name: 'Audio', extensions: ['wav', 'mp3', 'ogg', 'flac'] }],
      });

      if (result) {
        const filePath = typeof result === 'string' ? result : result.path;
        const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'audio';
        updateNodeData(id, { audio_data: filePath, fileName });
      }
    } catch {
      // Tauri not available or user cancelled
    }
  }
</script>

<div class="audio-input-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {nodeColor}">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Audio Input'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      <div class="space-y-2">
        <button
          class="w-full text-xs px-2 py-1 rounded bg-neutral-700 hover:bg-neutral-600 text-neutral-300 border border-neutral-600 cursor-pointer"
          onclick={pickFile}
        >
          Choose File
        </button>
        {#if data.fileName}
          <div class="text-[10px] text-neutral-400 truncate">{data.fileName}</div>
        {/if}
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .audio-input-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .audio-input-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }
</style>

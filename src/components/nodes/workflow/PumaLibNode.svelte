<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';
  import { open } from '@tauri-apps/plugin-dialog';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      modelPath?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let modelPath = $state(data.modelPath || '');

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    modelPath = target.value;
    updateNodeData(id, { modelPath });
  }

  async function browseForModel() {
    try {
      const result = await open({
        title: 'Select AI Model File',
        filters: [
          {
            name: 'GGUF Models',
            extensions: ['gguf'],
          },
          {
            name: 'All Files',
            extensions: ['*'],
          },
        ],
        multiple: false,
        directory: false,
      });

      if (result && typeof result === 'string') {
        modelPath = result;
        updateNodeData(id, { modelPath });
      }
    } catch (error) {
      console.error('File picker error:', error);
    }
  }
</script>

<div class="puma-lib-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-amber-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Puma-Lib'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      <div class="space-y-2">
        <label class="text-xs text-neutral-400">Model Path</label>
        <div class="flex gap-1">
          <input
            type="text"
            class="flex-1 bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-xs text-neutral-200 focus:outline-none focus:border-amber-500 font-mono truncate"
            placeholder="/path/to/model.gguf"
            value={modelPath}
            oninput={handleInput}
          />
          <button
            type="button"
            class="px-2 py-1 bg-amber-600 hover:bg-amber-500 text-white text-xs rounded flex-shrink-0"
            onclick={browseForModel}
          >
            Browse
          </button>
        </div>
        {#if modelPath}
          <div class="text-[10px] text-neutral-500 truncate" title={modelPath}>
            {modelPath.split('/').pop()}
          </div>
        {/if}
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .puma-lib-node-wrapper :global(.base-node) {
    border-color: rgba(217, 119, 6, 0.5);
  }

  .puma-lib-node-wrapper :global(.node-header) {
    background-color: rgba(217, 119, 6, 0.2);
    border-color: rgba(217, 119, 6, 0.3);
  }
</style>

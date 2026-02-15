<script lang="ts">
  import { onMount } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, PortOption, PortOptionsResult } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      modelPath?: string;
      modelName?: string;
      selectionMode?: 'library' | 'manual';
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let modelPath = $state(data.modelPath || '');
  let selectionMode = $state<'library' | 'manual'>(data.selectionMode || 'library');
  let availableModels: PortOption[] = $state([]);
  let isLoading = $state(false);
  let libraryAvailable = $state(true);
  let searchQuery = $state('');

  let filteredModels = $derived(
    searchQuery
      ? availableModels.filter((m) => {
          const q = searchQuery.toLowerCase();
          return (
            m.label.toLowerCase().includes(q) ||
            (m.description?.toLowerCase().includes(q) ?? false)
          );
        })
      : availableModels,
  );

  onMount(async () => {
    await loadModels();
  });

  async function loadModels() {
    isLoading = true;
    try {
      const result = await invoke<PortOptionsResult>('query_port_options', {
        nodeType: 'puma-lib',
        portId: 'model_path',
      });
      availableModels = result.options;
      libraryAvailable = result.options.length > 0;
      if (!libraryAvailable) {
        selectionMode = 'manual';
      }
    } catch {
      libraryAvailable = false;
      selectionMode = 'manual';
    } finally {
      isLoading = false;
    }
  }

  function handleModelSelect(e: Event) {
    const target = e.target as HTMLSelectElement;
    const selected = availableModels.find((m) => String(m.value) === target.value);
    if (selected) {
      modelPath = String(selected.value);
      updateNodeData(id, {
        modelPath,
        modelName: selected.label,
        selectionMode: 'library',
      });
    }
  }

  function handleManualInput(e: Event) {
    const target = e.target as HTMLInputElement;
    modelPath = target.value;
    updateNodeData(id, { modelPath, selectionMode: 'manual' });
  }

  async function browseForModel() {
    try {
      const result = await open({
        title: 'Select AI Model File',
        filters: [
          { name: 'GGUF Models', extensions: ['gguf'] },
          { name: 'All Files', extensions: ['*'] },
        ],
        multiple: false,
        directory: false,
      });

      if (result && typeof result === 'string') {
        modelPath = result;
        updateNodeData(id, { modelPath, selectionMode: 'manual' });
      }
    } catch (error) {
      console.error('File picker error:', error);
    }
  }

  function switchMode(mode: 'library' | 'manual') {
    selectionMode = mode;
    updateNodeData(id, { selectionMode: mode });
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
        <!-- Mode toggle -->
        {#if libraryAvailable}
          <div class="flex gap-1 text-[10px]">
            <button
              type="button"
              class="px-2 py-0.5 rounded transition-colors {selectionMode === 'library'
                ? 'bg-amber-600/30 text-amber-400'
                : 'text-neutral-500 hover:text-neutral-400'}"
              onclick={() => switchMode('library')}
            >
              Library
            </button>
            <button
              type="button"
              class="px-2 py-0.5 rounded transition-colors {selectionMode === 'manual'
                ? 'bg-amber-600/30 text-amber-400'
                : 'text-neutral-500 hover:text-neutral-400'}"
              onclick={() => switchMode('manual')}
            >
              Manual
            </button>
            {#if selectionMode === 'library'}
              <button
                type="button"
                class="ml-auto text-neutral-500 hover:text-neutral-400"
                onclick={loadModels}
                disabled={isLoading}
              >
                {isLoading ? '...' : 'Refresh'}
              </button>
            {/if}
          </div>
        {/if}

        {#if selectionMode === 'library'}
          <!-- Library mode: model dropdown -->
          <div class="space-y-1">
            {#if availableModels.length > 6}
              <input
                type="text"
                class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-300 focus:outline-none focus:border-amber-500"
                placeholder="Filter models..."
                bind:value={searchQuery}
              />
            {/if}
            <select
              class="w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-xs text-neutral-200 focus:outline-none focus:border-amber-500"
              style="color-scheme: dark;"
              onchange={handleModelSelect}
              value={modelPath}
              disabled={isLoading}
            >
              <option value="" class="bg-neutral-900 text-neutral-500">
                {isLoading ? 'Loading...' : 'Select a model'}
              </option>
              {#each filteredModels as model}
                <option value={String(model.value)} class="bg-neutral-900 text-neutral-200">
                  {model.label}
                </option>
              {/each}
            </select>
          </div>

          {#if modelPath}
            <div class="text-[10px] text-neutral-500 truncate" title={modelPath}>
              {modelPath.split('/').pop()}
            </div>
          {/if}
        {:else}
          <!-- Manual mode: text input + browse -->
          <div class="space-y-1">
            <label class="text-xs text-neutral-400">Model Path</label>
            <div class="flex gap-1">
              <input
                type="text"
                class="flex-1 bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-xs text-neutral-200 focus:outline-none focus:border-amber-500 font-mono truncate"
                placeholder="/path/to/model.gguf"
                value={modelPath}
                oninput={handleManualInput}
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

<script lang="ts">
  import { onMount } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, PortOption } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';
  import { invoke } from '@tauri-apps/api/core';
  import { loadPumasModelOptions } from '../../../services/workflow/pumaModelOptionsCache';
  import {
    buildPumaLibSelectionNodeData,
    findPumasModelOptionById,
    isSelectablePumasModelOption,
    pumasModelIdFromOption,
    pumasModelOptionKey,
  } from './pumaLibNodeState';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      modelName?: string;
      model_id?: string;
      pumas_model_ref?: Record<string, unknown>;
    };
    selected?: boolean;
  }

  interface PumaLibHydrationResult {
    nodeData: Record<string, unknown>;
  }

  let { id, data, selected = false }: Props = $props();

  let modelId = $state<string | undefined>(undefined);
  let availableModels: PortOption[] = $state([]);
  let isLoading = $state(false);
  let loadError = $state<string | null>(null);
  let selectionError = $state<string | null>(null);
  let searchQuery = $state('');

  $effect(() => {
    modelId = typeof data.model_id === 'string' ? data.model_id : undefined;
  });

  let filteredModels = $derived(
    searchQuery
      ? availableModels.filter((model) => {
          const query = searchQuery.toLowerCase();
          return (
            model.label.toLowerCase().includes(query) ||
            pumasModelOptionKey(model).toLowerCase().includes(query) ||
            (model.description?.toLowerCase().includes(query) ?? false)
          );
        })
      : availableModels,
  );
  let selectedModelOption = $derived(findPumasModelOptionById(availableModels, modelId));
  let selectedModelValue = $derived(selectedModelOption ? pumasModelOptionKey(selectedModelOption) : (modelId ?? ''));

  onMount(async () => {
    await loadModels();

    if (data.model_id && !data.pumas_model_ref) {
      await hydrateNodeState(data.model_id);
    }
  });

  async function loadModels(forceRefresh = false) {
    isLoading = true;
    try {
      availableModels = (await loadPumasModelOptions({ forceRefresh })).filter(
        isSelectablePumasModelOption,
      );
      loadError = null;
    } catch (error) {
      loadError = error instanceof Error ? error.message : 'Failed to load models from pumas library';
    } finally {
      isLoading = false;
    }
  }

  async function applyNodeData(nodeData: Record<string, unknown>) {
    const result = await updateNodeData(id, nodeData);
    if (result.status !== 'applied') {
      throw result.error ?? new Error(`Puma-Lib node update ${result.status}`);
    }
    selectionError = null;
  }

  async function hydrateNodeState(nextModelId: string) {
    const response = await invoke<PumaLibHydrationResult>('hydrate_puma_lib_node', {
      modelId: nextModelId,
    });
    await applyNodeData(response.nodeData);
  }

  async function handleModelSelect(event: Event) {
    const target = event.target as HTMLSelectElement;
    const selectedOption = findPumasModelOptionById(availableModels, target.value);
    if (!selectedOption) return;

    try {
      modelId = pumasModelIdFromOption(selectedOption) ?? undefined;
      await applyNodeData(buildPumaLibSelectionNodeData(selectedOption));
    } catch (error) {
      selectionError = error instanceof Error ? error.message : String(error);
      console.error('Failed to select Puma-Lib model:', error);
    }
  }
</script>

<div class="puma-lib-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-amber-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7v10a2 2 0 002 2zM9 9h6v6H9V9z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Puma-Lib'}</span>
      </div>
    {/snippet}

    <div class="space-y-2">
      <div class="flex justify-end text-[10px]">
        <button
          type="button"
          class="text-neutral-500 hover:text-neutral-400 disabled:opacity-50"
          onclick={() => loadModels(true)}
          disabled={isLoading}
        >
          {isLoading ? '...' : 'Refresh'}
        </button>
      </div>

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
          value={selectedModelValue}
          disabled={isLoading}
        >
          <option value="" class="bg-neutral-900 text-neutral-500">
            {isLoading ? 'Loading...' : 'Select a model'}
          </option>
          {#each filteredModels as model (pumasModelOptionKey(model))}
            <option value={pumasModelOptionKey(model)} class="bg-neutral-900 text-neutral-200">
              {model.label}
            </option>
          {/each}
        </select>
        {#if loadError}
          <div class="text-[10px] text-red-400 truncate" title={loadError}>
            Failed to load models from pumas library
          </div>
        {:else if selectionError}
          <div class="text-[10px] text-red-400 truncate" title={selectionError}>
            Failed to select model
          </div>
        {:else if !isLoading && availableModels.length === 0}
          <div class="text-[10px] text-neutral-500">
            No selectable models found in pumas library
          </div>
        {/if}
      </div>

      {#if modelId}
        <div class="text-[10px] text-neutral-500 truncate" title={modelId}>
          {modelId}
        </div>
      {/if}
    </div>
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

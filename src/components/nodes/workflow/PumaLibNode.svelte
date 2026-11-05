<script lang="ts">
  import { onMount } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
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

  // Follows the stored model id; a selection overrides it until the node data changes.
  let modelId = $derived<string | undefined>(
    typeof data.model_id === 'string' ? data.model_id : undefined,
  );
  let availableModels: PortOption[] = $state([]);
  let isLoading = $state(false);
  let loadError = $state<string | null>(null);
  let selectionError = $state<string | null>(null);
  let searchQuery = $state('');

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
      <NodeHeaderTitle label={data.label || 'Puma-Lib'} />
    {/snippet}

    <div class="space-y-2">
      <div class="flex justify-end text-[10px]">
        <button
          type="button"
          class="text-pg-text-muted hover:text-pg-text disabled:opacity-50"
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
            class="w-full bg-pg-panel-sunken border border-pg-border rounded px-2 py-1 text-[10px] text-pg-text-soft"
            placeholder="Filter models..."
            bind:value={searchQuery}
          />
        {/if}
        <select
          class="w-full bg-pg-panel-sunken border border-pg-border rounded px-2 py-1 text-xs text-pg-text"
          style="color-scheme: dark;"
          onchange={handleModelSelect}
          value={selectedModelValue}
          disabled={isLoading}
        >
          <option value="" class="bg-pg-panel-sunken text-pg-text-muted">
            {isLoading ? 'Loading...' : 'Select a model'}
          </option>
          {#each filteredModels as model (pumasModelOptionKey(model))}
            <option value={pumasModelOptionKey(model)} class="bg-pg-panel-sunken text-pg-text">
              {model.label}
            </option>
          {/each}
        </select>
        {#if loadError}
          <div class="text-[10px] text-pg-danger truncate" title={loadError}>
            Failed to load models from pumas library
          </div>
        {:else if selectionError}
          <div class="text-[10px] text-pg-danger truncate" title={selectionError}>
            Failed to select model
          </div>
        {:else if !isLoading && availableModels.length === 0}
          <div class="text-[10px] text-pg-text-muted">
            No selectable models found in pumas library
          </div>
        {/if}
      </div>

      {#if modelId}
        <div class="text-[10px] text-pg-text-muted truncate" title={modelId}>
          {modelId}
        </div>
      {/if}
    </div>
  </BaseNode>
</div>

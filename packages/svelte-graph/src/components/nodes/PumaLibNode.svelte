<script lang="ts">
  import { onMount } from 'svelte';
  import BaseNode from './BaseNode.svelte';
  import type { NodeDefinition } from '../../types/workflow.js';
  import type { PortOption } from '../../types/backend.js';
  import { useGraphContext } from '../../context/useGraphContext.js';

  const { backend, stores } = useGraphContext();

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      modelName?: string;
      model_id?: string;
      pumas_model_ref?: Record<string, unknown>;
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let modelId = $derived(typeof data.model_id === 'string' ? data.model_id : '');
  let availableModels: PortOption[] = $state([]);
  let isLoading = $state(false);
  let loadError = $state<string | null>(null);
  let selectionError = $state<string | null>(null);
  let searchQuery = $state('');

  let filteredModels = $derived(
    searchQuery
      ? availableModels.filter((m) => {
          const q = searchQuery.toLowerCase();
          return (
            m.label.toLowerCase().includes(q) ||
            pumasModelOptionKey(m).toLowerCase().includes(q) ||
            (m.description?.toLowerCase().includes(q) ?? false)
          );
        })
      : availableModels,
  );
  let selectedModelOption = $derived(findPumasModelOptionById(availableModels, modelId));
  let selectedModelValue = $derived(
    selectedModelOption ? pumasModelOptionKey(selectedModelOption) : modelId,
  );

  onMount(async () => {
    await loadModels();
  });

  async function loadModels() {
    if (!backend.queryPortOptions) return;
    isLoading = true;
    try {
      const result = await backend.queryPortOptions('puma-lib', 'pumas_model_ref');
      availableModels = result.options.filter(isSelectablePumasModelOption);
      loadError = null;
    } catch (error) {
      loadError = error instanceof Error ? error.message : 'Failed to load models from pumas library';
      console.error('[PumaLibNode] Failed to load models:', error);
    } finally {
      isLoading = false;
    }
  }

  function pumasModelRefFromOption(option: PortOption): Record<string, unknown> | null {
    const metadataModelRef = option.metadata?.pumas_model_ref ?? option.metadata?.model_ref;
    if (isObjectRecord(metadataModelRef)) return metadataModelRef;
    if (isObjectRecord(option.value)) return option.value;
    return null;
  }

  function pumasModelIdFromOption(option: PortOption): string | null {
    const metadataId = readNonEmptyString(option.metadata?.id);
    if (metadataId) return metadataId;

    const modelRef = pumasModelRefFromOption(option);
    return readNonEmptyString(modelRef?.model_id);
  }

  function isSelectablePumasModelOption(option: PortOption): boolean {
    return pumasModelIdFromOption(option) !== null && pumasModelRefFromOption(option) !== null;
  }

  function pumasModelOptionKey(option: PortOption): string {
    return pumasModelIdFromOption(option) ?? option.label;
  }

  function findPumasModelOptionById(options: PortOption[], nextModelId: string | null | undefined) {
    const cleanedModelId = readNonEmptyString(nextModelId);
    if (!cleanedModelId) return null;
    return options.find((option) => pumasModelIdFromOption(option) === cleanedModelId) ?? null;
  }

  function isObjectRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
  }

  function readNonEmptyString(value: unknown): string | null {
    if (typeof value !== 'string') return null;
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
  }

  function handleModelSelect(e: Event) {
    const target = e.target as HTMLSelectElement;
    const match = findPumasModelOptionById(availableModels, target.value);
    if (!match) return;

    const nextModelId = pumasModelIdFromOption(match);
    const nextModelRef = pumasModelRefFromOption(match);
    if (!nextModelId || !nextModelRef) {
      selectionError = 'Selected model is missing canonical Pumas identity';
      return;
    }

    stores.workflow.updateNodeData(id, {
      modelName: match.label,
      model_id: nextModelId,
      pumas_model_ref: nextModelRef,
    });
    selectionError = null;
  }
</script>

<div class="puma-lib-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="header-content">
        <div class="header-icon">
          <svg class="icon-svg" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
          </svg>
        </div>
        <span class="header-label">{data.label || 'Puma-Lib'}</span>
      </div>
    {/snippet}

      <div class="puma-lib-body">
        <div class="toolbar-row">
          <button
            type="button"
            class="refresh-btn"
            onclick={loadModels}
            disabled={isLoading}
          >
            {isLoading ? '...' : 'Refresh'}
          </button>
        </div>

        {#if availableModels.length > 6}
          <input
            type="text"
            class="search-input"
            placeholder="Filter models..."
            bind:value={searchQuery}
          />
        {/if}

        <select
          class="model-select"
          onchange={handleModelSelect}
          value={selectedModelValue}
          disabled={isLoading}
        >
          <option value="">
            {isLoading ? 'Loading...' : 'Select a model'}
          </option>
          {#each filteredModels as model (pumasModelOptionKey(model))}
            <option value={pumasModelOptionKey(model)}>
              {model.label}
            </option>
          {/each}
        </select>

        {#if loadError}
          <div class="model-id-hint error" title={loadError}>
            Failed to load models from pumas library
          </div>
        {:else if selectionError}
          <div class="model-id-hint error" title={selectionError}>
            Failed to select model
          </div>
        {:else if !isLoading && availableModels.length === 0}
          <div class="model-id-hint">
            No selectable models found in pumas library
          </div>
        {:else if modelId}
          <div class="model-id-hint" title={modelId}>
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

  .header-content {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .header-icon {
    width: 1.25rem;
    height: 1.25rem;
    border-radius: 0.25rem;
    background-color: #d97706;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .icon-svg {
    width: 0.75rem;
    height: 0.75rem;
    color: white;
  }

  .header-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: #e5e5e5;
  }

  .puma-lib-body {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .toolbar-row {
    display: flex;
    justify-content: flex-end;
  }

  .refresh-btn {
    font-size: 0.625rem;
    color: #737373;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .refresh-btn:hover {
    color: #a3a3a3;
  }

  .refresh-btn:disabled {
    cursor: default;
  }

  .search-input {
    width: 100%;
    background-color: #171717;
    border: 1px solid #525252;
    border-radius: 0.25rem;
    padding: 0.25rem 0.5rem;
    font-size: 0.625rem;
    color: #d4d4d4;
    outline: none;
  }

  .search-input:focus {
    border-color: #d97706;
  }

  .model-select {
    width: 100%;
    background-color: #171717;
    border: 1px solid #525252;
    border-radius: 0.25rem;
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    color: #e5e5e5;
    outline: none;
    color-scheme: dark;
  }

  .model-select:focus {
    border-color: #d97706;
  }

  .model-id-hint {
    font-size: 0.625rem;
    color: #737373;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-id-hint.error {
    color: #f87171;
  }
</style>

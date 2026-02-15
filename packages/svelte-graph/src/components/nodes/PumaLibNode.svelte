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
      modelPath?: string;
      modelName?: string;
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let modelPath = $state(data.modelPath || '');
  let availableModels: PortOption[] = $state([]);
  let isLoading = $state(false);
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
    if (!backend.queryPortOptions) return;
    isLoading = true;
    try {
      const result = await backend.queryPortOptions('puma-lib', 'model_path');
      availableModels = result.options;
    } catch (e) {
      console.error('[PumaLibNode] Failed to load models:', e);
    } finally {
      isLoading = false;
    }
  }

  function handleModelSelect(e: Event) {
    const target = e.target as HTMLSelectElement;
    const match = availableModels.find((m) => String(m.value) === target.value);
    if (match) {
      modelPath = String(match.value);
      stores.workflow.updateNodeData(id, {
        modelPath,
        modelName: match.label,
      });
    }
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

    {#snippet children()}
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
          value={modelPath}
          disabled={isLoading}
        >
          <option value="">
            {isLoading ? 'Loading...' : 'Select a model'}
          </option>
          {#each filteredModels as model}
            <option value={String(model.value)}>
              {model.label}
            </option>
          {/each}
        </select>

        {#if modelPath}
          <div class="model-path-hint" title={modelPath}>
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

  .model-path-hint {
    font-size: 0.625rem;
    color: #737373;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

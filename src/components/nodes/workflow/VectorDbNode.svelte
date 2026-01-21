<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface DatabaseInfo {
    name: string;
    path: string;
    table_count: number;
  }

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      database_path?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let databases = $state<DatabaseInfo[]>([]);
  let selectedDb = $state(data.database_path || '');
  let newDbName = $state('');
  let showCreateDialog = $state(false);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    await loadDatabases();
  });

  async function loadDatabases() {
    isLoading = true;
    error = null;
    try {
      databases = await invoke<DatabaseInfo[]>('list_vector_databases');
      // If we have a selected database, make sure it's in the list
      if (selectedDb && !databases.find(db => db.path === selectedDb)) {
        selectedDb = databases[0]?.path || '';
        updateNodeData(id, { database_path: selectedDb });
      }
    } catch (e) {
      error = String(e);
      console.error('Failed to load databases:', e);
    } finally {
      isLoading = false;
    }
  }

  function handleSelect(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    selectedDb = value;
    updateNodeData(id, { database_path: value });
  }

  async function createDatabase() {
    if (!newDbName.trim()) return;

    try {
      const path = await invoke<string>('create_vector_database', { name: newDbName.trim() });
      await loadDatabases();
      selectedDb = path;
      updateNodeData(id, { database_path: path });
      showCreateDialog = false;
      newDbName = '';
    } catch (e) {
      error = String(e);
      console.error('Failed to create database:', e);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      createDatabase();
    } else if (e.key === 'Escape') {
      showCreateDialog = false;
      newDbName = '';
    }
  }
</script>

<div class="vector-db-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-purple-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Vector Database'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      <div class="space-y-2">
        {#if isLoading}
          <div class="text-xs text-neutral-400 italic py-1">Loading databases...</div>
        {:else if error}
          <div class="text-xs text-red-400 py-1">{error}</div>
        {:else}
          <select
            class="w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-sm text-neutral-200 focus:outline-none focus:border-purple-500"
            onchange={handleSelect}
            value={selectedDb}
          >
            <option value="">Select database...</option>
            {#each databases as db}
              <option value={db.path}>{db.name}</option>
            {/each}
          </select>

          {#if showCreateDialog}
            <div class="flex gap-1">
              <input
                type="text"
                class="flex-1 bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-sm text-neutral-200 focus:outline-none focus:border-purple-500"
                placeholder="Database name..."
                bind:value={newDbName}
                onkeydown={handleKeydown}
              />
              <button
                class="px-2 py-1 bg-purple-600 text-white text-xs rounded hover:bg-purple-700"
                onclick={createDatabase}
              >
                Create
              </button>
            </div>
          {:else}
            <button
              class="w-full px-2 py-1 bg-neutral-700 text-neutral-300 text-xs rounded hover:bg-neutral-600 flex items-center justify-center gap-1"
              onclick={() => showCreateDialog = true}
            >
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
              </svg>
              New Database
            </button>
          {/if}
        {/if}
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .vector-db-node-wrapper :global(.base-node) {
    border-color: rgba(168, 85, 247, 0.5);
  }

  .vector-db-node-wrapper :global(.node-header) {
    background-color: rgba(168, 85, 247, 0.2);
    border-color: rgba(168, 85, 247, 0.3);
  }
</style>

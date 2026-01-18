<script lang="ts">
  import {
    visibleCategories,
    visibleConnectionTypes,
    searchQuery,
    toggleCategory,
    toggleConnectionType,
    showAllCategories,
    showAllConnectionTypes,
    clearSearch,
  } from '../stores/architectureStore';
  import { CATEGORY_COLORS, CONNECTION_STYLES } from '../services/architecture/types';
  import type { ArchNodeCategory, ArchConnectionType } from '../services/architecture/types';

  const categories: { key: ArchNodeCategory; label: string }[] = [
    { key: 'component', label: 'Components' },
    { key: 'service', label: 'Services' },
    { key: 'store', label: 'Stores' },
    { key: 'command', label: 'Commands' },
    { key: 'backend', label: 'Backend' },
  ];

  const connectionTypes: { key: ArchConnectionType; label: string }[] = [
    { key: 'import', label: 'Import' },
    { key: 'command', label: 'Command' },
    { key: 'subscription', label: 'Subscribe' },
    { key: 'event', label: 'Event' },
    { key: 'uses', label: 'Uses' },
  ];

  function handleResetFilters() {
    showAllCategories();
    showAllConnectionTypes();
    clearSearch();
  }
</script>

<div class="architecture-toolbar h-auto min-h-12 px-4 py-2 bg-neutral-900 border-b border-neutral-700 flex flex-wrap items-center gap-4">
  <!-- Search -->
  <div class="flex items-center gap-2">
    <span class="text-xs text-neutral-500 uppercase">Search:</span>
    <input
      type="text"
      class="px-2 py-1 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 placeholder-neutral-500 w-40"
      placeholder="Filter nodes..."
      bind:value={$searchQuery}
    />
    {#if $searchQuery}
      <button
        class="text-xs text-neutral-400 hover:text-neutral-200"
        onclick={clearSearch}
        title="Clear search"
      >
        [x]
      </button>
    {/if}
  </div>

  <div class="w-px h-6 bg-neutral-700"></div>

  <!-- Category Filters -->
  <div class="flex items-center gap-2">
    <span class="text-xs text-neutral-500 uppercase">Nodes:</span>
    {#each categories as cat}
      <button
        class="px-2 py-1 text-xs rounded border transition-all"
        class:opacity-40={!$visibleCategories.has(cat.key)}
        style="
          background: {$visibleCategories.has(cat.key) ? CATEGORY_COLORS[cat.key] + '20' : 'transparent'};
          border-color: {CATEGORY_COLORS[cat.key]};
          color: {CATEGORY_COLORS[cat.key]};
        "
        onclick={() => toggleCategory(cat.key)}
        title="Toggle {cat.label}"
      >
        {cat.label}
      </button>
    {/each}
  </div>

  <div class="w-px h-6 bg-neutral-700"></div>

  <!-- Connection Type Filters -->
  <div class="flex items-center gap-2">
    <span class="text-xs text-neutral-500 uppercase">Edges:</span>
    {#each connectionTypes as conn}
      <button
        class="px-2 py-1 text-xs rounded border transition-all"
        class:opacity-40={!$visibleConnectionTypes.has(conn.key)}
        style="
          background: {$visibleConnectionTypes.has(conn.key) ? CONNECTION_STYLES[conn.key].stroke + '20' : 'transparent'};
          border-color: {CONNECTION_STYLES[conn.key].stroke};
          color: {CONNECTION_STYLES[conn.key].stroke};
        "
        onclick={() => toggleConnectionType(conn.key)}
        title="Toggle {conn.label} connections"
      >
        {conn.label}
      </button>
    {/each}
  </div>

  <div class="flex-1"></div>

  <!-- Reset button -->
  <button
    class="px-3 py-1.5 text-xs bg-neutral-800 hover:bg-neutral-700 border border-neutral-600 rounded text-neutral-300 transition-colors"
    onclick={handleResetFilters}
    title="Reset all filters"
  >
    Reset Filters
  </button>
</div>

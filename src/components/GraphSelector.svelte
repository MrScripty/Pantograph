<script lang="ts">
  import {
    currentGraphId,
    currentGraphName,
    currentGraphType,
    availableWorkflows,
    SYSTEM_GRAPHS,
    switchGraph,
    refreshWorkflowList,
    type GraphType,
  } from '../stores/graphSessionStore';
  import { isReadOnly } from '../stores/graphSessionStore';

  let isOpen = $state(false);
  let dropdownRef: HTMLDivElement | null = $state(null);

  function handleToggle() {
    if (!isOpen) {
      refreshWorkflowList();
    }
    isOpen = !isOpen;
  }

  function handleSelect(id: string, type: GraphType) {
    switchGraph(id, type);
    isOpen = false;
  }

  function handleClickOutside(event: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
      isOpen = false;
    }
  }

  $effect(() => {
    if (isOpen) {
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });
</script>

<div class="graph-selector relative" bind:this={dropdownRef}>
  <button
    class="selector-button flex items-center gap-2 px-3 py-1.5 text-sm bg-neutral-800 hover:bg-neutral-700 border border-neutral-600 rounded text-neutral-200 transition-colors"
    onclick={handleToggle}
    title="Switch graph"
  >
    <span class="text-neutral-400 text-xs">
      {#if $currentGraphType === 'system'}
        [SYS]
      {:else}
        [WF]
      {/if}
    </span>
    <span class="max-w-[150px] truncate">{$currentGraphName}</span>
    <svg
      class="w-4 h-4 text-neutral-400 transition-transform"
      class:rotate-180={isOpen}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  {#if isOpen}
    <div
      class="dropdown absolute top-full left-0 mt-1 w-64 bg-neutral-800 border border-neutral-600 rounded shadow-lg z-50 overflow-hidden"
    >
      <!-- Workflows Section -->
      <div class="section">
        <div class="section-header px-3 py-2 text-xs text-neutral-400 uppercase tracking-wider bg-neutral-900">
          Workflows
        </div>
        {#if $availableWorkflows.length === 0}
          <div class="px-3 py-2 text-sm text-neutral-500 italic">No saved workflows</div>
        {:else}
          {#each $availableWorkflows as workflow}
            <button
              class="w-full px-3 py-2 text-left text-sm hover:bg-neutral-700 transition-colors flex items-center gap-2"
              class:bg-neutral-700={$currentGraphId === workflow.id && $currentGraphType === 'workflow'}
              onclick={() => handleSelect(workflow.id ?? workflow.name, 'workflow')}
            >
              <span class="text-neutral-300 flex-1 truncate">{workflow.name}</span>
              {#if $currentGraphId === workflow.id && $currentGraphType === 'workflow'}
                <svg class="w-4 h-4 text-green-400" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fill-rule="evenodd"
                    d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                    clip-rule="evenodd"
                  />
                </svg>
              {/if}
            </button>
          {/each}
        {/if}
      </div>

      <!-- System Graphs Section -->
      <div class="section border-t border-neutral-700">
        <div class="section-header px-3 py-2 text-xs text-neutral-400 uppercase tracking-wider bg-neutral-900">
          System
        </div>
        {#each SYSTEM_GRAPHS as graph}
          <button
            class="w-full px-3 py-2 text-left text-sm hover:bg-neutral-700 transition-colors flex items-center gap-2"
            class:bg-neutral-700={$currentGraphId === graph.id && $currentGraphType === 'system'}
            onclick={() => handleSelect(graph.id, 'system')}
          >
            <span class="text-neutral-300 flex-1 truncate">{graph.name}</span>
            <span class="text-xs text-neutral-500">(read-only)</span>
            {#if $currentGraphId === graph.id && $currentGraphType === 'system'}
              <svg class="w-4 h-4 text-green-400" fill="currentColor" viewBox="0 0 20 20">
                <path
                  fill-rule="evenodd"
                  d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                  clip-rule="evenodd"
                />
              </svg>
            {/if}
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .graph-selector {
    font-family: inherit;
  }

  .dropdown {
    max-height: 400px;
    overflow-y: auto;
  }
</style>

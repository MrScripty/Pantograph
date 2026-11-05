<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity';
  import {
    dispatchWorkflowPaletteDragEnd,
    dispatchWorkflowPaletteDragStart,
  } from '@pantograph/svelte-graph';
  import { nodeDefinitionsByCategory, addNode } from '../stores/workflowStore';
  import {
    clearActiveWorkflowPaletteDragDefinition,
    setActiveWorkflowPaletteDragDefinition,
    WORKFLOW_PALETTE_DRAG_DATA_TYPES,
  } from './workflowPaletteDrag';
  import { buildNodePaletteTree } from './workbench/graphInspectionPresenters';
  import type { NodeDefinition } from '../services/workflow/types';

  /** Where a double-clicked (or Enter-activated) node lands on the canvas. */
  const DOUBLE_CLICK_ADD_POSITION = { x: 200, y: 200 };

  let searchQuery = $state('');
  let expandedCategories = new SvelteSet(['input', 'processing', 'output']);

  let tree = $derived(buildNodePaletteTree($nodeDefinitionsByCategory, searchQuery, expandedCategories));
  let searching = $derived(searchQuery.trim().length > 0);

  function toggleCategory(category: string) {
    if (expandedCategories.has(category)) {
      expandedCategories.delete(category);
    } else {
      expandedCategories.add(category);
    }
  }

  function handleDragStart(event: DragEvent, definition: NodeDefinition) {
    if (!event.dataTransfer) {
      return;
    }

    const payload = JSON.stringify(definition);
    for (const type of WORKFLOW_PALETTE_DRAG_DATA_TYPES) {
      event.dataTransfer.setData(type, payload);
    }
    event.dataTransfer.effectAllowed = 'copy';
    setActiveWorkflowPaletteDragDefinition(definition);
    dispatchWorkflowPaletteDragStart(window);
  }

  function handleDragEnd() {
    dispatchWorkflowPaletteDragEnd(window);
    clearActiveWorkflowPaletteDragDefinition();
  }

  function addDefinition(definition: NodeDefinition) {
    addNode(definition, DOUBLE_CLICK_ADD_POSITION);
  }

  function handleNodeKeyDown(event: KeyboardEvent, definition: NodeDefinition) {
    if (event.key !== 'Enter' && event.key !== ' ') {
      return;
    }
    event.preventDefault();
    addDefinition(definition);
  }
</script>

<section class="node-palette pg-panel pg-col" aria-label="Node palette">
  <div class="np-top">
    <label class="pg-sr-only" for="graph-node-palette-search">Search nodes</label>
    <input
      id="graph-node-palette-search"
      class="pg-field np-search"
      type="search"
      placeholder="Search nodes"
      bind:value={searchQuery}
    />
  </div>

  <div class="pg-scroll np-tree">
    {#each tree as category (category.key)}
      <button
        type="button"
        class="np-cat"
        aria-expanded={category.expanded}
        aria-controls={`graph-node-palette-${category.key}`}
        disabled={searching}
        title={searching ? 'Clear the search to fold categories' : undefined}
        onclick={() => toggleCategory(category.key)}
      >
        <span class="np-chev" aria-hidden="true">{category.expanded ? '▾' : '▸'}</span>
        {category.label}
        <span class="np-count">{category.count}</span>
      </button>

      {#if category.expanded}
        <div id={`graph-node-palette-${category.key}`}>
          {#each category.items as item (item.definition.node_type)}
            <!-- A div with role="button" (not a <button>) keeps HTML5 drag-and-drop working in every supported webview. -->
            <div
              class="np-item"
              draggable="true"
              ondragstart={(event) => handleDragStart(event, item.definition)}
              ondragend={handleDragEnd}
              ondblclick={() => addDefinition(item.definition)}
              onkeydown={(event) => handleNodeKeyDown(event, item.definition)}
              title={item.definition.description}
              role="button"
              tabindex="0"
              aria-label={`${item.label}, ${item.portsTitle}. Press Enter to add to the canvas.`}
            >
              <span class="np-name">{item.label}</span>
              <span class="np-ports" title={item.portsTitle}>{item.ports}</span>
            </div>
          {/each}
        </div>
      {/if}
    {:else}
      <p class="np-empty">
        {searching ? 'No nodes match this search.' : 'Node definitions are not loaded yet.'}
      </p>
    {/each}
  </div>

  <div class="np-foot">Drag onto the canvas, or double-click to add.</div>
</section>

<style>
  .node-palette {
    flex: 0 0 232px;
    min-height: 0;
  }

  .np-top {
    padding: 8px 10px;
    border-bottom: 1px solid var(--pg-border);
    flex: none;
  }

  .np-search {
    width: 100%;
    height: 28px;
  }

  .np-tree {
    padding: 4px 0;
  }

  .np-cat {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 12px;
    border: 0;
    background: transparent;
    color: var(--pg-text-soft);
    text-align: left;
    cursor: pointer;
    font-size: 12.5px;
    font-weight: 700;
  }

  .np-cat:hover {
    background: var(--pg-panel-raised);
  }

  .np-cat:disabled {
    cursor: default;
  }

  .np-cat:disabled:hover {
    background: transparent;
  }

  .np-chev {
    font-size: 10px;
    color: var(--pg-border-strong);
  }

  .np-count,
  .np-ports {
    margin-left: auto;
    font-family: var(--pg-font-mono);
    font-size: 11px;
    font-weight: 400;
    color: var(--pg-border-strong);
  }

  .np-item {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0 8px;
    align-items: center;
    padding: 5px 12px 5px 30px;
    font-size: 12.5px;
    color: var(--pg-text);
    cursor: grab;
  }

  .np-item:hover {
    background: var(--pg-panel-raised);
  }

  .np-item:active {
    cursor: grabbing;
  }

  .np-name {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .np-empty {
    margin: 0;
    padding: 16px 12px;
    font-size: 12.5px;
    color: var(--pg-text-muted);
  }

  .np-foot {
    padding: 6px 12px;
    min-height: 36px;
    display: flex;
    align-items: center;
    border-top: 1px solid var(--pg-border);
    background: var(--pg-panel-head);
    font-size: 12.5px;
    color: var(--pg-text-muted);
    flex: none;
  }
</style>

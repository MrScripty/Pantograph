<script lang="ts">
  import type { NodeDefinition } from '../types/workflow.js';
  import { useGraphContext } from '../context/useGraphContext.js';

  const { stores } = useGraphContext();
  const nodeDefinitionsByCategory = stores.workflow.nodeDefinitionsByCategory;

  let searchQuery = $state('');
  let expandedCategories = $state(new Set(['input', 'processing', 'output']));

  function formatCategoryName(category: string): string {
    return category.charAt(0).toUpperCase() + category.slice(1);
  }

  let filteredByCategory = $derived(() => {
    const result = new Map<string, NodeDefinition[]>();
    for (const [category, defs] of $nodeDefinitionsByCategory) {
      const filtered = defs.filter(
        (d) =>
          d.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
          d.description.toLowerCase().includes(searchQuery.toLowerCase())
      );
      if (filtered.length > 0) {
        result.set(category, filtered);
      }
    }
    return result;
  });

  function toggleCategory(category: string) {
    if (expandedCategories.has(category)) {
      expandedCategories.delete(category);
    } else {
      expandedCategories.add(category);
    }
    expandedCategories = new Set(expandedCategories);
  }

  function handleDragStart(event: DragEvent, definition: NodeDefinition) {
    event.dataTransfer?.setData('application/json', JSON.stringify(definition));
    event.dataTransfer!.effectAllowed = 'copy';
  }

  function handleDoubleClick(definition: NodeDefinition) {
    stores.workflow.addNode(definition, { x: 200, y: 200 });
  }

  const categoryIcons: Record<string, string> = {
    input: '[ ]',
    processing: '[~]',
    tool: '[#]',
    output: '[>]',
    control: '[?]',
  };
</script>

<div class="node-palette">
  <div class="palette-header">
    <h3 class="palette-title">Nodes</h3>
    <input
      type="text"
      placeholder="Search nodes..."
      bind:value={searchQuery}
      class="search-input"
    />
  </div>

  <div class="palette-list">
    {#each [...filteredByCategory()] as [category, definitions]}
      <div class="category">
        <button
          class="category-header"
          onclick={() => toggleCategory(category)}
        >
          <span class="category-icon" data-category={category}>
            {categoryIcons[category] || '[*]'}
          </span>
          <span class="category-name">{formatCategoryName(category)}</span>
          <span class="category-count">{definitions.length}</span>
          <span class="category-toggle">
            {expandedCategories.has(category) ? '[-]' : '[+]'}
          </span>
        </button>

        {#if expandedCategories.has(category)}
          <div class="category-items">
            {#each definitions as definition}
              <div
                class="node-item"
                draggable="true"
                ondragstart={(e) => handleDragStart(e, definition)}
                ondblclick={() => handleDoubleClick(definition)}
                title={definition.description}
                role="button"
                tabindex="0"
              >
                <span>{definition.label}</span>
                <span class="port-count">
                  {definition.inputs.length}:{definition.outputs.length}
                </span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="palette-footer">
    Drag or double-click to add
  </div>
</div>

<style>
  .node-palette {
    width: 14rem;
    background-color: #171717;
    border-right: 1px solid #404040;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .palette-header {
    padding: 0.75rem;
    border-bottom: 1px solid #404040;
  }

  .palette-title {
    font-size: 0.875rem;
    font-weight: 500;
    color: #e5e5e5;
    margin: 0 0 0.5rem 0;
  }

  .search-input {
    width: 100%;
    box-sizing: border-box;
    padding: 0.375rem 0.5rem;
    background-color: #262626;
    border: 1px solid #525252;
    border-radius: 0.25rem;
    font-size: 0.875rem;
    color: #e5e5e5;
  }

  .search-input::placeholder {
    color: #737373;
  }

  .search-input:focus {
    outline: none;
    border-color: #3b82f6;
  }

  .palette-list {
    flex: 1;
    overflow-y: auto;
  }

  /* --- Category Header --- */
  .category-header {
    width: 100%;
    padding: 0.5rem 0.75rem;
    background-color: rgba(38, 38, 38, 0.5);
    border: none;
    border-bottom: 1px solid #404040;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    text-align: left;
    cursor: pointer;
    transition: background-color 150ms;
  }

  .category-header:hover {
    background-color: #262626;
  }

  .category-icon {
    font-family: monospace;
    font-size: 0.75rem;
    color: #a3a3a3;
  }

  .category-icon[data-category="input"] { color: #60a5fa; }
  .category-icon[data-category="processing"] { color: #4ade80; }
  .category-icon[data-category="tool"] { color: #fbbf24; }
  .category-icon[data-category="output"] { color: #22d3ee; }
  .category-icon[data-category="control"] { color: #c084fc; }

  .category-name {
    flex: 1;
    font-size: 0.875rem;
    color: #e5e5e5;
  }

  .category-count,
  .category-toggle {
    font-size: 0.75rem;
    color: #737373;
  }

  /* --- Node Items --- */
  .category-items {
    padding: 0.25rem 0;
  }

  .node-item {
    padding: 0.5rem 1rem;
    cursor: grab;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.875rem;
    color: #d4d4d4;
    transition: background-color 150ms;
  }

  .node-item:hover {
    background-color: #262626;
  }

  .node-item:active {
    cursor: grabbing;
  }

  .port-count {
    font-size: 0.75rem;
    color: #525252;
  }

  /* --- Footer --- */
  .palette-footer {
    padding: 0.5rem;
    border-top: 1px solid #404040;
    font-size: 0.75rem;
    color: #737373;
  }
</style>

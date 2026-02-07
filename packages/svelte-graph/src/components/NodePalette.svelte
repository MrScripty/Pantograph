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

  const categoryColors: Record<string, string> = {
    input: 'text-blue-400',
    processing: 'text-green-400',
    tool: 'text-amber-400',
    output: 'text-cyan-400',
    control: 'text-purple-400',
  };
</script>

<div class="node-palette w-56 bg-neutral-900 border-r border-neutral-700 flex flex-col overflow-hidden">
  <div class="p-3 border-b border-neutral-700">
    <h3 class="text-sm font-medium text-neutral-200 mb-2">Nodes</h3>
    <input
      type="text"
      placeholder="Search nodes..."
      bind:value={searchQuery}
      class="w-full px-2 py-1.5 bg-neutral-800 border border-neutral-600 rounded text-sm text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-blue-500"
    />
  </div>

  <div class="flex-1 overflow-y-auto">
    {#each [...filteredByCategory()] as [category, definitions]}
      <div class="category">
        <button
          class="w-full px-3 py-2 bg-neutral-800/50 border-b border-neutral-700 flex items-center gap-2 text-left hover:bg-neutral-800 transition-colors"
          onclick={() => toggleCategory(category)}
        >
          <span class="font-mono text-xs {categoryColors[category] || 'text-neutral-400'}">
            {categoryIcons[category] || '[*]'}
          </span>
          <span class="flex-1 text-sm text-neutral-200">{formatCategoryName(category)}</span>
          <span class="text-xs text-neutral-500">{definitions.length}</span>
          <span class="text-xs text-neutral-500">
            {expandedCategories.has(category) ? '[-]' : '[+]'}
          </span>
        </button>

        {#if expandedCategories.has(category)}
          <div class="py-1">
            {#each definitions as definition}
              <div
                class="node-item px-4 py-2 cursor-grab flex justify-between items-center text-sm text-neutral-300 hover:bg-neutral-800 transition-colors"
                draggable="true"
                ondragstart={(e) => handleDragStart(e, definition)}
                ondblclick={() => handleDoubleClick(definition)}
                title={definition.description}
                role="button"
                tabindex="0"
              >
                <span>{definition.label}</span>
                <span class="text-xs text-neutral-600">
                  {definition.inputs.length}:{definition.outputs.length}
                </span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="p-2 border-t border-neutral-700 text-xs text-neutral-500">
    Drag or double-click to add
  </div>
</div>

<style>
  .node-item:active {
    cursor: grabbing;
  }
</style>

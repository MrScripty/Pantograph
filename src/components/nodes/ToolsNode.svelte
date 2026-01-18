<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { ToolsNodeData } from '../../types/nodes';

  interface Props {
    data: ToolsNodeData;
  }

  let { data }: Props = $props();

  // Default tools list
  const defaultTools = [
    { name: 'read_gui_file', description: 'Read component source', enabled: true },
    { name: 'write_gui_file', description: 'Create/update component', enabled: true },
    { name: 'list_components', description: 'List existing components', enabled: true },
    { name: 'get_tailwind_colors', description: 'Get color palette', enabled: true },
    { name: 'list_templates', description: 'List templates', enabled: true },
    { name: 'read_template', description: 'Read template source', enabled: true },
  ];

  let tools = $derived(data.tools || defaultTools);
</script>

<div class="node-container bg-neutral-800 border border-amber-600/50 rounded-lg p-3 w-[200px]">
  <div class="flex items-center gap-2 mb-2">
    <div class="w-6 h-6 rounded bg-amber-600 flex items-center justify-center">
      <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    </div>
    <span class="text-sm font-medium text-neutral-200">{data.label}</span>
  </div>

  <div class="text-xs space-y-1 max-h-32 overflow-y-auto">
    {#each tools as tool}
      <div class="flex items-center gap-2 text-neutral-400 hover:text-neutral-200 transition-colors" title={tool.description}>
        <span class="w-1.5 h-1.5 rounded-full {tool.enabled ? 'bg-green-500' : 'bg-neutral-600'}"></span>
        <span class="font-mono text-xs">{tool.name}</span>
      </div>
    {/each}
  </div>

  <Handle type="source" position={Position.Right} id="output" class="!bg-amber-500 !w-3 !h-3" />
</div>

<style>
  .node-container {
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
  }
</style>

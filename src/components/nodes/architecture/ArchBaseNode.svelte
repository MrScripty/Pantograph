<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import {
    Box,
    Cog,
    Database,
    Server,
    Zap,
    type Icon as LucideIcon,
  } from 'lucide-svelte';
  import type { ArchNodeCategory } from '../../../services/architecture/types';
  import type { Component } from 'svelte';

  interface Props {
    id: string;
    data: {
      label: string;
      description?: string;
      filePath?: string;
      category: ArchNodeCategory;
      color: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  const categoryIcons: Record<ArchNodeCategory, Component<{ class?: string; size?: number }>> = {
    component: Box,
    service: Cog,
    store: Database,
    backend: Server,
    command: Zap,
  };

  const categoryLabels: Record<ArchNodeCategory, string> = {
    component: 'Component',
    service: 'Service',
    store: 'Store',
    backend: 'Backend',
    command: 'Command',
  };

  let IconComponent = $derived(categoryIcons[data.category] || Box);

  function handleClick() {
    if (data.filePath) {
      console.log(`[Architecture] File: ${data.filePath}`);
    }
  }
</script>

<button
  class="arch-node bg-neutral-800 rounded-lg min-w-[180px] relative cursor-pointer transition-all hover:brightness-110"
  class:selected
  onclick={handleClick}
  style="--node-color: {data.color};"
>
  <!-- Category indicator bar -->
  <div
    class="absolute top-0 left-0 right-0 h-1 rounded-t-lg"
    style="background: {data.color};"
  ></div>

  <!-- Header -->
  <div class="flex items-center gap-2 px-3 py-2 pt-3">
    <div
      class="w-6 h-6 rounded flex items-center justify-center"
      style="background: {data.color}20;"
    >
      <IconComponent size={14} class="text-white" style="color: {data.color};" />
    </div>
    <span class="text-sm font-medium text-neutral-200">{data.label}</span>
  </div>

  <!-- Category label -->
  <div class="px-3 pb-2">
    <span class="text-[10px] uppercase tracking-wider" style="color: {data.color};">
      {categoryLabels[data.category]}
    </span>
  </div>

  <!-- File path if available -->
  {#if data.filePath}
    <div class="px-3 pb-2 border-t border-neutral-700/50 pt-2">
      <span class="text-[10px] text-neutral-500 truncate block" title={data.filePath}>
        {data.filePath}
      </span>
    </div>
  {/if}

  <!-- Connection handles (for edge routing) -->
  <Handle
    type="target"
    position={Position.Left}
    id="in"
    style="background: {data.color}; width: 8px; height: 8px; border: 2px solid #262626;"
  />
  <Handle
    type="source"
    position={Position.Right}
    id="out"
    style="background: {data.color}; width: 8px; height: 8px; border: 2px solid #262626;"
  />
</button>

<style>
  .arch-node {
    border: 1px solid #404040;
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
  }

  .arch-node:hover {
    border-color: var(--node-color);
  }

  .arch-node.selected {
    border-color: var(--node-color);
    box-shadow: 0 0 0 2px var(--node-color);
  }

  :global(.arch-node .svelte-flow__handle) {
    border-radius: 50%;
  }
</style>

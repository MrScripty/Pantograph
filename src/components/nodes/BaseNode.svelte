<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { NodeDefinition, PortDefinition } from '../../services/workflow/types';
  import type { Snippet } from 'svelte';

  interface Props {
    id: string;
    data: { definition?: NodeDefinition; label?: string } & Record<string, unknown>;
    selected?: boolean;
    header?: Snippet;
    children?: Snippet;
  }

  let { id, data, selected = false, header, children }: Props = $props();

  let definition = $derived(data.definition);
  let inputs = $derived(definition?.inputs || []);
  let outputs = $derived(definition?.outputs || []);
  let label = $derived(data.label || definition?.label || 'Node');

  const typeColors: Record<string, string> = {
    String: '#22c55e',
    Prompt: '#3b82f6',
    Number: '#f59e0b',
    Boolean: '#ef4444',
    Image: '#8b5cf6',
    Stream: '#06b6d4',
    Json: '#f97316',
    Component: '#ec4899',
    Document: '#14b8a6',
    Tools: '#d97706',
    Embedding: '#6366f1',
    Any: '#6b7280',
  };

  function getPortColor(port: PortDefinition): string {
    return typeColors[port.data_type] || typeColors.Any;
  }

  // Calculate node height based on port count
  let nodeHeight = $derived(Math.max(inputs.length, outputs.length) * 28 + 60);
</script>

<div
  class="base-node bg-neutral-800 rounded-lg min-w-[200px] relative"
  class:selected
  style="min-height: {nodeHeight}px;"
>
  <div class="node-header px-3 py-2 bg-neutral-700/50 rounded-t-lg border-b border-neutral-600">
    {#if header}
      {@render header()}
    {:else}
      <span class="text-sm font-medium text-neutral-200">{label}</span>
    {/if}
  </div>

  <div class="node-content px-3 py-2">
    {#if children}
      {@render children()}
    {/if}
  </div>

  <!-- Input Handles -->
  {#each inputs as input, i}
    {@const yPos = 48 + i * 28}
    <Handle
      type="target"
      position={Position.Left}
      id={input.id}
      style="top: {yPos}px; background: {getPortColor(input)}; width: 12px; height: 12px; border: 2px solid #262626;"
    />
    <span
      class="port-label input-label text-[10px] text-neutral-400 absolute pointer-events-none"
      style="top: {yPos - 6}px; left: 16px;"
      title="{input.label} ({input.data_type})"
    >
      {input.label}
    </span>
  {/each}

  <!-- Output Handles -->
  {#each outputs as output, i}
    {@const yPos = 48 + i * 28}
    <Handle
      type="source"
      position={Position.Right}
      id={output.id}
      style="top: {yPos}px; background: {getPortColor(output)}; width: 12px; height: 12px; border: 2px solid #262626;"
    />
    <span
      class="port-label output-label text-[10px] text-neutral-400 absolute pointer-events-none text-right"
      style="top: {yPos - 6}px; right: 16px;"
      title="{output.label} ({output.data_type})"
    >
      {output.label}
    </span>
  {/each}
</div>

<style>
  .base-node {
    border: 1px solid #404040;
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
  }

  .base-node.selected {
    border-color: #4f46e5;
    box-shadow: 0 0 0 2px #4f46e5;
  }

  :global(.base-node .svelte-flow__handle) {
    border-radius: 50%;
  }
</style>

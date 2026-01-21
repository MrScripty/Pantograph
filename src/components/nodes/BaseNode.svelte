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
    string: '#22c55e',
    prompt: '#3b82f6',
    number: '#f59e0b',
    boolean: '#ef4444',
    image: '#8b5cf6',
    audio: '#f472b6',
    stream: '#06b6d4',
    json: '#f97316',
    component: '#ec4899',
    document: '#14b8a6',
    tools: '#d97706',
    embedding: '#6366f1',
    vector_db: '#a855f7',
    any: '#6b7280',
  };

  function getPortColor(port: PortDefinition): string {
    return typeColors[port.data_type] || typeColors.any;
  }

  // Calculate node height based on port count
  let nodeHeight = $derived(Math.max(inputs.length, outputs.length) * 28 + 60);
</script>

<div
  class="base-node bg-neutral-800 rounded-lg min-w-[180px] relative"
  class:selected
>
  <!-- Node Header -->
  <div class="node-header px-3 py-2 bg-neutral-700/50 rounded-t-lg border-b border-neutral-600">
    {#if header}
      {@render header()}
    {:else}
      <span class="text-sm font-medium text-neutral-200">{label}</span>
    {/if}
  </div>

  <!-- Ports Section -->
  <div class="ports-section px-3 py-2">
    <div class="ports-grid" style="min-height: {Math.max(inputs.length, outputs.length) * 20}px;">
      <!-- Input labels (left column) -->
      <div class="input-labels flex flex-col gap-1">
        {#each inputs as input}
          <span class="text-[10px] text-neutral-400 h-4 leading-4" title="{input.data_type}">
            {input.label}
          </span>
        {/each}
      </div>
      <!-- Output labels (right column) -->
      <div class="output-labels flex flex-col gap-1 text-right">
        {#each outputs as output}
          <span class="text-[10px] text-neutral-400 h-4 leading-4" title="{output.data_type}">
            {output.label}
          </span>
        {/each}
      </div>
    </div>
  </div>

  <!-- Node Content (below ports) -->
  {#if children}
    <div class="node-content px-3 py-2 border-t border-neutral-700">
      {@render children()}
    </div>
  {/if}

  <!-- Handles positioned absolutely on edges -->
  {#each inputs as input, i}
    {@const yPos = 52 + i * 20}
    <Handle
      type="target"
      position={Position.Left}
      id={input.id}
      style="top: {yPos}px; background: {getPortColor(input)}; width: 10px; height: 10px; border: 2px solid #262626;"
    />
  {/each}

  {#each outputs as output, i}
    {@const yPos = 52 + i * 20}
    <Handle
      type="source"
      position={Position.Right}
      id={output.id}
      style="top: {yPos}px; background: {getPortColor(output)}; width: 10px; height: 10px; border: 2px solid #262626;"
    />
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

  .ports-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  :global(.base-node .svelte-flow__handle) {
    border-radius: 50%;
  }
</style>

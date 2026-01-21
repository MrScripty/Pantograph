<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, PortDataType } from '../../../services/workflow/types';
  import { updateNodeData, edges, nodes } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      text?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let text = $state(data.text || '');

  // Port type colors (same as BaseNode)
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

  // Default color (blue - input category)
  const defaultColor = '#2563eb';

  // Check if the 'text' input is connected
  let isTextConnected = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'text')
  );

  // Find what this node's output is connected to and get the target port type
  let connectedTargetPortType = $derived.by(() => {
    // Find an edge where this node is the source
    const outEdge = $edges.find((edge) => edge.source === id && edge.sourceHandle === 'text');
    if (!outEdge) return null;

    // Find the target node
    const targetNode = $nodes.find((n) => n.id === outEdge.target);
    if (!targetNode?.data?.definition) return null;

    const def = targetNode.data.definition as NodeDefinition;
    const port = def.inputs.find((p) => p.id === outEdge.targetHandle);
    return port?.data_type || null;
  });

  // Get the color based on connected port type
  let nodeColor = $derived(
    connectedTargetPortType ? (typeColors[connectedTargetPortType] || defaultColor) : defaultColor
  );

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    text = target.value;
    updateNodeData(id, { text });
  }
</script>

<div class="text-input-node-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {nodeColor}">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Text Input'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      {#if isTextConnected}
        <div class="text-xs text-neutral-400 italic py-1">
          Connected to external input
        </div>
      {:else}
        <textarea
          class="w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-sm text-neutral-200 resize-none focus:outline-none"
          style="--focus-color: {nodeColor}"
          rows="3"
          placeholder="Enter text..."
          value={text}
          oninput={handleInput}
        ></textarea>
      {/if}
    {/snippet}
  </BaseNode>
</div>

<style>
  .text-input-node-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .text-input-node-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }

  .text-input-node-wrapper textarea:focus {
    border-color: var(--focus-color, #2563eb);
  }
</style>

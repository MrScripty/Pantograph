<script lang="ts">
  import BaseNode from './BaseNode.svelte';
  import type { NodeDefinition } from '../../types/workflow.js';
  import { useGraphContext } from '../../context/useGraphContext.js';
  import { getPortColor } from '../../constants/portColors.js';

  const { stores } = useGraphContext();
  const edgesStore = stores.workflow.edges;
  const nodesStore = stores.workflow.nodes;

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      text?: string;
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let text = $state(data.text || '');

  // Default color (blue — input category)
  const defaultColor = '#2563eb';

  // Check if the 'text' input is connected
  let isTextConnected = $derived(
    $edgesStore.some((edge) => edge.target === id && edge.targetHandle === 'text')
  );

  // Find the connected target port type to determine node accent color
  let connectedTargetPortType = $derived.by(() => {
    const outEdge = $edgesStore.find((edge) => edge.source === id && edge.sourceHandle === 'text');
    if (!outEdge) return null;

    const targetNode = $nodesStore.find((n) => n.id === outEdge.target);
    if (!targetNode?.data?.definition) return null;

    const def = targetNode.data.definition as NodeDefinition;
    const port = def.inputs.find((p) => p.id === outEdge.targetHandle);
    return port?.data_type || null;
  });

  let nodeColor = $derived(
    connectedTargetPortType ? getPortColor(connectedTargetPortType) : defaultColor
  );

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    text = target.value;
    stores.workflow.updateNodeData(id, { text });
  }
</script>

<div class="text-input-node-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="header-content">
        <div class="header-icon" style="background-color: {nodeColor}">
          <svg class="icon-svg" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
          </svg>
        </div>
        <span class="header-label">{data.label || 'Text Input'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      {#if isTextConnected}
        <div class="connected-hint">
          Connected to external input
        </div>
      {:else}
        <textarea
          class="text-area"
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

  .header-content {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .header-icon {
    width: 1.25rem;
    height: 1.25rem;
    border-radius: 0.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .icon-svg {
    width: 0.75rem;
    height: 0.75rem;
    color: white;
  }

  .header-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: #e5e5e5;
  }

  .connected-hint {
    font-size: 0.75rem;
    color: #a3a3a3;
    font-style: italic;
    padding: 0.25rem 0;
  }

  .text-area {
    width: 100%;
    background-color: #171717;
    border: 1px solid #525252;
    border-radius: 0.25rem;
    padding: 0.25rem 0.5rem;
    font-size: 0.875rem;
    color: #e5e5e5;
    resize: none;
    outline: none;
    font-family: inherit;
  }

  .text-area:focus {
    border-color: var(--focus-color, #2563eb);
  }
</style>

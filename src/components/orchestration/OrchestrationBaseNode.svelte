<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { Snippet } from 'svelte';
  import { selectedOrchestrationNodeId } from '../../stores/orchestrationStore';

  interface HandleDef {
    id: string;
    label: string;
  }

  interface Props {
    id: string;
    label: string;
    color: string;
    icon?: Snippet;
    inputHandles?: HandleDef[];
    outputHandles?: HandleDef[];
    children?: Snippet;
  }

  let {
    id,
    label,
    color,
    icon,
    inputHandles = [],
    outputHandles = [],
    children,
  }: Props = $props();

  let isSelected = $derived($selectedOrchestrationNodeId === id);
  let handleCount = $derived(Math.max(inputHandles.length, outputHandles.length, 1));
</script>

<div
  class="orchestration-node"
  class:selected={isSelected}
  style="--node-color: {color};"
>
  <!-- Header -->
  <div class="node-header">
    {#if icon}
      <div class="icon-wrapper">
        {@render icon()}
      </div>
    {/if}
    <span class="node-label">{label}</span>
  </div>

  <!-- Handles Section -->
  <div class="handles-section" style="min-height: {handleCount * 24 + 8}px;">
    <!-- Input labels -->
    <div class="input-handles">
      {#each inputHandles as handle, i}
        <div class="handle-label input">{handle.label}</div>
      {/each}
    </div>

    <!-- Output labels -->
    <div class="output-handles">
      {#each outputHandles as handle, i}
        <div class="handle-label output">{handle.label}</div>
      {/each}
    </div>
  </div>

  <!-- Content -->
  {#if children}
    <div class="node-content">
      {@render children()}
    </div>
  {/if}

  <!-- Input Handles -->
  {#each inputHandles as handle, i}
    {@const yPos = 44 + i * 24}
    <Handle
      type="target"
      position={Position.Left}
      id={handle.id}
      style="top: {yPos}px; background: {color}; width: 12px; height: 12px; border: 2px solid #1a1a1a;"
    />
  {/each}

  <!-- Output Handles -->
  {#each outputHandles as handle, i}
    {@const yPos = 44 + i * 24}
    <Handle
      type="source"
      position={Position.Right}
      id={handle.id}
      style="top: {yPos}px; background: {color}; width: 12px; height: 12px; border: 2px solid #1a1a1a;"
    />
  {/each}
</div>

<style>
  .orchestration-node {
    min-width: 140px;
    background: #252525;
    border: 2px solid #404040;
    border-radius: 8px;
    font-family: inherit;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.3);
  }

  .orchestration-node.selected {
    border-color: var(--node-color);
    box-shadow: 0 0 0 2px var(--node-color), 0 4px 8px rgba(0, 0, 0, 0.3);
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.05);
    border-bottom: 1px solid #333;
    border-radius: 6px 6px 0 0;
  }

  .icon-wrapper {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--node-color);
    color: white;
  }

  .icon-wrapper :global(svg) {
    width: 14px;
    height: 14px;
  }

  .node-label {
    font-size: 13px;
    font-weight: 600;
    color: #e5e5e5;
  }

  .handles-section {
    display: flex;
    justify-content: space-between;
    padding: 8px 12px;
  }

  .input-handles,
  .output-handles {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .output-handles {
    text-align: right;
  }

  .handle-label {
    font-size: 10px;
    color: #888;
    height: 16px;
    line-height: 16px;
  }

  .node-content {
    padding: 8px 12px;
    border-top: 1px solid #333;
  }

  :global(.orchestration-node .svelte-flow__handle) {
    border-radius: 50%;
  }
</style>

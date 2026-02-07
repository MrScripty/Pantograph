<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { NodeDefinition, PortDefinition } from '../../types/workflow.js';
  import type { Snippet } from 'svelte';
  import { useGraphContext } from '../../context/useGraphContext.js';
  import { PORT_TYPE_COLORS, getPortColor as getPortColorFn } from '../../constants/portColors.js';

  const { stores } = useGraphContext();
  const nodeExecutionStates = stores.workflow.nodeExecutionStates;
  const edgesStore = stores.workflow.edges;

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

  // Get execution state for this node
  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let errorMessage = $derived(executionInfo?.errorMessage);

  function getPortColor(port: PortDefinition): string {
    return getPortColorFn(port.data_type);
  }

  // Check if an input port is connected
  function isInputConnected(portId: string): boolean {
    return $edgesStore.some(edge => edge.target === id && edge.targetHandle === portId);
  }

  // Check if an output port is connected
  function isOutputConnected(portId: string): boolean {
    return $edgesStore.some(edge => edge.source === id && edge.sourceHandle === portId);
  }

  // Calculate node height based on port count
  let nodeHeight = $derived(Math.max(inputs.length, outputs.length) * 28 + 60);
</script>

<div
  class="base-node bg-neutral-800 rounded-lg min-w-[180px] relative"
  class:selected
  class:error={executionState === 'error'}
  class:running={executionState === 'running'}
  class:success={executionState === 'success'}
>
  <!-- Node Header -->
  <div class="node-header px-3 py-2 bg-neutral-700/50 rounded-t-lg border-b border-neutral-600 flex items-center justify-between gap-2">
    <div class="flex-1 min-w-0">
      {#if header}
        {@render header()}
      {:else}
        <span class="text-sm font-medium text-neutral-200">{label}</span>
      {/if}
    </div>
    <!-- Execution status indicator -->
    {#if executionState !== 'idle'}
      <div
        class="status-dot w-2.5 h-2.5 rounded-full flex-shrink-0"
        class:bg-green-500={executionState === 'success'}
        class:bg-red-500={executionState === 'error'}
        class:bg-amber-500={executionState === 'running'}
        class:animate-pulse={executionState === 'running'}
        title={executionState === 'error' && errorMessage ? errorMessage : executionState}
      ></div>
    {/if}
  </div>

  <!-- Error message banner -->
  {#if executionState === 'error' && errorMessage}
    <div class="error-banner px-3 py-1.5 bg-red-900/50 border-b border-red-700 text-xs text-red-300 truncate" title={errorMessage}>
      {errorMessage}
    </div>
  {/if}

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
    {@const color = getPortColor(input)}
    {@const connected = isInputConnected(input.id)}
    <Handle
      type="target"
      position={Position.Left}
      id={input.id}
      style="top: {yPos}px; background: {color}; width: 10px; height: 10px; border: none;{connected ? ` box-shadow: 0 0 8px ${color};` : ''}"
    />
  {/each}

  {#each outputs as output, i}
    {@const yPos = 52 + i * 20}
    {@const color = getPortColor(output)}
    {@const connected = isOutputConnected(output.id)}
    <Handle
      type="source"
      position={Position.Right}
      id={output.id}
      style="top: {yPos}px; background: {color}; width: 10px; height: 10px; border: none;{connected ? ` box-shadow: 0 0 8px ${color};` : ''}"
    />
  {/each}
</div>

<style>
  .base-node {
    border: 1px solid #60a5fa;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.3),
      0 0 15px rgba(59, 130, 246, 0.15),
      0 0 30px rgba(59, 130, 246, 0.08);
  }

  .base-node.selected {
    border-color: #4f46e5;
    box-shadow:
      0 0 0 2px #4f46e5,
      0 0 20px rgba(79, 70, 229, 0.4),
      0 0 40px rgba(79, 70, 229, 0.2);
  }

  .base-node.error {
    border-color: #ef4444;
    border-width: 2px;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.3),
      0 0 20px rgba(239, 68, 68, 0.5),
      0 0 40px rgba(239, 68, 68, 0.3);
    animation: error-pulse 2s ease-in-out infinite;
  }

  @keyframes error-pulse {
    0%, 100% {
      box-shadow:
        0 4px 6px -1px rgba(0, 0, 0, 0.3),
        0 0 20px rgba(239, 68, 68, 0.5),
        0 0 40px rgba(239, 68, 68, 0.3);
    }
    50% {
      box-shadow:
        0 4px 6px -1px rgba(0, 0, 0, 0.3),
        0 0 30px rgba(239, 68, 68, 0.7),
        0 0 60px rgba(239, 68, 68, 0.4);
    }
  }

  .base-node.running {
    border-color: #f59e0b;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.3),
      0 0 15px rgba(245, 158, 11, 0.3),
      0 0 30px rgba(245, 158, 11, 0.15);
  }

  .base-node.success {
    border-color: #22c55e;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.3),
      0 0 15px rgba(34, 197, 94, 0.2),
      0 0 30px rgba(34, 197, 94, 0.1);
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

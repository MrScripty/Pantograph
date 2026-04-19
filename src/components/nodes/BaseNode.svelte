<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { NodeDefinition, PortDefinition } from '../../services/workflow/types';
  import type { Snippet } from 'svelte';
  import { connectionIntent, edges, nodeExecutionStates } from '../../stores/workflowStore';

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
  let activeConnectionIntent = $derived($connectionIntent);
  let hasConnectionIntent = $derived(activeConnectionIntent !== null);
  let intentSourceAnchor = $derived(activeConnectionIntent?.sourceAnchor ?? null);
  let intentCompatibleNodeIds = $derived(new Set(activeConnectionIntent?.compatibleNodeIds ?? []));
  let intentCompatibleTargetKeys = $derived(new Set(activeConnectionIntent?.compatibleTargetKeys ?? []));
  let isIntentSourceNode = $derived(intentSourceAnchor?.node_id === id);
  let isIntentCompatibleNode = $derived(
    !hasConnectionIntent || isIntentSourceNode || intentCompatibleNodeIds.has(id)
  );

  const typeColors: Record<string, string> = {
    string: '#22c55e',
    prompt: '#3b82f6',
    number: '#f59e0b',
    boolean: '#ef4444',
    image: '#8b5cf6',
    audio: '#f472b6',
    audio_stream: '#0ea5e9',
    stream: '#06b6d4',
    json: '#f97316',
    kv_cache: '#84cc16',
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

  // Check if an input port is connected
  function isInputConnected(portId: string): boolean {
    return $edges.some(edge => edge.target === id && edge.targetHandle === portId);
  }

  // Check if an output port is connected
  function isOutputConnected(portId: string): boolean {
    return $edges.some(edge => edge.source === id && edge.sourceHandle === portId);
  }

  function isIntentTarget(portId: string): boolean {
    return intentCompatibleTargetKeys.has(`${id}:${portId}`);
  }

  function getInputHandleClass(portId: string): string {
    if (!hasConnectionIntent || isIntentSourceNode) return '';
    return isIntentTarget(portId) ? 'intent-eligible' : 'intent-ineligible';
  }

  function getOutputHandleClass(portId: string): string {
    if (!hasConnectionIntent) return '';
    return isIntentSourceNode && intentSourceAnchor?.port_id === portId
      ? 'intent-source'
      : 'intent-ineligible';
  }

</script>

<div
  class="base-node bg-neutral-800 rounded-lg min-w-[180px] relative"
  class:selected
  class:error={executionState === 'error'}
  class:running={executionState === 'running'}
  class:success={executionState === 'success'}
  class:intent-dimmed={hasConnectionIntent && !isIntentCompatibleNode}
  class:intent-compatible={hasConnectionIntent && !isIntentSourceNode && intentCompatibleNodeIds.has(id)}
  class:intent-source={isIntentSourceNode}
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
        {#each inputs as input (input.id)}
          <span class="text-[10px] text-neutral-400 h-4 leading-4" title="{input.data_type}">
            {input.label}
          </span>
        {/each}
      </div>
      <!-- Output labels (right column) -->
      <div class="output-labels flex flex-col gap-1 text-right">
        {#each outputs as output (output.id)}
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
  {#each inputs as input, i (input.id)}
    {@const yPos = 52 + i * 20}
    {@const color = getPortColor(input)}
    {@const connected = isInputConnected(input.id)}
    <Handle
      type="target"
      position={Position.Left}
      id={input.id}
      class={getInputHandleClass(input.id)}
      style="top: {yPos}px; background: {color}; width: 10px; height: 10px; border: none;{connected ? ` box-shadow: 0 0 8px ${color};` : ''}"
    />
  {/each}

  {#each outputs as output, i (output.id)}
    {@const yPos = 52 + i * 20}
    {@const color = getPortColor(output)}
    {@const connected = isOutputConnected(output.id)}
    <Handle
      type="source"
      position={Position.Right}
      id={output.id}
      class={getOutputHandleClass(output.id)}
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

  .base-node.intent-dimmed {
    opacity: 0.35;
    filter: saturate(0.65);
  }

  .base-node.intent-compatible {
    border-color: #34d399;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.3),
      0 0 18px rgba(52, 211, 153, 0.22),
      0 0 36px rgba(52, 211, 153, 0.12);
  }

  .base-node.intent-source {
    border-color: #f59e0b;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.3),
      0 0 18px rgba(245, 158, 11, 0.24),
      0 0 36px rgba(245, 158, 11, 0.12);
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
    transition:
      opacity 120ms ease,
      transform 120ms ease,
      box-shadow 120ms ease;
  }

  :global(.base-node .svelte-flow__handle.intent-eligible) {
    opacity: 1;
    transform: scale(1.15);
    box-shadow: 0 0 0 2px rgba(52, 211, 153, 0.35);
  }

  :global(.base-node .svelte-flow__handle.intent-ineligible) {
    opacity: 0.2;
    transform: scale(0.92);
  }

  :global(.base-node .svelte-flow__handle.intent-source) {
    opacity: 1;
    transform: scale(1.1);
    box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.35);
  }
</style>

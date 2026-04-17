<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { NodeDefinition, PortDefinition } from '../../types/workflow.js';
  import type { Snippet } from 'svelte';
  import { useGraphContext } from '../../context/useGraphContext.js';
  import { getPortColor as getPortColorFn } from '../../constants/portColors.js';

  const { stores } = useGraphContext();
  const nodeExecutionStates = stores.workflow.nodeExecutionStates;
  const edgesStore = stores.workflow.edges;
  const connectionIntent = stores.workflow.connectionIntent;

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

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let statusMessage = $derived(executionInfo?.message);
  let activeConnectionIntent = $derived($connectionIntent);
  let hasConnectionIntent = $derived(activeConnectionIntent !== null);
  let intentSourceAnchor = $derived(activeConnectionIntent?.sourceAnchor ?? null);
  let intentCompatibleNodeIds = $derived(new Set(activeConnectionIntent?.compatibleNodeIds ?? []));
  let intentCompatibleTargetKeys = $derived(new Set(activeConnectionIntent?.compatibleTargetKeys ?? []));
  let isIntentSourceNode = $derived(intentSourceAnchor?.node_id === id);
  let isIntentCompatibleNode = $derived(
    !hasConnectionIntent || isIntentSourceNode || intentCompatibleNodeIds.has(id)
  );

  function getPortColor(port: PortDefinition): string {
    return getPortColorFn(port.data_type);
  }

  function isInputConnected(portId: string): boolean {
    return $edgesStore.some(edge => edge.target === id && edge.targetHandle === portId);
  }

  function isOutputConnected(portId: string): boolean {
    return $edgesStore.some(edge => edge.source === id && edge.sourceHandle === portId);
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
  class="base-node"
  class:selected
  class:intent-dimmed={hasConnectionIntent && !isIntentCompatibleNode}
  class:intent-compatible={hasConnectionIntent && !isIntentSourceNode && intentCompatibleNodeIds.has(id)}
  class:intent-source={isIntentSourceNode}
  data-state={executionState}
>
  <!-- Node Header -->
  <div class="node-header">
    <div class="node-header-label">
      {#if header}
        {@render header()}
      {:else}
        <span class="node-label">{label}</span>
      {/if}
    </div>
    {#if executionState !== 'idle'}
      <div
        class="status-dot"
        data-state={executionState}
        title={statusMessage || executionState}
      ></div>
    {/if}
  </div>

  <!-- Status message banner -->
  {#if (executionState === 'error' || executionState === 'waiting') && statusMessage}
    <div class:waiting-banner={executionState === 'waiting'} class="error-banner" title={statusMessage}>
      {statusMessage}
    </div>
  {/if}

  <!-- Ports Section -->
  <div class="ports-section">
    <div class="ports-grid" style="min-height: {Math.max(inputs.length, outputs.length) * 20}px;">
      <div class="input-labels">
        {#each inputs as input (input.id)}
          <span class="port-label" title="{input.data_type}">
            {input.label}
          </span>
        {/each}
      </div>
      <div class="output-labels">
        {#each outputs as output (output.id)}
          <span class="port-label" title="{output.data_type}">
            {output.label}
          </span>
        {/each}
      </div>
    </div>
  </div>

  <!-- Node Content (below ports) -->
  {#if children}
    <div class="node-content">
      {@render children()}
    </div>
  {/if}

  <!-- Handles positioned absolutely on edges -->
  {#each inputs as input, i (input.id)}
    {@const yPos = 54 + i * 20}
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
    {@const yPos = 54 + i * 20}
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
    background-color: #262626;
    border-radius: 0.5rem;
    min-width: 180px;
    position: relative;
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

  .base-node[data-state="error"] {
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

  .base-node[data-state="running"] {
    border-color: #f59e0b;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.3),
      0 0 15px rgba(245, 158, 11, 0.3),
      0 0 30px rgba(245, 158, 11, 0.15);
  }

  .base-node[data-state="waiting"] {
    border-color: #3b82f6;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.3),
      0 0 15px rgba(59, 130, 246, 0.28),
      0 0 30px rgba(59, 130, 246, 0.14);
  }

  .base-node[data-state="success"] {
    border-color: #22c55e;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.3),
      0 0 15px rgba(34, 197, 94, 0.2),
      0 0 30px rgba(34, 197, 94, 0.1);
  }

  /* --- Header --- */
  .node-header {
    padding: 0.5rem 0.75rem;
    background-color: rgba(64, 64, 64, 0.5);
    border-top-left-radius: 0.5rem;
    border-top-right-radius: 0.5rem;
    border-bottom: 1px solid #525252;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .node-header-label {
    flex: 1;
    min-width: 0;
  }

  .node-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: #e5e5e5;
  }

  .status-dot {
    width: 0.625rem;
    height: 0.625rem;
    border-radius: 9999px;
    flex-shrink: 0;
  }

  .status-dot[data-state="success"] {
    background-color: #22c55e;
  }

  .status-dot[data-state="error"] {
    background-color: #ef4444;
  }

  .status-dot[data-state="running"] {
    background-color: #f59e0b;
    animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  .status-dot[data-state="waiting"] {
    background-color: #3b82f6;
    animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  /* --- Error Banner --- */
  .error-banner {
    padding: 0.375rem 0.75rem;
    background-color: rgba(127, 29, 29, 0.5);
    border-bottom: 1px solid #b91c1c;
    font-size: 0.75rem;
    color: #fca5a5;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .waiting-banner {
    background-color: rgba(30, 64, 175, 0.42);
    border-bottom-color: #2563eb;
    color: #bfdbfe;
  }

  /* --- Ports --- */
  .ports-section {
    padding: 0.5rem 0.75rem;
  }

  .ports-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .input-labels,
  .output-labels {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .output-labels {
    text-align: right;
  }

  .port-label {
    font-size: 10px;
    color: #a3a3a3;
    height: 1rem;
    line-height: 1rem;
  }

  /* --- Node Content --- */
  .node-content {
    padding: 0.5rem 0.75rem;
    border-top: 1px solid #404040;
  }

  /* --- Handle overrides --- */
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

<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { NodeDefinition } from '../../services/workflow/types';
  import type { Snippet } from 'svelte';
  import { connectionIntent, edges, nodeExecutionStates } from '../../stores/workflowStore';
  import { inferenceProposalAffectsPort } from '../workflowInferenceDriftEdgeOverlays';
  import StatusGlyph from '../workbench/ui/StatusGlyph.svelte';
  import { editNodeExecutionGlyph } from '../workbench/graphInspectionPresenters';

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
  let executionGlyph = $derived(editNodeExecutionGlyph(executionState));
  let activeConnectionIntent = $derived($connectionIntent);
  let hasConnectionIntent = $derived(activeConnectionIntent !== null);
  let intentSourceAnchor = $derived(activeConnectionIntent?.sourceAnchor ?? null);
  let intentCompatibleNodeIds = $derived(new Set(activeConnectionIntent?.compatibleNodeIds ?? []));
  let intentCompatibleTargetKeys = $derived(new Set(activeConnectionIntent?.compatibleTargetKeys ?? []));
  let isIntentSourceNode = $derived(intentSourceAnchor?.node_id === id);
  let isIntentCompatibleNode = $derived(
    !hasConnectionIntent || isIntentSourceNode || intentCompatibleNodeIds.has(id)
  );

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
    const driftClass = inferenceProposalAffectsPort({ data, nodeId: id, portId })
      ? 'inference-drift-affected'
      : '';
    if (!hasConnectionIntent || isIntentSourceNode) return driftClass;
    const intentClass = isIntentTarget(portId) ? 'intent-eligible' : 'intent-ineligible';
    return `${intentClass} ${driftClass}`.trim();
  }

  function getOutputHandleClass(portId: string): string {
    const driftClass = inferenceProposalAffectsPort({ data, nodeId: id, portId })
      ? 'inference-drift-affected'
      : '';
    if (!hasConnectionIntent) return driftClass;
    const intentClass = isIntentSourceNode && intentSourceAnchor?.port_id === portId
      ? 'intent-source'
      : 'intent-ineligible';
    return `${intentClass} ${driftClass}`.trim();
  }

</script>

<div
  class="base-node relative min-w-[180px]"
  class:selected
  class:error={executionState === 'error'}
  class:running={executionState === 'running'}
  class:success={executionState === 'success'}
  class:intent-dimmed={hasConnectionIntent && !isIntentCompatibleNode}
  class:intent-compatible={hasConnectionIntent && !isIntentSourceNode && intentCompatibleNodeIds.has(id)}
  class:intent-source={isIntentSourceNode}
>
  <!-- Node header. Its height (37px) and the ports padding below place the
       handles at 52px + 20px per port; keep them in step. -->
  <div class="node-header flex items-center gap-2 px-3 py-2">
    <span
      class="node-glyph"
      title={executionState === 'error' && errorMessage ? errorMessage : executionGlyph.label}
    >
      <StatusGlyph state={executionGlyph.glyph} small />
      <span class="pg-sr-only">{executionGlyph.label}</span>
    </span>
    <div class="min-w-0 flex-1">
      {#if header}
        {@render header()}
      {:else}
        <span class="node-title">{label}</span>
      {/if}
    </div>
  </div>

  <!-- Error message, in the node it belongs to -->
  {#if executionState === 'error' && errorMessage}
    <div class="node-error truncate px-3 py-1.5 text-xs" title={errorMessage}>
      {errorMessage}
    </div>
  {/if}

  <!-- Ports Section -->
  <div class="ports-section px-3 py-2">
    <div class="ports-grid" style="min-height: {Math.max(inputs.length, outputs.length) * 20}px;">
      <!-- Input labels (left column) -->
      <div class="input-labels flex flex-col gap-1">
        {#each inputs as input (input.id)}
          <span class="port-label h-4 leading-4" title="{input.data_type}">
            {input.label}
          </span>
        {/each}
      </div>
      <!-- Output labels (right column) -->
      <div class="output-labels flex flex-col gap-1 text-right">
        {#each outputs as output (output.id)}
          <span class="port-label h-4 leading-4" title="{output.data_type}">
            {output.label}
          </span>
        {/each}
      </div>
    </div>
  </div>

  <!-- Node Content (below ports) -->
  {#if children}
    <div class="node-content px-3 py-2">
      {@render children()}
    </div>
  {/if}

  <!-- Handles positioned absolutely on edges -->
  {#each inputs as input, i (input.id)}
    {@const yPos = 52 + i * 20}
    <Handle
      type="target"
      position={Position.Left}
      id={input.id}
      class={`${getInputHandleClass(input.id)}${isInputConnected(input.id) ? ' port-connected' : ''}`}
      style="top: {yPos}px; width: 10px; height: 10px;"
    />
  {/each}

  {#each outputs as output, i (output.id)}
    {@const yPos = 52 + i * 20}
    <Handle
      type="source"
      position={Position.Right}
      id={output.id}
      class={`${getOutputHandleClass(output.id)}${isOutputConnected(output.id) ? ' port-connected' : ''}`}
      style="top: {yPos}px; width: 10px; height: 10px;"
    />
  {/each}
</div>

<style>
  /* Token colours only (docs/design/workbench-redesign-spec.md); no glow, no pulse. */
  .base-node {
    background: var(--pg-panel);
    border: 1px solid var(--pg-scroll-thumb);
    border-radius: 4px;
    color: var(--pg-text);
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.25);
  }

  .node-header {
    border-bottom: 1px solid var(--pg-border);
    border-radius: 4px 4px 0 0;
    background: var(--pg-panel);
    font-size: 14px;
    line-height: 20px;
  }

  .node-glyph {
    display: inline-flex;
    align-items: center;
    flex: none;
  }

  .node-title {
    font-size: 12.5px;
    font-weight: 700;
    color: var(--pg-text-bright);
  }

  .node-error {
    border-bottom: 1px solid var(--pg-border);
    background: var(--pg-panel-head);
    color: var(--pg-danger);
  }

  .port-label {
    font-size: 10.5px;
    color: var(--pg-text-soft);
  }

  .node-content {
    border-top: 1px solid var(--pg-border);
  }

  .base-node.selected {
    border-color: var(--pg-text);
    box-shadow: 0 0 0 1px var(--pg-text);
  }

  .base-node.running {
    border-color: var(--pg-accent);
  }

  .base-node.error {
    border-color: var(--pg-danger);
  }

  .base-node.intent-dimmed {
    opacity: 0.35;
  }

  .base-node.intent-compatible {
    border-color: var(--pg-accent);
  }

  .base-node.intent-source {
    border-color: var(--pg-accent);
    box-shadow: 0 0 0 1px var(--pg-accent);
  }

  .ports-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  :global(.base-node .svelte-flow__handle) {
    border-radius: 50%;
    background: var(--pg-panel-head);
    border: 1.5px solid var(--pg-border-strong);
    transition:
      opacity 120ms ease,
      transform 120ms ease;
  }

  :global(.base-node .svelte-flow__handle.port-connected) {
    background: var(--pg-text-soft);
    border-color: var(--pg-text-soft);
  }

  :global(.base-node .svelte-flow__handle.intent-eligible) {
    opacity: 1;
    transform: scale(1.15);
    border-color: var(--pg-accent);
  }

  :global(.base-node .svelte-flow__handle.intent-ineligible) {
    opacity: 0.2;
    transform: scale(0.92);
  }

  :global(.base-node .svelte-flow__handle.intent-source) {
    opacity: 1;
    transform: scale(1.1);
    background: var(--pg-accent);
    border-color: var(--pg-accent);
  }

  :global(.base-node .svelte-flow__handle.inference-drift-affected) {
    outline: 2px dashed var(--pg-danger);
    outline-offset: 2px;
  }
</style>

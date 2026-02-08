<script lang="ts">
  import BaseNode from './BaseNode.svelte';
  import type { NodeDefinition } from '../../types/workflow.js';
  import { useGraphContext } from '../../context/useGraphContext.js';

  const { stores } = useGraphContext();
  const nodeExecutionStates = stores.workflow.nodeExecutionStates;
  const edges = stores.workflow.edges;

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      streamContent?: string;
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let streamContent = $derived(data.streamContent || '');

  let isModelConnected = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'model_path')
  );

  const nodeColor = '#9333ea';

  let statusText = $derived(
    {
      idle: 'Ready',
      running: 'Generating...',
      success: 'Complete',
      error: 'Error',
    }[executionState]
  );
</script>

<div class="llamacpp-node-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="header-content">
        <div class="header-icon">
          <svg class="icon-svg" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
          </svg>
        </div>
        <span class="header-label">{data.label || 'LlamaCpp Inference'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      <div class="body-content">
        <div class="status-row">
          <span>{statusText}</span>
        </div>
        {#if !isModelConnected}
          <div class="model-warning">
            Connect a Puma-Lib node
          </div>
        {/if}
        {#if streamContent}
          <div class="stream-output">
            {streamContent}
          </div>
        {/if}
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .llamacpp-node-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .llamacpp-node-wrapper :global(.node-header) {
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
    background-color: #9333ea;
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

  .body-content {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: #a3a3a3;
  }

  .model-warning {
    font-size: 0.625rem;
    color: #fbbf24;
  }

  .stream-output {
    padding: 0.5rem;
    background-color: #171717;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    color: #d4d4d4;
    max-height: 5rem;
    overflow-y: auto;
  }
</style>

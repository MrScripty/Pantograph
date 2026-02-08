<script lang="ts">
  import BaseNode from './BaseNode.svelte';
  import type { NodeDefinition } from '../../types/workflow.js';
  import { useGraphContext } from '../../context/useGraphContext.js';

  const { stores } = useGraphContext();
  const nodeExecutionStates = stores.workflow.nodeExecutionStates;

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

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let text = $derived(data.text || '');
</script>

<div class="output-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="header-content">
        <div class="header-icon">
          <svg class="icon-svg" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
        </div>
        <span class="header-label">{data.label || 'Text Output'}</span>
        {#if executionState !== 'idle'}
          <span class="status-dot" data-state={executionState}></span>
        {/if}
      </div>
    {/snippet}

    {#snippet children()}
      {#if text}
        <div class="output-text">
          {text}
        </div>
      {:else}
        <div class="no-output">
          No output yet
        </div>
      {/if}
    {/snippet}
  </BaseNode>
</div>

<style>
  .output-node-wrapper :global(.base-node) {
    border-color: rgba(8, 145, 178, 0.5);
  }

  .output-node-wrapper :global(.node-header) {
    background-color: rgba(8, 145, 178, 0.2);
    border-color: rgba(8, 145, 178, 0.3);
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
    background-color: #0891b2;
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

  .status-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 9999px;
    margin-left: auto;
  }

  .status-dot[data-state="running"] {
    background-color: #0891b2;
    animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  .status-dot[data-state="success"] {
    background-color: #0891b2;
  }

  .status-dot[data-state="error"] {
    background-color: #ef4444;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .output-text {
    padding: 0.5rem;
    background-color: #171717;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    color: #d4d4d4;
    max-height: 8rem;
    overflow-y: auto;
    white-space: pre-wrap;
  }

  .no-output {
    font-size: 0.75rem;
    color: #737373;
    font-style: italic;
  }
</style>

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
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');

  let statusText = $derived(
    {
      idle: 'Idle',
      running: 'Running...',
      success: 'Complete',
      error: 'Error',
    }[executionState as string]
  );
</script>

<div class="generic-node-wrapper" data-category={data.definition?.category || 'processing'}>
  <BaseNode {id} {data} {selected}>
    {#snippet children()}
      <div class="status-row">
        <span class="status-dot" data-state={executionState}></span>
        <span class="status-text">{statusText}</span>
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .generic-node-wrapper :global(.base-node) {
    border-color: inherit;
  }

  /* Category border colors */
  .generic-node-wrapper[data-category="input"] { border-color: rgba(37, 99, 235, 0.5); }
  .generic-node-wrapper[data-category="processing"] { border-color: rgba(22, 163, 74, 0.5); }
  .generic-node-wrapper[data-category="tool"] { border-color: rgba(217, 119, 6, 0.5); }
  .generic-node-wrapper[data-category="output"] { border-color: rgba(8, 145, 178, 0.5); }
  .generic-node-wrapper[data-category="control"] { border-color: rgba(147, 51, 234, 0.5); }

  /* Status row */
  .status-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .status-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 9999px;
  }

  .status-dot[data-state="idle"] { background-color: #737373; }
  .status-dot[data-state="running"] { background-color: #3b82f6; animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite; }
  .status-dot[data-state="success"] { background-color: #22c55e; }
  .status-dot[data-state="error"] { background-color: #ef4444; }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .status-text {
    font-size: 0.75rem;
    color: #a3a3a3;
  }
</style>

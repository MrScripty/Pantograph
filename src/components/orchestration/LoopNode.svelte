<script lang="ts">
  import OrchestrationBaseNode from './OrchestrationBaseNode.svelte';
  import type { LoopConfig } from '../../stores/orchestrationStore';

  interface Props {
    id: string;
    data: {
      label?: string;
      config?: LoopConfig;
    };
  }

  let { id, data }: Props = $props();

  const inputHandles = [
    { id: 'input', label: 'Input' },
    { id: 'loop_back', label: 'Loop' },
  ];
  const outputHandles = [
    { id: 'iteration', label: 'Iterate' },
    { id: 'complete', label: 'Done' },
  ];

  let maxIterations = $derived(data.config?.maxIterations ?? 10);
  let exitKey = $derived(data.config?.exitConditionKey);
</script>

<OrchestrationBaseNode
  {id}
  label="Loop"
  color="#8b5cf6"
  {inputHandles}
  {outputHandles}
>
  {#snippet icon()}
    <svg viewBox="0 0 24 24" fill="currentColor">
      <path
        d="M12 4V1L8 5l4 4V6c3.31 0 6 2.69 6 6 0 1.01-.25 1.97-.7 2.8l1.46 1.46C19.54 15.03 20 13.57 20 12c0-4.42-3.58-8-8-8zm0 14c-3.31 0-6-2.69-6-6 0-1.01.25-1.97.7-2.8L5.24 7.74C4.46 8.97 4 10.43 4 12c0 4.42 3.58 8 8 8v3l4-4-4-4v3z"
      />
    </svg>
  {/snippet}

  {#snippet children()}
    <div class="loop-config">
      <div class="config-row">
        <span class="config-label">Max:</span>
        <span class="config-value">{maxIterations}</span>
      </div>
      {#if exitKey}
        <div class="config-row">
          <span class="config-label">Exit on:</span>
          <span class="config-value">{exitKey}</span>
        </div>
      {/if}
    </div>
  {/snippet}
</OrchestrationBaseNode>

<style>
  .loop-config {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
  }

  .config-row {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .config-label {
    color: #666;
  }

  .config-value {
    color: #8b5cf6;
    font-family: monospace;
    background: rgba(139, 92, 246, 0.1);
    padding: 2px 6px;
    border-radius: 4px;
  }
</style>

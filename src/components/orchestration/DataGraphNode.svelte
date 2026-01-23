<script lang="ts">
  import OrchestrationBaseNode from './OrchestrationBaseNode.svelte';
  import type { DataGraphConfig } from '../../stores/orchestrationStore';

  interface Props {
    id: string;
    data: {
      label?: string;
      config?: DataGraphConfig;
    };
  }

  let { id, data }: Props = $props();

  const inputHandles = [{ id: 'input', label: 'Input' }];
  const outputHandles = [
    { id: 'next', label: 'Next' },
    { id: 'error', label: 'Error' },
  ];

  let dataGraphId = $derived(data.config?.dataGraphId ?? 'Not set');
  let inputCount = $derived(Object.keys(data.config?.inputMappings ?? {}).length);
  let outputCount = $derived(Object.keys(data.config?.outputMappings ?? {}).length);
</script>

<OrchestrationBaseNode
  {id}
  label="Data Graph"
  color="#3b82f6"
  {inputHandles}
  {outputHandles}
>
  {#snippet icon()}
    <svg viewBox="0 0 24 24" fill="currentColor">
      <path
        d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z"
      />
    </svg>
  {/snippet}

  {#snippet children()}
    <div class="datagraph-config">
      <div class="graph-id">{dataGraphId}</div>
      <div class="mapping-info">
        <span class="mapping">{inputCount} in</span>
        <span class="separator">/</span>
        <span class="mapping">{outputCount} out</span>
      </div>
    </div>
  {/snippet}
</OrchestrationBaseNode>

<style>
  .datagraph-config {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
  }

  .graph-id {
    color: #3b82f6;
    font-family: monospace;
    background: rgba(59, 130, 246, 0.1);
    padding: 4px 8px;
    border-radius: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 120px;
  }

  .mapping-info {
    display: flex;
    align-items: center;
    gap: 4px;
    color: #666;
  }

  .mapping {
    color: #888;
  }

  .separator {
    color: #444;
  }
</style>

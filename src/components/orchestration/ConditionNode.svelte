<script lang="ts">
  import OrchestrationBaseNode from './OrchestrationBaseNode.svelte';
  import type { ConditionConfig } from '../../stores/orchestrationStore';

  interface Props {
    id: string;
    data: {
      label?: string;
      config?: ConditionConfig;
    };
  }

  let { id, data }: Props = $props();

  const inputHandles = [{ id: 'input', label: 'Input' }];
  const outputHandles = [
    { id: 'true', label: 'True' },
    { id: 'false', label: 'False' },
  ];

  let conditionKey = $derived(data.config?.conditionKey ?? 'condition');
</script>

<OrchestrationBaseNode
  {id}
  label="Condition"
  color="#f59e0b"
  {inputHandles}
  {outputHandles}
>
  {#snippet icon()}
    <svg viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 2L2 12l10 10 10-10L12 2zm0 3.5L19.5 12 12 18.5 4.5 12 12 5.5z" />
    </svg>
  {/snippet}

  {#snippet children()}
    <div class="condition-config">
      <span class="config-label">Check:</span>
      <span class="config-value">{conditionKey}</span>
    </div>
  {/snippet}
</OrchestrationBaseNode>

<style>
  .condition-config {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
  }

  .config-label {
    color: #666;
  }

  .config-value {
    color: #f59e0b;
    font-family: monospace;
    background: rgba(245, 158, 11, 0.1);
    padding: 2px 6px;
    border-radius: 4px;
  }
</style>

<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { nodeExecutionStates, edges } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let errorMessage = $derived(executionInfo?.errorMessage || '');

  let hasTextInput = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'text')
  );

  let statusText = $derived(
    {
      idle: 'Ready',
      running: 'Embedding...',
      success: 'Vector Ready',
      error: 'Error',
    }[executionState]
  );
</script>

<div class="embedding-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'LlamaCpp Embedding'} />
    {/snippet}

      <div class="space-y-2">
        <div class="text-xs text-pg-text-muted">{statusText}</div>
        {#if !hasTextInput}
          <div class="text-[10px] text-pg-text-soft">Connect text input</div>
        {/if}
        {#if executionState === 'error' && errorMessage}
          <div class="p-2 bg-pg-panel-head border border-pg-border-danger rounded text-[10px] text-pg-danger">
            {errorMessage}
          </div>
        {/if}
      </div>
  </BaseNode>
</div>

<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData, edges } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      model_name?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let modelName = $derived(data.model_name || 'llama2');
  const modelNameInputId = $derived(`model-provider-${id}-model-name`);


  // Check if model_name input is connected
  let isModelConnected = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'model_name')
  );

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    updateNodeData(id, { model_name: target.value });
  }
</script>

<div class="model-provider-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'Model Provider'} />
    {/snippet}

      {#if isModelConnected}
        <div class="text-xs text-pg-text-muted italic py-1">
          Model from connection
        </div>
      {:else}
        <div class="flex flex-col gap-1">
          <label class="text-[10px] text-pg-text-muted" for={modelNameInputId}>Model Name</label>
          <input
            id={modelNameInputId}
            type="text"
            class="w-full bg-pg-panel-sunken border border-pg-border rounded px-2 py-1 text-sm text-pg-text"
            placeholder="e.g., llama2, codellama:7b"
            value={modelName}
            oninput={handleInput}
          />
        </div>
      {/if}
  </BaseNode>
</div>

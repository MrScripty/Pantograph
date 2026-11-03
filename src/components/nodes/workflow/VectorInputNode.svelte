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
      vector?: unknown;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let vectorText = $state('[]');
  let parseError = $state('');

  function parseVector(raw: string): number[] | null {
    try {
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return null;
      const values: number[] = [];
      for (const item of parsed) {
        if (typeof item !== 'number' || !Number.isFinite(item)) return null;
        values.push(item);
      }
      return values;
    } catch {
      return null;
    }
  }

  $effect(() => {
    if (typeof data.vector === 'string') {
      vectorText = data.vector;
      return;
    }
    if (Array.isArray(data.vector)) {
      vectorText = JSON.stringify(data.vector);
      return;
    }
    vectorText = '[]';
  });

  let isVectorConnected = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'vector')
  );

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    vectorText = target.value;

    const parsed = parseVector(vectorText);
    if (parsed) {
      parseError = '';
      updateNodeData(id, { vector: parsed });
      return;
    }

    parseError = 'Expected JSON array of numbers';
    updateNodeData(id, { vector: vectorText });
  }
</script>

<div class="vector-input-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'Vector Input'} />
    {/snippet}

      {#if isVectorConnected}
        <div class="text-xs text-pg-text-muted italic py-1">
          Connected to upstream vector
        </div>
      {:else}
        <textarea
          class="w-full bg-pg-panel-sunken border border-pg-border rounded px-2 py-1 text-xs text-pg-text resize-none"
          rows="3"
          placeholder="[0.1, 0.2, 0.3]"
          value={vectorText}
          oninput={handleInput}
        ></textarea>
        {#if parseError}
          <div class="mt-1 text-[10px] text-pg-danger">{parseError}</div>
        {/if}
      {/if}
  </BaseNode>
</div>

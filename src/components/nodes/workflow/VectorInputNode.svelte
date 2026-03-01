<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
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
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-indigo-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9h8M8 15h5M4 6h16v12H4z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Vector Input'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      {#if isVectorConnected}
        <div class="text-xs text-neutral-400 italic py-1">
          Connected to upstream vector
        </div>
      {:else}
        <textarea
          class="w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-xs text-neutral-200 resize-none focus:outline-none focus:border-indigo-500"
          rows="3"
          placeholder="[0.1, 0.2, 0.3]"
          value={vectorText}
          oninput={handleInput}
        ></textarea>
        {#if parseError}
          <div class="mt-1 text-[10px] text-red-300">{parseError}</div>
        {/if}
      {/if}
    {/snippet}
  </BaseNode>
</div>

<style>
  .vector-input-node-wrapper :global(.base-node) {
    border-color: rgba(79, 70, 229, 0.5);
  }

  .vector-input-node-wrapper :global(.node-header) {
    background-color: rgba(79, 70, 229, 0.2);
    border-color: rgba(79, 70, 229, 0.3);
  }
</style>

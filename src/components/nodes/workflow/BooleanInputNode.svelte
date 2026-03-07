<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData, edges, nodes } from '../../../stores/workflowStore';
  import {
    findConnectedTargetPort,
    normalizePortDefaultValue,
    parseBooleanNodeValue,
  } from './primitiveInputMetadata';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      value?: unknown;
    };
    selected?: boolean;
  }

  const NODE_COLOR = '#ef4444';

  let { id, data, selected = false }: Props = $props();

  let selectId = $derived(`boolean-input-${id}-value`);
  let targetPort = $derived.by(() => findConnectedTargetPort(id, 'value', $nodes, $edges));
  let defaultValue = $derived(parseBooleanNodeValue(normalizePortDefaultValue(targetPort?.default_value)));
  let currentValue = $derived(parseBooleanNodeValue(data.value));
  let selectedValue = $state('');

  $effect(() => {
    const nextValue = currentValue === null ? '' : String(currentValue);
    if (nextValue !== selectedValue) {
      selectedValue = nextValue;
    }
  });

  $effect(() => {
    if (data.value !== undefined || defaultValue === null) {
      return;
    }

    updateNodeData(id, { value: defaultValue });
  });

  function handleChange(event: Event) {
    const target = event.currentTarget as HTMLSelectElement | null;
    const nextValue = target?.value ?? '';
    selectedValue = nextValue;
    updateNodeData(id, { value: parseBooleanNodeValue(nextValue) });
  }
</script>

<div class="boolean-input-node-wrapper" style="--node-color: {NODE_COLOR}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {NODE_COLOR}">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h10M7 17h10M7 12h10" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Boolean Input'}</span>
      </div>
    {/snippet}

    <div class="flex flex-col gap-2">
      <div class="flex flex-col gap-1">
        <label class="text-[10px] text-neutral-400" for={selectId}>
          {targetPort?.label || 'Value'}
        </label>
        <select
          id={selectId}
          class="nodrag nopan nowheel w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-sm text-neutral-200 focus:outline-none"
          style="--focus-color: {NODE_COLOR}"
          value={selectedValue}
          onchange={handleChange}
        >
          <option value="">
            {#if defaultValue === null}
              Unset
            {:else}
              Use default ({String(defaultValue)})
            {/if}
          </option>
          <option value="true">True</option>
          <option value="false">False</option>
        </select>
      </div>
      {#if targetPort?.description}
        <div class="text-[10px] text-neutral-500">{targetPort.description}</div>
      {/if}
    </div>
  </BaseNode>
</div>

<style>
  .boolean-input-node-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .boolean-input-node-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }

  .boolean-input-node-wrapper select:focus {
    border-color: var(--focus-color, #ef4444);
  }
</style>

<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
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

<div class="boolean-input-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'Boolean Input'} />
    {/snippet}

    <div class="flex flex-col gap-2">
      <div class="flex flex-col gap-1">
        <label class="text-[10px] text-pg-text-muted" for={selectId}>
          {targetPort?.label || 'Value'}
        </label>
        <select
          id={selectId}
          class="nodrag nopan nowheel w-full bg-pg-panel-sunken border border-pg-border rounded px-2 py-1 text-sm text-pg-text"
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
        <div class="text-[10px] text-pg-text-muted">{targetPort.description}</div>
      {/if}
    </div>
  </BaseNode>
</div>

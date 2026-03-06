<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, PortDefinition } from '../../../services/workflow/types';
  import { updateNodeData, edges, nodes } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      value?: unknown;
    };
    selected?: boolean;
  }

  interface SelectionOption {
    label: string;
    value: unknown;
  }

  let { id, data, selected = false }: Props = $props();

  const nodeColor = '#2563eb';
  const selectId = $derived(`selection-input-${id}-value`);

  function getTargetPort(): PortDefinition | null {
    const edge = $edges.find((candidate) => candidate.source === id && candidate.sourceHandle === 'value');
    if (!edge) return null;

    const targetNode = $nodes.find((node) => node.id === edge.target);
    if (!targetNode?.data?.definition) return null;

    const definition = targetNode.data.definition as NodeDefinition;
    return definition.inputs.find((port) => port.id === edge.targetHandle) ?? null;
  }

  function normalizeOption(option: unknown): SelectionOption | null {
    if (typeof option === 'string' || typeof option === 'number' || typeof option === 'boolean') {
      return { label: String(option), value: option };
    }

    if (!option || typeof option !== 'object') return null;

    const record = option as Record<string, unknown>;
    const value = record.value ?? record.id ?? record.key ?? record.name;
    if (value === undefined) return null;

    const labelSource = record.label ?? record.name ?? record.value ?? value;
    return {
      label: String(labelSource),
      value,
    };
  }

  function normalizeDefaultValue(value: unknown): unknown {
    if (!value || typeof value !== 'object') return value;
    const record = value as Record<string, unknown>;
    return record.value ?? value;
  }

  let targetPort = $derived.by(() => getTargetPort());
  let options = $derived.by(() => {
    const allowedValues = targetPort?.constraints?.allowed_values;
    if (!Array.isArray(allowedValues)) return [];

    return allowedValues
      .map(normalizeOption)
      .filter((option): option is SelectionOption => option !== null);
  });
  let defaultValue = $derived(normalizeDefaultValue(targetPort?.default_value));
  let hasTarget = $derived(Boolean(targetPort));
  let hasOptions = $derived(options.length > 0);
  let selectedString = $derived.by(() => {
    if (data.value === null || data.value === undefined) return '';
    return JSON.stringify(data.value);
  });

  $effect(() => {
    if (!hasOptions) {
      return;
    }

    const optionValues = options.map((option) => option.value);
    const currentValue = data.value;
    const hasCurrent = optionValues.some((value) => JSON.stringify(value) === JSON.stringify(currentValue));

    if (hasCurrent) {
      return;
    }

    const nextValue = optionValues.some(
      (value) => JSON.stringify(value) === JSON.stringify(defaultValue)
    )
      ? defaultValue
      : options[0]?.value;

    updateNodeData(id, { value: nextValue ?? null });
  });

  function handleChange(event: Event) {
    const target = event.currentTarget as HTMLSelectElement | null;
    const nextValue = options.find((option) => JSON.stringify(option.value) === target?.value)?.value ?? null;
    updateNodeData(id, { value: nextValue });
  }
</script>

<div class="selection-input-node-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {nodeColor}">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l4-4 4 4m0 6l-4 4-4-4" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Selection Input'}</span>
      </div>
    {/snippet}

    <div class="flex flex-col gap-2">
      {#if !hasTarget}
        <div class="text-xs text-neutral-500 italic">
          Connect this node to an enum-constrained input
        </div>
      {:else if !hasOptions}
        <div class="text-xs text-neutral-500 italic">
          Target input does not expose selectable options
        </div>
      {:else}
        <div class="flex flex-col gap-1">
          <label class="text-[10px] text-neutral-400" for={selectId}>
            {targetPort?.label || 'Value'}
          </label>
          <select
            id={selectId}
            class="nodrag nopan nowheel w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-sm text-neutral-200 focus:outline-none"
            style="--focus-color: {nodeColor}"
            value={selectedString}
            onchange={handleChange}
          >
            {#each options as option (JSON.stringify(option.value))}
              <option value={JSON.stringify(option.value)}>{option.label}</option>
            {/each}
          </select>
        </div>
      {/if}
    </div>
  </BaseNode>
</div>

<style>
  .selection-input-node-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .selection-input-node-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }

  .selection-input-node-wrapper select:focus {
    border-color: var(--focus-color, #2563eb);
  }
 </style>

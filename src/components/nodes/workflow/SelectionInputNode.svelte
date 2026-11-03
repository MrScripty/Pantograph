<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
  import type { NodeDefinition, PortDefinition } from '../../../services/workflow/types';
  import { updateNodeData, edges, nodes } from '../../../stores/workflowStore';
  import {
    buildSelectionInputControlModel,
    buildSelectionInputState,
    resolveSelectionAutoUpdate,
    stringifySelectionValue,
    type SelectionInputOption,
  } from './selectionInputState';
  import {
    buildSelectionInputProviderQuery,
    loadLatestSelectionInputProviderOptions,
    type SelectionInputTargetNode,
  } from './selectionInputProviderOptions';

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

  const selectId = $derived(`selection-input-${id}-value`);

  interface SelectionTarget {
    node: SelectionInputTargetNode;
    port: PortDefinition;
  }

  function getTarget(): SelectionTarget | null {
    const edge = $edges.find((candidate) => candidate.source === id && candidate.sourceHandle === 'value');
    if (!edge) return null;

    const targetNode = $nodes.find((node) => node.id === edge.target);
    if (!targetNode?.data?.definition) return null;

    const definition = targetNode.data.definition as NodeDefinition;
    const port = definition.inputs.find((candidate) => candidate.id === edge.targetHandle);
    if (!port) return null;

    return {
      node: targetNode as SelectionInputTargetNode,
      port,
    };
  }

  function normalizeOption(option: unknown): SelectionInputOption | null {
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

  let target = $derived.by(() => getTarget());
  let targetPort = $derived(target?.port ?? null);
  let providerQuery = $derived.by(() => buildSelectionInputProviderQuery(target?.node ?? null, targetPort));
  let providerOptions = $state<SelectionInputOption[]>([]);
  let providerOptionsLoading = $state(false);
  let providerOptionsError = $state<string | null>(null);
  let options = $derived.by(() => {
    if (targetPort?.options_provider) return providerOptions;

    const allowedValues = targetPort?.constraints?.allowed_values;
    if (!Array.isArray(allowedValues)) return [];

    return allowedValues
      .map(normalizeOption)
      .filter((option): option is SelectionInputOption => option !== null);
  });
  let defaultValue = $derived(normalizeDefaultValue(targetPort?.default_value));
  let hasTarget = $derived(Boolean(targetPort));
  let isProviderBacked = $derived(Boolean(targetPort?.options_provider));
  let hasOptions = $derived(isProviderBacked || options.length > 0);
  let selectionState = $derived(buildSelectionInputState(targetPort, options, data.value));
  let controlModel = $derived(buildSelectionInputControlModel(targetPort, options));

  $effect(() => {
    const query = providerQuery;
    if (!query) {
      providerOptions = [];
      providerOptionsLoading = false;
      providerOptionsError = null;
      return;
    }

    providerOptionsLoading = true;
    providerOptionsError = null;
    let active = true;
    loadLatestSelectionInputProviderOptions(query, () =>
      active ? providerQuery?.requestKey ?? null : null,
    )
      .then((result) => {
        if (!active || result.status === 'stale') return;
        providerOptions = result.options;
        providerOptionsLoading = false;
      })
      .catch((error: unknown) => {
        if (!active || providerQuery?.requestKey !== query.requestKey) return;
        providerOptions = [];
        providerOptionsLoading = false;
        providerOptionsError = error instanceof Error ? error.message : 'Unable to load options';
      });

    return () => {
      active = false;
    };
  });

  $effect(() => {
    const update = resolveSelectionAutoUpdate(targetPort, options, data.value, defaultValue);
    if (update.shouldUpdate) {
      updateNodeData(id, { value: update.value ?? null });
    }
  });

  function handleChange(event: Event) {
    const target = event.currentTarget as HTMLSelectElement | null;
    const nextValue = options.find(
      (option) => !option.disabled && stringifySelectionValue(option.value) === target?.value,
    )?.value ?? null;
    updateNodeData(id, { value: nextValue });
  }
</script>

<div class="selection-input-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'Selection Input'} />
    {/snippet}

    <div class="flex flex-col gap-2">
      {#if !hasTarget}
        <div class="text-xs text-pg-text-muted italic">
          Connect this node to an enum-constrained input
        </div>
      {:else if providerOptionsLoading}
        <div class="text-xs text-pg-text-muted italic" role="status">
          Loading options
        </div>
      {:else if providerOptionsError}
        <div class="text-xs text-pg-danger" role="status">
          Options unavailable
        </div>
      {:else if !hasOptions}
        <div class="text-xs text-pg-text-muted italic">
          Target input does not expose selectable options
        </div>
      {:else}
        <div class="flex flex-col gap-1">
          <label class="text-[10px] text-pg-text-muted" for={selectId}>
            {controlModel.accessibleName}
          </label>
          <select
            id={selectId}
            class="{controlModel.graphGestureClass} w-full bg-pg-panel-sunken border border-pg-border rounded px-2 py-1 text-sm text-pg-text"
            value={selectionState.displayValue}
            onchange={handleChange}
            disabled={controlModel.disabled}
          >
            {#if selectionState.placeholderLabel}
              <option value="" disabled>{selectionState.placeholderLabel}</option>
            {/if}
            {#each options as option (stringifySelectionValue(option.value))}
              <option
                value={stringifySelectionValue(option.value)}
                disabled={option.disabled}
                title={option.unavailableReason}
              >
                {option.label}
              </option>
            {/each}
          </select>
        </div>
      {/if}
    </div>
  </BaseNode>
</div>

<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData, edges, nodes } from '../../../stores/workflowStore';
  import {
    findConnectedTargetPort,
    normalizePortDefaultValue,
    DESKTOP_SEED_MAX,
    hasInferenceSeedTarget,
    numberInputState,
    numberInputPersistedValue,
    numberInputDisplayValue,
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

  const DEFAULT_PLACEHOLDER = 'Enter number...';

  let { id, data, selected = false }: Props = $props();

  let inputId = $derived(`number-input-${id}-value`);
  let errorId = $derived(`${inputId}-error`);
  let seedInput = $derived(hasInferenceSeedTarget(id, $nodes, $edges));
  let targetPort = $derived.by(() => findConnectedTargetPort(id, 'value', $nodes, $edges));
  let defaultValue = $derived(numberInputState(normalizePortDefaultValue(targetPort?.default_value), seedInput).value);
  let currentState = $derived(numberInputState(data.value, seedInput));
  let inputValue = $state('');

  $effect(() => {
    const nextText = numberInputDisplayValue(data.value, seedInput);
    if (nextText !== inputValue) {
      inputValue = nextText;
    }
  });

  $effect(() => {
    if (data.value !== undefined || defaultValue === null) {
      return;
    }

    updateNodeData(id, { value: defaultValue });
  });

  function handleInput(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    const nextValue = target?.value ?? '';
    inputValue = nextValue;

    updateNodeData(id, { value: numberInputPersistedValue(nextValue, seedInput) });
  }
</script>

<div class="number-input-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'Number Input'} />
    {/snippet}

    <div class="flex flex-col gap-2">
      <div class="flex flex-col gap-1">
        <label class="text-[10px] text-pg-text-muted" for={inputId}>
          {targetPort?.label || 'Value'}
        </label>
        <input
          id={inputId}
          class="nodrag nopan nowheel w-full bg-pg-panel-sunken border border-pg-border rounded px-2 py-1 text-sm text-pg-text"
          type={seedInput ? 'text' : 'number'}
          inputmode={seedInput ? 'numeric' : 'decimal'}
          min={seedInput ? 0 : targetPort?.constraints?.min}
          max={seedInput ? DESKTOP_SEED_MAX : targetPort?.constraints?.max}
          step={seedInput ? 1 : 'any'}
          aria-invalid={currentState.error !== null ? true : undefined}
          aria-describedby={currentState.error !== null ? errorId : undefined}
          placeholder={DEFAULT_PLACEHOLDER}
          value={inputValue}
          oninput={handleInput}
        />
        {#if currentState.error !== null}
          <p id={errorId} class="text-[10px] text-pg-danger" role="alert">{currentState.error}</p>
        {/if}
      </div>
      {#if targetPort?.description}
        <div class="text-[10px] text-pg-text-muted">{targetPort.description}</div>
      {/if}
      {#if targetPort?.constraints?.min !== undefined || targetPort?.constraints?.max !== undefined}
        <div class="text-[10px] text-pg-text-muted">
          {#if targetPort?.constraints?.min !== undefined}min {targetPort.constraints.min}{/if}
          {#if targetPort?.constraints?.min !== undefined && targetPort?.constraints?.max !== undefined} · {/if}
          {#if targetPort?.constraints?.max !== undefined}max {targetPort.constraints.max}{/if}
        </div>
      {/if}
    </div>
  </BaseNode>
</div>

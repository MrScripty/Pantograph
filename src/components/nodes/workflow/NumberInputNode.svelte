<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
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

  const NODE_COLOR = '#f59e0b';
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

<div class="number-input-node-wrapper" style="--node-color: {NODE_COLOR}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {NODE_COLOR}">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 7h6m-7 5h8m-9 5h10" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Number Input'}</span>
      </div>
    {/snippet}

    <div class="flex flex-col gap-2">
      <div class="flex flex-col gap-1">
        <label class="text-[10px] text-neutral-400" for={inputId}>
          {targetPort?.label || 'Value'}
        </label>
        <input
          id={inputId}
          class="nodrag nopan nowheel w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-sm text-neutral-200 focus:outline-none"
          style="--focus-color: {NODE_COLOR}"
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
          <p id={errorId} class="text-[10px] text-red-400" role="alert">{currentState.error}</p>
        {/if}
      </div>
      {#if targetPort?.description}
        <div class="text-[10px] text-neutral-500">{targetPort.description}</div>
      {/if}
      {#if targetPort?.constraints?.min !== undefined || targetPort?.constraints?.max !== undefined}
        <div class="text-[10px] text-neutral-500">
          {#if targetPort?.constraints?.min !== undefined}min {targetPort.constraints.min}{/if}
          {#if targetPort?.constraints?.min !== undefined && targetPort?.constraints?.max !== undefined} · {/if}
          {#if targetPort?.constraints?.max !== undefined}max {targetPort.constraints.max}{/if}
        </div>
      {/if}
    </div>
  </BaseNode>
</div>

<style>
  .number-input-node-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .number-input-node-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }

  .number-input-node-wrapper input:focus {
    border-color: var(--focus-color, #f59e0b);
  }
</style>

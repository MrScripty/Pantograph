<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface ParamSchema {
    key: string;
    label: string;
    param_type: string;
    default: unknown;
    description?: string;
    constraints?: { min?: number; max?: number; allowed_values?: unknown[] };
  }

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      inference_settings?: ParamSchema[];
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');

  // Parse the inference settings schema from the upstream connection
  let settings = $derived(
    Array.isArray(data.inference_settings) ? data.inference_settings as ParamSchema[] : []
  );

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-green-500 animate-pulse',
      success: 'bg-green-500',
      error: 'bg-red-500',
    }[executionState]
  );

  function formatValue(value: unknown): string {
    if (value === null || value === undefined) return '—';
    if (typeof value === 'number') return String(value);
    if (typeof value === 'boolean') return value ? 'true' : 'false';
    if (typeof value === 'string') return value || '—';
    return JSON.stringify(value);
  }

  function formatConstraints(param: ParamSchema): string {
    if (!param.constraints) return '';
    const parts: string[] = [];
    if (param.constraints.min !== undefined) parts.push(`min: ${param.constraints.min}`);
    if (param.constraints.max !== undefined) parts.push(`max: ${param.constraints.max}`);
    return parts.join(', ');
  }
</script>

<div class="expand-settings-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-green-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Expand Settings'}</span>
        <span class="w-2 h-2 rounded-full {statusColor} ml-auto"></span>
      </div>
    {/snippet}

      {#if settings.length > 0}
        <div class="settings-list">
          {#each settings as param (param.key)}
            <div class="setting-row" title={param.description || ''}>
              <span class="setting-label">{param.label}</span>
              <span class="setting-value">{formatValue(param.default)}</span>
            </div>
            {#if formatConstraints(param)}
              <div class="setting-constraint">{formatConstraints(param)}</div>
            {/if}
          {/each}
        </div>
      {:else}
        <div class="text-xs text-neutral-500 italic">
          Connect settings schema to expose override ports
        </div>
      {/if}
  </BaseNode>
</div>

<style>
  .expand-settings-wrapper :global(.base-node) {
    border-color: rgba(22, 163, 74, 0.5);
  }

  .expand-settings-wrapper :global(.node-header) {
    background-color: rgba(22, 163, 74, 0.2);
    border-color: rgba(22, 163, 74, 0.3);
  }

  .settings-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
  }

  .setting-label {
    font-size: 0.675rem;
    color: #a3a3a3;
    white-space: nowrap;
  }

  .setting-value {
    font-size: 0.675rem;
    color: #d4d4d4;
    font-family: monospace;
    text-align: right;
  }

  .setting-constraint {
    font-size: 0.6rem;
    color: #737373;
    text-align: right;
    margin-top: -2px;
    margin-bottom: 2px;
  }
</style>

<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { edges, nodeExecutionStates, nodes, updateNodeData } from '../../../stores/workflowStore';
  import {
    parseRuntimeSettingInput,
    resolveEffectiveSettingValue,
    runtimeSettingDraftValue,
    runtimeSettingOptions,
  } from './expandSettingsDisplay';

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

  function getEffectiveValue(param: ParamSchema): unknown {
    return resolveEffectiveSettingValue(id, data, param, $nodes, $edges);
  }

  function isSettingConnected(param: ParamSchema): boolean {
    return $edges.some((edge) => edge.target === id && edge.targetHandle === param.key);
  }

  function updateSetting(param: ParamSchema, value: string | boolean): void {
    if (isSettingConnected(param)) return;
    const parsed = parseRuntimeSettingInput(param, value);
    void updateNodeData(id, { [param.key]: parsed });
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
            {@const options = runtimeSettingOptions(param)}
            {@const connected = isSettingConnected(param)}
            <div class="setting-row" title={param.description || ''}>
              <span class="setting-label">{param.label}</span>
              <span class="setting-value">{formatValue(getEffectiveValue(param))}</span>
            </div>
            <div class="setting-control-row">
              {#if options.length > 0}
                <select
                  class="setting-control"
                  disabled={connected}
                  value={runtimeSettingDraftValue(getEffectiveValue(param))}
                  title={param.description || param.label}
                  onchange={(event) => updateSetting(param, event.currentTarget.value)}
                >
                  {#each options as option}
                    <option value={runtimeSettingDraftValue(option.value)}>{option.label}</option>
                  {/each}
                </select>
              {:else if param.param_type === 'Boolean'}
                <label class="setting-toggle" title={param.description || param.label}>
                  <input
                    type="checkbox"
                    disabled={connected}
                    checked={Boolean(getEffectiveValue(param))}
                    onchange={(event) => updateSetting(param, event.currentTarget.checked)}
                  />
                  <span>{Boolean(getEffectiveValue(param)) ? 'true' : 'false'}</span>
                </label>
              {:else}
                <input
                  class="setting-control"
                  type={param.param_type === 'Number' || param.param_type === 'Integer' ? 'number' : 'text'}
                  step={param.param_type === 'Integer' ? '1' : 'any'}
                  min={param.constraints?.min}
                  max={param.constraints?.max}
                  disabled={connected}
                  value={runtimeSettingDraftValue(getEffectiveValue(param))}
                  title={param.description || param.label}
                  onchange={(event) => updateSetting(param, event.currentTarget.value)}
                />
              {/if}
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

  .setting-control-row {
    display: flex;
    justify-content: flex-end;
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

  .setting-control {
    width: 100%;
    max-width: 8.5rem;
    height: 1.5rem;
    border: 1px solid #3f3f46;
    border-radius: 4px;
    background: #18181b;
    color: #e5e5e5;
    font-size: 0.675rem;
    padding: 0 0.35rem;
  }

  .setting-control:disabled {
    color: #737373;
    background: #111113;
  }

  .setting-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    color: #d4d4d4;
    font-size: 0.675rem;
  }

  .setting-toggle input {
    width: 0.8rem;
    height: 0.8rem;
  }
</style>

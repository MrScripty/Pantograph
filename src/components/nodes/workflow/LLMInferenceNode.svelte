<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type {
    InferenceInterfaceDriftReport,
    InferenceInterfaceUpdateProposal,
    NodeDefinition,
    WorkflowGraphValidationSummary,
  } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';
  import { buildInferencePayloadDisplay } from './inferencePayloadDisplay';
  import {
    buildInferenceDriftDisplay,
    buildInferenceValidationDisplay,
  } from './inferenceValidationDisplay';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      inference_interface_drift_report?: InferenceInterfaceDriftReport | null;
      inference_interface_update_proposal?: InferenceInterfaceUpdateProposal | null;
      inference_interface_validation_summary?: WorkflowGraphValidationSummary | null;
      label?: string;
      modelName?: string;
      streamContent?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  // Get execution info (new format with state and errorMessage)
  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let modelName = $derived(data.modelName || 'Local LLM');
  let streamContent = $derived(data.streamContent || '');
  let inferenceDisplay = $derived(buildInferencePayloadDisplay(data.definition));
  let validationDisplay = $derived(
    buildInferenceValidationDisplay(data.inference_interface_validation_summary),
  );
  let driftDisplay = $derived(
    buildInferenceDriftDisplay(
      data.inference_interface_drift_report,
      data.inference_interface_update_proposal,
    ),
  );

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-green-500 animate-pulse',
      success: 'bg-green-500',
      error: 'bg-red-500',
    }[executionState]
  );

  let statusText = $derived(
    {
      idle: 'Idle',
      running: 'Running...',
      success: 'Complete',
      error: 'Error',
    }[executionState]
  );
</script>

<div class="llm-node-wrapper border-green-600/50">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-green-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
          </svg>
        </div>
        <div class="flex-1 min-w-0">
          <span class="text-sm font-medium text-neutral-200">{data.label || 'LLM Inference'}</span>
        </div>
      </div>
    {/snippet}

      <div class="space-y-2">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full {statusColor}"></span>
          <span class="text-xs text-neutral-400">{statusText}</span>
        </div>
        <div class="flex justify-between items-center text-xs">
          <span class="text-neutral-400">Model:</span>
          <span class="text-neutral-200 font-mono text-[10px]">{modelName}</span>
        </div>
        {#if inferenceDisplay}
          <div class="space-y-1 border-t border-neutral-800 pt-2">
            {#if inferenceDisplay.tasks.length > 0}
              <div class="flex flex-wrap gap-1">
                {#each inferenceDisplay.tasks as task (task)}
                  <span class="rounded bg-neutral-800 px-1.5 py-0.5 text-[10px] text-neutral-300">
                    {task}
                  </span>
                {/each}
              </div>
            {/if}
            {#each inferenceDisplay.rows as row (row.label)}
              <div class="flex justify-between gap-2 text-[10px]">
                <span class="text-neutral-500">{row.label}:</span>
                <span class="truncate text-right text-neutral-300">{row.value}</span>
              </div>
            {/each}
          </div>
        {/if}
        {#if validationDisplay}
          <div
            class="inference-validation inference-validation--{validationDisplay.tone}"
            title={validationDisplay.detail ?? validationDisplay.label}
          >
            <span class="truncate">{validationDisplay.label}</span>
            {#if validationDisplay.detail}
              <span class="shrink-0">{validationDisplay.detail}</span>
            {/if}
          </div>
        {/if}
        {#if driftDisplay}
          <div
            class="inference-validation inference-validation--{driftDisplay.tone}"
            title={driftDisplay.detail ?? driftDisplay.label}
          >
            <span class="truncate">{driftDisplay.label}</span>
            {#if driftDisplay.detail}
              <span class="shrink-0">{driftDisplay.detail}</span>
            {/if}
          </div>
        {/if}
        {#if streamContent}
          <div class="p-2 bg-neutral-900 rounded text-xs text-neutral-300 max-h-20 overflow-y-auto">
            {streamContent}
          </div>
        {/if}
      </div>
  </BaseNode>
</div>

<style>
  .llm-node-wrapper :global(.base-node) {
    border-color: rgba(22, 163, 74, 0.5);
  }

  .llm-node-wrapper :global(.node-header) {
    background-color: rgba(22, 163, 74, 0.2);
    border-color: rgba(22, 163, 74, 0.3);
  }

  .inference-validation {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    border-radius: 0.25rem;
    border: 1px solid #404040;
    padding: 0.25rem 0.375rem;
    font-size: 0.625rem;
    line-height: 0.875rem;
  }

  .inference-validation--info {
    border-color: #1d4ed8;
    background: rgba(30, 64, 175, 0.24);
    color: #bfdbfe;
  }

  .inference-validation--warning {
    border-color: #a16207;
    background: rgba(113, 63, 18, 0.24);
    color: #fde68a;
  }

  .inference-validation--error {
    border-color: #991b1b;
    background: rgba(127, 29, 29, 0.28);
    color: #fecaca;
  }

  .inference-validation--success {
    border-color: #166534;
    background: rgba(20, 83, 45, 0.24);
    color: #bbf7d0;
  }
</style>

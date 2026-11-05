<script lang="ts">
  import { getContext, hasContext } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
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
    buildInferenceUpdateApplyDisplay,
    buildInferenceUpdatePreviewDisplay,
    buildInferenceValidationDisplay,
  } from './inferenceValidationDisplay';
  import {
    INFERENCE_INTERFACE_UPDATE_COORDINATOR_CONTEXT,
    type InferenceInterfaceUpdateCoordinator,
  } from '../../inferenceInterfaceUpdateContext.ts';

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

  const inferenceUpdateCoordinator = hasContext(INFERENCE_INTERFACE_UPDATE_COORDINATOR_CONTEXT)
    ? getContext<InferenceInterfaceUpdateCoordinator>(
        INFERENCE_INTERFACE_UPDATE_COORDINATOR_CONTEXT
      )
    : null;

  let updateApplyBusy = $state(false);
  let updateApplyError = $state<string | null>(null);

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
  let updateApplyDisplay = $derived(
    buildInferenceUpdateApplyDisplay(data.inference_interface_update_proposal),
  );
  let updatePreviewDisplay = $derived(
    buildInferenceUpdatePreviewDisplay(
      data.inference_interface_drift_report,
      data.inference_interface_update_proposal,
    ),
  );


  let statusText = $derived(
    {
      idle: 'Idle',
      running: 'Running...',
      success: 'Complete',
      error: 'Error',
    }[executionState]
  );

  async function applyInferenceInterfaceUpdate(event: MouseEvent): Promise<void> {
    event.stopPropagation();
    if (
      !inferenceUpdateCoordinator ||
      !updateApplyDisplay?.enabled ||
      updateApplyBusy ||
      !data.inference_interface_update_proposal
    ) {
      return;
    }

    updateApplyBusy = true;
    updateApplyError = null;
    try {
      await inferenceUpdateCoordinator({
        proposal: data.inference_interface_update_proposal,
      });
    } catch (error) {
      updateApplyError = error instanceof Error ? error.message : String(error);
    } finally {
      updateApplyBusy = false;
    }
  }
</script>

<div class="llm-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'LLM Inference'} />
    {/snippet}

      <div class="space-y-2">
        <div class="text-xs text-pg-text-muted">{statusText}</div>
        <div class="flex justify-between items-center text-xs">
          <span class="text-pg-text-muted">Model:</span>
          <span class="text-pg-text font-mono text-[10px]">{modelName}</span>
        </div>
        {#if inferenceDisplay}
          <div class="space-y-1 border-t border-pg-border-soft pt-2">
            {#if inferenceDisplay.tasks.length > 0}
              <div class="flex flex-wrap gap-1">
                {#each inferenceDisplay.tasks as task (task)}
                  <span class="rounded bg-pg-panel-head px-1.5 py-0.5 text-[10px] text-pg-text-soft">
                    {task}
                  </span>
                {/each}
              </div>
            {/if}
            {#each inferenceDisplay.rows as row (row.label)}
              <div class="flex justify-between gap-2 text-[10px]">
                <span class="text-pg-text-muted">{row.label}:</span>
                <span class="truncate text-right text-pg-text-soft">{row.value}</span>
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
          {#if updateApplyDisplay}
            <div class="inference-update-action">
              <button
                type="button"
                class="inference-update-button"
                disabled={!updateApplyDisplay.enabled || updateApplyBusy || !inferenceUpdateCoordinator}
                title={updateApplyDisplay.detail ?? updateApplyDisplay.label}
                onclick={applyInferenceInterfaceUpdate}
              >
                {updateApplyBusy ? 'Applying' : updateApplyDisplay.label}
              </button>
              {#if updateApplyDisplay.detail}
                <span class="truncate">{updateApplyDisplay.detail}</span>
              {/if}
            </div>
          {/if}
          {#if updatePreviewDisplay}
            <div class="inference-update-preview">
              <div class="inference-update-preview__title">{updatePreviewDisplay.title}</div>
              {#if updatePreviewDisplay.operationSummary}
                <div class="truncate">{updatePreviewDisplay.operationSummary}</div>
              {/if}
              {#each updatePreviewDisplay.rows as row, index (`${index}:${row}`)}
                <div class="truncate" title={row}>{row}</div>
              {/each}
              {#if updatePreviewDisplay.extraCount > 0}
                <div>{updatePreviewDisplay.extraCount} more</div>
              {/if}
            </div>
          {/if}
          {#if updateApplyError}
            <div class="inference-validation inference-validation--error" title={updateApplyError}>
              <span class="truncate">{updateApplyError}</span>
            </div>
          {/if}
        {/if}
        {#if streamContent}
          <div class="p-2 bg-pg-panel-sunken rounded text-xs text-pg-text-soft max-h-20 overflow-y-auto">
            {streamContent}
          </div>
        {/if}
      </div>
  </BaseNode>
</div>

<style>
  .inference-validation {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    border-radius: 0.25rem;
    border: 1px solid var(--pg-border);
    padding: 0.25rem 0.375rem;
    font-size: 0.625rem;
    line-height: 0.875rem;
  }

  .inference-validation--info {
    background: var(--pg-panel-head);
    color: var(--pg-text-soft);
  }

  .inference-validation--warning {
    border-color: var(--pg-accent);
    background: var(--pg-panel-head);
    color: var(--pg-text);
  }

  .inference-validation--error {
    border-color: var(--pg-border-danger);
    background: var(--pg-panel-head);
    color: var(--pg-danger);
  }

  .inference-validation--success {
    background: var(--pg-panel-head);
    color: var(--pg-ok);
  }

  .inference-update-action {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.625rem;
    line-height: 0.875rem;
    color: var(--pg-text-soft);
  }

  .inference-update-button {
    border-radius: 0.25rem;
    border: 1px solid var(--pg-border);
    background: var(--pg-panel-raised);
    padding: 0.125rem 0.375rem;
    color: var(--pg-text);
  }

  .inference-update-button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .inference-update-preview {
    display: grid;
    gap: 0.125rem;
    border-left: 2px solid var(--pg-border);
    padding-left: 0.375rem;
    font-size: 0.625rem;
    line-height: 0.875rem;
    color: var(--pg-text-muted);
  }

  .inference-update-preview__title {
    color: var(--pg-text);
    font-weight: 500;
  }
</style>

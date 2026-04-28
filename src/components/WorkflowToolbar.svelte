<script lang="ts">
  import { Loader2, Send } from 'lucide-svelte';
  import {
    isDirty,
    isExecuting,
    resetExecutionStates,
    clearNodeRuntimeData,
    clearStreamContent,
  } from '../stores/workflowStore';
  import {
    availableWorkflows,
    currentGraphId,
    currentGraphType,
    isReadOnly,
  } from '../stores/graphSessionStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import {
    AUDIO_RUNTIME_DATA_KEYS,
  } from './nodes/workflow/audioOutputState';
  import {
    selectActiveWorkflowRun,
    setWorkbenchPage,
  } from '../stores/workbenchStore';
  import WorkflowPersistenceControls from './WorkflowPersistenceControls.svelte';

  const DEFAULT_WORKFLOW_SEMANTIC_VERSION = '0.1.0';

  let workflowError = $state<string | null>(null);

  let currentSavedWorkflow = $derived(
    $currentGraphType === 'workflow'
      ? $availableWorkflows.find((workflow) => (workflow.id ?? workflow.name) === $currentGraphId)
      : undefined,
  );
  let submitDisabled = $derived(
    $isExecuting || $isReadOnly || $isDirty || !currentSavedWorkflow || !$currentGraphId,
  );
  let submitTitle = $derived(submitButtonTitle());

  function normalizeError(error: unknown): string {
    if (error instanceof Error && error.message.trim().length > 0) {
      return error.message;
    }
    if (typeof error === 'string' && error.trim().length > 0) {
      return error;
    }
    return String(error);
  }

  function submitButtonTitle(): string {
    if ($isReadOnly) return 'Cannot submit a read-only graph';
    if ($isDirty) return 'Save workflow changes before submitting';
    if (!currentSavedWorkflow || !$currentGraphId) return 'Save the workflow before submitting';
    if ($isExecuting) return 'Workflow submission is in progress';
    return 'Submit workflow to the scheduler';
  }

  async function closeExecutionSession(sessionId: string): Promise<void> {
    try {
      await workflowService.closeWorkflowExecutionSession({ session_id: sessionId });
    } catch (error) {
      console.warn(`Failed to close execution session "${sessionId}":`, error);
    }
  }

  async function handleSubmit() {
    if ($isExecuting) return;

    workflowError = null;
    isExecuting.set(true);
    clearNodeRuntimeData([...AUDIO_RUNTIME_DATA_KEYS]);
    resetExecutionStates();
    clearStreamContent();

    try {
      if ($isReadOnly) {
        throw new Error('Read-only graphs cannot be submitted');
      }
      if ($isDirty) {
        throw new Error('Save workflow changes before submitting');
      }
      if (!currentSavedWorkflow || !$currentGraphId) {
        throw new Error('Save the workflow before submitting');
      }

      const executionSession = await workflowService.createWorkflowExecutionSession({
        workflow_id: $currentGraphId,
        usage_profile: null,
        keep_alive: false,
      });
      const runPromise = workflowService.runWorkflowExecutionSession({
        session_id: executionSession.session_id,
        workflow_semantic_version: DEFAULT_WORKFLOW_SEMANTIC_VERSION,
        inputs: [],
        output_targets: null,
        override_selection: null,
        timeout_ms: null,
        priority: null,
      });

      try {
        const response = await runPromise;
        selectActiveWorkflowRun({
          workflow_run_id: response.workflow_run_id,
          workflow_id: $currentGraphId,
          workflow_semantic_version: DEFAULT_WORKFLOW_SEMANTIC_VERSION,
          status: 'completed',
        });
        setWorkbenchPage('scheduler');
      } finally {
        await closeExecutionSession(executionSession.session_id);
      }
    } catch (error) {
      console.error('Workflow submission failed:', error);
      workflowError = normalizeError(error);
    } finally {
      isExecuting.set(false);
    }
  }

</script>

<div>
  <div class="workflow-toolbar h-12 px-4 bg-neutral-900 border-b border-neutral-700 flex items-center justify-between">
    <WorkflowPersistenceControls />

    <div class="flex items-center gap-2">
      {#if $isReadOnly}
        <span class="text-xs text-neutral-500 bg-neutral-800 px-2 py-0.5 rounded">(read-only)</span>
      {/if}
      {#if $isDirty && !$isReadOnly}
        <span class="text-amber-400 text-sm" title="Unsaved changes">*</span>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      <button type="button"
        class="px-4 py-1.5 text-sm rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        class:bg-green-600={!$isExecuting}
        class:hover:bg-green-500={!$isExecuting}
        class:bg-amber-600={$isExecuting}
        class:text-white={true}
        onclick={handleSubmit}
        disabled={submitDisabled}
        title={submitTitle}
      >
        {#if $isExecuting}
          <Loader2 size={14} aria-hidden="true" class="inline-block align-[-2px] mr-1" />
          Submitting...
        {:else}
          <Send size={14} aria-hidden="true" class="inline-block align-[-2px] mr-1" />
          Submit
        {/if}
      </button>
    </div>
  </div>

  {#if workflowError}
    <div class="px-4 py-2 bg-red-900/70 border-b border-red-700 text-red-200 text-xs truncate" title={workflowError}>
      Workflow submit failed: {workflowError}
    </div>
  {/if}
</div>

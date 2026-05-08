<script lang="ts">
  import { AlertTriangle, Loader2, Send } from 'lucide-svelte';
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
    normalizeWorkflowServiceError,
    type WorkflowServiceError,
  } from '../services/workflow/workflowServiceErrors';
  import {
    AUDIO_RUNTIME_DATA_KEYS,
  } from './nodes/workflow/audioOutputState';
  import {
    focusWorkflowDiagnostics,
    selectActiveWorkflowRun,
    setWorkbenchPage,
  } from '../stores/workbenchStore';
  import { formatWorkflowCommandError } from './workbench/workflowErrorPresenters';
  import WorkflowPersistenceControls from './WorkflowPersistenceControls.svelte';
  import {
    isCurrentWorkflowSubmitFailure,
    isNumericWorkflowSemanticVersion,
    isWorkflowSemanticVersionConflictError,
    nextWorkflowPatchSemanticVersion,
    workflowSubmitDisabledReason,
  } from './workflowToolbarEvents';

  const DEFAULT_WORKFLOW_SEMANTIC_VERSION = '0.1.0';
  const WORKFLOW_SEMANTIC_VERSION_STORAGE_KEY_PREFIX = 'pantograph.workflowSemanticVersion.';
  const MAX_WORKFLOW_VERSION_CONFLICT_RETRIES = 25;

  let workflowError = $state<WorkflowServiceError | null>(null);
  let workflowErrorWorkflowId = $state<string | null>(null);
  let workflowSemanticVersion = $state(DEFAULT_WORKFLOW_SEMANTIC_VERSION);
  let previousWorkflowId = $state<string | null>(null);

  let currentSavedWorkflow = $derived(
    $currentGraphType === 'workflow'
      ? $availableWorkflows.find((workflow) => (workflow.id ?? workflow.name) === $currentGraphId)
      : undefined,
  );
  let workflowSemanticVersionInvalid = $derived(
    workflowSemanticVersion.trim().length > 0 &&
      !isNumericWorkflowSemanticVersion(workflowSemanticVersion),
  );
  let submitDisabledReason = $derived(
    workflowSubmitDisabledReason({
      isExecuting: $isExecuting,
      isReadOnly: $isReadOnly,
      isDirty: $isDirty,
      hasSavedWorkflow: Boolean(currentSavedWorkflow),
      hasWorkflowId: Boolean($currentGraphId),
      semanticVersionInvalid: workflowSemanticVersionInvalid,
    }),
  );
  let submitDisabled = $derived(submitDisabledReason !== null);
  let submitTitle = $derived(submitDisabledReason ?? 'Submit workflow to the scheduler');
  let workflowErrorMessage = $derived(workflowError ? formatWorkflowCommandError(workflowError) : null);
  let workflowErrorDiagnostics = $derived(workflowError?.diagnostics ?? null);
  let canOpenWorkflowErrorDiagnostics = $derived(
    Boolean(workflowErrorDiagnostics?.workflow_run_id) &&
      isCurrentWorkflowSubmitFailure({
        submittedWorkflowId: workflowErrorWorkflowId,
        currentGraphId: $currentGraphId,
        currentGraphType: $currentGraphType,
      }),
  );

  function workflowSemanticVersionStorageKey(workflowId: string): string {
    return `${WORKFLOW_SEMANTIC_VERSION_STORAGE_KEY_PREFIX}${workflowId}`;
  }

  function readStoredWorkflowSemanticVersion(workflowId: string): string {
    try {
      const stored = localStorage.getItem(workflowSemanticVersionStorageKey(workflowId));
      return stored && isNumericWorkflowSemanticVersion(stored)
        ? stored
        : DEFAULT_WORKFLOW_SEMANTIC_VERSION;
    } catch {
      return DEFAULT_WORKFLOW_SEMANTIC_VERSION;
    }
  }

  function persistWorkflowSemanticVersion(workflowId: string, version: string): void {
    if (!isNumericWorkflowSemanticVersion(version)) {
      return;
    }

    try {
      localStorage.setItem(workflowSemanticVersionStorageKey(workflowId), version);
    } catch {
      // Ignore storage failures; the explicit field still controls this submit.
    }
  }

  function bumpWorkflowSemanticVersion(): void {
    workflowSemanticVersion = nextWorkflowPatchSemanticVersion(workflowSemanticVersion);
  }

  $effect(() => {
    const workflowId = $currentGraphType === 'workflow' ? $currentGraphId : null;
    if (workflowId === previousWorkflowId) {
      return;
    }

    previousWorkflowId = workflowId;
    workflowSemanticVersion = workflowId
      ? readStoredWorkflowSemanticVersion(workflowId)
      : DEFAULT_WORKFLOW_SEMANTIC_VERSION;
  });

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
    workflowErrorWorkflowId = null;
    isExecuting.set(true);
    clearNodeRuntimeData([...AUDIO_RUNTIME_DATA_KEYS]);
    resetExecutionStates();
    clearStreamContent();
    const submittedWorkflowId = $currentGraphId;

    try {
      if ($isReadOnly) {
        throw new Error('Read-only graphs cannot be submitted');
      }
      if ($isDirty) {
        throw new Error('Save workflow changes before submitting');
      }
      if (!currentSavedWorkflow || !submittedWorkflowId) {
        throw new Error('Save the workflow before submitting');
      }
      if (!isNumericWorkflowSemanticVersion(workflowSemanticVersion)) {
        throw new Error('Workflow version must use numeric major.minor.patch format');
      }

      const executionSession = await workflowService.createWorkflowExecutionSession({
        workflow_id: submittedWorkflowId,
        usage_profile: null,
        keep_alive: false,
      });

      try {
        const runRequestBase = {
          session_id: executionSession.session_id,
          inputs: [],
          output_targets: null,
          override_selection: null,
          timeout_ms: null,
          priority: null,
        };
        let submittedVersion = workflowSemanticVersion;
        let response = null;
        let lastConflictError: WorkflowServiceError | null = null;
        for (let attempt = 0; attempt <= MAX_WORKFLOW_VERSION_CONFLICT_RETRIES; attempt += 1) {
          if (attempt > 0) {
            submittedVersion = nextWorkflowPatchSemanticVersion(submittedVersion);
            workflowSemanticVersion = submittedVersion;
          }
          try {
            response = await workflowService.runWorkflowExecutionSession({
              ...runRequestBase,
              workflow_semantic_version: submittedVersion,
            });
            lastConflictError = null;
            break;
          } catch (runError) {
            const normalizedRunError = normalizeWorkflowServiceError(runError);
            if (!isWorkflowSemanticVersionConflictError(normalizedRunError)) {
              throw normalizedRunError;
            }
            lastConflictError = normalizedRunError;
          }
        }
        if (!response) {
          throw lastConflictError ?? new Error('Workflow submit failed');
        }

        persistWorkflowSemanticVersion(submittedWorkflowId, submittedVersion);
        selectActiveWorkflowRun({
          workflow_run_id: response.workflow_run_id,
          workflow_id: submittedWorkflowId,
          workflow_semantic_version: submittedVersion,
          status: 'completed',
        });
        setWorkbenchPage('scheduler');
      } finally {
        await closeExecutionSession(executionSession.session_id);
      }
    } catch (error) {
      console.error('Workflow submission failed:', error);
      workflowError = normalizeWorkflowServiceError(error);
      workflowErrorWorkflowId = submittedWorkflowId;
    } finally {
      isExecuting.set(false);
    }
  }

  function openWorkflowErrorDiagnostics(): void {
    if (!workflowErrorDiagnostics?.workflow_run_id || !canOpenWorkflowErrorDiagnostics) {
      return;
    }
    focusWorkflowDiagnostics(
      {
        workflow_run_id: workflowErrorDiagnostics.workflow_run_id,
        workflow_id: $currentGraphId,
        workflow_semantic_version: workflowSemanticVersion,
        status: 'failed',
      },
      {
        diagnostic_event_id: workflowErrorDiagnostics.diagnostic_event_id ?? null,
      },
    );
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
      <label class="flex items-center gap-2 text-xs text-neutral-400">
        Version
        <input
          class="h-8 w-24 rounded border bg-neutral-950 px-2 font-mono text-xs text-neutral-200 outline-none transition-colors focus:border-cyan-500 {workflowSemanticVersionInvalid ? 'border-red-600' : 'border-neutral-700'}"
          bind:value={workflowSemanticVersion}
          disabled={$isExecuting}
          title="Workflow semantic version for run attribution"
          inputmode="numeric"
          aria-invalid={workflowSemanticVersionInvalid}
        />
      </label>
      <button
        type="button"
        class="h-8 rounded border border-neutral-700 px-2 text-xs text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
        onclick={bumpWorkflowSemanticVersion}
        disabled={$isExecuting}
        title="Increment workflow patch version"
      >
        +patch
      </button>
      <button type="button"
        class="px-4 py-1.5 text-sm rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        class:bg-green-600={!$isExecuting}
        class:hover:bg-green-500={!$isExecuting}
        class:bg-amber-600={$isExecuting}
        class:text-white={true}
        onclick={handleSubmit}
        disabled={submitDisabled}
        title={submitTitle}
        aria-describedby={submitDisabledReason ? 'workflow-submit-disabled-reason' : undefined}
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

  {#if submitDisabledReason && !$isExecuting}
    <div
      id="workflow-submit-disabled-reason"
      class="border-b border-amber-900 bg-amber-950/40 px-4 py-2 text-xs text-amber-200"
    >
      Submit unavailable: {submitDisabledReason}
    </div>
  {/if}

  {#if workflowErrorMessage}
    <div class="flex items-center justify-between gap-3 border-b border-red-700 bg-red-900/70 px-4 py-2 text-xs text-red-200">
      <div class="min-w-0 truncate" title={workflowErrorMessage}>
        Workflow submit failed: {workflowErrorMessage}
      </div>
      {#if canOpenWorkflowErrorDiagnostics}
        <button
          type="button"
          class="inline-flex shrink-0 items-center gap-1 rounded border border-red-500 px-2 py-1 text-red-100 transition-colors hover:border-red-300 hover:text-white focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
          onclick={openWorkflowErrorDiagnostics}
          aria-label="Open workflow diagnostics for this submit error"
        >
          <AlertTriangle size={13} aria-hidden="true" />
          Diagnostics
        </button>
      {/if}
    </div>
  {/if}
</div>

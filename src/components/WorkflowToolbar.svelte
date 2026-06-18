<script lang="ts">
  import { AlertTriangle, Loader2, Send } from 'lucide-svelte';
  import {
    isDirty,
    isExecuting,
    resetExecutionStates,
    clearNodeRuntimeData,
    clearStreamContent,
    edges,
    workflowGraph,
    setNodeExecutionState,
    updateNodeRuntimeData,
    appendStreamContent,
    setStreamContent,
  } from '../stores/workflowStore';
  import {
    availableWorkflows,
    currentGraphId,
    currentGraphType,
    currentSessionId,
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
  import type { WorkflowGraphCurrentValidationSummaryResponse } from '../services/workflow/types';
  import { subscribeGraphValidationLifecycleEvents } from '../services/workflow/WorkflowGraphValidationLifecycleSubscriptionService';
  import {
    focusWorkflowDiagnostics,
    selectActiveWorkflowRun,
    setWorkbenchPage,
  } from '../stores/workbenchStore';
  import { formatWorkflowCommandError } from './workbench/workflowErrorPresenters';
  import WorkflowPersistenceControls from './WorkflowPersistenceControls.svelte';
  import {
    applyWorkflowToolbarEvent,
    isCurrentWorkflowSubmitFailure,
    isNumericWorkflowSemanticVersion,
    isWorkflowSemanticVersionConflictError,
    nextWorkflowPatchSemanticVersion,
    shouldRefreshValidationFromLifecycleEvent,
    workflowSubmitSuccessWorkbenchPage,
    workflowSubmitDisabledReason,
    workflowValidationRefreshKey,
  } from './workflowToolbarEvents';
  import {
    INFERENCE_INTERFACE_VALIDATION_RUNTIME_KEYS,
    workflowValidationProjectionOverlays,
  } from './workflowValidationProjectionOverlays';

  const DEFAULT_WORKFLOW_SEMANTIC_VERSION = '0.1.0';
  const WORKFLOW_SEMANTIC_VERSION_STORAGE_KEY_PREFIX = 'pantograph.workflowSemanticVersion.';
  const MAX_WORKFLOW_VERSION_CONFLICT_RETRIES = 25;

  let workflowError = $state<WorkflowServiceError | null>(null);
  let workflowErrorWorkflowId = $state<string | null>(null);
  let workflowSemanticVersion = $state(DEFAULT_WORKFLOW_SEMANTIC_VERSION);
  let previousWorkflowId = $state<string | null>(null);
  let activeWorkflowRunId = $state<string | null>(null);
  let waitingForInput = $state(false);
  let currentValidationSummary = $state<WorkflowGraphCurrentValidationSummaryResponse | null>(null);
  let currentValidationSummaryKey = $state<string | null>(null);

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
      submitGate: currentValidationSummary?.submit_gate ?? null,
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

  $effect(() => {
    const graphSessionId = $currentSessionId;
    const graphRevision = $workflowGraph.derived_graph?.graph_fingerprint ?? null;
    const requestKey = workflowValidationRefreshKey({
      currentGraphType: $currentGraphType,
      graphSessionId,
      graphRevision,
    });
    if (!requestKey) {
      currentValidationSummary = null;
      currentValidationSummaryKey = null;
      clearNodeRuntimeData([...INFERENCE_INTERFACE_VALIDATION_RUNTIME_KEYS]);
      return;
    }

    if (currentValidationSummaryKey !== requestKey) {
      clearNodeRuntimeData([...INFERENCE_INTERFACE_VALIDATION_RUNTIME_KEYS]);
    }
    currentValidationSummaryKey = requestKey;
    let cancelled = false;

    void workflowService
      .refreshCurrentGraphValidationSummary({
        graph_session_id: graphSessionId,
        graph_revision: graphRevision,
      })
      .then((refresh) => {
        if (!cancelled && currentValidationSummaryKey === requestKey) {
          currentValidationSummary = refresh.summary;
          for (const overlay of workflowValidationProjectionOverlays(
            refresh.node_projections ?? [],
          )) {
            updateNodeRuntimeData(overlay.nodeId, overlay.data);
          }
        }
      })
      .catch(() => {
        if (!cancelled && currentValidationSummaryKey === requestKey) {
          currentValidationSummary = null;
        }
      });

    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    return workflowService.subscribeEvents((event) => {
      const result = applyWorkflowToolbarEvent({
        event,
        activeWorkflowRunId,
        waitingForInput,
        edges: $edges,
        workflow: {
          setNodeExecutionState,
          updateNodeRuntimeData,
          appendStreamContent,
          setStreamContent,
        },
      });
      activeWorkflowRunId = result.activeWorkflowRunId;
      waitingForInput = result.waitingForInput;
    });
  });

  $effect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    void subscribeGraphValidationLifecycleEvents({
      getActiveGraphSessionId: () => $currentSessionId,
      handleEvent: async (event) => {
        const graphSessionId = $currentSessionId;
        const graphRevision = $workflowGraph.derived_graph?.graph_fingerprint ?? null;
        if (!graphSessionId || !graphRevision) {
          return;
        }
        if (!shouldRefreshValidationFromLifecycleEvent({
          event,
          currentGraphType: $currentGraphType,
          graphSessionId,
          graphRevision,
          currentValidationSummaryKey,
        })) {
          return;
        }
        const projection = await workflowService.currentGraphValidationProjection({
          graph_session_id: graphSessionId,
          graph_revision: graphRevision,
        });
        if (!cancelled && currentValidationSummaryKey === `${graphSessionId}:${graphRevision}`) {
          currentValidationSummary = projection.summary;
          for (const overlay of workflowValidationProjectionOverlays(
            projection.node_projections ?? [],
          )) {
            updateNodeRuntimeData(overlay.nodeId, overlay.data);
          }
        }
      },
    }).then((nextUnlisten) => {
      if (cancelled) {
        nextUnlisten();
      } else {
        unlisten = nextUnlisten;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
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
    activeWorkflowRunId = null;
    waitingForInput = false;
    clearNodeRuntimeData([...AUDIO_RUNTIME_DATA_KEYS]);
    resetExecutionStates();
    clearStreamContent();
    const submittedWorkflowId = $currentGraphId;
    const submittedGraphSessionId = $currentSessionId;
    const submittedValidationSummary = currentValidationSummary;

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
      if (!submittedGraphSessionId) {
        throw new Error('No active workflow graph session');
      }
      if (!submittedValidationSummary?.submit_gate.allowed) {
        throw new Error(
          submittedValidationSummary?.submit_gate.message ??
            'Workflow validation summary unavailable',
        );
      }
      if (!submittedValidationSummary.validation_session_id) {
        throw new Error('Workflow validation session is unavailable');
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
            await workflowService.publishGraphSessionExecutableValidationSnapshot({
              workflow_id: submittedWorkflowId,
              workflow_semantic_version: submittedVersion,
              graph_session_id: submittedGraphSessionId,
              validation_session_id: submittedValidationSummary.validation_session_id,
              validation_snapshot_id: null,
            });
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
        setWorkbenchPage(workflowSubmitSuccessWorkbenchPage(response));
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

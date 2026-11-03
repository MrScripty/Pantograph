<script lang="ts">
  import type { Snippet } from 'svelte';
  import {
    isDirty,
    isExecuting,
    resetExecutionStates,
    clearNodeRuntimeData,
    clearStreamContent,
    edges,
    nodes,
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
    graphSessionError,
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
  import {
    formatEditCurrentLabel,
    formatEditDimensionLabel,
    formatWorkflowSaveLine,
    type GraphPageMode,
  } from './workbench/graphInspectionPresenters';
  import Chip from './workbench/ui/Chip.svelte';
  import StatusGlyph from './workbench/ui/StatusGlyph.svelte';
  import type { GlyphState } from './workbench/ui/statusGlyph';
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

  /** The selected run as the toolbar shows it in run mode. */
  interface ToolbarRun {
    runId: string;
    workflowId: string | null;
    version: string | null;
    glyph: GlyphState;
  }

  interface Props {
    mode: GraphPageMode;
    run: ToolbarRun | null;
    /** Dimension line text in run mode; edit mode builds its own from the editor. */
    runDimLabel: string;
    modeSwitch: Snippet;
    panelToggles: Snippet;
    /** A workflow was picked in the selector: the page switches to editing it. */
    onworkflowpicked: () => void;
    /** "Edit current vX" in run mode. */
    oneditcurrent: () => void;
  }

  let { mode, run, runDimLabel, modeSwitch, panelToggles, onworkflowpicked, oneditcurrent }: Props =
    $props();

  const DEFAULT_WORKFLOW_SEMANTIC_VERSION = '0.1.0';
  const SAVE_AS_NEW_UNAVAILABLE =
    'Creating a workflow from a run snapshot is not available yet: no service saves a run graph as a new workflow.';
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
  let persistenceError = $state<string | null>(null);

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
  let loadedWorkflowId = $derived($currentGraphType === 'workflow' ? $currentGraphId : null);
  let saveLine = $derived(
    formatWorkflowSaveLine({
      isDirty: $isDirty,
      savedModifiedAt: currentSavedWorkflow?.modified ?? null,
    }),
  );
  let dimLabel = $derived(
    mode === 'run'
      ? runDimLabel
      : formatEditDimensionLabel(
          $isReadOnly ? null : workflowSemanticVersion,
          $nodes.length,
          $edges.length,
        ),
  );
  let editCurrentVersion = $derived(
    run?.workflowId
      ? run.workflowId === loadedWorkflowId
        ? workflowSemanticVersion
        : readStoredWorkflowSemanticVersionOrNull(run.workflowId)
      : null,
  );
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

  function readStoredWorkflowSemanticVersionOrNull(workflowId: string): string | null {
    try {
      const stored = localStorage.getItem(workflowSemanticVersionStorageKey(workflowId));
      return stored && isNumericWorkflowSemanticVersion(stored) ? stored : null;
    } catch {
      return null;
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

<div class="wt">
  <div class="wt-row">
    {@render modeSwitch()}

    <WorkflowPersistenceControls
      {mode}
      editVersion={workflowSemanticVersion}
      runWorkflow={run?.workflowId ? { workflowId: run.workflowId, version: run.version } : null}
      {onworkflowpicked}
      onactionerror={(message) => (persistenceError = message)}
    >
      {#snippet middle()}
        {#if mode === 'edit'}
          {#if $isReadOnly}
            <Chip title="System graphs cannot be changed or submitted">read-only system graph</Chip>
          {:else}
            <span class="wt-save" class:wt-dirty={$isDirty} role="status">{saveLine}</span>
          {/if}
        {:else}
          <span class="wt-ro">
            {#if run}<StatusGlyph state={run.glyph} />{/if}
            read-only, as it ran
          </span>
        {/if}
        <div class="pg-dim wt-dim" title={dimLabel}>
          <span class="pg-dim-label" title={dimLabel}>{dimLabel}</span>
        </div>
        {@render panelToggles()}
        <span class="wt-vdiv" aria-hidden="true"></span>
      {/snippet}
    </WorkflowPersistenceControls>

    {#if mode === 'edit'}
      <label class="wt-version">
        Version
        <input
          data-testid="workflow-semantic-version-input"
          class="pg-field pg-mono wt-version-input"
          class:wt-invalid={workflowSemanticVersionInvalid}
          bind:value={workflowSemanticVersion}
          disabled={$isExecuting}
          title="Workflow semantic version for run attribution"
          inputmode="numeric"
          aria-invalid={workflowSemanticVersionInvalid}
        />
      </label>
      <button
        type="button"
        class="pg-tgl wt-patch"
        onclick={bumpWorkflowSemanticVersion}
        disabled={$isExecuting}
        title="Increment workflow patch version"
      >
        +patch
      </button>
      <button
        type="button"
        data-testid="workflow-submit-button"
        class="pg-btn pg-btn-primary"
        onclick={handleSubmit}
        disabled={submitDisabled}
        title={submitTitle}
        aria-describedby={submitDisabledReason ? 'workflow-submit-disabled-reason' : undefined}
      >
        {$isExecuting ? 'Submitting…' : 'Submit run'}
      </button>
    {:else}
      <button
        type="button"
        class="pg-btn"
        onclick={oneditcurrent}
        title={run?.workflowId
          ? `Opens the saved ${run.workflowId} for editing`
          : 'This run names no saved workflow'}
        disabled={!run?.workflowId}
      >
        {formatEditCurrentLabel(editCurrentVersion)}
      </button>
      <button
        type="button"
        class="pg-btn"
        aria-disabled="true"
        title={SAVE_AS_NEW_UNAVAILABLE}
      >
        Save as new workflow
      </button>
    {/if}
  </div>

  {#if mode === 'edit' && submitDisabledReason && !$isExecuting}
    <p
      id="workflow-submit-disabled-reason"
      data-testid="workflow-submit-disabled-reason"
      class="wt-note"
    >
      Submit unavailable: {submitDisabledReason}
    </p>
  {/if}

  {#if mode === 'edit' && workflowErrorMessage}
    <div class="wt-note wt-error">
      <StatusGlyph state="failed" />
      <span class="wt-error-text" title={workflowErrorMessage} data-testid="workflow-submit-error">
        Workflow submit failed: {workflowErrorMessage}
      </span>
      {#if canOpenWorkflowErrorDiagnostics}
        <button
          type="button"
          class="pg-btn pg-btn-sm"
          onclick={openWorkflowErrorDiagnostics}
          aria-label="Open workflow diagnostics for this submit error"
        >
          Diagnostics
        </button>
      {/if}
    </div>
  {/if}

  {#if persistenceError}
    <div class="wt-note wt-error">
      <StatusGlyph state="failed" />
      <span class="wt-error-text" title={persistenceError}>{persistenceError}</span>
    </div>
  {/if}

  {#if $graphSessionError && $graphSessionError !== persistenceError}
    <div class="wt-note wt-error" data-testid="graph-session-error">
      <StatusGlyph state="failed" />
      <span class="wt-error-text" title={$graphSessionError}>{$graphSessionError}</span>
    </div>
  {/if}
</div>

<style>
  .wt {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: none;
  }

  .wt-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 6px 0 2px;
  }

  .wt-save,
  .wt-ro {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--pg-text-muted);
    white-space: nowrap;
  }

  .wt-save.wt-dirty {
    color: var(--pg-text);
  }

  /* The dimension line gives way first when the Assistant panel narrows the page. */
  .wt-dim {
    flex: 1 1 32px;
    min-width: 32px;
    container-type: inline-size;
  }

  /* Too narrow for a readable summary: keep the rule, drop the words (still in the title). */
  @container (max-width: 140px) {
    .wt-dim :global(.pg-dim-label) {
      display: none;
    }
  }

  .wt-vdiv {
    width: 1px;
    height: 18px;
    margin: 0 4px;
    background: var(--pg-border);
    flex: none;
  }

  .wt-version {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--pg-text-muted);
  }

  .wt-version-input {
    width: 76px;
    font-size: 12px;
  }

  .wt-version-input.wt-invalid {
    border-color: var(--pg-danger);
  }

  .wt-patch {
    height: 30px;
  }

  .wt-note {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 12.5px;
    color: var(--pg-text-muted);
    min-width: 0;
  }

  .wt-error {
    color: var(--pg-text);
  }

  .wt-error-text {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>

<script lang="ts">
  import {
    workflowGraph,
    isDirty,
    isExecuting,
    setNodeExecutionState,
    resetExecutionStates,
    clearWorkflow,
    edges,
    updateNodeRuntimeData,
    clearNodeRuntimeData,
    appendStreamContent,
    setStreamContent,
    clearStreamContent,
  } from '../stores/workflowStore';
  import {
    isReadOnly,
    currentGraphId,
    currentGraphName,
    currentSessionId,
    createNewWorkflow,
    saveLastGraph,
    refreshWorkflowList,
  } from '../stores/graphSessionStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import type { WorkflowEvent } from '../services/workflow/types';
  import {
    AUDIO_RUNTIME_DATA_KEYS,
  } from './nodes/workflow/audioOutputState';
  import { applyWorkflowToolbarEvent } from './workflowToolbarEvents.ts';
  import { get } from 'svelte/store';
  import GraphSelector from './GraphSelector.svelte';
  import { diagnosticsSnapshot, toggleDiagnosticsPanel } from '../stores/diagnosticsStore';
  import { formatDiagnosticsDuration, getDiagnosticsStatusClasses } from './diagnostics/presenters';

  let workflowName = $derived($currentGraphName || 'Untitled Workflow');
  let workflowError = $state<string | null>(null);
  let selectedDiagnosticsRun = $derived($diagnosticsSnapshot.selectedRun);
  let diagnosticsPanelOpen = $derived($diagnosticsSnapshot.state.panelOpen);

  // Store unsubscribe function at module scope so event handler can access it
  let currentUnsubscribe: (() => void) | null = null;
  let activeExecutionId: string | null = null;
  let waitingForInput = $state(false);

  function normalizeError(error: unknown): string {
    if (error instanceof Error && error.message.trim().length > 0) {
      return error.message;
    }
    if (typeof error === 'string' && error.trim().length > 0) {
      return error;
    }
    return String(error);
  }

  async function handleRun() {
    if ($isExecuting) return;

    workflowError = null;
    isExecuting.set(true);
    clearNodeRuntimeData([...AUDIO_RUNTIME_DATA_KEYS]);
    resetExecutionStates();
    clearStreamContent();
    activeExecutionId = null;
    waitingForInput = false;

    // Subscribe to events - will be cleaned up in handleWorkflowEvent on completion/failure
    currentUnsubscribe = workflowService.subscribeEvents(handleWorkflowEvent);

    try {
      if ($currentSessionId) {
        await workflowService.runSession($currentSessionId);
      } else {
        await workflowService.executeWorkflow($workflowGraph);
      }
      // Don't unsubscribe here - wait for Completed/Failed events
    } catch (error) {
      console.error('Workflow execution failed:', error);
      workflowError = normalizeError(error);
      // Only cleanup on synchronous errors (e.g., invoke failed)
      isExecuting.set(false);
      if (currentUnsubscribe) {
        currentUnsubscribe();
        currentUnsubscribe = null;
      }
      activeExecutionId = null;
      waitingForInput = false;
    }
  }

  function cleanupExecution() {
    isExecuting.set(false);
    if (currentUnsubscribe) {
      currentUnsubscribe();
      currentUnsubscribe = null;
    }
    activeExecutionId = null;
    waitingForInput = false;
  }

  function handleWorkflowEvent(event: WorkflowEvent) {
    const result = applyWorkflowToolbarEvent({
      event,
      activeExecutionId,
      waitingForInput,
      edges: get(edges),
      workflow: {
        setNodeExecutionState,
        updateNodeRuntimeData,
        appendStreamContent,
        setStreamContent,
      },
    });

    activeExecutionId = result.activeExecutionId;
    waitingForInput = result.waitingForInput;

    if (!result.handled) {
      return;
    }

    console.log('Workflow event:', event.type, event.data);

    switch (event.type) {
      case 'NodeError': {
        const errorData = event.data as { node_id: string; error: string };
        console.error(`Node ${errorData.node_id} failed:`, errorData.error);
        break;
      }
      case 'Completed':
        console.log('Workflow completed successfully');
        workflowError = null;
        break;
      case 'Failed': {
        const failedData = event.data as { error: string };
        console.error('Workflow failed:', failedData.error);
        workflowError = failedData.error;
        break;
      }
      case 'Cancelled': {
        const cancelledData = event.data as { error: string };
        console.warn('Workflow cancelled:', cancelledData.error);
        workflowError = null;
        break;
      }
    }

    if (result.shouldCleanup) {
      cleanupExecution();
    }
  }

  async function handleSave() {
    const name = prompt('Workflow name:', workflowName);
    if (!name) return;

    try {
      await workflowService.saveWorkflow(name, $workflowGraph);
      isDirty.set(false);

      // Update frontend state to match saved workflow
      currentGraphId.set(name);
      currentGraphName.set(name);
      saveLastGraph(name, 'workflow');
      await refreshWorkflowList();
    } catch (error) {
      console.error('Failed to save workflow:', error);
    }
  }

  function handleNew() {
    if ($isReadOnly) return;
    if ($isDirty && !confirm('Discard unsaved changes?')) return;
    createNewWorkflow();
  }

  function handleClear() {
    if ($isReadOnly) return;
    if (!confirm('Clear all nodes?')) return;
    clearWorkflow();
  }
</script>

<div>
  <div class="workflow-toolbar h-12 px-4 bg-neutral-900 border-b border-neutral-700 flex items-center justify-between">
    <div class="flex items-center gap-3">
      <GraphSelector />

      <div class="h-6 w-px bg-neutral-700"></div>

      <div class="flex items-center gap-2">
        <button type="button"
          class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors"
          class:hover:bg-neutral-700={!$isReadOnly}
          class:opacity-50={$isReadOnly}
          class:cursor-not-allowed={$isReadOnly}
          onclick={handleNew}
          disabled={$isReadOnly}
          title={$isReadOnly ? 'Cannot create new in read-only mode' : 'New Workflow'}
        >
          [+] New
        </button>
        <button type="button"
          class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors"
          class:hover:bg-neutral-700={!$isReadOnly}
          class:opacity-50={$isReadOnly}
          class:cursor-not-allowed={$isReadOnly}
          onclick={handleSave}
          disabled={$isReadOnly}
          title={$isReadOnly ? 'Cannot save read-only graph' : 'Save Workflow'}
        >
          [S] Save
        </button>
        <button type="button"
          class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors hover:bg-red-900"
          class:opacity-50={$isReadOnly}
          class:cursor-not-allowed={$isReadOnly}
          class:hover:bg-transparent={$isReadOnly}
          onclick={handleClear}
          disabled={$isReadOnly}
          title={$isReadOnly ? 'Cannot clear read-only graph' : 'Clear All'}
        >
          [X] Clear
        </button>
      </div>
    </div>

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
        class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors hover:bg-neutral-700"
        class:border-cyan-700={diagnosticsPanelOpen}
        class:text-cyan-200={diagnosticsPanelOpen}
        onclick={toggleDiagnosticsPanel}
        title="Toggle workflow diagnostics panel"
      >
        [::] Diagnostics
      </button>

      {#if selectedDiagnosticsRun}
        <div class="hidden xl:flex items-center gap-2 rounded border border-neutral-800 bg-neutral-950/70 px-3 py-1.5 text-xs text-neutral-400">
          <span class={`inline-flex rounded-full border px-2 py-0.5 font-medium ${getDiagnosticsStatusClasses(selectedDiagnosticsRun.status)}`}>
            {selectedDiagnosticsRun.status}
          </span>
          <span>{selectedDiagnosticsRun.eventCount} events</span>
          <span>{formatDiagnosticsDuration(selectedDiagnosticsRun.durationMs)}</span>
        </div>
      {/if}

      <button type="button"
        class="px-4 py-1.5 text-sm rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        class:bg-green-600={!$isExecuting}
        class:hover:bg-green-500={!$isExecuting}
        class:bg-amber-600={$isExecuting}
        class:text-white={true}
        onclick={handleRun}
        disabled={$isExecuting}
      >
        {#if $isExecuting}
          {#if waitingForInput}
            [?] Waiting...
          {:else}
            [||] Running...
          {/if}
        {:else}
          [>] Run
        {/if}
      </button>
    </div>
  </div>

  {#if workflowError}
    <div class="px-4 py-2 bg-red-900/70 border-b border-red-700 text-red-200 text-xs truncate" title={workflowError}>
      Workflow failed: {workflowError}
    </div>
  {/if}
</div>

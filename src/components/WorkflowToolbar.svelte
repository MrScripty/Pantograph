<script lang="ts">
  import { CircleHelp, Loader2, Play } from 'lucide-svelte';
  import {
    isDirty,
    isExecuting,
    setNodeExecutionState,
    resetExecutionStates,
    edges,
    updateNodeRuntimeData,
    clearNodeRuntimeData,
    appendStreamContent,
    setStreamContent,
    clearStreamContent,
  } from '../stores/workflowStore';
  import {
    isReadOnly,
    currentSessionId,
  } from '../stores/graphSessionStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import type { WorkflowEvent } from '../services/workflow/types';
  import {
    AUDIO_RUNTIME_DATA_KEYS,
  } from './nodes/workflow/audioOutputState';
  import { applyWorkflowToolbarEvent } from './workflowToolbarEvents.ts';
  import { get } from 'svelte/store';
  import WorkflowPersistenceControls from './WorkflowPersistenceControls.svelte';

  let workflowError = $state<string | null>(null);

  // Store unsubscribe function at module scope so event handler can access it
  let currentUnsubscribe: (() => void) | null = null;
  let activeWorkflowRunId: string | null = null;
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
    activeWorkflowRunId = null;
    waitingForInput = false;

    // Subscribe to events - will be cleaned up in handleWorkflowEvent on completion/failure
    currentUnsubscribe = workflowService.subscribeEvents(handleWorkflowEvent);

    try {
      if (!$currentSessionId) {
        throw new Error('No active workflow session');
      }
      await workflowService.runSession($currentSessionId);
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
      activeWorkflowRunId = null;
      waitingForInput = false;
    }
  }

  function cleanupExecution() {
    isExecuting.set(false);
    if (currentUnsubscribe) {
      currentUnsubscribe();
      currentUnsubscribe = null;
    }
    activeWorkflowRunId = null;
    waitingForInput = false;
  }

  function handleWorkflowEvent(event: WorkflowEvent) {
    const result = applyWorkflowToolbarEvent({
      event,
      activeWorkflowRunId,
      waitingForInput,
      edges: get(edges),
      workflow: {
        setNodeExecutionState,
        updateNodeRuntimeData,
        appendStreamContent,
        setStreamContent,
      },
    });

    activeWorkflowRunId = result.activeWorkflowRunId;
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
        onclick={handleRun}
        disabled={$isExecuting}
      >
        {#if $isExecuting}
          {#if waitingForInput}
            <CircleHelp size={14} aria-hidden="true" class="inline-block align-[-2px] mr-1" />
            Waiting...
          {:else}
            <Loader2 size={14} aria-hidden="true" class="inline-block align-[-2px] mr-1" />
            Running...
          {/if}
        {:else}
          <Play size={14} aria-hidden="true" class="inline-block align-[-2px] mr-1" />
          Run
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

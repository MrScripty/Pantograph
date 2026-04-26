<script lang="ts">
  import { get } from 'svelte/store';
  import { useGraphContext } from '../context/useGraphContext.js';
  import type { WorkflowEvent } from '../types/workflow.js';
  import { applyWorkflowExecutionEvent } from '../stores/workflowExecutionEvents.js';

  const { backend, stores } = useGraphContext();
  const { isExecuting, edges: edgesStore } = stores.workflow;
  const { currentSessionId } = stores.session;

  let currentUnsubscribe: (() => void) | null = null;
  let activeWorkflowRunId: string | null = null;
  let waitingForInput = $state(false);

  async function handleRun() {
    if ($isExecuting) return;

    isExecuting.set(true);
    stores.workflow.resetExecutionStates();
    activeWorkflowRunId = null;
    waitingForInput = false;

    currentUnsubscribe = backend.subscribeEvents(handleWorkflowEvent);

    try {
      if (!$currentSessionId) {
        throw new Error('No active workflow session');
      }
      await backend.runSession($currentSessionId);
    } catch (error) {
      console.error('Workflow execution failed:', error);
      cleanupExecution();
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
    const result = applyWorkflowExecutionEvent({
      event,
      activeWorkflowRunId,
      waitingForInput,
      edges: get(edgesStore),
      workflow: stores.workflow,
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
        break;
      case 'Failed': {
        const failedData = event.data as { error: string };
        console.error('Workflow failed:', failedData.error);
        break;
      }
      case 'Cancelled': {
        const cancelledData = event.data as { error: string };
        console.warn('Workflow cancelled:', cancelledData.error);
        break;
      }
    }

    if (result.shouldCleanup) {
      cleanupExecution();
    }
  }
</script>

<button type="button"
  class="run-btn"
  data-executing={$isExecuting || undefined}
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

<style>
  .run-btn {
    padding: 0.375rem 1rem;
    font-size: 0.875rem;
    border-radius: 0.25rem;
    border: none;
    color: white;
    cursor: pointer;
    transition: background-color 150ms;
    background-color: #16a34a;
  }

  .run-btn:not(:disabled):hover {
    background-color: #22c55e;
  }

  .run-btn[data-executing] {
    background-color: #d97706;
  }

  .run-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
